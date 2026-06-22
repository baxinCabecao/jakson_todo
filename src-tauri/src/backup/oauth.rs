use crate::db::DbConnection;
use crate::backup::types::TokenResponse;
use std::net::TcpListener;
use std::io::{Read, Write};
use tauri::{AppHandle, Emitter, Manager};

/// Start a temporary HTTP server on localhost to capture the OAuth redirect code.
pub async fn start_oauth_flow(
    provider: String,
    client_id: String,
    client_secret: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let port = 59135;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;
    
    // Set non-blocking to allow us to handle timeouts or async operations
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;

    let provider_clone = provider.clone();
    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        let mut stream_opt = None;
        let start_time = std::time::Instant::now();
        
        // Wait for connection with a 5-minute timeout
        while start_time.elapsed().as_secs() < 300 {
            match listener.accept() {
                Ok((stream, _addr)) => {
                    stream_opt = Some(stream);
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => {
                    eprintln!("Error accepting OAuth connection: {}", e);
                    break;
                }
            }
        }

        let mut stream = match stream_opt {
            Some(s) => s,
            None => {
                let _ = app_handle_clone.emit("oauth-error", format!("OAuth flow for {} timed out after 5 minutes.", provider_clone));
                return;
            }
        };

        // Read request bytes
        let mut buffer = [0; 2048];
        let mut request_str = String::new();
        match stream.read(&mut buffer) {
            Ok(size) if size > 0 => {
                request_str = String::from_utf8_lossy(&buffer[..size]).to_string();
            }
            _ => {}
        }

        // Parse code from request (e.g. GET /?code=... HTTP/1.1)
        let code = extract_code_from_request(&request_str);
        
        if code.is_empty() {
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<h1>Authentication Failed</h1><p>No authorization code found in redirect.</p>";
            let _ = stream.write_all(response.as_bytes());
            let _ = app_handle_clone.emit("oauth-error", format!("Failed to obtain authentication code for {}", provider_clone));
            return;
        }

        // Respond to user browser
        let success_html = format!(
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
            provider_clone
        );
        let _ = stream.write_all(success_html.as_bytes());
        let _ = stream.flush();

        // Exchange code for token
        match exchange_code_for_tokens(&provider_clone, &client_id, &client_secret, &code).await {
            Ok((_access_token, refresh_token)) => {
                // Save tokens to DB
                let db = DbConnection::new(app_handle_clone.path().app_data_dir().unwrap());
                if let Ok(mut settings) = db.get_settings() {
                    if provider_clone.to_lowercase() == "gdrive" {
                        settings.gdrive_client_id = Some(client_id);
                        settings.gdrive_client_secret = Some(client_secret);
                        settings.gdrive_refresh_token = refresh_token.clone();
                        settings.gdrive_enabled = true;
                    } else {
                        settings.onedrive_client_id = Some(client_id);
                        settings.onedrive_client_secret = Some(client_secret);
                        settings.onedrive_refresh_token = refresh_token.clone();
                        settings.onedrive_enabled = true;
                    }
                    let _ = db.save_settings(settings);
                }

                // Emit event to frontend
                let _ = app_handle_clone.emit("oauth-success", provider_clone);
            }
            Err(e) => {
                let _ = app_handle_clone.emit("oauth-error", format!("Token exchange failed: {}", e));
            }
        }
    });

    Ok(())
}

fn extract_code_from_request(request: &str) -> String {
    // Look for "?code=" or "&code="
    if let Some(start) = request.find("code=") {
        let code_part = &request[start + 5..];
        if let Some(end) = code_part.find(' ') {
            let raw_code = &code_part[..end];
            // Split off any other query params like &state=
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
    client_secret: &str,
    code: &str,
) -> Result<(String, Option<String>), String> {
    let client = reqwest::Client::new();
    let port = 59135;
    let redirect_uri = format!("http://localhost:{}", port);

    if provider.to_lowercase() == "gdrive" {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &redirect_uri),
        ];

        let res = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Google OAuth Request Failed: {}", e))?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Google token exchange failed: {}", err_text));
        }

        let token_resp: TokenResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse Google token response: {}", e))?;

        Ok((token_resp.access_token, token_resp.refresh_token))
    } else {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &redirect_uri),
            ("scope", "files.readwrite offline_access"),
        ];

        let res = client
            .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("OneDrive OAuth Request Failed: {}", e))?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("OneDrive token exchange failed: {}", err_text));
        }

        let token_resp: TokenResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse OneDrive token response: {}", e))?;

        Ok((token_resp.access_token, token_resp.refresh_token))
    }
}
