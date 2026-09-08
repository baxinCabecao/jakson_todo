use crate::backup::types::TokenResponse;
use crate::db::DbConnection;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

#[allow(dead_code)]
pub const DESKTOP_GDRIVE_CLIENT_ID: &str = match option_env!("GDRIVE_DESKTOP_CLIENT_ID") {
    Some(val) => val,
    None => "",
};

#[allow(dead_code)]
pub const DESKTOP_GDRIVE_CLIENT_SECRET: &str = match option_env!("GDRIVE_DESKTOP_CLIENT_SECRET") {
    Some(val) => val,
    None => "",
};

#[allow(dead_code)]
pub const ANDROID_GDRIVE_CLIENT_ID: &str = match option_env!("GDRIVE_ANDROID_CLIENT_ID") {
    Some(val) => val,
    None => "",
};

#[allow(dead_code)]
pub const IOS_GDRIVE_CLIENT_ID: &str = match option_env!("GDRIVE_IOS_CLIENT_ID") {
    Some(val) => val,
    None => "",
};

pub const DEFAULT_ONEDRIVE_CLIENT_ID: &str = match option_env!("ONEDRIVE_CLIENT_ID") {
    Some(val) => val,
    None => "",
};

#[allow(dead_code)]
pub const OAUTH_PORT: u16 = 59135;
#[allow(dead_code)]
pub const MOBILE_REDIRECT_URI: &str = "com.baxin.jaksontodo:/oauth2redirect";

#[derive(Clone, Debug)]
pub struct PendingOAuthSession {
    pub provider: String,
    pub client_id: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    #[allow(dead_code)]
    pub created_at: Instant,
}

static PENDING_OAUTH: Mutex<Option<PendingOAuthSession>> = Mutex::new(None);

pub fn get_effective_gdrive_client_id() -> String {
    #[cfg(target_os = "android")]
    {
        ANDROID_GDRIVE_CLIENT_ID.to_string()
    }
    #[cfg(target_os = "ios")]
    {
        IOS_GDRIVE_CLIENT_ID.to_string()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        // Desktop: Linux, Windows, macOS
        DESKTOP_GDRIVE_CLIENT_ID.to_string()
    }
}

pub fn get_effective_gdrive_client_secret() -> Option<&'static str> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        if !DESKTOP_GDRIVE_CLIENT_SECRET.is_empty() {
            return Some(DESKTOP_GDRIVE_CLIENT_SECRET);
        }
    }
    None
}

pub fn get_effective_onedrive_client_id() -> String {
    DEFAULT_ONEDRIVE_CLIENT_ID.to_string()
}

