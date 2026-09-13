use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;

use crate::db::connection::DbConnection;
use crate::db::types::{Note, Task};

#[derive(Debug, Default, Clone)]
pub struct ReconcileStats {
    pub tasks_pulled: usize,
    pub tasks_pushed: usize,
    pub notes_pulled: usize,
    pub notes_pushed: usize,
}

impl ReconcileStats {
    pub fn has_pulled_changes(&self) -> bool {
        self.tasks_pulled > 0 || self.notes_pulled > 0
    }

    pub fn has_pushed_changes(&self) -> bool {
        self.tasks_pushed > 0 || self.notes_pushed > 0
    }
}

/// Helper to parse ISO 8601 string or fallback to UNIX epoch
fn parse_timestamp(ts: Option<&str>) -> DateTime<Utc> {
    match ts {
        Some(s) => DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| DateTime::from_timestamp(0, 0).unwrap()),
        None => DateTime::from_timestamp(0, 0).unwrap(),
    }
}

impl DbConnection {
    /// Retrieve all tasks including soft-deleted ones for synchronization
    pub fn get_all_sync_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, uuid, title, description, due_date, priority, status, created_at, updated_at, is_deleted, deleted_at 
             FROM tasks"
        )?;

        let task_iter = stmt.query_map([], |row| {
            let is_del_int: i32 = row.get(9)?;
            Ok(Task {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                due_date: row.get(4)?,
                priority: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                is_deleted: is_del_int == 1,
                deleted_at: row.get(10)?,
                subtasks: None,
            })
        })?;

        let mut tasks = Vec::new();
        for task_res in task_iter {
            let mut task = task_res?;
            if let Some(id) = task.id {
                let subtasks = self.get_subtasks_for_task(&conn, id)?;
                task.subtasks = Some(subtasks);
            }
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Retrieve all notes including soft-deleted ones for synchronization
    pub fn get_all_sync_notes(&self) -> Result<Vec<Note>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, uuid, title, content, is_pinned, created_at, updated_at, is_deleted, deleted_at 
             FROM notes"
        )?;

        let note_iter = stmt.query_map([], |row| {
            let pinned_int: i32 = row.get(4)?;
            let is_del_int: i32 = row.get(7)?;
            Ok(Note {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                is_pinned: pinned_int == 1,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                is_deleted: is_del_int == 1,
                deleted_at: row.get(8)?,
            })
        })?;

        let mut notes = Vec::new();
        for note_res in note_iter {
            notes.push(note_res?);
        }

        Ok(notes)
    }

    /// Reconcile remote tasks and notes into local SQLite database
    /// Returns stats of changes pulled and pushed
    pub fn reconcile_with_remote(
        &self,
        remote_tasks: &[Task],
        remote_notes: &[Note],
    ) -> Result<ReconcileStats> {
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        let mut stats = ReconcileStats::default();

        // 1. Reconcile Tasks
        let local_tasks = Self::fetch_tasks_in_tx(&tx)?;
        let mut local_tasks_map: HashMap<String, Task> = HashMap::new();
        for task in local_tasks {
            if let Some(ref u) = task.uuid {
                local_tasks_map.insert(u.clone(), task);
            }
        }

        for r_task in remote_tasks {
            let r_uuid = match &r_task.uuid {
                Some(u) => u.clone(),
                None => continue,
            };

            let r_updated = parse_timestamp(r_task.updated_at.as_deref());

            if let Some(l_task) = local_tasks_map.get(&r_uuid) {
                let l_updated = parse_timestamp(l_task.updated_at.as_deref());

                if r_updated > l_updated {
                    // Remote is newer: pull remote into local
                    Self::apply_remote_task_update(&tx, l_task.id.unwrap(), r_task)?;
                    stats.tasks_pulled += 1;
                } else if l_updated > r_updated {
                    // Local is newer: will be pushed back
                    stats.tasks_pushed += 1;
                }
            } else {
                // Task exists on remote but not locally: insert local
                Self::insert_synced_task(&tx, r_task)?;
                stats.tasks_pulled += 1;
            }
        }

        // Check for local tasks that remote doesn't have yet
        let remote_uuids: std::collections::HashSet<&str> = remote_tasks
            .iter()
            .filter_map(|t| t.uuid.as_deref())
            .collect();

        for (u, _) in &local_tasks_map {
            if !remote_uuids.contains(u.as_str()) {
                stats.tasks_pushed += 1;
            }
        }

        // 2. Reconcile Notes
        let local_notes = Self::fetch_notes_in_tx(&tx)?;
        let mut local_notes_map: HashMap<String, Note> = HashMap::new();
        for note in local_notes {
            if let Some(ref u) = note.uuid {
                local_notes_map.insert(u.clone(), note);
            }
        }

        for r_note in remote_notes {
            let r_uuid = match &r_note.uuid {
                Some(u) => u.clone(),
                None => continue,
            };

            let r_updated = parse_timestamp(Some(&r_note.updated_at));

            if let Some(l_note) = local_notes_map.get(&r_uuid) {
                let l_updated = parse_timestamp(Some(&l_note.updated_at));

                if r_updated > l_updated {
                    Self::apply_remote_note_update(&tx, l_note.id.unwrap(), r_note)?;
                    stats.notes_pulled += 1;
                } else if l_updated > r_updated {
                    stats.notes_pushed += 1;
                }
            } else {
                Self::insert_synced_note(&tx, r_note)?;
                stats.notes_pulled += 1;
            }
        }

        let remote_note_uuids: std::collections::HashSet<&str> = remote_notes
            .iter()
            .filter_map(|n| n.uuid.as_deref())
            .collect();

        for (u, _) in &local_notes_map {
            if !remote_note_uuids.contains(u.as_str()) {
                stats.notes_pushed += 1;
            }
        }

        tx.commit()?;
        Ok(stats)
    }

    /// Internal helper to fetch tasks inside a transaction
    fn fetch_tasks_in_tx(conn: &Connection) -> Result<Vec<Task>> {
        let mut stmt = conn.prepare(
            "SELECT id, uuid, title, description, due_date, priority, status, created_at, updated_at, is_deleted, deleted_at 
             FROM tasks"
        )?;

        let task_iter = stmt.query_map([], |row| {
            let is_del_int: i32 = row.get(9)?;
            Ok(Task {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                due_date: row.get(4)?,
                priority: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                is_deleted: is_del_int == 1,
                deleted_at: row.get(10)?,
                subtasks: None,
            })
        })?;

        let mut tasks = Vec::new();
        for t in task_iter {
            tasks.push(t?);
        }
        Ok(tasks)
    }

    /// Internal helper to fetch notes inside a transaction
    fn fetch_notes_in_tx(conn: &Connection) -> Result<Vec<Note>> {
        let mut stmt = conn.prepare(
            "SELECT id, uuid, title, content, is_pinned, created_at, updated_at, is_deleted, deleted_at 
             FROM notes"
        )?;

        let note_iter = stmt.query_map([], |row| {
            let pinned_int: i32 = row.get(4)?;
            let is_del_int: i32 = row.get(7)?;
            Ok(Note {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                is_pinned: pinned_int == 1,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                is_deleted: is_del_int == 1,
                deleted_at: row.get(8)?,
            })
        })?;

        let mut notes = Vec::new();
        for n in note_iter {
            notes.push(n?);
        }
        Ok(notes)
    }

    /// Insert task coming from sync payload
    fn insert_synced_task(conn: &Connection, task: &Task) -> Result<i64> {
        let task_uuid = task
            .uuid
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let updated_at = task
            .updated_at
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        conn.execute(
            "INSERT INTO tasks (uuid, title, description, due_date, priority, status, created_at, updated_at, is_deleted, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                task_uuid,
                task.title,
                task.description,
                task.due_date,
                task.priority,
                task.status,
                task.created_at,
                updated_at,
                if task.is_deleted { 1 } else { 0 },
                task.deleted_at
            ],
        )?;

        let task_id = conn.last_insert_rowid();

        if let Some(ref subtasks) = task.subtasks {
            for subtask in subtasks {
                let sub_uuid = subtask
                    .uuid
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let sub_updated = subtask
                    .updated_at
                    .clone()
                    .unwrap_or_else(|| updated_at.clone());
                conn.execute(
                    "INSERT INTO subtasks (uuid, task_id, title, description, due_date, priority, completed, created_at, updated_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        sub_uuid,
                        task_id,
                        subtask.title,
                        subtask.description,
                        subtask.due_date,
                        subtask.priority,
                        if subtask.completed { 1 } else { 0 },
                        subtask.created_at,
                        sub_updated
                    ],
                )?;
            }
        }

        Ok(task_id)
    }

    /// Apply remote task update to local task
    fn apply_remote_task_update(
        conn: &Connection,
        local_id: i64,
        remote_task: &Task,
    ) -> Result<()> {
        let updated_at = remote_task
            .updated_at
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        conn.execute(
            "UPDATE tasks 
             SET title = ?1, description = ?2, due_date = ?3, priority = ?4, status = ?5, 
                 updated_at = ?6, is_deleted = ?7, deleted_at = ?8 
             WHERE id = ?9",
            params![
                remote_task.title,
                remote_task.description,
                remote_task.due_date,
                remote_task.priority,
                remote_task.status,
                updated_at,
                if remote_task.is_deleted { 1 } else { 0 },
                remote_task.deleted_at,
                local_id
            ],
        )?;

        // Reconcile subtasks: delete existing and replace with remote subtasks
        if let Some(ref subtasks) = remote_task.subtasks {
            conn.execute("DELETE FROM subtasks WHERE task_id = ?1", params![local_id])?;

            for subtask in subtasks {
                let sub_uuid = subtask
                    .uuid
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let sub_updated = subtask
                    .updated_at
                    .clone()
                    .unwrap_or_else(|| updated_at.clone());
                conn.execute(
                    "INSERT INTO subtasks (uuid, task_id, title, description, due_date, priority, completed, created_at, updated_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        sub_uuid,
                        local_id,
                        subtask.title,
                        subtask.description,
                        subtask.due_date,
                        subtask.priority,
                        if subtask.completed { 1 } else { 0 },
                        subtask.created_at,
                        sub_updated
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// Insert note coming from sync payload
    fn insert_synced_note(conn: &Connection, note: &Note) -> Result<i64> {
        let note_uuid = note
            .uuid
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        conn.execute(
            "INSERT INTO notes (uuid, title, content, is_pinned, created_at, updated_at, is_deleted, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                note_uuid,
                note.title,
                note.content,
                if note.is_pinned { 1 } else { 0 },
                note.created_at,
                note.updated_at,
                if note.is_deleted { 1 } else { 0 },
                note.deleted_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Apply remote note update to local note
    fn apply_remote_note_update(
        conn: &Connection,
        local_id: i64,
        remote_note: &Note,
    ) -> Result<()> {
        conn.execute(
            "UPDATE notes 
             SET title = ?1, content = ?2, is_pinned = ?3, updated_at = ?4, 
                 is_deleted = ?5, deleted_at = ?6 
             WHERE id = ?7",
            params![
                remote_note.title,
                remote_note.content,
                if remote_note.is_pinned { 1 } else { 0 },
                remote_note.updated_at,
                if remote_note.is_deleted { 1 } else { 0 },
                remote_note.deleted_at,
                local_id
            ],
        )?;
        Ok(())
    }

    /// Clean up tombstones older than `retention_days` (default 30 days)
    pub fn purge_old_tombstones(&self, retention_days: i64) -> Result<()> {
        let conn = self.get_conn()?;
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.to_rfc3339();

        conn.execute(
            "DELETE FROM tasks WHERE is_deleted = 1 AND deleted_at < ?1",
            params![cutoff_str],
        )?;
        conn.execute(
            "DELETE FROM notes WHERE is_deleted = 1 AND deleted_at < ?1",
            params![cutoff_str],
        )?;
        Ok(())
    }
}
