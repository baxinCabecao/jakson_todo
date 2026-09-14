use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::fs;

pub struct DbConnection {
    pub path: PathBuf,
}

impl DbConnection {
    /// Initialize the connection helper with the path to the SQLite file
    pub fn new(app_dir: PathBuf) -> Self {
        // Ensure the directory exists
        if !app_dir.exists() {
            let _ = fs::create_dir_all(&app_dir);
        }
        let db_path = app_dir.join("todo.db");

        // Migration helper: If new db doesn't exist yet, check if old package directory exists and migrate
        if !db_path.exists() {
            if let Some(parent) = app_dir.parent() {
                let legacy_dirs = [
                    parent.join("com.f4613569.tauri-app").join("todo.db"),
                    parent.join("tauri-app").join("todo.db"),
                ];
                for legacy_db in &legacy_dirs {
                    if legacy_db.exists() {
                        let _ = fs::copy(legacy_db, &db_path);
                        break;
                    }
                }
            }
        }

        // Automatic self-healing: If db_path exists, ensure it is not compressed (gzip)
        Self::ensure_uncompressed(&db_path);

        DbConnection { path: db_path }
    }

    /// Helper to ensure the database file on disk is decompressed if it was stored as gzip
    pub fn ensure_uncompressed(db_path: &PathBuf) {
        if db_path.exists() {
            if let Ok(bytes) = fs::read(db_path) {
                if crate::backup::compression::is_gzipped(&bytes) {
                    eprintln!("[DbConnection] Detectado arquivo de banco de dados comprimido (gzip). Descomprimindo automaticamente...");
                    if let Ok(decompressed) = crate::backup::compression::decompress_db_if_needed(&bytes) {
                        let _ = fs::write(db_path, decompressed);
                    }
                }
            }
        }
    }

    /// Open a connection to the SQLite database with WAL mode, busy timeout, and corruption recovery
    pub fn get_conn(&self) -> Result<Connection> {
        Self::ensure_uncompressed(&self.path);

        let conn = match Connection::open(&self.path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[DbConnection] Erro ao abrir banco de dados ({:?})", e);
                return Err(e);
            }
        };

        // Configure 5000ms busy timeout to prevent SQLITE_BUSY under concurrent access
        let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
        // Configure WAL mode for concurrent readers and writer
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");

        // Verify that it is actually a valid SQLite database
        if conn.query_row("PRAGMA schema_version", [], |_| Ok(())).is_err() {
            eprintln!("[DbConnection] Banco de dados corrompido ou inválido. Tentando restaurar de backup...");
            let bak_path = self.path.with_extension("db.bak");
            if bak_path.exists() {
                let _ = fs::copy(&bak_path, &self.path);
                let restored_conn = Connection::open(&self.path)?;
                let _ = restored_conn.busy_timeout(std::time::Duration::from_millis(5000));
                let _ = restored_conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");
                return Ok(restored_conn);
            }
        }

