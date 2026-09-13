use crate::backup::compression::{compress_db, decompress_db_if_needed};
use crate::backup::gdrive::{
    download_file_from_gdrive, get_gdrive_backup_info, refresh_gdrive_access_token,
    upload_file_to_gdrive, SAFETY_BACKUP_FILENAME as GDRIVE_SAFETY_NAME,
    SYNC_PAYLOAD_FILENAME as GDRIVE_SYNC_NAME,
};
use crate::backup::oauth::{get_effective_gdrive_client_id, get_effective_onedrive_client_id};
use crate::backup::onedrive::{
    download_file_from_onedrive, get_onedrive_backup_info, refresh_onedrive_access_token,
    upload_file_to_onedrive, SAFETY_BACKUP_FILENAME as ONEDRIVE_SAFETY_NAME,
    SYNC_PAYLOAD_FILENAME as ONEDRIVE_SYNC_NAME,
};
use crate::backup::types::{BackupReport, BackupStatus, CloudBackupsCheck, SyncPayload};
use crate::db::{AppSettings, DbConnection};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

/// Helper to get or initialize client device id
pub fn get_or_create_device_id(db: &DbConnection) -> String {
    if let Ok(Some(id)) = db.get_setting("device_id") {
        if !id.trim().is_empty() {
            return id;
        }
    }
    let new_id = uuid::Uuid::new_v4().to_string();
    let _ = db.save_setting("device_id", &new_id);
    new_id
}

#[allow(dead_code)]
/// Prepare sanitized database copy for legacy fallback
pub fn prepare_sanitized_db_bytes(db_path: &PathBuf) -> Result<Vec<u8>, String> {
    if !db_path.exists() {
        return Err("O arquivo de banco de dados não existe localmente.".to_string());
    }

    let temp_upload_path = db_path.with_extension("upload_temp.db");
    if fs::copy(db_path, &temp_upload_path).is_ok() {
        if let Ok(conn) = rusqlite::Connection::open(&temp_upload_path) {
            let _ = conn.execute(
                "UPDATE settings SET value = '' WHERE key IN ('gdrive_refresh_token', 'onedrive_refresh_token', 'gdrive_client_secret', 'onedrive_client_secret')",
                [],
            );
        }
        let bytes = fs::read(&temp_upload_path).unwrap_or_else(|_| fs::read(db_path).unwrap_or_default());
        let _ = fs::remove_file(&temp_upload_path);
        Ok(bytes)
    } else {
        fs::read(db_path).map_err(|e| format!("Falha ao ler o banco de dados local: {}", e))
    }
}

