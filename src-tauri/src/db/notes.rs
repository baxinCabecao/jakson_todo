use rusqlite::{params, Result};
use crate::db::connection::DbConnection;
use crate::db::types::Note;

impl DbConnection {
    /// Retrieve all notes ordered with pinned notes first, then by updated_at descending
    pub fn get_all_notes(&self) -> Result<Vec<Note>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, content, is_pinned, created_at, updated_at 
             FROM notes 
             ORDER BY is_pinned DESC, updated_at DESC, id DESC"
        )?;

        let note_iter = stmt.query_map([], |row| {
            let pinned_int: i32 = row.get(3)?;
            Ok(Note {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                content: row.get(2)?,
                is_pinned: pinned_int == 1,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
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
        conn.execute(
            "INSERT INTO notes (title, content, is_pinned, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                note.title,
                note.content,
                if note.is_pinned { 1 } else { 0 },
                note.created_at,
                note.updated_at,
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

        conn.execute(
            "UPDATE notes 
             SET title = ?1, content = ?2, is_pinned = ?3, updated_at = ?4 
             WHERE id = ?5",
            params![
                note.title,
                note.content,
                if note.is_pinned { 1 } else { 0 },
                note.updated_at,
                note_id,
            ],
        )?;

        Ok(())
    }

    /// Delete a note
    pub fn delete_note(&self, note_id: i64) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute("DELETE FROM notes WHERE id = ?1", params![note_id])?;
        Ok(())
    }
}
