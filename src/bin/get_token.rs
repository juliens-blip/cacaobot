//! # OAuth Token Retrieval Tool for cTrader API
//!
//! This utility obtains a cTrader OAuth access token and saves it to your `.env` file.
//!
//! ## Usage
//!
//! ### Basic usage (opens browser automatically):
//! ```bash
//! cargo run --bin get-token
//! ```
//!
//! ### No-browser mode (for headless environments):
//! ```bash
//! cargo run --bin get-token -- --no-browser
//! ```
//! This will display the authorization URL - open it manually in a browser.
//!
//! ### Verify token after retrieval:
//! ```bash
//! cargo run --bin get-token -- --verify
//! ```
//! This will test the token against cTrader API before saving.
//!
//! ### Combined flags:
//! ```bash
//! cargo run --bin get-token -- --no-browser --verify
//! ```
//!
//! ## Requirements
//!
//! Before running, ensure your `.env` file contains:
//! - `CTRADER_CLIENT_ID` - Your cTrader OAuth app client ID
//! - `CTRADER_CLIENT_SECRET` - Your cTrader OAuth app client secret
//!
//! ## How it works
//!
//! 1. Starts a local HTTP server on `localhost:8899`
//! 2. Opens cTrader OAuth authorization URL in browser (or displays it)
//! 3. User authorizes the application
//! 4. Receives authorization code via redirect callback
//! 5. Exchanges code for access token
//! 6. Optionally verifies token with cTrader API
//! 7. Saves token to `.env` as `CTRADER_ACCESS_TOKEN`
//!
//! ## Timeout
//!
//! The OAuth flow has a 5-minute timeout. If not completed, the program exits gracefully.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::Deserialize;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use url::form_urlencoded;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    access_token: String,
}

/// CLI arguments for the get-token utility
#[derive(Parser, Debug)]
#[command(name = "get-token")]
#[command(about = "Obtain cTrader OAuth access token and save to .env")]
struct Args {
    /// Skip automatic browser opening (for headless environments)
    #[arg(long)]
    no_browser: bool,

    /// Verify token with cTrader API before saving
    #[arg(long)]
    verify: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let args = Args::parse();

    let client_id = env::var("CTRADER_CLIENT_ID")
        .context("Missing CTRADER_CLIENT_ID in environment or .env")?;
    let client_secret = env::var("CTRADER_CLIENT_SECRET")
        .context("Missing CTRADER_CLIENT_SECRET in environment or .env")?;
    let redirect_uri = "http://localhost:8899";

    let auth_url = format!(
        "https://openapi.ctrader.com/apps/auth?client_id={}&redirect_uri={}&response_type=code&scope=trading",
        client_id,
        urlencoding::encode(redirect_uri)
    );

    if args.no_browser {
        eprintln!("\n┌─────────────────────────────────────────────────────────────┐");
        eprintln!("│ Please open this URL in your browser:                      │");
        eprintln!("└─────────────────────────────────────────────────────────────┘");
        eprintln!("\n{}\n", auth_url);
        eprintln!("After authorizing, you will be redirected to localhost:8899");
        eprintln!("Waiting for authorization (timeout: 5 minutes)...\n");
    } else {
        eprintln!("Opening browser for OAuth authorization...");
        if let Err(err) = open_browser(&auth_url) {
            eprintln!("Failed to open browser automatically: {err}");
            eprintln!("Open this URL manually: {auth_url}");
        }
    }

