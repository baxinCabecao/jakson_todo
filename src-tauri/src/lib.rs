mod db;
mod backup;
mod systray;

use crate::db::{AppSettings, DbConnection, Task};
use crate::backup::{BackupReport, CloudBackupsCheck};
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
async fn start_oauth(provider: String, client_id: String, client_secret: String, app_handle: AppHandle) -> Result<(), String> {
    crate::backup::start_oauth_flow(provider, client_id, client_secret, app_handle).await
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

    let _ = app_handle.notification().builder()
        .title("Jakson Todo - Backup Manual")
        .body(&body)
        .show();
}

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
        let title = "Jakson Todo - Resumo de Tarefas".to_string();
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

/// Background thread/task for automatic backups
fn start_auto_backup_loop(app_handle: AppHandle, db_path: PathBuf) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Check settings and frequency every 60 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
            if let Ok(settings) = db.get_settings() {
                if settings.onedrive_enabled || settings.gdrive_enabled {
                    let should_backup = match settings.last_backup_time {
                        Some(ref last_time) => {
                            if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(last_time) {
                                let now = chrono::Utc::now();
                                let elapsed = now.signed_duration_since(last_dt.with_timezone(&chrono::Utc));
                                elapsed.num_minutes() >= settings.backup_frequency_mins
                            } else {
                                true
                            }
                        }
                        None => true,
                    };

                    if should_backup {
                        match crate::backup::run_backup_to_clouds(db_path.clone(), settings).await {
                            Ok(report) => {
                                let now_str = chrono::Utc::now().to_rfc3339();
                                let _ = db.save_setting("last_backup_time", &now_str);
                                show_backup_notification(&app_handle, report);
                            }
                            Err(e) => {
                                eprintln!("Auto backup error: {}", e);
                            }
                        }
                    }
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_dir = app_handle.path().app_data_dir().unwrap();
            let db_path = app_dir.join("todo.db");

            // Initialize DB
            let db = DbConnection::new(app_dir.clone());
            db.init_db().expect("Failed to initialize database");

            // Register AppState
            app.manage(AppState { db_path: db_path.clone() });

            // Setup system tray
            crate::systray::setup_systray(app).expect("Failed to setup system tray");

            // Run startup notifications
            check_startup_tasks_and_notify(&app_handle, db_path.clone());

            // Run auto backup loop
            start_auto_backup_loop(app_handle, db_path);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent app from exiting and hide the window instead
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            create_task,
            update_task,
            delete_task,
            get_settings,
            save_settings,
            trigger_backup,
            start_oauth,
            check_backups,
            restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