pub fn get_redirect_uri() -> String {
    #[cfg(target_os = "android")]
    {
        MOBILE_REDIRECT_URI.to_string()
    }
    #[cfg(target_os = "ios")]
    {
        MOBILE_REDIRECT_URI.to_string()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        format!("http://localhost:{}", OAUTH_PORT)
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
fn build_authorization_url(provider: &str, client_id: &str, challenge: &str, redirect_uri: &str) -> String {
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

/// Start OAuth flow across Desktop and Mobile
pub async fn start_oauth_flow(
    provider: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let is_gdrive = provider.to_lowercase() == "gdrive";
    let client_id = if is_gdrive {
        get_effective_gdrive_client_id()
    } else {
        get_effective_onedrive_client_id()
    };

    if client_id.is_empty() {
        return Err(format!("Client ID para {} não está configurado.", provider));
    }

    let (code_verifier, code_challenge) = generate_pkce();
    let redirect_uri = get_redirect_uri();
    let auth_url = build_authorization_url(&provider, &client_id, &code_challenge, &redirect_uri);

    // Save pending session for callback matching
    if let Ok(mut lock) = PENDING_OAUTH.lock() {
        *lock = Some(PendingOAuthSession {
            provider: provider.clone(),
            client_id: client_id.clone(),
            code_verifier: code_verifier.clone(),
            redirect_uri: redirect_uri.clone(),
            created_at: Instant::now(),
        });
    }

    // Persist to database to survive Android activity/process recreation
    if let Ok(app_dir) = app_handle.path().app_data_dir() {
        let db = DbConnection::new(app_dir);
        let _ = db.save_setting("pending_oauth_provider", &provider);
        let _ = db.save_setting("pending_oauth_client_id", &client_id);
        let _ = db.save_setting("pending_oauth_code_verifier", &code_verifier);
        let _ = db.save_setting("pending_oauth_redirect_uri", &redirect_uri);
    }

    // On Desktop (Linux, Windows, macOS), start local TCP listener on port 59135
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    start_local_tcp_listener(
        provider.clone(),
        client_id.clone(),
        code_verifier.clone(),
        redirect_uri.clone(),
        app_handle.clone(),
    );

    // Open browser with generated authorization URL
    let _ = app_handle.opener().open_url(&auth_url, None::<&str>);

    Ok(())
}

/// Process OAuth callback from deep link / custom URL scheme
pub async fn process_oauth_callback_url(raw_url: &str, app_handle: AppHandle) -> Result<(), String> {
    println!("[OAuth] Processing callback URL: {}", raw_url);

    // Check for error in callback URL
    if let Some(error) = extract_param_from_url(raw_url, "error") {
        let desc = extract_param_from_url(raw_url, "error_description")
            .map(|d| format!(": {}", d))
            .unwrap_or_default();
        let err_msg = format!("Erro retornado pelo provedor ({}{})", error, desc);
        let _ = app_handle.emit("oauth-error", &err_msg);
        return Err(err_msg);
    }

    let code = match extract_param_from_url(raw_url, "code") {
        Some(c) if !c.is_empty() => c,
        _ => {
            let err_msg = "Código de autorização não encontrado na URL recebida.".to_string();
            let _ = app_handle.emit("oauth-error", &err_msg);
            return Err(err_msg);
        }
    };

    let session = {
        let in_memory = PENDING_OAUTH.lock().ok().and_then(|mut lock| lock.take());
        if let Some(s) = in_memory {
            s
        } else {
            // Fallback to SQLite settings table
            let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            let db = DbConnection::new(app_dir);
            let conn = db.get_conn().map_err(|e| e.to_string())?;
            let get_opt = |key: &str| -> Option<String> {
                conn.query_row("SELECT value FROM settings WHERE key = ?1", rusqlite::params![key], |r| r.get(0)).ok()
            };
            let provider = get_opt("pending_oauth_provider")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Nenhuma sessão OAuth pendente encontrada.".to_string())?;
            let client_id = get_opt("pending_oauth_client_id")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Client ID da sessão OAuth não encontrado.".to_string())?;
            let code_verifier = get_opt("pending_oauth_code_verifier")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Code verifier da sessão OAuth não encontrado.".to_string())?;
            let redirect_uri = get_opt("pending_oauth_redirect_uri")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Redirect URI da sessão OAuth não encontrado.".to_string())?;

            PendingOAuthSession {
                provider,
                client_id,
                code_verifier,
                redirect_uri,
                created_at: Instant::now(),
            }
        }
    };

    // Clean up pending settings from SQLite
    if let Ok(app_dir) = app_handle.path().app_data_dir() {
        let db = DbConnection::new(app_dir);
        let _ = db.save_setting("pending_oauth_provider", "");
        let _ = db.save_setting("pending_oauth_client_id", "");
        let _ = db.save_setting("pending_oauth_code_verifier", "");
        let _ = db.save_setting("pending_oauth_redirect_uri", "");
    }

    let provider = session.provider;
    match exchange_code_for_tokens(
        &provider,
        &session.client_id,
        &code,
        &session.code_verifier,
        &session.redirect_uri,
    )
    .await
    {
        Ok((_access_token, refresh_token)) => {
            save_tokens_to_db(&app_handle, &provider, session.client_id, refresh_token)?;
            let _ = app_handle.emit("oauth-success", &provider);
            Ok(())
        }
        Err(e) => {
            let _ = app_handle.emit("oauth-error", format!("Falha na troca do token: {}", e));
            Err(e)
        }
    }
}

fn save_tokens_to_db(
    app_handle: &AppHandle,
    provider: &str,
    client_id: String,
    refresh_token: Option<String>,
) -> Result<(), String> {
    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let db = DbConnection::new(app_dir);
    let mut settings = db.get_settings().map_err(|e| e.to_string())?;

    if provider.to_lowercase() == "gdrive" {
        settings.gdrive_client_id = Some(client_id);
        settings.gdrive_refresh_token = refresh_token;
        settings.gdrive_enabled = true;
    } else {
        settings.onedrive_client_id = Some(client_id);
        settings.onedrive_refresh_token = refresh_token;
        settings.onedrive_enabled = true;
    }

    db.save_settings(settings).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn extract_param_from_url(raw_input: &str, param_name: &str) -> Option<String> {
    let target = if let Some(line) = raw_input.lines().next() {
        let trimmed = line.trim();
        if trimmed.starts_with("GET ") || trimmed.starts_with("POST ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                parts[1]
            } else {
                trimmed
            }
        } else {
            trimmed
        }
    } else {
        raw_input.trim()
    };

    // 1. Direct query string parse from '?'
    if let Some(pos) = target.find('?') {
        let query_str = &target[pos + 1..];
        for (key, val) in url::form_urlencoded::parse(query_str.as_bytes()) {
            if key == param_name {
                return Some(val.into_owned());
            }
        }
    }

    // 2. Try parsing with url::Url
    let full_url_str = if target.starts_with('/') {
        format!("http://localhost:59135{}", target)
    } else {
        target.to_string()
    };

    if let Ok(parsed) = url::Url::parse(&full_url_str) {
        for (key, val) in parsed.query_pairs() {
            if key == param_name {
                return Some(val.into_owned());
            }
        }
    }

    // 3. Fallback: search for param_name= in string
    let pattern = format!("{}=", param_name);
    if let Some(pos) = target.find(&pattern) {
        let rest = &target[pos + pattern.len()..];
        let end = rest.find('&').or_else(|| rest.find(' ')).or_else(|| rest.find('#')).unwrap_or(rest.len());
        let raw_val = &rest[..end];
        return url::form_urlencoded::parse(format!("v={}", raw_val).as_bytes())
            .next()
            .map(|(_, val)| val.into_owned());
    }

    None
}

#[allow(dead_code)]
fn extract_code_from_url(raw_input: &str) -> String {
    extract_param_from_url(raw_input, "code").unwrap_or_default()
}

pub async fn exchange_code_for_tokens(
    provider: &str,
    client_id: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<(String, Option<String>), String> {
    let client = reqwest::Client::new();
    let is_gdrive = provider.to_lowercase() == "gdrive";

    let (token_url, params) = if is_gdrive {
        let mut p = vec![
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", code_verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];
        if let Some(sec) = get_effective_gdrive_client_secret() {
            p.push(("client_secret", sec));
        }
        ("https://oauth2.googleapis.com/token", p)
    } else {
        (
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            vec![
                ("client_id", client_id),
                ("code", code),
                ("code_verifier", code_verifier),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri),
                ("scope", "files.readwrite offline_access"),
            ],
        )
    };

    println!("[OAuth] Exchanging code with endpoint: {}", token_url);
    println!("[OAuth] Provider: {}, Client ID: {}, Redirect URI: {}", provider, client_id, redirect_uri);

    let res = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Falha na requisição HTTP OAuth: {}", e))?;

    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    println!("[OAuth] Token response status: {}, body: {}", status, body);

    if !status.is_success() {
        return Err(format!("Erro ao obter token (HTTP {}): {}", status, body));
    }

    let token_resp: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Falha ao processar JSON da resposta ({})", e))?;

    Ok((token_resp.access_token, token_resp.refresh_token))
}

#[allow(dead_code)]
fn start_local_tcp_listener(
    provider: String,
    client_id: String,
    code_verifier: String,
    redirect_uri: String,
    app_handle: AppHandle,
) {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    let listener = match TcpListener::bind(format!("127.0.0.1:{}", OAUTH_PORT)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Não foi possível escutar na porta {}: {}", OAUTH_PORT, e);
            let _ = app_handle.emit("oauth-error", format!("Porta {} ocupada: {}", OAUTH_PORT, e));
            return;
        }
    };
    let _ = listener.set_nonblocking(true);

    tauri::async_runtime::spawn(async move {
        let start_time = Instant::now();
        let mut stream = None;

        while start_time.elapsed().as_secs() < 300 {
            match listener.accept() {
                Ok((s, _)) => {
                    stream = Some(s);
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    eprintln!("Erro ao aceitar conexão TCP: {}", e);
                    break;
                }
            }
        }

        let mut stream = match stream {
            Some(s) => s,
            None => return,
        };

        let mut buffer = [0; 4096];
        let size = stream.read(&mut buffer).unwrap_or(0);
        let request_str = String::from_utf8_lossy(&buffer[..size]).to_string();
        let code = extract_code_from_url(&request_str);

        if code.is_empty() {
            let err_html = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                <html><head><style>body { font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background-color: #0f172a; color: #f8fafc; } .card { background: #1e293b; padding: 2rem; border-radius: 12px; text-align: center; } h1 { color: #f43f5e; }</style></head>\
                <body><div class='card'><h1>Falha na Autenticação</h1><p>Nenhum código de autorização encontrado.</p></div></body></html>";
            let _ = stream.write_all(err_html.as_bytes());
            let _ = stream.flush();
            let _ = app_handle.emit("oauth-error", format!("Não foi possível obter o código de autorização para {}", provider));
            return;
        }

        let provider_name = if provider.to_lowercase() == "gdrive" { "Google Drive" } else { "OneDrive" };

        match exchange_code_for_tokens(
            &provider,
            &client_id,
            &code,
            &code_verifier,
            &redirect_uri,
        ).await {
            Ok((_access_token, refresh_token)) => {
                if let Err(e) = save_tokens_to_db(&app_handle, &provider, client_id, refresh_token) {
                    eprintln!("Erro ao salvar no banco: {}", e);
                    let err_html = format!(
                        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                        <html><head><style>body {{ font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background-color: #0f172a; color: #f8fafc; }} .card {{ background: #1e293b; padding: 2rem; border-radius: 12px; text-align: center; }} h1 {{ color: #f43f5e; }}</style></head>\
                        <body><div class='card'><h1>Erro ao Salvar Configurações</h1><p>{}</p></div></body></html>",
                        e
                    );
                    let _ = stream.write_all(err_html.as_bytes());
                    let _ = stream.flush();
                    let _ = app_handle.emit("oauth-error", format!("Erro ao salvar tokens: {}", e));
                } else {
                    let success_html = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                        <html><head><style>body {{ font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background-color: #0f172a; color: #f8fafc; }} .card {{ background: #1e293b; padding: 2rem; border-radius: 12px; text-align: center; }} h1 {{ color: #10b981; }}</style></head>\
                        <body><div class='card'><h1>Conexão com {} Realizada!</h1><p>Você já pode fechar esta aba e retornar ao aplicativo Jakson ToDo.</p></div></body></html>",
                        provider_name
                    );
                    let _ = stream.write_all(success_html.as_bytes());
                    let _ = stream.flush();
                    let _ = app_handle.emit("oauth-success", provider);
                }
            }
            Err(e) => {
                eprintln!("Erro na troca de código por token: {}", e);
                let err_html = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                    <html><head><style>body {{ font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; background-color: #0f172a; color: #f8fafc; }} .card {{ background: #1e293b; padding: 2rem; border-radius: 12px; text-align: center; }} h1 {{ color: #f43f5e; }}</style></head>\
                    <body><div class='card'><h1>Falha na Autenticação</h1><p>{}</p></div></body></html>",
                    e
                );
                let _ = stream.write_all(err_html.as_bytes());
                let _ = stream.flush();
                let _ = app_handle.emit("oauth-error", format!("Falha na troca do token: {}", e));
            }
        }
    });
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

    #[test]
    fn test_extract_param_from_urls() {
        // 1. Android Custom Scheme single slash
        let url1 = "com.baxin.jaksontodo:/oauth2redirect?code=4/0AbCd123_xyz&scope=drive";
        assert_eq!(extract_param_from_url(url1, "code"), Some("4/0AbCd123_xyz".to_string()));

        // 2. Android Custom Scheme double slash and urlencoded
        let url2 = "com.baxin.jaksontodo://oauth2redirect?code=4%2F0AbCd123_xyz&state=123";
        assert_eq!(extract_param_from_url(url2, "code"), Some("4/0AbCd123_xyz".to_string()));

        // 3. HTTP GET request line (Desktop)
        let url3 = "GET /?code=4/0AbCd123_xyz&scope=offline HTTP/1.1";
        assert_eq!(extract_param_from_url(url3, "code"), Some("4/0AbCd123_xyz".to_string()));

        // 4. Error response
        let url4 = "com.baxin.jaksontodo:/oauth2redirect?error=access_denied&error_description=User+cancelled";
        assert_eq!(extract_param_from_url(url4, "error"), Some("access_denied".to_string()));
        assert_eq!(extract_param_from_url(url4, "error_description"), Some("User cancelled".to_string()));
        assert_eq!(extract_param_from_url(url4, "code"), None);
    }

    #[test]
    fn test_dotenv_loaded_constants() {
        assert!(!DESKTOP_GDRIVE_CLIENT_ID.is_empty(), "DESKTOP_GDRIVE_CLIENT_ID deve ser carregado do .env");
        assert!(!DESKTOP_GDRIVE_CLIENT_SECRET.is_empty(), "DESKTOP_GDRIVE_CLIENT_SECRET deve ser carregado do .env");
        assert!(!ANDROID_GDRIVE_CLIENT_ID.is_empty(), "ANDROID_GDRIVE_CLIENT_ID deve ser carregado do .env");
    }
}