/// Execute manual backup/push to OneDrive and Google Drive with Gzip compression
pub async fn run_backup_to_clouds(
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<BackupReport, String> {
    let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
    let all_tasks = db.get_all_sync_tasks().map_err(|e| e.to_string())?;
    let all_notes = db.get_all_sync_notes().map_err(|e| e.to_string())?;

    let now_str = Utc::now().to_rfc3339();
    let payload = SyncPayload {
        version: 2,
        schema_version: 1,
        client_device_id: get_or_create_device_id(&db),
        synced_at: now_str.clone(),
        tasks: all_tasks,
        notes: all_notes,
    };

    let json_bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("Falha ao serializar dados: {}", e))?;
    let compressed_bytes = compress_db(&json_bytes)
        .map_err(|e| format!("Falha ao comprimir dados: {}", e))?;

    let mut onedrive_status = BackupStatus {
        provider: "OneDrive".to_string(),
        enabled: settings.onedrive_enabled,
        success: false,
        error_message: None,
    };

    let mut gdrive_status = BackupStatus {
        provider: "GDrive".to_string(),
        enabled: settings.gdrive_enabled,
        success: false,
        error_message: None,
    };

    // 1. Run OneDrive Backup if enabled
    if settings.onedrive_enabled {
        if let Some(ref ref_token) = settings.onedrive_refresh_token {
            let client_id = get_effective_onedrive_client_id();

            if client_id.is_empty() {
                onedrive_status.error_message = Some("Client ID do OneDrive não configurado.".to_string());
            } else {
                match refresh_onedrive_access_token(&client_id, ref_token).await {
                    Ok(access_token) => {
                        match upload_file_to_onedrive(&access_token, ONEDRIVE_SYNC_NAME, compressed_bytes.clone()).await {
                            Ok(_) => onedrive_status.success = true,
                            Err(e) => onedrive_status.error_message = Some(e),
                        }
                    }
                    Err(e) => onedrive_status.error_message = Some(format!("Erro ao renovar token: {}", e)),
                }
            }
        } else {
            onedrive_status.error_message = Some("Conta OneDrive não conectada.".to_string());
        }
    } else {
        onedrive_status.error_message = Some("Backup do OneDrive desativado.".to_string());
    }

    // 2. Run GDrive Backup if enabled
    if settings.gdrive_enabled {
        if let Some(ref ref_token) = settings.gdrive_refresh_token {
            let client_id = get_effective_gdrive_client_id();

            match refresh_gdrive_access_token(&client_id, ref_token).await {
                Ok(access_token) => {
                    match upload_file_to_gdrive(&access_token, GDRIVE_SYNC_NAME, "Dados de Sincronização Jakson ToDo", compressed_bytes.clone()).await {
                        Ok(_) => gdrive_status.success = true,
                        Err(e) => gdrive_status.error_message = Some(e),
                    }
                }
                Err(e) => gdrive_status.error_message = Some(format!("Erro ao renovar token: {}", e)),
            }
        } else {
            gdrive_status.error_message = Some("Conta Google Drive não conectada.".to_string());
        }
    } else {
        gdrive_status.error_message = Some("Backup do Google Drive desativado.".to_string());
    }

    // Update last backup time if at least one provider succeeded
    if onedrive_status.success || gdrive_status.success {
        let _ = db.save_setting("last_backup_time", &now_str);
    }

    Ok(BackupReport {
        onedrive: onedrive_status,
        gdrive: gdrive_status,
    })
}

/// Check remote cloud backups on startup and compare with recorded last sync time
pub async fn check_cloud_backups(
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<CloudBackupsCheck, String> {
    let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
    let local_last_sync = db.get_setting("last_backup_time").unwrap_or(None).unwrap_or_default();

    let onedrive_info = get_onedrive_backup_info(&settings).await;
    let gdrive_info = get_gdrive_backup_info(&settings).await;

    let mut newer_backup_available = false;
    let mut recommended_provider = None;
    let local_dt = DateTime::parse_from_rfc3339(&local_last_sync).ok();

    if let (true, Some(ref remote_time)) = (onedrive_info.exists, &onedrive_info.last_modified) {
        if let Ok(remote_dt) = DateTime::parse_from_rfc3339(remote_time) {
            let is_newer = match local_dt {
                Some(ldt) => remote_dt > ldt,
                None => true,
            };
            if is_newer {
                newer_backup_available = true;
                recommended_provider = Some("OneDrive".to_string());
            }
        }
    }

    if let (true, Some(ref remote_time)) = (gdrive_info.exists, &gdrive_info.last_modified) {
        if let Ok(remote_dt) = DateTime::parse_from_rfc3339(remote_time) {
            let gdrive_is_newer = match local_dt {
                Some(ldt) => remote_dt > ldt,
                None => true,
            };

            if gdrive_is_newer {
                newer_backup_available = true;
                recommended_provider = Some("GDrive".to_string());
            }
        }
    }

    Ok(CloudBackupsCheck {
        onedrive: onedrive_info,
        gdrive: gdrive_info,
        local_last_modified: local_last_sync,
        newer_backup_available,
        recommended_provider,
    })
}

/// Helper to restore payload or legacy database from bytes
pub fn restore_payload_or_db_bytes(
    raw_bytes: &[u8],
    db_path: &PathBuf,
    settings: AppSettings,
) -> Result<String, String> {
    let decompressed = decompress_db_if_needed(raw_bytes)
        .map_err(|e| format!("Falha ao descomprimir dados: {}", e))?;

    let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());

    // 1. Try parsing as structured SyncPayload JSON
    if let Ok(payload) = serde_json::from_slice::<SyncPayload>(&decompressed) {
        let stats = db.reconcile_with_remote(&payload.tasks, &payload.notes)
            .map_err(|e| format!("Falha ao reconciliar dados restaurados: {}", e))?;

        let now_str = Utc::now().to_rfc3339();
        let _ = db.save_setting("last_backup_time", &now_str);

        return Ok(format!(
            "Restauração e mesclagem concluídas: {} tarefas e {} notas atualizadas.",
            stats.tasks_pulled, stats.notes_pulled
        ));
    }

    // 2. Fallback: Legacy raw SQLite database
    if decompressed.starts_with(b"SQLite format 3") {
        if db_path.exists() {
            let mut backup_temp = db_path.clone();
            backup_temp.set_extension("db.bak");
            let _ = fs::copy(db_path, &backup_temp);
        }

        fs::write(db_path, &decompressed)
            .map_err(|e| format!("Falha ao salvar banco de dados local: {}", e))?;

        // Re-open and preserve local settings and OAuth tokens
        let _ = db.save_settings(settings);
        let _ = db.init_db();

        return Ok("Banco de dados legado restaurado com sucesso.".to_string());
    }

    Err("Formato de arquivo de backup não reconhecido.".to_string())
}