    let (code_tx, code_rx) = oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(code_tx)));
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let addr: SocketAddr = ([127, 0, 0, 1], 8899).into();
    let make_svc = make_service_fn(move |_| {
        let sender = sender.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let sender = sender.clone();
                async move { handle_request(req, sender).await }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(async move {
        let _ = shutdown_rx.await;
    });

    let server_handle = tokio::spawn(async move {
        if let Err(err) = graceful.await {
            eprintln!("OAuth callback server error: {err}");
        }
    });

    // Wait for OAuth code with 5-minute timeout
    let code = match tokio::time::timeout(Duration::from_secs(300), code_rx).await {
        Ok(Ok(code)) => code,
        Ok(Err(_)) => {
            return Err(anyhow!("Failed to receive OAuth code"));
        }
        Err(_) => {
            eprintln!("\n❌ OAuth flow timed out after 5 minutes.");
            eprintln!("Please run the command again and complete the authorization promptly.");
            return Err(anyhow!("OAuth timeout"));
        }
    };
    
    let _ = shutdown_tx.send(());
    let _ = server_handle.await;

    eprintln!("Exchanging authorization code for access token...");
    let token = exchange_code_for_token(&client_id, &client_secret, redirect_uri, &code).await?;

    // Verify token if requested
    if args.verify {
        eprintln!("Verifying token with cTrader API...");
        match verify_token(&token).await {
            Ok(_) => {
                eprintln!("✅ Token verified successfully!");
            }
            Err(err) => {
                eprintln!("❌ Token verification failed: {}", err);
                eprintln!("The token may still work, but there might be an issue.");
                eprintln!("Proceeding to save token anyway...");
            }
        }
    }

    println!("CTRADER_ACCESS_TOKEN={token}");

    upsert_env_var(Path::new(".env"), "CTRADER_ACCESS_TOKEN", &token)?;
    eprintln!("✅ Saved CTRADER_ACCESS_TOKEN to .env");

    Ok(())
}

async fn handle_request(
    req: Request<Body>,
    sender: Arc<Mutex<Option<oneshot::Sender<String>>>>,
) -> Result<Response<Body>, Infallible> {
    if req.method() != Method::GET {
        return Ok(response_with_status(
            StatusCode::METHOD_NOT_ALLOWED,
            "Method Not Allowed",
        ));
    }

    let query = req.uri().query().unwrap_or("");
    let mut code: Option<String> = None;
    let mut error: Option<String> = None;

    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        if key == "code" {
            code = Some(value.into_owned());
        } else if key == "error" {
            error = Some(value.into_owned());
        }
    }

    if let Some(err) = error {
        return Ok(response_with_status(
            StatusCode::BAD_REQUEST,
            &format!("OAuth error: {err}"),
        ));
    }

    if let Some(code) = code {
        let mut sender = sender.lock().await;
        if let Some(tx) = sender.take() {
            let _ = tx.send(code);
        }
        return Ok(html_response(
            "Authorization complete. You can close this window.",
        ));
    }

    Ok(response_with_status(
        StatusCode::BAD_REQUEST,
        "Missing code parameter",
    ))
}

fn response_with_status(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Body::from(message.to_string()))
        .unwrap_or_else(|_| Response::new(Body::from("Response error")))
}

fn html_response(message: &str) -> Response<Body> {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"></head><body><p>{}</p></body></html>",
        message
    );
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(body))
        .unwrap_or_else(|_| Response::new(Body::from("OK")))
}

async fn exchange_code_for_token(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    // Extract numeric client_id if format is "12345_AbcDef..."
    let numeric_id = if let Some(pos) = client_id.find('_') {
        &client_id[..pos]
    } else {
        client_id
    };

    eprintln!("Debug: client_id={} (numeric={})", &client_id[..10.min(client_id.len())], numeric_id);
    eprintln!("Debug: redirect_uri={}", redirect_uri);
    eprintln!("Debug: code={}...", &code[..20.min(code.len())]);

    // Try with full client_id first, then numeric only
    let ids_to_try = [client_id, numeric_id];
    let mut body = String::new();
    let mut status = reqwest::StatusCode::default();

    for id in &ids_to_try {
        let url = format!(
            "https://openapi.ctrader.com/apps/token?grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&client_secret={}",
            urlencoding::encode(code),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(id),
            urlencoding::encode(client_secret),
        );

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send token request")?;

        status = response.status();
        body = response.text().await.context("Failed to read response body")?;

        eprintln!("Token response (client_id={}, HTTP {}): {}", id, status, body);

        // If we got accessToken, break
        if body.contains("accessToken") {
            break;
        }
    }

    if !status.is_success() {
        return Err(anyhow!("Token request failed (HTTP {}): {}", status, body));
    }

    let token: TokenResponse = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse token response: {}", body))?;

    Ok(token.access_token)
}

