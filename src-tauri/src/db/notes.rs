use rusqlite::{params, Result};
use crate::db::connection::DbConnection;
use crate::db::types::Note;
use chrono::Utc;

impl DbConnection {
    /// Retrieve all active notes ordered with pinned notes first, then by updated_at descending
    pub fn get_all_notes(&self) -> Result<Vec<Note>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, uuid, title, content, is_pinned, created_at, updated_at 
             FROM notes 
             WHERE is_deleted = 0 
             ORDER BY is_pinned DESC, updated_at DESC, id DESC"
        )?;

        let note_iter = stmt.query_map([], |row| {
            let pinned_int: i32 = row.get(4)?;
            Ok(Note {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                is_pinned: pinned_int == 1,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                is_deleted: false,
                deleted_at: None,
            })
        })?;

        let mut notes = Vec::new();
        for note_res in note_iter {
            notes.push(note_res?);
        }

        Ok(notes)
    }

    /// Create a new note
    pub fn create_note(&self, note: Note) -> Result<i64> {
        let conn = self.get_conn()?;
        let now = Utc::now().to_rfc3339();
        let note_uuid = note.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let updated_at = if note.updated_at.is_empty() { now.clone() } else { note.updated_at };

        conn.execute(
            "INSERT INTO notes (uuid, title, content, is_pinned, created_at, updated_at, is_deleted, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                note_uuid,
                note.title,
                note.content,
                if note.is_pinned { 1 } else { 0 },
                note.created_at,
                updated_at,
                if note.is_deleted { 1 } else { 0 },
                note.deleted_at
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Update an existing note
    pub fn update_note(&self, note: Note) -> Result<()> {
        let conn = self.get_conn()?;
        let note_id = match note.id {
            Some(id) => id,
            None => return Err(rusqlite::Error::InvalidQuery),
        };

        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE notes 
             SET title = ?1, content = ?2, is_pinned = ?3, updated_at = ?4 
             WHERE id = ?5",
            params![
                note.title,
                note.content,
                if note.is_pinned { 1 } else { 0 },
                now,
                note_id,
            ],
        )?;

        Ok(())
    }

    /// Soft delete a note (sets is_deleted = 1 and records deleted_at)
    pub fn delete_note(&self, note_id: i64) -> Result<()> {
        let conn = self.get_conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE notes SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, note_id],
        )?;
        Ok(())
    }
}