        Ok(conn)
    }

    /// Check if a column exists in a specific SQLite table
    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let pragma_sql = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&pragma_sql)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name.eq_ignore_ascii_case(column) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Automatic schema migration to ensure UUIDs, timestamps, and tombstones exist
    fn migrate_schema(conn: &Connection) -> Result<()> {
        // 1. Tasks table columns
        if !Self::column_exists(conn, "tasks", "uuid")? {
            conn.execute("ALTER TABLE tasks ADD COLUMN uuid TEXT", [])?;
        }
        if !Self::column_exists(conn, "tasks", "updated_at")? {
            conn.execute("ALTER TABLE tasks ADD COLUMN updated_at TEXT", [])?;
        }
        if !Self::column_exists(conn, "tasks", "is_deleted")? {
            conn.execute("ALTER TABLE tasks ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0", [])?;
        }
        if !Self::column_exists(conn, "tasks", "deleted_at")? {
            conn.execute("ALTER TABLE tasks ADD COLUMN deleted_at TEXT", [])?;
        }

        // 2. Subtasks table columns
        if !Self::column_exists(conn, "subtasks", "uuid")? {
            conn.execute("ALTER TABLE subtasks ADD COLUMN uuid TEXT", [])?;
        }
        if !Self::column_exists(conn, "subtasks", "updated_at")? {
            conn.execute("ALTER TABLE subtasks ADD COLUMN updated_at TEXT", [])?;
        }

        // 3. Notes table columns
        if !Self::column_exists(conn, "notes", "uuid")? {
            conn.execute("ALTER TABLE notes ADD COLUMN uuid TEXT", [])?;
        }
        if !Self::column_exists(conn, "notes", "is_deleted")? {
            conn.execute("ALTER TABLE notes ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0", [])?;
        }
        if !Self::column_exists(conn, "notes", "deleted_at")? {
            conn.execute("ALTER TABLE notes ADD COLUMN deleted_at TEXT", [])?;
        }

        // Populate missing UUIDs for tasks
        {
            let mut stmt = conn.prepare("SELECT id FROM tasks WHERE uuid IS NULL OR uuid = ''")?;
            let ids: Vec<i64> = stmt.query_map([], |row| row.get(0))?.filter_map(Result::ok).collect();
            for id in ids {
                let u = uuid::Uuid::new_v4().to_string();
                conn.execute("UPDATE tasks SET uuid = ?1 WHERE id = ?2", params![u, id])?;
            }
        }

        // Populate missing updated_at for tasks
        conn.execute(
            "UPDATE tasks SET updated_at = created_at WHERE updated_at IS NULL OR updated_at = ''",
            [],
        )?;

        // Populate missing UUIDs for subtasks
        {
            let mut stmt = conn.prepare("SELECT id, created_at FROM subtasks WHERE uuid IS NULL OR uuid = ''")?;
            let sub_data: Vec<(i64, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(Result::ok)
                .collect();
            for (id, created) in sub_data {
                let u = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "UPDATE subtasks SET uuid = ?1, updated_at = COALESCE(updated_at, ?2) WHERE id = ?3",
                    params![u, created, id],
                )?;
            }
        }

        // Populate missing UUIDs for notes
        {
            let mut stmt = conn.prepare("SELECT id FROM notes WHERE uuid IS NULL OR uuid = ''")?;
            let ids: Vec<i64> = stmt.query_map([], |row| row.get(0))?.filter_map(Result::ok).collect();
            for id in ids {
                let u = uuid::Uuid::new_v4().to_string();
                conn.execute("UPDATE notes SET uuid = ?1 WHERE id = ?2", params![u, id])?;
            }
        }

        // Create indexes for fast UUID lookup
        let _ = conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_uuid ON tasks(uuid)", []);
        let _ = conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_subtasks_uuid ON subtasks(uuid)", []);
        let _ = conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_notes_uuid ON notes(uuid)", []);

        Ok(())
    }

    /// Create tables if they do not exist
    pub fn init_db(&self) -> Result<()> {
        let conn = self.get_conn()?;

        // Create tasks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                due_date TEXT,
                priority TEXT NOT NULL CHECK(priority IN ('high', 'medium', 'low')),
                status TEXT NOT NULL CHECK(status IN ('todo', 'in_progress', 'completed')),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT
            )",
            [],
        )?;

        // Create subtasks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subtasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL,
                task_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                due_date TEXT,
                priority TEXT NOT NULL CHECK(priority IN ('high', 'medium', 'low')),
                completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1)),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create notes table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0, 1)),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT
            )",
            [],
        )?;

        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Run schema migration and seed default settings
        Self::migrate_schema(&conn)?;
        self.seed_default_settings(&conn)?;

        Ok(())
    }

    /// Seeds default settings values into the settings table
    fn seed_default_settings(&self, conn: &Connection) -> Result<()> {
        let defaults = [
            ("onedrive_enabled", "0"),
            ("gdrive_enabled", "0"),
            ("backup_frequency_mins", "60"),
            ("desktop_sidebar_pinned", "1"),
        ];

        for &(key, val) in &defaults {
            conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, val],
            )?;
        }

        Ok(())
    }
}
