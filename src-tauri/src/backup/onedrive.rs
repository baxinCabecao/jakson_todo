use crate::backup::oauth::get_effective_onedrive_client_id;
use crate::backup::types::{OneDriveFileResponse, RemoteBackupInfo, TokenResponse};
use crate::db::AppSettings;

pub const SYNC_PAYLOAD_FILENAME: &str = "jakson_todo_sync_v2.json.gz";
pub const SAFETY_BACKUP_FILENAME: &str = "jakson_todo_safety_24h.json.gz";
pub const BACKUP_FILENAME: &str = "jakson_todo_sync_v2.json.gz";
#[allow(dead_code)]
pub const LEGACY_BACKUP_FILENAME: &str = "jakson_todo_backup.db";

/// Refresh OneDrive access token
pub async fn refresh_onedrive_access_token(
    client_id: &str,
    refresh_token: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let params = vec![
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let res = client
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Falha ao renovar token do OneDrive: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Erro na API de renovação do OneDrive: {}", err_text));
    }

    let token_resp: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Falha ao processar resposta do OneDrive: {}", e))?;

    Ok(token_resp.access_token)
}

/// Upload a file to OneDrive root directory
pub async fn upload_file_to_onedrive(
    access_token: &str,
    filename: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://graph.microsoft.com/v1.0/me/drive/root:/{}:/content",
        filename
    );

    let res = client
        .put(&url)
        .bearer_auth(access_token)
        .header("Content-Type", "application/octet-stream")
        .body(db_bytes)
        .send()
        .await
        .map_err(|e| format!("Falha no envio para o OneDrive: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Falha no upload do OneDrive: {}", err_text));
    }

    Ok(())
}

#[allow(dead_code)]
/// Upload main backup file to OneDrive
pub async fn upload_to_onedrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    upload_file_to_onedrive(access_token, BACKUP_FILENAME, db_bytes).await
}

#[allow(dead_code)]
/// Upload 24h safety backup file to OneDrive
pub async fn upload_safety_to_onedrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    upload_file_to_onedrive(access_token, SAFETY_BACKUP_FILENAME, db_bytes).await
}

/// Download a file from OneDrive root directory.
/// Returns Ok(Some(bytes)) if found, Ok(None) if 404 (file doesn't exist yet), or Err on network/API failure.
pub async fn download_file_from_onedrive(
    access_token: &str,
    filename: &str,
) -> Result<Option<Vec<u8>>, String> {
    let client = reqwest::Client::new();
    let download_url = format!(
        "https://graph.microsoft.com/v1.0/me/drive/root:/{}:/content",
        filename
    );
    let download_res = client
        .get(&download_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Falha ao baixar '{}' do OneDrive: {}", filename, e))?;

    if download_res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !download_res.status().is_success() {
        let err_text = download_res.text().await.unwrap_or_default();
        return Err(format!("Erro no download do OneDrive: {}", err_text));
    }

    let bytes = download_res
        .bytes()
        .await
        .map_err(|e| format!("Falha ao ler dados baixados do OneDrive: {}", e))?
        .to_vec();

    Ok(Some(bytes))
}

/// Query OneDrive for file modified time
pub async fn get_onedrive_file_info(
    settings: &AppSettings,
    filename: &str,
) -> RemoteBackupInfo {
    let mut info = RemoteBackupInfo {
        provider: "OneDrive".to_string(),
        exists: false,
        last_modified: None,
    };

    if !settings.onedrive_enabled {
        return info;
    }

    let refresh_token = match &settings.onedrive_refresh_token {
        Some(token) if !token.trim().is_empty() => token,
        _ => return info,
    };

    let client_id = get_effective_onedrive_client_id();
    if client_id.is_empty() {
        return info;
    }

    if let Ok(access_token) = refresh_onedrive_access_token(&client_id, refresh_token).await {
        let client = reqwest::Client::new();
        let url = format!("https://graph.microsoft.com/v1.0/me/drive/root:/{}", filename);

        if let Ok(res) = client.get(&url).bearer_auth(&access_token).send().await {
            if res.status().is_success() {
                if let Ok(file_info) = res.json::<OneDriveFileResponse>().await {
                    info.exists = true;
                    info.last_modified = file_info.last_modified_date_time;
                }
            }
        }
    }

    info
}

/// Query OneDrive for main backup file modified time
pub async fn get_onedrive_backup_info(
    settings: &AppSettings,
) -> RemoteBackupInfo {
    get_onedrive_file_info(settings, BACKUP_FILENAME).await
}

#[allow(dead_code)]
/// Query OneDrive for 24h safety backup file modified time
pub async fn get_onedrive_safety_backup_info(
    settings: &AppSettings,
) -> RemoteBackupInfo {
    get_onedrive_file_info(settings, SAFETY_BACKUP_FILENAME).await
}
