use crate::backup::oauth::get_effective_onedrive_client_id;
use crate::backup::types::{OneDriveFileResponse, RemoteBackupInfo, TokenResponse};
use crate::db::AppSettings;

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

/// Upload backup file to OneDrive
pub async fn upload_to_onedrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let filename = "jakson_todo_backup.db";
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

/// Query OneDrive for backup file modified time
pub async fn get_onedrive_backup_info(
    settings: &AppSettings,
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
        let url = "https://graph.microsoft.com/v1.0/me/drive/root:/jakson_todo_backup.db";

        if let Ok(res) = client.get(url).bearer_auth(&access_token).send().await {
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