/// Helper to download and restore database/sync payload from a cloud provider
async fn download_and_restore(
    provider: &str,
    filename: &str,
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<(), String> {
    let raw_bytes = if provider.to_lowercase() == "gdrive" {
        let refresh_token = settings
            .gdrive_refresh_token
            .as_ref()
            .ok_or_else(|| "Conta Google Drive não conectada.".to_string())?;

        let client_id = get_effective_gdrive_client_id();
        let access_token = refresh_gdrive_access_token(&client_id, refresh_token).await?;
        download_file_from_gdrive(&access_token, filename).await?
    } else {
        let refresh_token = settings
            .onedrive_refresh_token
            .as_ref()
            .ok_or_else(|| "Conta OneDrive não conectada.".to_string())?;

        let client_id = get_effective_onedrive_client_id();
        if client_id.is_empty() {
            return Err("Client ID do OneDrive não configurado.".to_string());
        }

        let access_token = refresh_onedrive_access_token(&client_id, refresh_token).await?;
        download_file_from_onedrive(&access_token, filename).await?
    };

    restore_payload_or_db_bytes(&raw_bytes, &db_path, settings).map(|_| ())
}

/// Download main backup and restore/reconcile local DB
pub async fn restore_db_from_cloud(
    provider: String,
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<(), String> {
    let filename = if provider.to_lowercase() == "gdrive" {
        GDRIVE_SYNC_NAME
    } else {
        ONEDRIVE_SYNC_NAME
    };

    // Try new sync format first, fallback to legacy if not found
    match download_and_restore(&provider, filename, db_path.clone(), settings.clone()).await {
        Ok(()) => Ok(()),
        Err(_) => {
            let legacy_name = crate::backup::gdrive::LEGACY_BACKUP_FILENAME;
            download_and_restore(&provider, legacy_name, db_path, settings).await
        }
    }
}

/// Download 24h safety backup snapshot and restore local DB
pub async fn restore_safety_backup_from_cloud(
    provider: String,
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<(), String> {
    let filename = if provider.to_lowercase() == "gdrive" {
        GDRIVE_SAFETY_NAME
    } else {
        ONEDRIVE_SAFETY_NAME
    };
    download_and_restore(&provider, filename, db_path, settings).await
}
