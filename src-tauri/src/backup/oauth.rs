use crate::backup::types::TokenResponse;
use crate::db::DbConnection;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

pub const DEFAULT_GDRIVE_CLIENT_ID: &str = match option_env!("GDRIVE_CLIENT_ID") {
    Some(val) => val,
    None => "692484307374-2m873d6l2j561flv9s8u0j1e54911vkm.apps.googleusercontent.com",
};

pub const DEFAULT_ONEDRIVE_CLIENT_ID: &str = match option_env!("ONEDRIVE_CLIENT_ID") {
    Some(val) => val,
    None => "",
};

pub const OAUTH_PORT: u16 = 59135;

pub fn get_effective_gdrive_client_id(custom: Option<&str>) -> String {
    match custom {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => DEFAULT_GDRIVE_CLIENT_ID.to_string(),
    }
}

pub fn get_effective_onedrive_client_id(custom: Option<&str>) -> String {
    match custom {
        Some(cid) if !cid.trim().is_empty() => cid.trim().to_string(),
        _ => DEFAULT_ONEDRIVE_CLIENT_ID.to_string(),
    }
}

/// Generates a PKCE (code_verifier, code_challenge) pair using SHA-256 and Base64URL without padding
pub fn generate_pkce() -> (String, String) {
    let mut random_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(random_bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    (verifier, challenge)
}

/// Constructs the OAuth 2.0 authorization URL with PKCE parameters
fn build_authorization_url(provider: &str, client_id: &str, challenge: &str) -> String {
    let redirect_uri = format!("http://localhost:{}", OAUTH_PORT);

    if provider.to_lowercase() == "gdrive" {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=https://www.googleapis.com/auth/drive.appdata&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
            client_id, redirect_uri, challenge
        )
    } else {
        format!(
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope=files.readwrite%20offline_access&code_challenge={}&code_challenge_method=S256&response_mode=query",
            client_id, redirect_uri, challenge
        )
    }
}

/// Start a temporary HTTP server on localhost to capture the OAuth redirect code with PKCE.
pub async fn start_oauth_flow(
    provider: String,
    custom_client_id: Option<String>,
    custom_client_secret: Option<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let is_gdrive = provider.to_lowercase() == "gdrive";
    let client_id = if is_gdrive {
        get_effective_gdrive_client_id(custom_client_id.as_deref())
    } else {
        get_effective_onedrive_client_id(custom_client_id.as_deref())
    };

    if client_id.is_empty() {
        return Err(format!("Client ID para {} não está configurado.", provider));
    }

    let client_secret = custom_client_secret
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());

    let (code_verifier, code_challenge) = generate_pkce();
    let auth_url = build_authorization_url(&provider, &client_id, &code_challenge);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", OAUTH_PORT))
        .map_err(|e| format!("Não foi possível escutar na porta {}: {}", OAUTH_PORT, e))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;

    // Open the browser with the generated authorization URL
    let _ = app_handle.opener().open_url(&auth_url, None::<&str>);

    let provider_clone = provider.clone();
    let app_handle_clone = app_handle.clone();
    let client_id_clone = client_id.clone();
    let client_secret_clone = client_secret.clone();

    tauri::async_runtime::spawn(async move {
        handle_oauth_callback(
            listener,
            provider_clone,
            client_id_clone,
            client_secret_clone,
            code_verifier,
            app_handle_clone,
        )
        .await;
    });

    Ok(())
}

async fn accept_connection_with_timeout(listener: &TcpListener, timeout_secs: u64) -> Option<std::net::TcpStream> {
    let start_time = Instant::now();
    while start_time.elapsed().as_secs() < timeout_secs {
        match listener.accept() {
            Ok((stream, _)) => return Some(stream),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(_) => return None,
        }
    }
    None
}

