use rusqlite::{params, Connection, Result};
use crate::db::connection::DbConnection;
use crate::db::types::{Task, Subtask};
use chrono::Utc;

impl DbConnection {
    /// Retrieve all active tasks (excluding soft-deleted) including their subtasks
    pub fn get_all_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, uuid, title, description, due_date, priority, status, created_at, updated_at 
             FROM tasks 
             WHERE is_deleted = 0 
             ORDER BY id DESC"
        )?;

        let task_iter = stmt.query_map([], |row| {
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
                is_deleted: false,
                deleted_at: None,
                subtasks: None,
            })
        })?;

        let mut tasks = Vec::new();
        for task_res in task_iter {
            let mut task = task_res?;
            let subtasks = self.get_subtasks_for_task(&conn, task.id.unwrap())?;
            task.subtasks = Some(subtasks);
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Helper function to retrieve subtasks of a specific task
    pub fn get_subtasks_for_task(&self, conn: &Connection, task_id: i64) -> Result<Vec<Subtask>> {
        let mut stmt = conn.prepare(
            "SELECT id, uuid, task_id, title, description, due_date, priority, completed, created_at, updated_at 
             FROM subtasks WHERE task_id = ?1"
        )?;

        let subtask_iter = stmt.query_map(params![task_id], |row| {
            let completed_int: i32 = row.get(7)?;
            Ok(Subtask {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                task_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                due_date: row.get(5)?,
                priority: row.get(6)?,
                completed: completed_int == 1,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut subtasks = Vec::new();
        for subtask in subtask_iter {
            subtasks.push(subtask?);
        }

        Ok(subtasks)
    }

    /// Create a new task in the database
    pub fn create_task(&self, task: Task) -> Result<i64> {
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        let now = Utc::now().to_rfc3339();
        let task_uuid = task.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let updated_at = task.updated_at.unwrap_or_else(|| now.clone());

        tx.execute(
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

        let task_id = tx.last_insert_rowid();

        // Insert subtasks if any
        if let Some(subtasks) = task.subtasks {
            for subtask in subtasks {
                let sub_uuid = subtask.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let sub_updated = subtask.updated_at.unwrap_or_else(|| now.clone());
                tx.execute(
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

        tx.commit()?;
        Ok(task_id)
    }

    /// Update an existing task and its subtasks
    pub fn update_task(&self, task: Task) -> Result<()> {
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        let task_id = match task.id {
            Some(id) => id,
            None => return Err(rusqlite::Error::InvalidQuery),
        };

        let now = Utc::now().to_rfc3339();
        let updated_at = task.updated_at.unwrap_or(now.clone());

        tx.execute(
            "UPDATE tasks 
             SET title = ?1, description = ?2, due_date = ?3, priority = ?4, status = ?5, updated_at = ?6 
             WHERE id = ?7",
            params![
                task.title,
                task.description,
                task.due_date,
                task.priority,
                task.status,
                updated_at,
                task_id
            ],
        )?;

        // Update subtasks
        if let Some(subtasks) = task.subtasks {
            let mut stmt = tx.prepare("SELECT id FROM subtasks WHERE task_id = ?1")?;
            let current_ids: Vec<i64> = stmt
                .query_map(params![task_id], |row| row.get(0))?
                .filter_map(Result::ok)
                .collect();
            drop(stmt);

            let new_subtask_ids: Vec<i64> = subtasks.iter().filter_map(|s| s.id).collect();

            // Delete subtasks that are no longer present
            for id in current_ids {
                if !new_subtask_ids.contains(&id) {
                    tx.execute("DELETE FROM subtasks WHERE id = ?1", params![id])?;
                }
            }

            // Insert or update subtasks list
            for subtask in subtasks {
                let sub_uuid = subtask.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                match subtask.id {
                    Some(sub_id) => {
                        tx.execute(
                            "UPDATE subtasks 
                             SET title = ?1, description = ?2, due_date = ?3, priority = ?4, completed = ?5, updated_at = ?6 
                             WHERE id = ?7",
                            params![
                                subtask.title,
                                subtask.description,
                                subtask.due_date,
                                subtask.priority,
                                if subtask.completed { 1 } else { 0 },
                                now,
                                sub_id
                            ],
                        )?;
                    }
                    None => {
                        tx.execute(
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
                                now
                            ],
                        )?;
                    }
                }
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Soft delete a task (sets is_deleted = 1 and records deleted_at)
    pub fn delete_task(&self, task_id: i64) -> Result<()> {
        let conn = self.get_conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )?;
        Ok(())
    }
}
