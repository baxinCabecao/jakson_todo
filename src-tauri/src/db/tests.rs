use crate::db::connection::DbConnection;
use crate::db::types::{Note, Subtask, Task};

#[test]
fn test_db_initialization_and_seed() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
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
    let temp_dir = std::env::temp_dir().join(format!("test_db_crud_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let db = DbConnection::new(temp_dir.clone());
    db.init_db().unwrap();

    let sub = Subtask {
        id: None,
        uuid: None,
        task_id: 0,
        title: "Sub 1".to_string(),
        description: Some("Desc 1".to_string()),
        due_date: Some("2026-06-07".to_string()),
        priority: "medium".to_string(),
        completed: false,
        created_at: "2026-06-06T12:00:00Z".to_string(),
        updated_at: None,
    };

    let task = Task {
        id: None,
        uuid: None,
        title: "Task 1".to_string(),
        description: Some("Description 1".to_string()),
        due_date: Some("2026-06-08".to_string()),
        priority: "high".to_string(),
        status: "todo".to_string(),
        created_at: "2026-06-06T12:00:00Z".to_string(),
        updated_at: None,
        is_deleted: false,
        deleted_at: None,
        subtasks: Some(vec![sub]),
    };

    let task_id = db.create_task(task).unwrap();
    assert!(task_id > 0);

    let tasks = db.get_all_tasks().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Task 1");
    assert!(tasks[0].uuid.is_some(), "Task should have an auto-generated UUID");
    
    let subtasks = tasks[0].subtasks.as_ref().unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].title, "Sub 1");
    assert_eq!(subtasks[0].completed, false);
    assert!(subtasks[0].uuid.is_some(), "Subtask should have an auto-generated UUID");

    // Update task and subtask status
    let mut updated_task = tasks[0].clone();
    updated_task.status = "in_progress".to_string();
    updated_task.subtasks.as_mut().unwrap()[0].completed = true;

    db.update_task(updated_task).unwrap();

    let tasks_after_update = db.get_all_tasks().unwrap();
    assert_eq!(tasks_after_update[0].status, "in_progress");
    assert_eq!(tasks_after_update[0].subtasks.as_ref().unwrap()[0].completed, true);

    // Soft Delete task
    db.delete_task(task_id).unwrap();
    let tasks_after_delete = db.get_all_tasks().unwrap();
    assert_eq!(tasks_after_delete.len(), 0, "Active tasks must be 0 after delete");

    // Must still exist in sync tasks as tombstone
    let sync_tasks = db.get_all_sync_tasks().unwrap();
    assert_eq!(sync_tasks.len(), 1, "Sync task must exist as tombstone");
    assert!(sync_tasks[0].is_deleted);
    assert!(sync_tasks[0].deleted_at.is_some());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_note_crud_and_pinning() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_notes_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let db = DbConnection::new(temp_dir.clone());
    db.init_db().unwrap();

    let note1 = Note {
        id: None,
        uuid: None,
        title: "Nota Comum".to_string(),
        content: "# Conteúdo Comum".to_string(),
        is_pinned: false,
        created_at: "2026-06-06T10:00:00Z".to_string(),
        updated_at: "2026-06-06T10:00:00Z".to_string(),
        is_deleted: false,
        deleted_at: None,
    };

    let note2 = Note {
        id: None,
        uuid: None,
        title: "Nota Importante Fixada".to_string(),
        content: "### Detalhes Importantes".to_string(),
        is_pinned: true,
        created_at: "2026-06-05T10:00:00Z".to_string(),
        updated_at: "2026-06-05T10:00:00Z".to_string(),
        is_deleted: false,
        deleted_at: None,
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

#[test]
fn test_self_healing_gzipped_db() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_gzip_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("todo.db");

    // Create an initial valid db
    {
        let db = DbConnection::new(temp_dir.clone());
        db.init_db().unwrap();
        let task = Task {
            id: None,
            uuid: None,
            title: "Task Antes da Compressão".to_string(),
            description: None,
            due_date: None,
            priority: "low".to_string(),
            status: "todo".to_string(),
            created_at: "2026-06-06T12:00:00Z".to_string(),
            updated_at: None,
            is_deleted: false,
            deleted_at: None,
            subtasks: None,
        };
        db.create_task(task).unwrap();
    }

    // Now compress todo.db with Gzip simulating an old app or external sync writing gzipped bytes
    let raw_db_bytes = std::fs::read(&db_path).unwrap();
    let compressed_bytes = crate::backup::compression::compress_db(&raw_db_bytes).unwrap();
    std::fs::write(&db_path, &compressed_bytes).unwrap();

    assert!(crate::backup::compression::is_gzipped(&std::fs::read(&db_path).unwrap()));

    // Opening DbConnection must automatically heal/decompress the database!
    let db_healed = DbConnection::new(temp_dir.clone());
    let init_res = db_healed.init_db();
    assert!(init_res.is_ok(), "init_db must succeed on gzipped db via self-healing");

    let tasks = db_healed.get_all_tasks().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Task Antes da Compressão");

    // File on disk must now be uncompressed SQLite
    let healed_bytes = std::fs::read(&db_path).unwrap();
    assert!(!crate::backup::compression::is_gzipped(&healed_bytes));

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_two_way_merge_multi_device_sync() {
    let dir_a = std::env::temp_dir().join(format!("test_dev_a_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let dir_b = std::env::temp_dir().join(format!("test_dev_b_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));

    let db_a = DbConnection::new(dir_a.clone());
    let db_b = DbConnection::new(dir_b.clone());
    db_a.init_db().unwrap();
    db_b.init_db().unwrap();

    // 1. Device A creates Task A ("Comprar Leite")
    let task_a = Task {
        id: None,
        uuid: Some("uuid-task-a".to_string()),
        title: "Comprar Leite".to_string(),
        description: Some("2 caixas".to_string()),
        due_date: Some("2026-10-01".to_string()),
        priority: "medium".to_string(),
        status: "todo".to_string(),
        created_at: "2026-09-12T10:00:00Z".to_string(),
        updated_at: Some("2026-09-12T10:00:00Z".to_string()),
        is_deleted: false,
        deleted_at: None,
        subtasks: None,
    };
    db_a.create_task(task_a).unwrap();

    // Device A creates Note A
    let note_a = Note {
        id: None,
        uuid: Some("uuid-note-a".to_string()),
        title: "Ideias de Projeto".to_string(),
        content: "Criar app multiplataforma".to_string(),
        is_pinned: false,
        created_at: "2026-09-12T10:00:00Z".to_string(),
        updated_at: "2026-09-12T10:00:00Z".to_string(),
        is_deleted: false,
        deleted_at: None,
    };
    db_a.create_note(note_a).unwrap();

    // 2. Device B concurrently creates Task B ("Ir ao Dentista") and Note B
    let task_b = Task {
        id: None,
        uuid: Some("uuid-task-b".to_string()),
        title: "Ir ao Dentista".to_string(),
        description: Some("Consulta às 15h".to_string()),
        due_date: Some("2026-10-02".to_string()),
        priority: "high".to_string(),
        status: "todo".to_string(),
        created_at: "2026-09-12T10:01:00Z".to_string(),
        updated_at: Some("2026-09-12T10:01:00Z".to_string()),
        is_deleted: false,
        deleted_at: None,
        subtasks: None,
    };
    db_b.create_task(task_b).unwrap();

    let note_b = Note {
        id: None,
        uuid: Some("uuid-note-b".to_string()),
        title: "Lista de Livros".to_string(),
        content: "Clean Code".to_string(),
        is_pinned: true,
        created_at: "2026-09-12T10:01:00Z".to_string(),
        updated_at: "2026-09-12T10:01:00Z".to_string(),
        is_deleted: false,
        deleted_at: None,
    };
    db_b.create_note(note_b).unwrap();

    // 3. Device B reconciles Device A's data (Simulating cloud sync)
    let sync_tasks_a = db_a.get_all_sync_tasks().unwrap();
    let sync_notes_a = db_a.get_all_sync_notes().unwrap();

    let stats_b = db_b.reconcile_with_remote(&sync_tasks_a, &sync_notes_a).unwrap();
    assert_eq!(stats_b.tasks_pulled, 1, "Device B should pull Task A");
    assert_eq!(stats_b.notes_pulled, 1, "Device B should pull Note A");

    // Verify Device B has BOTH Task A and Task B! (Zero data loss!)
    let tasks_b_after_sync = db_b.get_all_tasks().unwrap();
    assert_eq!(tasks_b_after_sync.len(), 2, "Device B must now have 2 tasks");
    let titles_b: Vec<String> = tasks_b_after_sync.into_iter().map(|t| t.title).collect();
    assert!(titles_b.contains(&"Comprar Leite".to_string()));
    assert!(titles_b.contains(&"Ir ao Dentista".to_string()));

    let notes_b_after_sync = db_b.get_all_notes().unwrap();
    assert_eq!(notes_b_after_sync.len(), 2, "Device B must now have 2 notes");

    // 4. Device A reconciles Device B's merged state
    let sync_tasks_b = db_b.get_all_sync_tasks().unwrap();
    let sync_notes_b = db_b.get_all_sync_notes().unwrap();

    let stats_a = db_a.reconcile_with_remote(&sync_tasks_b, &sync_notes_b).unwrap();
    assert_eq!(stats_a.tasks_pulled, 1, "Device A should pull Task B");
    assert_eq!(stats_a.notes_pulled, 1, "Device A should pull Note B");

    let tasks_a_after_sync = db_a.get_all_tasks().unwrap();
    assert_eq!(tasks_a_after_sync.len(), 2, "Device A must now have 2 tasks");

    // 5. Conflict Resolution (Last-Write-Wins):
    // Device A updates Task A at 10:10
    let mut task_a_mod = db_a.get_all_tasks().unwrap().into_iter().find(|t| t.uuid == Some("uuid-task-a".to_string())).unwrap();
    task_a_mod.title = "Comprar Leite Desnatado".to_string();
    task_a_mod.updated_at = Some("2026-09-12T10:10:00Z".to_string());
    db_a.update_task(task_a_mod).unwrap();

    // Device B updates Task A at 10:15 (later timestamp!)
    let mut task_a_b_mod = db_b.get_all_tasks().unwrap().into_iter().find(|t| t.uuid == Some("uuid-task-a".to_string())).unwrap();
    task_a_b_mod.title = "Comprar Leite de Aveia".to_string();
    task_a_b_mod.updated_at = Some("2026-09-12T10:15:00Z".to_string());
    db_b.update_task(task_a_b_mod).unwrap();

    // Sync from B to A: B's version is newer (10:15 > 10:10), so B should win!
    let sync_tasks_b_new = db_b.get_all_sync_tasks().unwrap();
    let _ = db_a.reconcile_with_remote(&sync_tasks_b_new, &[]).unwrap();

    let task_a_final = db_a.get_all_tasks().unwrap().into_iter().find(|t| t.uuid == Some("uuid-task-a".to_string())).unwrap();
    assert_eq!(task_a_final.title, "Comprar Leite de Aveia", "Newer update (10:15) must win via LWW");

    // 6. Tombstone / Soft Delete propagation:
    // Device A deletes Task B
    let task_b_on_a = db_a.get_all_tasks().unwrap().into_iter().find(|t| t.uuid == Some("uuid-task-b".to_string())).unwrap();
    db_a.delete_task(task_b_on_a.id.unwrap()).unwrap();

    // Device A syncs to Device B
    let sync_tasks_a_del = db_a.get_all_sync_tasks().unwrap();
    let _ = db_b.reconcile_with_remote(&sync_tasks_a_del, &[]).unwrap();

    // Verify Task B is now soft-deleted on Device B too!
    let active_tasks_b = db_b.get_all_tasks().unwrap();
    assert_eq!(active_tasks_b.len(), 1);
    assert_eq!(active_tasks_b[0].uuid, Some("uuid-task-a".to_string()));

    let _ = std::fs::remove_dir_all(dir_a);
    let _ = std::fs::remove_dir_all(dir_b);
}

#[test]
fn test_sqlite_wal_mode_and_concurrency() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_wal_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let db = DbConnection::new(temp_dir.clone());
    db.init_db().unwrap();

    // Verify WAL mode is active
    let conn = db.get_conn().unwrap();
    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0)).unwrap();
    assert_eq!(journal_mode.to_lowercase(), "wal");

    // Multiple connections can read simultaneously in WAL mode
    let conn2 = db.get_conn().unwrap();
    let count1: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0)).unwrap();
    let count2: i64 = conn2.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0)).unwrap();
    assert_eq!(count1, 0);
    assert_eq!(count2, 0);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_reconcile_stats_distinction() {
    let temp_dir = std::env::temp_dir().join(format!("test_db_stats_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let db = DbConnection::new(temp_dir.clone());
    db.init_db().unwrap();

    let remote_note = Note {
        id: None,
        uuid: Some("remote-uuid-1".to_string()),
        title: "Nota Remota".to_string(),
        content: "Remoto".to_string(),
        is_pinned: false,
        created_at: "2026-09-13T10:00:00Z".to_string(),
        updated_at: "2026-09-13T10:00:00Z".to_string(),
        is_deleted: false,
        deleted_at: None,
    };

    // When pulling a note that exists remotely but not locally:
    let stats = db.reconcile_with_remote(&[], &[remote_note]).unwrap();
    assert_eq!(stats.notes_pulled, 1);
    assert_eq!(stats.notes_pushed, 0);
    assert!(stats.has_pulled_changes());
    assert!(!stats.has_pushed_changes(), "Should not have pushed changes when only pulling");

    let _ = std::fs::remove_dir_all(temp_dir);
}