async fn handle_oauth_callback(
    listener: TcpListener,
    provider: String,
    client_id: String,
    client_secret: Option<String>,
    code_verifier: String,
    app_handle: AppHandle,
) {
    let mut stream = match accept_connection_with_timeout(&listener, 300).await {
        Some(s) => s,
        None => {
            let _ = app_handle.emit("oauth-error", format!("Autenticação para {} expirou.", provider));
            return;
        }
    };

    let mut buffer = [0; 2048];
    let size = stream.read(&mut buffer).unwrap_or(0);
    let request_str = String::from_utf8_lossy(&buffer[..size]).to_string();

    let code = extract_code_from_request(&request_str);
    if code.is_empty() {
        let err_html = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<h1>Falha na Autenticação</h1><p>Nenhum código de autorização encontrado.</p>";
        let _ = stream.write_all(err_html.as_bytes());
        let _ = app_handle.emit(
            "oauth-error",
            format!("Não foi possível obter o código de autorização para {}", provider),
        );
        return;
    }

    let provider_name = if provider.to_lowercase() == "gdrive" { "Google Drive" } else { "OneDrive" };
    let success_html = build_success_response_html(provider_name);
    let _ = stream.write_all(success_html.as_bytes());
    let _ = stream.flush();

    match exchange_code_for_tokens(&provider, &client_id, client_secret.as_deref(), &code, &code_verifier).await {
        Ok((_access_token, refresh_token)) => {
            let app_dir = app_handle.path().app_data_dir().unwrap();
            let db = DbConnection::new(app_dir);
            if let Ok(mut settings) = db.get_settings() {
                if provider.to_lowercase() == "gdrive" {
                    settings.gdrive_client_id = Some(client_id);
                    settings.gdrive_client_secret = client_secret;
                    settings.gdrive_refresh_token = refresh_token;
                    settings.gdrive_enabled = true;
                } else {
                    settings.onedrive_client_id = Some(client_id);
                    settings.onedrive_client_secret = client_secret;
                    settings.onedrive_refresh_token = refresh_token;
                    settings.onedrive_enabled = true;
                }
                let _ = db.save_settings(settings);
            }
            let _ = app_handle.emit("oauth-success", provider);
        }
        Err(e) => {
            let _ = app_handle.emit("oauth-error", format!("Falha na troca do token: {}", e));
        }
    }
}

fn build_success_response_html(provider_name: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
        <html>\
        <head><style>body {{ font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background-color: #0f172a; color: #f8fafc; }} .card {{ background: #1e293b; padding: 2rem; border-radius: 12px; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1); text-align: center; }} h1 {{ color: #10b981; }}</style></head>\
        <body>\
        <div class='card'>\
        <h1>Conexão com {} Realizada!</h1>\
        <p>Você já pode fechar esta aba e retornar ao aplicativo.</p>\
        </div>\
        </body>\
        </html>",
        provider_name
    )
}

fn extract_code_from_request(request: &str) -> String {
    if let Some(start) = request.find("code=") {
        let code_part = &request[start + 5..];
        if let Some(end) = code_part.find(' ') {
            let raw_code = &code_part[..end];
            if let Some(amp) = raw_code.find('&') {
                return raw_code[..amp].to_string();
            }
            return raw_code.to_string();
        }
    }
    String::new()
}

async fn exchange_code_for_tokens(
    provider: &str,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    code_verifier: &str,
) -> Result<(String, Option<String>), String> {
    let client = reqwest::Client::new();
    let redirect_uri = format!("http://localhost:{}", OAUTH_PORT);
    let is_gdrive = provider.to_lowercase() == "gdrive";

    let (token_url, mut params) = if is_gdrive {
        (
            "https://oauth2.googleapis.com/token",
            vec![
                ("client_id", client_id),
                ("code", code),
                ("code_verifier", code_verifier),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri.as_str()),
            ],
        )
    } else {
        (
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            vec![
                ("client_id", client_id),
                ("code", code),
                ("code_verifier", code_verifier),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri.as_str()),
                ("scope", "files.readwrite offline_access"),
            ],
        )
    };

    if let Some(sec) = client_secret.filter(|s| !s.is_empty()) {
        params.push(("client_secret", sec));
    }

    let res = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Falha na requisição OAuth: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Erro ao obter token: {}", err_text));
    }

    let token_resp: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Falha ao processar resposta do token: {}", e))?;

    Ok((token_resp.access_token, token_resp.refresh_token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let (verifier, challenge) = generate_pkce();
        assert!(!verifier.is_empty());
        assert!(!challenge.is_empty());
        assert_ne!(verifier, challenge);
        assert_eq!(verifier.len(), 43);
    }
}
