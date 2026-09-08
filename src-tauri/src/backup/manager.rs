use crate::backup::gdrive::{get_gdrive_backup_info, refresh_gdrive_access_token, upload_to_gdrive};
use crate::backup::oauth::{get_effective_gdrive_client_id, get_effective_onedrive_client_id};
use crate::backup::onedrive::{get_onedrive_backup_info, refresh_onedrive_access_token, upload_to_onedrive};
use crate::backup::types::{BackupReport, BackupStatus, CloudBackupsCheck, GDriveListResponse};
use crate::db::{AppSettings, DbConnection};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

/// Execute backup to OneDrive and Google Drive
pub async fn run_backup_to_clouds(
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<BackupReport, String> {
    if !db_path.exists() {
        return Err("O arquivo de banco de dados não existe localmente.".to_string());
    }

    // Prepare sanitized database copy for cloud upload (strips device-specific OAuth refresh tokens)
    let db_bytes = {
        let temp_upload_path = db_path.with_extension("upload_temp.db");
        if fs::copy(&db_path, &temp_upload_path).is_ok() {
            if let Ok(conn) = rusqlite::Connection::open(&temp_upload_path) {
                let _ = conn.execute(
                    "UPDATE settings SET value = '' WHERE key IN ('gdrive_refresh_token', 'onedrive_refresh_token', 'gdrive_client_secret', 'onedrive_client_secret')",
                    [],
                );
            }
            let bytes = fs::read(&temp_upload_path).unwrap_or_else(|_| fs::read(&db_path).unwrap_or_default());
            let _ = fs::remove_file(&temp_upload_path);
            bytes
        } else {
            fs::read(&db_path).map_err(|e| format!("Falha ao ler o banco de dados local: {}", e))?
        }
    };

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
                    Ok(access_token) => match upload_to_onedrive(&access_token, db_bytes.clone()).await {
                        Ok(_) => onedrive_status.success = true,
                        Err(e) => onedrive_status.error_message = Some(e),
                    },
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
                Ok(access_token) => match upload_to_gdrive(&access_token, db_bytes.clone()).await {
                    Ok(_) => gdrive_status.success = true,
                    Err(e) => gdrive_status.error_message = Some(e),
                },
                Err(e) => gdrive_status.error_message = Some(format!("Erro ao renovar token: {}", e)),
            }
        } else {
            gdrive_status.error_message = Some("Conta Google Drive não conectada.".to_string());
        }
    } else {
        gdrive_status.error_message = Some("Backup do Google Drive desativado.".to_string());
    }

    Ok(BackupReport {
        onedrive: onedrive_status,
        gdrive: gdrive_status,
    })
}

/// Check remote cloud backups on startup and compare with local DB modification time
pub async fn check_cloud_backups(
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<CloudBackupsCheck, String> {
    let local_mod_time = if db_path.exists() {
        let meta = fs::metadata(&db_path).map_err(|e| e.to_string())?;
        let sys_time = meta.modified().map_err(|e| e.to_string())?;
        let datetime: DateTime<Utc> = sys_time.into();
        datetime.to_rfc3339()
    } else {
        "".to_string()
    };

    let onedrive_info = get_onedrive_backup_info(&settings).await;
    let gdrive_info = get_gdrive_backup_info(&settings).await;

    let mut newer_backup_available = false;
    let mut recommended_provider = None;
    let local_dt = DateTime::parse_from_rfc3339(&local_mod_time).ok();

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
                if let Some(ref rec) = recommended_provider {
                    if rec == "OneDrive" {
                        if let Some(ref od_time) = onedrive_info.last_modified {
                            if let Ok(od_dt) = DateTime::parse_from_rfc3339(od_time) {
                                if remote_dt > od_dt {
                                    recommended_provider = Some("GDrive".to_string());
                                }
                            }
                        }
                    }
                } else {
                    recommended_provider = Some("GDrive".to_string());
                }
            }
        }
    }

    Ok(CloudBackupsCheck {
        onedrive: onedrive_info,
        gdrive: gdrive_info,
        local_last_modified: local_mod_time,
        newer_backup_available,
        recommended_provider,
    })
}

/// Download backup and restore local DB
pub async fn restore_db_from_cloud(
    provider: String,
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let db_bytes;

    if provider.to_lowercase() == "gdrive" {
        let refresh_token = settings
            .gdrive_refresh_token
            .as_ref()
            .ok_or_else(|| "Conta Google Drive não conectada.".to_string())?;

        let client_id = get_effective_gdrive_client_id();

        let access_token = refresh_gdrive_access_token(&client_id, refresh_token).await?;

        // Search file in appDataFolder
        let search_url = "https://www.googleapis.com/drive/v3/files?spaces=appDataFolder&q=name='jakson_todo_backup.db' and trashed=false&fields=files(id)";
        let search_res = client
            .get(search_url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| format!("Falha ao buscar arquivo de backup no Google Drive: {}", e))?;

        let list: GDriveListResponse = search_res
            .json()
            .await
            .map_err(|e| format!("Falha ao processar lista de arquivos: {}", e))?;

        if let Some(file) = list.files.first() {
            let download_url = format!(
                "https://www.googleapis.com/drive/v3/files/{}?alt=media",
                file.id
            );
            let download_res = client
                .get(&download_url)
                .bearer_auth(&access_token)
                .send()
                .await
                .map_err(|e| format!("Falha ao baixar arquivo do Google Drive: {}", e))?;

            if !download_res.status().is_success() {
                let err_text = download_res.text().await.unwrap_or_default();
                return Err(format!("Erro no download do Google Drive: {}", err_text));
            }

            db_bytes = download_res
                .bytes()
                .await
                .map_err(|e| format!("Falha ao ler dados baixados: {}", e))?
                .to_vec();
        } else {
            return Err("Nenhum arquivo de backup encontrado na pasta do app no Google Drive.".to_string());
        }
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

        let download_url = "https://graph.microsoft.com/v1.0/me/drive/root:/jakson_todo_backup.db:/content";
        let download_res = client
            .get(download_url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| format!("Falha ao baixar arquivo do OneDrive: {}", e))?;

        if !download_res.status().is_success() {
            let err_text = download_res.text().await.unwrap_or_default();
            return Err(format!("Erro no download do OneDrive: {}", err_text));
        }

        db_bytes = download_res
            .bytes()
            .await
            .map_err(|e| format!("Falha ao ler dados baixados do OneDrive: {}", e))?
            .to_vec();
    }

    // Safety backup of existing local file
    if db_path.exists() {
        let mut backup_temp = db_path.clone();
        backup_temp.set_extension("db.bak");
        let _ = fs::copy(&db_path, &backup_temp);
    }

    fs::write(&db_path, db_bytes)
        .map_err(|e| format!("Falha ao salvar banco de dados local: {}", e))?;

    // Crucial: preserve the current device's local settings and OAuth tokens!
    // Restoring from another device (e.g. Desktop to Android or vice-versa)
    // must NOT overwrite the local device's OAuth credentials with the remote ones.
    let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
    if let Err(e) = db.save_settings(settings) {
        eprintln!("[Restore] Aviso: Falha ao preservar configurações locais no banco restaurado: {}", e);
    } else {
        println!("[Restore] Configurações e tokens locais preservados com sucesso após restauração.");
    }

    Ok(())
}