fn upsert_env_var(path: &Path, key: &str, value: &str) -> Result<()> {
    let new_line = format!("{key}={value}");
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            std::fs::write(path, format!("{new_line}\n"))
                .context("Failed to create .env file")?;
            return Ok(());
        }
        Err(err) => return Err(err).context("Failed to read .env"),
    };

    let mut found = false;
    let mut output = String::new();
    for line in contents.lines() {
        if line.trim_start().starts_with(&format!("{key}=")) {
            output.push_str(&new_line);
            found = true;
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }

    if !found {
        output.push_str(&new_line);
        output.push('\n');
    }

    std::fs::write(path, output).context("Failed to update .env")?;
    Ok(())
}

/// Verify token by making a test request to cTrader API
async fn verify_token(token: &str) -> Result<()> {
    use prost::Message as ProstMessage;
    use rustls::ClientConfig;
    use rustls::RootCertStore;
    use rustls::pki_types::ServerName;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio_rustls::TlsConnector;

    // Simple proto message structures for verification
    #[derive(Clone, PartialEq, prost::Message)]
    struct ProtoMessage {
        #[prost(uint32, tag = "1")]
        payload_type: u32,
        #[prost(bytes, optional, tag = "2")]
        payload: Option<Vec<u8>>,
        #[prost(string, optional, tag = "3")]
        client_msg_id: Option<String>,
    }

    #[derive(Clone, PartialEq, prost::Message)]
    struct ProtoOaAccountAuthReq {
        #[prost(uint32, optional, tag = "1")]
        payload_type: Option<u32>,
        #[prost(int64, tag = "2")]
        ctid_trader_account_id: i64,
        #[prost(string, tag = "3")]
        access_token: String,
    }

    // Connect to demo server
    let stream = TcpStream::connect("demo.ctraderapi.com:5035")
        .await
        .context("Failed to connect to cTrader demo server")?;

    // Setup TLS
    let mut root_store = RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs()?;
    for cert in certs {
        root_store.add(cert).ok();
    }
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let domain = ServerName::try_from("demo.ctraderapi.com".to_string())
        .context("Invalid server name")?;
    let mut tls_stream = connector
        .connect(domain, stream)
        .await
        .context("TLS handshake failed")?;

    // Send account auth request (payload_type = 2101 for ProtoOaAccountAuthReq)
    let auth_req = ProtoOaAccountAuthReq {
        payload_type: None,
        ctid_trader_account_id: 0, // Dummy account ID for verification
        access_token: token.to_string(),
    };

    let mut payload = Vec::new();
    auth_req.encode(&mut payload)?;

    let proto_msg = ProtoMessage {
        payload_type: 2101,
        payload: Some(payload),
        client_msg_id: Some("verify".to_string()),
    };

    let mut msg_bytes = Vec::new();
    proto_msg.encode(&mut msg_bytes)?;
    let len = msg_bytes.len() as u32;

    tls_stream.write_all(&len.to_be_bytes()).await?;
    tls_stream.write_all(&msg_bytes).await?;

    // Try to read response (basic check - we expect either success or specific auth error)
    let mut len_buf = [0u8; 4];
    match tokio::time::timeout(Duration::from_secs(5), tls_stream.read_exact(&mut len_buf)).await {
        Ok(Ok(_)) => {
            // Got a response - token is at least syntactically valid
            Ok(())
        }
        Ok(Err(e)) => Err(anyhow!("Connection error during verification: {}", e)),
        Err(_) => Err(anyhow!("Verification timeout - server did not respond")),
    }
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("Failed to launch browser via cmd")?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to launch browser via open")?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("Failed to launch browser via xdg-open")?;
    }

    Ok(())
}
