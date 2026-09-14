mod db;
mod backup;
#[cfg(desktop)]
mod systray;

use crate::db::{AppSettings, DbConnection, Task, Note};
use crate::backup::{BackupReport, CloudBackupsCheck, SyncResult};
use tauri::{AppHandle, Manager, State};
use std::path::PathBuf;
use chrono::Local;

// State management
pub struct AppState {
    pub db_path: PathBuf,
}

#[tauri::command]
async fn get_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.get_all_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_task(task: Task, state: State<'_, AppState>) -> Result<i64, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.create_task(task).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_task(task: Task, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.update_task(task).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_task(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.delete_task(id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.get_all_notes().map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_note(note: Note, state: State<'_, AppState>) -> Result<i64, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.create_note(note).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_note(note: Note, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.update_note(note).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_note(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.delete_note(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn exit_app(app_handle: AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.save_settings(settings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_setting(key: String, value: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.save_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_setting(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn trigger_backup(app_handle: AppHandle, state: State<'_, AppState>) -> Result<BackupReport, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    let settings = db.get_settings().map_err(|e| e.to_string())?;

    if !settings.onedrive_enabled && !settings.gdrive_enabled {
        return Err("Nenhum provedor de backup (OneDrive ou Google Drive) está configurado ou ativado.".to_string());
    }

    let report = crate::backup::run_backup_to_clouds(state.db_path.clone(), settings).await?;
    
    // Update last backup time on success
    let now_str = chrono::Utc::now().to_rfc3339();
    let _ = db.save_setting("last_backup_time", &now_str);

    // Trigger notification manually showing details
    show_backup_notification_manual(&app_handle, &report);

    Ok(report)
}

#[tauri::command]
async fn start_oauth(
    provider: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    crate::backup::start_oauth_flow(provider, app_handle).await
}

#[tauri::command]
async fn handle_oauth_url(url: String, app_handle: AppHandle) -> Result<(), String> {
    crate::backup::process_oauth_callback_url(&url, app_handle).await
}

#[tauri::command]
fn disconnect_provider(provider: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    let mut settings = db.get_settings().map_err(|e| e.to_string())?;
    if provider.to_lowercase() == "gdrive" {
        settings.gdrive_refresh_token = None;
        settings.gdrive_enabled = false;
    } else {
        settings.onedrive_refresh_token = None;
        settings.onedrive_enabled = false;
    }
    db.save_settings(settings).map_err(|e| e.to_string())?;
    Ok(())
}


#[tauri::command]
async fn check_backups(state: State<'_, AppState>) -> Result<CloudBackupsCheck, String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    crate::backup::check_cloud_backups(state.db_path.clone(), settings).await
}

#[tauri::command]
async fn restore_backup(provider: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    crate::backup::restore_db_from_cloud(provider, state.db_path.clone(), settings).await
}

#[tauri::command]
async fn auto_sync(app_handle: AppHandle, state: State<'_, AppState>) -> Result<SyncResult, String> {
    crate::backup::run_auto_sync(&app_handle, state.db_path.clone()).await
}

#[tauri::command]
async fn restore_safety_backup(provider: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = DbConnection::new(state.db_path.parent().unwrap().to_path_buf());
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    crate::backup::restore_safety_backup_from_cloud(provider, state.db_path.clone(), settings).await
}

/// Helper function to show notifications for manual backup triggers
fn show_backup_notification_manual(app_handle: &AppHandle, report: &BackupReport) {
    use tauri_plugin_notification::NotificationExt;
    
    let mut messages = Vec::new();
    
    if report.onedrive.enabled {
        if report.onedrive.success {
            messages.push("OneDrive: Sucesso".to_string());
        } else {
            messages.push(format!("OneDrive: Falhou ({})", report.onedrive.error_message.clone().unwrap_or_default()));
        }
    }
    
    if report.gdrive.enabled {
        if report.gdrive.success {
            messages.push("GDrive: Sucesso".to_string());
        } else {
            messages.push(format!("GDrive: Falhou ({})", report.gdrive.error_message.clone().unwrap_or_default()));
        }
    }
    
    let body = if messages.is_empty() {
        "Nenhum backup foi realizado (provedores desativados).".to_string()
    } else {
        messages.join(". ")
    };

    let res = app_handle.notification().builder()
        .title("Jakson ToDo - Backup Manual")
        .body(&body)
        .show();

    if let Err(e) = res {
        eprintln!("[Notification] Erro ao exibir notificação de backup: {}", e);
    }
}

#[allow(dead_code)]
/// Helper function to show notifications for background auto backup errors
fn show_backup_notification(app_handle: &AppHandle, report: BackupReport) {
    use tauri_plugin_notification::NotificationExt;

    let onedrive_failed = report.onedrive.enabled && !report.onedrive.success;
    let gdrive_failed = report.gdrive.enabled && !report.gdrive.success;

    let mut title = String::new();
    let mut body = String::new();
    let mut notify = false;

    if onedrive_failed && gdrive_failed {
        title = "Falha Crítica de Backup".to_string();
        body = format!(
            "Falha ao realizar backup automático em ambos os provedores.\nOneDrive: {}\nGDrive: {}",
            report.onedrive.error_message.clone().unwrap_or_default(),
            report.gdrive.error_message.clone().unwrap_or_default()
        );
        notify = true;
    } else if onedrive_failed {
        title = "Aviso de Backup (OneDrive)".to_string();
        body = format!(
            "Backup automático no OneDrive falhou: {}",
            report.onedrive.error_message.clone().unwrap_or_default()
        );
        notify = true;
    } else if gdrive_failed {
        title = "Aviso de Backup (Google Drive)".to_string();
        body = format!(
            "Backup automático no Google Drive falhou: {}",
            report.gdrive.error_message.clone().unwrap_or_default()
        );
        notify = true;
    }

    if notify {
        let _ = app_handle.notification().builder()
            .title(&title)
            .body(&body)
            .show();
    }
}

/// Check and alert tasks that are overdue, due today, or tomorrow
fn check_startup_tasks_and_notify(app_handle: &AppHandle, db_path: PathBuf) {
    use tauri_plugin_notification::NotificationExt;
    
    let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
    let tasks = match db.get_all_tasks() {
        Ok(t) => t,
        Err(_) => return,
    };

    let today = Local::now().naive_local().date();
    let tomorrow = today + chrono::Duration::days(1);

    let mut overdue_count = 0;
    let mut today_count = 0;
    let mut tomorrow_count = 0;

    for task in tasks {
        if task.status == "completed" {
            continue;
        }

        if let Some(ref due_str) = task.due_date {
            if let Ok(due_date) = chrono::NaiveDate::parse_from_str(due_str, "%Y-%m-%d") {
                if due_date < today {
                    overdue_count += 1;
                } else if due_date == today {
                    today_count += 1;
                } else if due_date == tomorrow {
                    tomorrow_count += 1;
                }
            }
        }
    }

    if overdue_count > 0 || today_count > 0 || tomorrow_count > 0 {
        let title = "Jakson ToDo - Resumo de Tarefas".to_string();
        let body = format!(
            "Atrasadas: {}. Vencem hoje: {}. Vencem amanhã: {}.",
            overdue_count, today_count, tomorrow_count
        );
        let _ = app_handle.notification().builder()
            .title(&title)
            .body(&body)
            .show();
    }
}

/// Background thread/task for automatic synchronization
fn start_auto_backup_loop(app_handle: AppHandle, db_path: PathBuf) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Check every 60 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
            if let Ok(settings) = db.get_settings() {
                if settings.onedrive_enabled || settings.gdrive_enabled {
                    if let Err(e) = crate::backup::run_auto_sync(&app_handle, db_path.clone()).await {
                        eprintln!("[AutoSync Background] Erro: {}", e);
                    }
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_dir = app_handle.path().app_data_dir().unwrap();
            let db_path = app_dir.join("todo.db");

            // Initialize DB
            let db = DbConnection::new(app_dir.clone());
            db.init_db().expect("Failed to initialize database");

            // Register AppState
            app.manage(AppState { db_path: db_path.clone() });

            // Setup system tray for desktop
            #[cfg(desktop)]
            crate::systray::setup_systray(app).expect("Failed to setup system tray");

            // Run startup notifications
            check_startup_tasks_and_notify(&app_handle, db_path.clone());

            // Run auto backup loop
            start_auto_backup_loop(app_handle, db_path);

            Ok(())
        });

    #[cfg(desktop)]
    {
        builder = builder.on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent app from exiting and hide the window instead
                let _ = window.hide();
                api.prevent_close();
            }
        });
    }

    builder
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            create_task,
            update_task,
            delete_task,
            get_notes,
            create_note,
            update_note,
            delete_note,
            exit_app,
            get_settings,
            save_settings,
            save_setting,
            get_setting,
            trigger_backup,
            start_oauth,
            handle_oauth_url,
            disconnect_provider,
            check_backups,
            restore_backup,
            auto_sync,
            restore_safety_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
