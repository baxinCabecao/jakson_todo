use crate::db::connection::DbConnection;
use crate::db::types::{Task, Subtask, Note};

#[test]
fn test_db_initialization_and_seed() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_{}", chrono::Utc::now().timestamp_millis()));
    let db = DbConnection::new(temp_dir.clone());
    let init_res = db.init_db();
    assert!(init_res.is_ok());

    let settings = db.get_settings();
    assert!(settings.is_ok());
    let settings = settings.unwrap();
    assert_eq!(settings.onedrive_enabled, false);
    assert_eq!(settings.gdrive_enabled, false);
    assert_eq!(settings.backup_frequency_mins, 60);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_task_crud() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_crud_{}", chrono::Utc::now().timestamp_millis()));
    let db = DbConnection::new(temp_dir.clone());
    db.init_db().unwrap();

    let sub = Subtask {
        id: None,
        task_id: 0,
        title: "Sub 1".to_string(),
        description: Some("Desc 1".to_string()),
        due_date: Some("2026-06-07".to_string()),
        priority: "medium".to_string(),
        completed: false,
        created_at: "2026-06-06T12:00:00".to_string(),
    };

    let task = Task {
        id: None,
        title: "Task 1".to_string(),
        description: Some("Description 1".to_string()),
        due_date: Some("2026-06-08".to_string()),
        priority: "high".to_string(),
        status: "todo".to_string(),
        created_at: "2026-06-06T12:00:00".to_string(),
        subtasks: Some(vec![sub]),
    };

    let task_id = db.create_task(task).unwrap();
    assert!(task_id > 0);

    let tasks = db.get_all_tasks().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Task 1");
    
    let subtasks = tasks[0].subtasks.as_ref().unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].title, "Sub 1");
    assert_eq!(subtasks[0].completed, false);

    // Update task and subtask status
    let mut updated_task = tasks[0].clone();
    updated_task.status = "in_progress".to_string();
    updated_task.subtasks.as_mut().unwrap()[0].completed = true;

    db.update_task(updated_task).unwrap();

    let tasks_after_update = db.get_all_tasks().unwrap();
    assert_eq!(tasks_after_update[0].status, "in_progress");
    assert_eq!(tasks_after_update[0].subtasks.as_ref().unwrap()[0].completed, true);

    // Delete task
    db.delete_task(task_id).unwrap();
    let tasks_after_delete = db.get_all_tasks().unwrap();
    assert_eq!(tasks_after_delete.len(), 0);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_note_crud_and_pinning() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_notes_{}", chrono::Utc::now().timestamp_millis()));
    let db = DbConnection::new(temp_dir.clone());
    db.init_db().unwrap();

    let note1 = Note {
        id: None,
        title: "Nota Comum".to_string(),
        content: "# Conteúdo Comum".to_string(),
        is_pinned: false,
        created_at: "2026-06-06T10:00:00".to_string(),
        updated_at: "2026-06-06T10:00:00".to_string(),
    };

    let note2 = Note {
        id: None,
        title: "Nota Importante Fixada".to_string(),
        content: "### Detalhes Importantes".to_string(),
        is_pinned: true,
        created_at: "2026-06-05T10:00:00".to_string(),
        updated_at: "2026-06-05T10:00:00".to_string(),
    };

    let id1 = db.create_note(note1).unwrap();
    let id2 = db.create_note(note2).unwrap();

    let notes = db.get_all_notes().unwrap();
    assert_eq!(notes.len(), 2);
    // Pinned note should be first even if created earlier
    assert_eq!(notes[0].id, Some(id2));
    assert_eq!(notes[0].title, "Nota Importante Fixada");
    assert_eq!(notes[0].is_pinned, true);
    assert_eq!(notes[1].id, Some(id1));

    // Update note content and unpin
    let mut updated_note2 = notes[0].clone();
    updated_note2.title = "Nota Atualizada".to_string();
    updated_note2.content = "Novo conteúdo".to_string();
    updated_note2.is_pinned = false;
    db.update_note(updated_note2).unwrap();

    let notes_after_update = db.get_all_notes().unwrap();
    let updated_target = notes_after_update.iter().find(|n| n.id == Some(id2)).unwrap();
    assert_eq!(updated_target.title, "Nota Atualizada");
    assert_eq!(updated_target.content, "Novo conteúdo");
    assert_eq!(updated_target.is_pinned, false);

    // Delete notes
    db.delete_note(id1).unwrap();
    db.delete_note(id2).unwrap();
    let notes_after_delete = db.get_all_notes().unwrap();
    assert_eq!(notes_after_delete.len(), 0);

    let _ = std::fs::remove_dir_all(temp_dir);
}
