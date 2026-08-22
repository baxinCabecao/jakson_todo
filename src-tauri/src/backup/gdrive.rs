use crate::backup::oauth::get_effective_gdrive_client_id;
use crate::backup::types::{GDriveListResponse, RemoteBackupInfo, TokenResponse};
use crate::db::AppSettings;

const BACKUP_FILENAME: &str = "jakson_todo_backup.db";
const APPDATA_FOLDER: &str = "appDataFolder";

/// Refresh Google access token (supports PKCE with optional client_secret)
pub async fn refresh_gdrive_access_token(
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let mut params = vec![
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    if let Some(sec) = client_secret {
        if !sec.is_empty() {
            params.push(("client_secret", sec));
        }
    }

    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Falha ao renovar token do Google: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Erro na API de renovação de token Google: {}", err_text));
    }

    let token_resp: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Falha ao processar resposta de renovação: {}", e))?;

    Ok(token_resp.access_token)
}

/// Upload backup file to Google Drive Application Data folder (appDataFolder)
pub async fn upload_to_gdrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    // 1. Search if file already exists in appDataFolder
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?spaces={}&q=name='{}' and trashed=false&fields=files(id,name)",
        APPDATA_FOLDER, BACKUP_FILENAME
    );
    let search_res = client
        .get(&search_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Falha ao consultar pasta do aplicativo no Google Drive: {}", e))?;

    let list: GDriveListResponse = search_res
        .json()
        .await
        .map_err(|e| format!("Falha ao ler listagem de arquivos do Google Drive: {}", e))?;

    if let Some(existing_file) = list.files.first() {
        // Update existing file in appDataFolder
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
            .map_err(|e| format!("Falha ao atualizar arquivo no Google Drive: {}", e))?;

        if !patch_res.status().is_success() {
            let err_text = patch_res.text().await.unwrap_or_default();
            return Err(format!("Atualização no Google Drive falhou: {}", err_text));
        }
    } else {
        // Create new file inside appDataFolder (multipart)
        let metadata = serde_json::json!({
            "name": BACKUP_FILENAME,
            "description": "Backup de tarefas do Jakson Todo",
            "parents": [APPDATA_FOLDER]
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
            .map_err(|e| format!("Falha ao enviar arquivo para o Google Drive: {}", e))?;

        if !upload_res.status().is_success() {
            let err_text = upload_res.text().await.unwrap_or_default();
            return Err(format!("Criação do arquivo no Google Drive falhou: {}", err_text));
        }
    }

    Ok(())
}

/// Query Google Drive appDataFolder for backup file modified time
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

    let refresh_token = match &settings.gdrive_refresh_token {
        Some(token) if !token.trim().is_empty() => token,
        _ => return info,
    };

    let client_id = get_effective_gdrive_client_id(settings.gdrive_client_id.as_deref());
    let client_secret = settings.gdrive_client_secret.as_deref();

    let access_token = match refresh_gdrive_access_token(&client_id, client_secret, refresh_token).await {
        Ok(token) => token,
        Err(_) => return info,
    };

    let client = reqwest::Client::new();
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?spaces={}&q=name='{}' and trashed=false&fields=files(id,modifiedTime)",
        APPDATA_FOLDER, BACKUP_FILENAME
    );

    if let Ok(res) = client.get(&search_url).bearer_auth(&access_token).send().await {
        if res.status().is_success() {
            if let Ok(list) = res.json::<GDriveListResponse>().await {
                if let Some(file) = list.files.first() {
                    info.exists = true;
                    info.last_modified = file.modified_time.clone();
                }
            }
        }
    }

    info
}
