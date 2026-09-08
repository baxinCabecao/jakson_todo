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

        DbConnection { path: db_path }
    }

    /// Open a connection to the SQLite database
    pub fn get_conn(&self) -> Result<Connection> {
        Connection::open(&self.path)
    }

    /// Create tables if they do not exist
    pub fn init_db(&self) -> Result<()> {
        let conn = self.get_conn()?;

        // Create tasks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                due_date TEXT,
                priority TEXT NOT NULL CHECK(priority IN ('high', 'medium', 'low')),
                status TEXT NOT NULL CHECK(status IN ('todo', 'in_progress', 'completed')),
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // Create subtasks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subtasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                due_date TEXT,
                priority TEXT NOT NULL CHECK(priority IN ('high', 'medium', 'low')),
                completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1)),
                created_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create notes table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0, 1)),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
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

        // Seed default settings if they don't exist
        self.seed_default_settings(&conn)?;

        Ok(())
    }

    /// Seeds default settings values into the settings table
    fn seed_default_settings(&self, conn: &Connection) -> Result<()> {
        let defaults = [
            ("onedrive_enabled", "0"),
            ("gdrive_enabled", "0"),
            ("backup_frequency_mins", "60"),
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
