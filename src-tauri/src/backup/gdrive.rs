use crate::db::AppSettings;
use crate::backup::types::{RemoteBackupInfo, TokenResponse, GDriveListResponse};

/// Refresh Google access token
pub async fn refresh_gdrive_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to refresh Google token: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Google token refresh API error: {}", err_text));
    }

    let token_resp: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Google refresh response: {}", e))?;

    Ok(token_resp.access_token)
}

/// Upload backup file to Google Drive
pub async fn upload_to_gdrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let filename = "jakson_todo_backup.db";

    // 1. Search if file already exists
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?q=name='{}' and trashed=false",
        filename
    );
    let search_res = client
        .get(&search_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to search Google Drive: {}", e))?;

    let list: GDriveListResponse = search_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Google Drive search result: {}", e))?;

    if let Some(existing_file) = list.files.first() {
        // Update existing file
        let update_url = format!(
            "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
            existing_file.id
        );
        let patch_res = client
            .patch(&update_url)
            .bearer_auth(access_token)
            .header("Content-Type", "application/octet-stream")
            .body(db_bytes)
            .send()
            .await
            .map_err(|e| format!("Failed to upload/update Google Drive file: {}", e))?;

        if !patch_res.status().is_success() {
            let err_text = patch_res.text().await.unwrap_or_default();
            return Err(format!("Google Drive update failed: {}", err_text));
        }
    } else {
        // Create new file (multipart)
        let metadata = serde_json::json!({
            "name": filename,
            "description": "Backup de tarefas do Jakson Todo"
        });

        let form = reqwest::multipart::Form::new()
            .part(
                "metadata",
                reqwest::multipart::Part::text(metadata.to_string())
                    .mime_str("application/json; charset=UTF-8")
                    .unwrap(),
            )
            .part(
                "file",
                reqwest::multipart::Part::bytes(db_bytes)
                    .mime_str("application/octet-stream")
                    .unwrap(),
            );

        let upload_res = client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .bearer_auth(access_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Failed to create Google Drive file: {}", e))?;

        if !upload_res.status().is_success() {
            let err_text = upload_res.text().await.unwrap_or_default();
            return Err(format!("Google Drive upload failed: {}", err_text));
        }
    }

    Ok(())
}

/// Query Google Drive for backup file modified time
pub async fn get_gdrive_backup_info(
    settings: &AppSettings,
) -> RemoteBackupInfo {
    let mut info = RemoteBackupInfo {
        provider: "GDrive".to_string(),
        exists: false,
        last_modified: None,
    };

    if !settings.gdrive_enabled {
        return info;
    }

    if let (Some(cid), Some(sec), Some(ref_token)) = (
        &settings.gdrive_client_id,
        &settings.gdrive_client_secret,
        &settings.gdrive_refresh_token,
    ) {
        if let Ok(access_token) = refresh_gdrive_access_token(cid, sec, ref_token).await {
            let client = reqwest::Client::new();
            let search_url = "https://www.googleapis.com/drive/v3/files?q=name='jakson_todo_backup.db' and trashed=false&fields=files(id,modifiedTime)";
            
            if let Ok(res) = client.get(search_url).bearer_auth(&access_token).send().await {
                if res.status().is_success() {
                    if let Ok(list) = res.json::<GDriveListResponse>().await {
                        if let Some(file) = list.files.first() {
                            info.exists = true;
                            info.last_modified = file.modified_time.clone();
                        }
                    }
                }
            }
        }
    }

    info
}
