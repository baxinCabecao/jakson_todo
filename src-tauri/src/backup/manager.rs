use crate::db::AppSettings;
use crate::backup::types::{BackupStatus, BackupReport, CloudBackupsCheck, GDriveListResponse};
use crate::backup::gdrive::{refresh_gdrive_access_token, upload_to_gdrive, get_gdrive_backup_info};
use crate::backup::onedrive::{refresh_onedrive_access_token, upload_to_onedrive, get_onedrive_backup_info};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::fs;

/// Execute backup to OneDrive and Google Drive
pub async fn run_backup_to_clouds(
    db_path: PathBuf,
    settings: AppSettings,
) -> Result<BackupReport, String> {
    if !db_path.exists() {
        return Err("Database file does not exist locally.".to_string());
    }

    // Read the database bytes safely
    let db_bytes = fs::read(&db_path).map_err(|e| format!("Failed to read local database: {}", e))?;

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
        if let (Some(cid), Some(sec), Some(ref_token)) = (
            &settings.onedrive_client_id,
            &settings.onedrive_client_secret,
            &settings.onedrive_refresh_token,
        ) {
            match refresh_onedrive_access_token(cid, sec, ref_token).await {
                Ok(access_token) => match upload_to_onedrive(&access_token, db_bytes.clone()).await {
                    Ok(_) => onedrive_status.success = true,
                    Err(e) => onedrive_status.error_message = Some(e),
                },
                Err(e) => onedrive_status.error_message = Some(format!("OAuth Refresh Error: {}", e)),
            }
        } else {
            onedrive_status.error_message = Some("OneDrive credentials not configured.".to_string());
        }
    } else {
        onedrive_status.error_message = Some("OneDrive backup disabled.".to_string());
    }

    // 2. Run GDrive Backup if enabled
    if settings.gdrive_enabled {
        if let (Some(cid), Some(sec), Some(ref_token)) = (
            &settings.gdrive_client_id,
            &settings.gdrive_client_secret,
            &settings.gdrive_refresh_token,
        ) {
            match refresh_gdrive_access_token(cid, sec, ref_token).await {
                Ok(access_token) => match upload_to_gdrive(&access_token, db_bytes.clone()).await {
                    Ok(_) => gdrive_status.success = true,
                    Err(e) => gdrive_status.error_message = Some(e),
                },
                Err(e) => gdrive_status.error_message = Some(format!("OAuth Refresh Error: {}", e)),
            }
        } else {
            gdrive_status.error_message = Some("Google Drive credentials not configured.".to_string());
        }
    } else {
        gdrive_status.error_message = Some("Google Drive backup disabled.".to_string());
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

    // Spawn both checks in parallel
    let onedrive_info = get_onedrive_backup_info(&settings).await;
    let gdrive_info = get_gdrive_backup_info(&settings).await;

    let mut newer_backup_available = false;
    let mut recommended_provider = None;

    let local_dt = DateTime::parse_from_rfc3339(&local_mod_time).ok();

    // Check OneDrive
    if let (true, Some(ref remote_time)) = (onedrive_info.exists, &onedrive_info.last_modified) {
        if let Ok(remote_dt) = DateTime::parse_from_rfc3339(remote_time) {
            match local_dt {
                Some(ldt) if remote_dt > ldt => {
                    newer_backup_available = true;
                    recommended_provider = Some("OneDrive".to_string());
                }
                None => {
                    newer_backup_available = true;
                    recommended_provider = Some("OneDrive".to_string());
                }
                _ => {}
            }
        }
    }

    // Check GDrive and see if it is even newer than OneDrive and local
    if let (true, Some(ref remote_time)) = (gdrive_info.exists, &gdrive_info.last_modified) {
        if let Ok(remote_dt) = DateTime::parse_from_rfc3339(remote_time) {
            let mut gdrive_is_newer = false;
            match local_dt {
                Some(ldt) if remote_dt > ldt => gdrive_is_newer = true,
                None => gdrive_is_newer = true,
                _ => {}
            }

            if gdrive_is_newer {
                newer_backup_available = true;
                // Compare with OneDrive if OneDrive also had a newer backup
                if let Some(rec) = &recommended_provider {
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
        if let (Some(cid), Some(sec), Some(ref_token)) = (
            &settings.gdrive_client_id,
            &settings.gdrive_client_secret,
            &settings.gdrive_refresh_token,
        ) {
            let access_token = refresh_gdrive_access_token(cid, sec, ref_token).await?;
            
            // Search file to get ID
            let search_url = "https://www.googleapis.com/drive/v3/files?q=name='jakson_todo_backup.db' and trashed=false";
            let search_res = client
                .get(search_url)
                .bearer_auth(&access_token)
                .send()
                .await
                .map_err(|e| format!("Failed to find backup file in GDrive: {}", e))?;

            let list: GDriveListResponse = search_res
                .json()
                .await
                .map_err(|e| format!("Failed to parse search results: {}", e))?;

            if let Some(file) = list.files.first() {
                // Download content
                let download_url = format!(
                    "https://www.googleapis.com/drive/v3/files/{}?alt=media",
                    file.id
                );
                let download_res = client
                    .get(&download_url)
                    .bearer_auth(&access_token)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to download file from Google Drive: {}", e))?;

                if !download_res.status().is_success() {
                    return Err(format!(
                        "GDrive download error: {}",
                        download_res.text().await.unwrap_or_default()
                    ));
                }

                db_bytes = download_res
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?
                    .to_vec();
            } else {
                return Err("No backup file found in Google Drive.".to_string());
            }
        } else {
            return Err("Google Drive credentials not configured.".to_string());
        }
    } else {
        // OneDrive
        if let (Some(cid), Some(sec), Some(ref_token)) = (
            &settings.onedrive_client_id,
            &settings.onedrive_client_secret,
            &settings.onedrive_refresh_token,
        ) {
            let access_token = refresh_onedrive_access_token(cid, sec, ref_token).await?;
            let download_url = "https://graph.microsoft.com/v1.0/me/drive/root:/jakson_todo_backup.db:/content";
            
            let download_res = client
                .get(download_url)
                .bearer_auth(&access_token)
                .send()
                .await
                .map_err(|e| format!("Failed to download file from OneDrive: {}", e))?;

            if !download_res.status().is_success() {
                return Err(format!(
                    "OneDrive download error: {}",
                    download_res.text().await.unwrap_or_default()
                ));
            }

            db_bytes = download_res
                .bytes()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?
                .to_vec();
        } else {
            return Err("OneDrive credentials not configured.".to_string());
        }
    }

    // Backup local file to a temp backup copy before overwriting, just in case
    if db_path.exists() {
        let mut backup_temp = db_path.clone();
        backup_temp.set_extension("db.bak");
        let _ = fs::copy(&db_path, &backup_temp);
    }

    // Write bytes to local db_path
    fs::write(&db_path, db_bytes)
        .map_err(|e| format!("Failed to write to local database file: {}", e))?;

    Ok(())
}
