use rusqlite::{params, Connection, Result};
use crate::db::connection::DbConnection;
use crate::db::types::{Task, Subtask};

impl DbConnection {
    /// Retrieve all tasks including their subtasks
    pub fn get_all_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, description, due_date, priority, status, created_at FROM tasks"
        )?;

        let task_iter = stmt.query_map([], |row| {
            Ok(Task {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                due_date: row.get(3)?,
                priority: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
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
    fn get_subtasks_for_task(&self, conn: &Connection, task_id: i64) -> Result<Vec<Subtask>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, title, description, due_date, priority, completed, created_at 
             FROM subtasks WHERE task_id = ?1"
        )?;

        let subtask_iter = stmt.query_map(params![task_id], |row| {
            let completed_int: i32 = row.get(6)?;
            Ok(Subtask {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                due_date: row.get(4)?,
                priority: row.get(5)?,
                completed: completed_int == 1,
                created_at: row.get(7)?,
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

        tx.execute(
            "INSERT INTO tasks (title, description, due_date, priority, status, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                task.title,
                task.description,
                task.due_date,
                task.priority,
                task.status,
                task.created_at
            ],
        )?;

        let task_id = tx.last_insert_rowid();

        // Insert subtasks if any
        if let Some(subtasks) = task.subtasks {
            for subtask in subtasks {
                tx.execute(
                    "INSERT INTO subtasks (task_id, title, description, due_date, priority, completed, created_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        task_id,
                        subtask.title,
                        subtask.description,
                        subtask.due_date,
                        subtask.priority,
                        if subtask.completed { 1 } else { 0 },
                        subtask.created_at
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

        tx.execute(
            "UPDATE tasks SET title = ?1, description = ?2, due_date = ?3, priority = ?4, status = ?5 
             WHERE id = ?6",
            params![
                task.title,
                task.description,
                task.due_date,
                task.priority,
                task.status,
                task_id
            ],
        )?;

        // To update subtasks simple and robust: delete old and insert new, or update individually.
        if let Some(subtasks) = task.subtasks {
            // Get current subtask IDs
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

            // Insert or update new subtasks list
            for subtask in subtasks {
                match subtask.id {
                    Some(sub_id) => {
                        // Update
                        tx.execute(
                            "UPDATE subtasks SET title = ?1, description = ?2, due_date = ?3, priority = ?4, completed = ?5 
                             WHERE id = ?6",
                            params![
                                subtask.title,
                                subtask.description,
                                subtask.due_date,
                                subtask.priority,
                                if subtask.completed { 1 } else { 0 },
                                sub_id
                            ],
                        )?;
                    }
                    None => {
                        // Insert
                        tx.execute(
                            "INSERT INTO subtasks (task_id, title, description, due_date, priority, completed, created_at) 
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                            params![
                                task_id,
                                subtask.title,
                                subtask.description,
                                subtask.due_date,
                                subtask.priority,
                                if subtask.completed { 1 } else { 0 },
                                subtask.created_at
                            ],
                        )?;
                    }
                }
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Delete a task (will automatically delete subtasks due to CASCADE constraint)
    pub fn delete_task(&self, task_id: i64) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![task_id])?;
        Ok(())
    }
}
