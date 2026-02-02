//! cTrader Open API client
//!
//! This module provides a Rust client for the cTrader Open API using Protobuf over TLS.
//! Documentation: https://help.ctrader.com/open-api/

use crate::config::CTraderConfig;
use crate::error::{CTraderError, Result};
use prost::Message as ProstMessage;
use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, timeout};
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info, warn};

use super::protobuf::*;
use super::oauth::{OAuthManager, OAuthConfig, FileTokenStorage, Environment};

/// cTrader environment (Demo or Live)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CTraderEnvironment {
    #[default]
    Demo,
    Live,
}

impl CTraderEnvironment {
    /// Get the server endpoint for this environment
    pub fn server_endpoint(&self) -> &'static str {
        match self {
            CTraderEnvironment::Demo => "demo.ctraderapi.com",
            CTraderEnvironment::Live => "live.ctraderapi.com",
        }
    }

    /// Get the default port (same for both environments)
    pub fn default_port(&self) -> u16 {
        5035
    }

    /// Check if this is a production environment
    pub fn is_live(&self) -> bool {
        matches!(self, CTraderEnvironment::Live)
    }
}

impl std::str::FromStr for CTraderEnvironment {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "demo" => Ok(CTraderEnvironment::Demo),
            "live" | "production" | "prod" => Ok(CTraderEnvironment::Live),
            _ => Err(()),
        }
    }
}

impl fmt::Display for CTraderEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CTraderEnvironment::Demo => write!(f, "DEMO"),
            CTraderEnvironment::Live => write!(f, "LIVE"),
        }
    }
}

/// Price information for a symbol
#[derive(Debug, Clone)]
pub struct Price {
    pub symbol_id: i64,
    pub bid: f64,
    pub ask: f64,
    pub spread: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Position information
#[derive(Debug, Clone)]
pub struct Position {
    pub position_id: i64,
    pub symbol_id: i64,
    pub volume: i64,
    pub side: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub profit: f64,
}

/// Order ticket for placing orders
#[derive(Debug, Clone)]
pub struct OrderTicket {
    pub symbol_id: i64,
    pub side: ProtoOaTradeSide,
    pub volume: i64, // in cents: 1 lot = 100 (volume in 0.01 units)
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub label: Option<String>,
}

/// cTrader API client
pub struct CTraderClient {
    config: CTraderConfig,
    environment: CTraderEnvironment,
    stream: Arc<Mutex<Option<TlsStream<TcpStream>>>>,
    access_token: Arc<RwLock<Option<String>>>,
    prices: Arc<RwLock<HashMap<i64, Price>>>,
    positions: Arc<RwLock<HashMap<i64, Position>>>,
    authenticated: Arc<RwLock<bool>>,
    message_tx: mpsc::UnboundedSender<ProtoMessage>,
    message_rx: Arc<Mutex<mpsc::UnboundedReceiver<ProtoMessage>>>,
    pending_messages: Arc<Mutex<HashMap<u32, VecDeque<ProtoMessage>>>>,
    reader_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    oauth_manager: Option<Arc<OAuthManager>>,
    subscribed_symbols: Arc<RwLock<Vec<i64>>>,
}

impl CTraderClient {
    /// Create a new cTrader client for DEMO environment (default)
    pub fn new(config: CTraderConfig) -> Self {
        Self::with_environment(config, CTraderEnvironment::Demo)
    }

    /// Create a new cTrader client with specified environment
    pub fn with_environment(config: CTraderConfig, environment: CTraderEnvironment) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        if environment.is_live() {
            warn!("⚠️  LIVE TRADING MODE - Real money at risk!");
        }

        info!("Creating cTrader client for {} environment", environment);

        // Initialize OAuth manager if in LIVE mode
        let oauth_manager = if environment.is_live() {
            let oauth_env = match environment {
                CTraderEnvironment::Live => Environment::Live,
                CTraderEnvironment::Demo => Environment::Demo,
            };

            let oauth_config = OAuthConfig {
                client_id: config.active_client_id().to_string(),
                client_secret: config.active_client_secret().to_string(),
                redirect_uri: oauth_redirect_uri(),
                environment: oauth_env,
            };

            let storage = FileTokenStorage::new("oauth_token.json");
            let manager = OAuthManager::new(oauth_config).with_storage(Box::new(storage));
            
            Some(Arc::new(manager))
        } else {
            None
        };

        CTraderClient {
            config,
            environment,
            stream: Arc::new(Mutex::new(None)),
            access_token: Arc::new(RwLock::new(None)),
            prices: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
            authenticated: Arc::new(RwLock::new(false)),
            message_tx,
            message_rx: Arc::new(Mutex::new(message_rx)),
            pending_messages: Arc::new(Mutex::new(HashMap::new())),
            reader_task: Arc::new(RwLock::new(None)),
            oauth_manager,
            subscribed_symbols: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create client from config, auto-detecting environment from server field
    pub fn from_config(config: CTraderConfig) -> Self {
        let environment = if config.server.contains("live") {
            CTraderEnvironment::Live
        } else {
            CTraderEnvironment::Demo
        };
        Self::with_environment(config, environment)
    }

    /// Get the current environment
    pub fn environment(&self) -> CTraderEnvironment {
        self.environment
    }

    /// Validate credentials for the current environment
    pub fn validate_credentials(&self) -> Result<()> {
        if self.config.client_id.is_empty() {
            return Err(CTraderError::AuthFailed("Client ID is required".into()).into());
        }
        if self.config.client_secret.is_empty() {
            return Err(CTraderError::AuthFailed("Client Secret is required".into()).into());
        }
        if self.config.active_account_id().is_empty() {
            return Err(CTraderError::AuthFailed("Account ID is required".into()).into());
        }

        if self.environment.is_live() {
            if !self.config.client_id.chars().all(|c| c.is_alphanumeric() || c == '_') {
                warn!("Client ID contains unexpected characters for LIVE environment");
            }
            info!("✅ LIVE credentials validated (format check only)");
        }

        Ok(())
    }

    /// Verify credentials including access token requirements
    pub fn verify_credentials(&self) -> Result<()> {
        self.validate_credentials()?;

        if self.environment.is_live() {
            return Ok(());
        }

        match self.config.access_token.as_deref() {
            Some(token) if !token.trim().is_empty() => Ok(()),
            _ => Err(CTraderError::AuthFailed(
                "CTRADER_ACCESS_TOKEN is required for DEMO environment".into(),
            )
            .into()),
        }
    }

    /// Connect to cTrader server
    pub async fn connect(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server, self.config.port);
        info!("Connecting to cTrader at {}", addr);

        let tcp_stream = timeout(Duration::from_secs(10), TcpStream::connect(&addr))
            .await
            .map_err(|_| CTraderError::Timeout)?
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        let tls_config = Arc::new(build_tls_config().map_err(|e| {
            CTraderError::ConnectionFailed(format!("TLS config error: {}", e))
        })?);
        let connector = TlsConnector::from(tls_config);
        let server_name = ServerName::try_from(self.config.server.clone())
            .map_err(|_| CTraderError::ConnectionFailed("Invalid TLS server name".into()))?;

        let tls_stream = timeout(Duration::from_secs(10), connector.connect(server_name, tcp_stream))
            .await
            .map_err(|_| CTraderError::Timeout)?
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        info!("Connected to cTrader with TLS");
        *self.stream.lock().await = Some(tls_stream);

        // Start reader task early to avoid concurrent reads later
        self.start_reader().await?;

        Ok(())
    }

    /// Authenticate with cTrader API
    pub async fn authenticate(&self) -> Result<()> {
        info!("Authenticating with cTrader API ({})", self.environment);

        // Step 1: Application authentication
        let app_auth_req = ProtoOaApplicationAuthReq {
            payload_type: None,
            client_id: self.config.active_client_id().to_string(),
            client_secret: self.config.active_client_secret().to_string(),
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaApplicationAuthReq, app_auth_req);
        self.send_message(msg).await?;

        // Wait for application auth response
        let _response =
            self.wait_for_message(ProtoOaPayloadType::ProtoOaApplicationAuthRes).await?;
        debug!("Application authenticated");

        // Step 2: Get access token
        let access_token = if self.environment.is_live() {
            // For LIVE, use OAuth access token
            if let Some(oauth_manager) = &self.oauth_manager {
                info!("Initializing OAuth manager for LIVE environment");
                oauth_manager.init().await?;
                
                if !oauth_manager.is_authenticated().await {
                    return Err(CTraderError::AuthFailed(
                        "No OAuth token available. Please run OAuth flow first. \
                        See docs/OAUTH_PRODUCTION.md for instructions.".into()
                    ).into());
                }
                
                let token = oauth_manager.get_valid_token().await?;
                info!("Using OAuth access token (expires soon check passed)");
                token
            } else {
                return Err(CTraderError::AuthFailed(
                    "OAuth manager not initialized for LIVE environment".into()
                ).into());
            }
        } else {
            // For DEMO, use CTRADER_ACCESS_TOKEN from config
            if let Some(token) = &self.config.access_token {
                info!("Using OAuth access token from CTRADER_ACCESS_TOKEN for DEMO environment");
                token.clone()
            } else {
                return Err(CTraderError::AuthFailed(
                    "Missing CTRADER_ACCESS_TOKEN for DEMO environment.\n\
                    cTrader requires a valid OAuth access token even for DEMO accounts.\n\
                    \n\
                    To obtain a token, run:\n\
                    cargo run --bin get-token\n\
                    \n\
                    Then add the token to your .env file:\n\
                    CTRADER_ACCESS_TOKEN=your_token_here".into()
                ).into());
            }
        };
        
        *self.access_token.write().await = Some(access_token.clone());

        // Step 3: Account authentication
        let account_id = self.config.active_account_id().parse::<i64>()
            .map_err(|e| CTraderError::AuthFailed(format!("Invalid account ID: {}", e)))?;

        let account_auth_req = ProtoOaAccountAuthReq {
            payload_type: None,
            ctid_trader_account_id: account_id,
            access_token,
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaAccountAuthReq, account_auth_req);
        self.send_message(msg).await?;

        // Wait for account auth response
        let _response = self.wait_for_message(ProtoOaPayloadType::ProtoOaAccountAuthRes).await?;
        info!("Account authenticated: {}", account_id);

        *self.authenticated.write().await = true;

        // Start heartbeat and message handling
        self.start_background_tasks().await;

        Ok(())
    }

    /// Get the OAuth manager (if in LIVE mode)
    pub fn oauth_manager(&self) -> Option<&Arc<OAuthManager>> {
        self.oauth_manager.as_ref()
    }

    /// Check if using OAuth authentication (LIVE mode)
    pub fn is_oauth_enabled(&self) -> bool {
        self.oauth_manager.is_some()
    }

    /// Subscribe to price updates for a symbol
    pub async fn subscribe_to_symbol(&self, symbol_id: i64) -> Result<()> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        let account_id = self.config.active_account_id().parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let subscribe_req = ProtoOaSubscribeSpotsReq {
            payload_type: None,
            ctid_trader_account_id: account_id,
            symbol_id: vec![symbol_id],
            subscribe_to_spot_timestamp: Some(true),
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaSubscribeSpotsReq, subscribe_req);
        self.send_message(msg).await?;

        // Track subscribed symbols for reconnection
        let mut symbols = self.subscribed_symbols.write().await;
        if !symbols.contains(&symbol_id) {
            symbols.push(symbol_id);
        }

        info!("Subscribed to symbol: {}", symbol_id);
        Ok(())
    }

    /// Get current price for a symbol
    pub async fn get_price(&self, symbol_id: i64) -> Result<Price> {
        let prices = self.prices.read().await;
        prices
            .get(&symbol_id)
            .cloned()
            .ok_or_else(|| CTraderError::InvalidResponse(format!("No price data for symbol {}", symbol_id)).into())
    }

    /// Place a market order and return (order_id, position_id)
    pub async fn place_order(&self, ticket: OrderTicket) -> Result<(i64, i64)> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        let account_id = self.config.active_account_id().parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let order_req = ProtoOaNewOrderReq {
            payload_type: None,
            ctid_trader_account_id: account_id,
            symbol_id: ticket.symbol_id,
            order_type: ProtoOaOrderType::Market as i32,
            trade_side: ticket.side as i32,
            volume: ticket.volume,
            limit_price: None,
            stop_price: None,
            time_in_force: None,
            expiration_timestamp: None,
            stop_loss: ticket.stop_loss,
            take_profit: ticket.take_profit,
            comment: None,
            base_slippage_price: None,
            slippage_in_points: None,
            label: ticket.label.clone(),
            position_id: None,
            client_order_id: None,
            relative_stop_loss: None,
            relative_take_profit: None,
            guaranteed_stop_loss: None,
            trailing_stop_loss: None,
            stop_trigger_method: None,
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaNewOrderReq, order_req);
        self.send_message(msg).await?;

        info!("Order placed: {:?}", ticket);

        // Wait for execution event
        let response = self.wait_for_message(ProtoOaPayloadType::ProtoOaExecutionEvent).await?;

        // Parse execution event to get order ID and position ID
        if let Some(payload) = response.payload {
            if let Ok(exec_event) = ProtoOaExecutionEvent::decode(payload.as_ref()) {
                let order_id = exec_event.order.as_ref().map(|o| o.order_id).unwrap_or(0);
                let position_id = exec_event.position.as_ref().map(|p| p.position_id).unwrap_or(0);
                info!("Order executed: order_id={} position_id={}", order_id, position_id);
                return Ok((order_id, position_id));
            }
        }

        Err(CTraderError::InvalidResponse("Failed to parse execution event".into()).into())
    }

    /// Get open positions
    pub async fn get_positions(&self) -> Result<Vec<Position>> {
        self.reconcile_positions().await
    }

    /// Reconcile positions from broker
    pub async fn reconcile_positions(&self) -> Result<Vec<Position>> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        let account_id = self
            .config
            .account_id
            .parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let reconcile_req = ProtoOaReconcileReq {
            payload_type: None,
            ctid_trader_account_id: account_id,
            return_protection_orders: None,
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaReconcileReq, reconcile_req);
        self.send_message(msg).await?;

        let response = self.wait_for_message(ProtoOaPayloadType::ProtoOaReconcileRes).await?;

        let mut positions = Vec::new();
        if let Some(payload) = response.payload {
            if let Ok(reconcile_res) = ProtoOaReconcileRes::decode(payload.as_ref()) {
                for pos in &reconcile_res.position {
                    let trade_data = &pos.trade_data;
                    let side = match ProtoOaTradeSide::try_from(trade_data.trade_side) {
                        Ok(ProtoOaTradeSide::Buy) => "BUY".to_string(),
                        Ok(ProtoOaTradeSide::Sell) => "SELL".to_string(),
                        _ => "UNKNOWN".to_string(),
                    };
                    positions.push(Position {
                        position_id: pos.position_id,
                        symbol_id: trade_data.symbol_id,
                        volume: trade_data.volume,
                        side,
                        entry_price: pos.price.unwrap_or(0.0),
                        current_price: 0.0, // Updated via spot events
                        profit: 0.0,        // Calculated from price difference
                    });
                }
            } else {
                warn!("Failed to decode reconcile response payload");
            }
        }

        // Update local cache
        let mut cache = self.positions.write().await;
        cache.clear();
        for position in &positions {
            cache.insert(position.position_id, position.clone());
        }

        Ok(positions)
    }

    /// Close a position
    pub async fn close_position(&self, position_id: i64, volume: i64) -> Result<()> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        let account_id = self.config.active_account_id().parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let close_req = ProtoOaClosePositionReq {
            payload_type: None,
            ctid_trader_account_id: account_id,
            position_id,
            volume,
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaClosePositionReq, close_req);
        self.send_message(msg).await?;

        info!("Position closed: {}", position_id);
        Ok(())
    }

    /// Start continuous reader task to process incoming messages
    pub async fn start_reader(&self) -> Result<()> {
        if self.reader_task.read().await.is_some() {
            return Ok(());
        }

        let stream_arc = self.stream.clone();
        let prices_arc = self.prices.clone();
        let positions_arc = self.positions.clone();
        let message_tx = self.message_tx.clone();
        let pending = self.pending_messages.clone();
        let authenticated_clone = self.authenticated.clone();
        let mut config_clone = self.config.clone();
        let environment = self.environment;
        let subscribed_symbols_clone = self.subscribed_symbols.clone();

        let task = tokio::spawn(async move {
            info!("cTrader reader task started");
            let mut reconnect_attempt = 0u32;
            let mut auth_failure_count = 0u32;

            loop {
                let mut stream_guard = stream_arc.lock().await;
                let stream = match stream_guard.as_mut() {
                    Some(s) => s,
                    None => {
                        warn!("Reader: No active stream, waiting...");
                        drop(stream_guard);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                // Read length prefix (4 bytes, big-endian)
                let mut len_buf = [0u8; 4];
                match timeout(Duration::from_secs(30), stream.read_exact(&mut len_buf)).await {
                    Ok(Ok(_)) => {
                        reconnect_attempt = 0; // Reset on successful read
                    }
                    Ok(Err(e)) => {
                        error!("Connection lost: {}", e);
                        drop(stream_guard);
                        
                        *authenticated_clone.write().await = false;
                        
                        reconnect_attempt += 1;
                        let backoff_secs = std::cmp::min(2u64.pow(reconnect_attempt - 1), 60);
                        warn!("Reconnection attempt {} in {}s...", reconnect_attempt, backoff_secs);
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        
                        match Self::reconnect_internal(
                            &mut config_clone,
                            environment,
                            &stream_arc,
                            &authenticated_clone,
                            &subscribed_symbols_clone,
                        ).await {
                            Ok(_) => {
                                info!("✅ Reconnected successfully");
                                reconnect_attempt = 0;
                                // Reset auth failure counter on successful reconnection
                                auth_failure_count = 0;
                                continue;
                            }
                            Err(reconnect_err) => {
                                error!("Reconnection failed: {}", reconnect_err);
                                if reconnect_attempt >= 10 {
                                    error!("Max reconnection attempts reached, giving up");
                                    break;
                                }
                                continue;
                            }
                        }
                    }
                    Err(_) => {
                        debug!("Reader: Heartbeat timeout - connection still alive");
                        continue;
                    }
                }

                let msg_len = u32::from_be_bytes(len_buf) as usize;
                if msg_len > 1_000_000 {
                    error!("Reader: Message too large: {}", msg_len);
                    break;
                }

                let mut msg_buf = vec![0u8; msg_len];
                if let Err(e) = stream.read_exact(&mut msg_buf).await {
                    error!("Reader: Failed to read message: {}", e);
                    break;
                }

                drop(stream_guard);

                let message = match ProtoMessage::decode(msg_buf.as_ref()) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Reader: Failed to decode message: {}", e);
                        continue;
                    }
                };

                let payload_type = message.payload_type;
                if let Some(msg_type) = payload_type_from_u32(payload_type) {
                    match msg_type {
                        ProtoOaPayloadType::ProtoOaSpotEvent => {
                            if let Some(payload) = &message.payload {
                                if let Ok(spot_event) = ProtoOaSpotEvent::decode(payload.as_ref()) {
                                    Self::handle_spot_event(spot_event, &prices_arc).await;
                                }
                            }
                        }
                        ProtoOaPayloadType::ProtoOaExecutionEvent => {
                            debug!("Reader: Execution event received");
                            if let Some(payload) = &message.payload {
                                if let Ok(exec) = ProtoOaExecutionEvent::decode(payload.as_ref()) {
                                    let side = if let Some(ref pos) = exec.position {
                                        match ProtoOaTradeSide::try_from(pos.trade_data.trade_side) {
                                            Ok(ProtoOaTradeSide::Buy) => "BUY",
                                            Ok(ProtoOaTradeSide::Sell) => "SELL",
                                            _ => "UNKNOWN",
                                        }
                                    } else {
                                        "UNKNOWN"
                                    };
                                    if let Some(ref pos) = exec.position {
                                        let mut positions = positions_arc.write().await;
                                        positions.insert(pos.position_id, Position {
                                            position_id: pos.position_id,
                                            symbol_id: pos.trade_data.symbol_id,
                                            volume: pos.trade_data.volume,
                                            side: side.to_string(),
                                            entry_price: pos.price.unwrap_or(0.0),
                                            current_price: 0.0,
                                            profit: 0.0,
                                        });
                                    }
                                }
                            }
                            let _ = message_tx.send(message);
                        }
                        ProtoOaPayloadType::ProtoOaErrorRes => {
                            if let Some(payload) = &message.payload {
                                if let Ok(err_res) = ProtoOaErrorRes::decode(payload.as_ref()) {
                                    let error_code = &err_res.error_code;
                                    let description = err_res.description.as_deref().unwrap_or("none");
                                    
                                    // Check for authentication failures
                                    if error_code.contains("AUTH_FAILURE") || error_code.contains("CH_CLIENT_AUTH_FAILURE") {
                                        auth_failure_count += 1;
                                        error!(
                                            "❌ AUTHENTICATION FAILED (attempt {}/3): code={} desc={}",
                                            auth_failure_count, error_code, description
                                        );
                                        
                                        if auth_failure_count >= 3 {
                                            error!("❌ CRITICAL: 3 consecutive authentication failures detected!");
                                            error!("❌ Invalid credentials - please verify CLIENT_ID, CLIENT_SECRET, and ACCOUNT_ID in your .env file");
                                            error!("❌ Stopping reconnection attempts to prevent infinite loop");
                                            break; // Exit reader task
                                        }
                                    } else {
                                        error!(
                                            "Reader: cTrader error: code={} desc={}",
                                            error_code, description
                                        );
                                    }
                                }
                            }
                            let _ = message_tx.send(message);
                        }
                        ProtoOaPayloadType::ProtoOaOrderErrorEvent => {
                            if let Some(payload) = &message.payload {
                                if let Ok(err_event) = ProtoOaOrderErrorEvent::decode(payload.as_ref()) {
                                    error!("Reader: Order error: code={:?} desc={:?}", 
                                        err_event.error_code, err_event.description);
                                }
                            }
                            let _ = message_tx.send(message);
                        }
                        _ => {
                            debug!("Reader: Message type {:?} queued", msg_type);
                            let _ = message_tx.send(message);
                        }
                    }
                } else if payload_type == ProtoPayloadType::HeartbeatEvent as i32 as u32 {
                    debug!("Reader: Heartbeat received");
                } else {
                    // Unknown type - queue for troubleshooting
                    debug!("Reader: Unknown message type {} queued", payload_type);
                    let mut pending_guard = pending.lock().await;
                    pending_guard.entry(payload_type).or_default().push_back(message);
                }
            }

            warn!("cTrader reader task stopped");
        });

        *self.reader_task.write().await = Some(task);
        info!("Reader task spawned");
        Ok(())
    }

    /// Handle spot event (price update)
    async fn handle_spot_event(event: ProtoOaSpotEvent, prices: &Arc<RwLock<HashMap<i64, Price>>>) {
        let symbol_id = event.symbol_id;
        // bid/ask are u64 in 1/100000 of unit (e.g. 123000 = 1.23)
        let bid = event.bid.unwrap_or(0) as f64 / 100000.0;
        let ask = event.ask.unwrap_or(0) as f64 / 100000.0;
        let spread = ask - bid;

        let price = Price {
            symbol_id,
            bid,
            ask,
            spread,
            timestamp: chrono::Utc::now(),
        };

        prices.write().await.insert(symbol_id, price);
        debug!("Price update: {} bid={} ask={} spread={}", symbol_id, bid, ask, spread);
    }

    /// Disconnect from server
    pub async fn disconnect(&self) -> Result<()> {
        // Stop reader task
        if let Some(task) = self.reader_task.write().await.take() {
            task.abort();
            info!("Reader task stopped");
        }

        if let Some(mut stream) = self.stream.lock().await.take() {
            stream
                .shutdown()
                .await
                .map_err(|_e| CTraderError::Disconnected)?;
            info!("Disconnected from cTrader");
        }
        *self.authenticated.write().await = false;
        Ok(())
    }

    /// Send a protobuf message
    async fn send_message(&self, message: ProtoMessage) -> Result<()> {
        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard.as_mut()
            .ok_or(CTraderError::Disconnected)?;

        let encoded = encode_with_length(&message);
        stream.write_all(&encoded).await
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        debug!("Sent message type: {}", message.payload_type);
        Ok(())
    }

    /// Wait for a specific message type
    async fn wait_for_message(&self, msg_type: ProtoOaPayloadType) -> Result<ProtoMessage> {
        let type_u32 = msg_type as i32 as u32;
        // Check pending queue first
        {
            let mut pending = self.pending_messages.lock().await;
            if let Some(queue) = pending.get_mut(&type_u32) {
                if let Some(message) = queue.pop_front() {
                    return Ok(message);
                }
            }
        }

        let timeout_duration = Duration::from_secs(30);
        let mut rx = self.message_rx.lock().await;

        let error_type = ProtoOaPayloadType::ProtoOaErrorRes as i32 as u32;
        let order_error_type = ProtoOaPayloadType::ProtoOaOrderErrorEvent as i32 as u32;

        loop {
            match timeout(timeout_duration, rx.recv()).await {
                Ok(Some(message)) => {
                    if message.payload_type == type_u32 {
                        return Ok(message);
                    }

                    // Fail-fast on error responses instead of queuing them
                    if message.payload_type == error_type {
                        if let Some(payload) = &message.payload {
                            if let Ok(err_res) = ProtoOaErrorRes::decode(payload.as_ref()) {
                                let desc = err_res.description.as_deref().unwrap_or("none");
                                return Err(CTraderError::ApiError(format!(
                                    "code={} desc={}", err_res.error_code, desc
                                )).into());
                            }
                        }
                        return Err(CTraderError::ApiError("Unknown error response".into()).into());
                    }

                    if message.payload_type == order_error_type {
                        if let Some(payload) = &message.payload {
                            if let Ok(err_event) = ProtoOaOrderErrorEvent::decode(payload.as_ref()) {
                                let desc = err_event.description.as_deref().unwrap_or("none");
                                return Err(CTraderError::ApiError(format!(
                                    "Order error: code={} desc={}", err_event.error_code, desc
                                )).into());
                            }
                        }
                        return Err(CTraderError::ApiError("Unknown order error".into()).into());
                    }

                    let mut pending = self.pending_messages.lock().await;
                    pending.entry(message.payload_type).or_default().push_back(message);
                }
                Ok(None) => return Err(CTraderError::Disconnected.into()),
                Err(_) => return Err(CTraderError::Timeout.into()),
            }
        }
    }

    /// Start background tasks (heartbeat, message handler)
    async fn start_background_tasks(&self) {
        let stream_clone = self.stream.clone();
        let authenticated_clone = self.authenticated.clone();

        // Heartbeat task
        tokio::spawn(async move {
            let mut heartbeat_interval = interval(Duration::from_secs(25));

            loop {
                heartbeat_interval.tick().await;

                if !*authenticated_clone.read().await {
                    break;
                }

                let mut payload = Vec::new();
                if let Err(err) = (ProtoHeartbeatEvent { payload_type: None }).encode(&mut payload) {
                    error!("Failed to encode heartbeat payload: {}", err);
                    break;
                }
                let heartbeat = ProtoMessage {
                    payload_type: ProtoPayloadType::HeartbeatEvent as i32 as u32,
                    payload: Some(payload),
                    client_msg_id: None,
                };

                let mut stream_guard = stream_clone.lock().await;
                if let Some(stream) = stream_guard.as_mut() {
                    let encoded = encode_with_length(&heartbeat);
                    if let Err(e) = stream.write_all(&encoded).await {
                        error!("Failed to send heartbeat: {}", e);
                        break;
                    }
                    debug!("Heartbeat sent");
                }
            }
        });

        info!("Background tasks started");
    }

    /// Check if client is authenticated
    pub async fn is_authenticated(&self) -> bool {
        *self.authenticated.read().await
    }

    /// Internal reconnection logic (static to avoid self borrow issues)
    async fn reconnect_internal(
        config: &mut CTraderConfig,
        environment: CTraderEnvironment,
        stream: &Arc<Mutex<Option<TlsStream<TcpStream>>>>,
        authenticated: &Arc<RwLock<bool>>,
        subscribed_symbols: &Arc<RwLock<Vec<i64>>>,
    ) -> Result<()> {
        info!("🔄 Initiating reconnection to {} server...", environment);
        
        // 1. Reconnect TLS
        let addr = format!("{}:{}", config.server, config.port);
        info!("Connecting to {}", addr);
        
        let tcp_stream = timeout(Duration::from_secs(10), TcpStream::connect(&addr))
            .await
            .map_err(|_| CTraderError::Timeout)?
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        let tls_config = Arc::new(build_tls_config().map_err(|e| {
            CTraderError::ConnectionFailed(format!("TLS config error: {}", e))
        })?);
        let connector = TlsConnector::from(tls_config);
        let server_name = ServerName::try_from(config.server.clone())
            .map_err(|_| CTraderError::ConnectionFailed("Invalid TLS server name".into()))?;

        let tls_stream = timeout(Duration::from_secs(10), connector.connect(server_name, tcp_stream))
            .await
            .map_err(|_| CTraderError::Timeout)?
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        info!("✅ TLS connection established");
        *stream.lock().await = Some(tls_stream);
        
        // 2. Re-authenticate
        info!("Re-authenticating...");
        
        // Application auth
        let app_auth_req = ProtoOaApplicationAuthReq {
            payload_type: None,
            client_id: config.active_client_id().to_string(),
            client_secret: config.active_client_secret().to_string(),
        };
        
        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaApplicationAuthReq, app_auth_req);
        let encoded = encode_with_length(&msg);
        
        {
            let mut stream_guard = stream.lock().await;
            if let Some(s) = stream_guard.as_mut() {
                s.write_all(&encoded).await
                    .map_err(|e| CTraderError::ConnectionFailed(format!("Write failed: {}", e)))?;
            }
        }
        
        // Account auth (refresh OAuth token for LIVE before using it)
        let mut access_token = config.access_token.clone().ok_or_else(|| {
            CTraderError::AuthFailed(
                "Missing CTRADER_ACCESS_TOKEN for reconnection.\n\
                cTrader requires a valid OAuth access token.\n\
                \n\
                To obtain a token, run:\n\
                cargo run --bin get-token\n\
                \n\
                Then add the token to your .env file:\n\
                CTRADER_ACCESS_TOKEN=your_token_here".into()
            )
        })?;

        if environment.is_live() {
            let oauth_config = OAuthConfig {
                client_id: config.active_client_id().to_string(),
                client_secret: config.active_client_secret().to_string(),
                redirect_uri: oauth_redirect_uri(),
                environment: Environment::Live,
            };
            let storage = FileTokenStorage::new("oauth_token.json");
            let manager = OAuthManager::new(oauth_config).with_storage(Box::new(storage));
            if let Err(err) = manager.init().await {
                warn!("OAuth init failed during reconnect, using existing token: {}", err);
            }
            match manager.refresh_token().await {
                Ok(token) => {
                    access_token = token.access_token.clone();
                    config.access_token = Some(access_token.clone());
                }
                Err(err) => {
                    warn!(
                        "OAuth refresh failed during reconnect, using existing token: {}",
                        err
                    );
                }
            }
        }
        
        let account_auth_req = ProtoOaAccountAuthReq {
            payload_type: None,
            ctid_trader_account_id: config.active_account_id().parse::<i64>()
                .map_err(|e| CTraderError::AuthFailed(format!("Invalid account ID: {}", e)))?,
            access_token,
        };
        
        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaAccountAuthReq, account_auth_req);
        let encoded = encode_with_length(&msg);
        
        {
            let mut stream_guard = stream.lock().await;
            if let Some(s) = stream_guard.as_mut() {
                s.write_all(&encoded).await
                    .map_err(|e| CTraderError::ConnectionFailed(format!("Write failed: {}", e)))?;
            }
        }
        
        *authenticated.write().await = true;
        info!("✅ Re-authenticated");
        
        // 3. Re-subscribe to symbols
        let symbols = subscribed_symbols.read().await.clone();
        if !symbols.is_empty() {
            info!("Re-subscribing to {} symbols...", symbols.len());
            
            for symbol_id in symbols {
                let account_id = config.active_account_id().parse::<i64>()
                    .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;
                
                let subscribe_req = ProtoOaSubscribeSpotsReq {
                    payload_type: None,
                    ctid_trader_account_id: account_id,
                    symbol_id: vec![symbol_id],
                    subscribe_to_spot_timestamp: Some(false),
                };
                
                let msg = new_proto_message(ProtoOaPayloadType::ProtoOaSubscribeSpotsReq, subscribe_req);
                let encoded = encode_with_length(&msg);
                
                {
                    let mut stream_guard = stream.lock().await;
                    if let Some(s) = stream_guard.as_mut() {
                        s.write_all(&encoded).await
                            .map_err(|e| CTraderError::ConnectionFailed(format!("Write failed: {}", e)))?;
                    }
                }
                
                debug!("Re-subscribed to symbol {}", symbol_id);
            }
            
            info!("✅ Re-subscribed to all symbols");
        }
        
        Ok(())
    }
    
    /// Public reconnection method
    pub async fn reconnect(&mut self) -> Result<()> {
        Self::reconnect_internal(
            &mut self.config,
            self.environment,
            &self.stream,
            &self.authenticated,
            &self.subscribed_symbols,
        ).await
    }

    /// Resolve a symbol name to its numeric ID
    pub async fn get_symbol_id(&self, symbol_name: &str) -> Result<i64> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        info!("Resolving symbol ID for: {}", symbol_name);

        let account_id = self.config.active_account_id().parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let symbols_req = ProtoOaSymbolsListReq {
            payload_type: None,
            ctid_trader_account_id: account_id,
            include_archived_symbols: None,
        };

        let msg = new_proto_message(ProtoOaPayloadType::ProtoOaSymbolsListReq, symbols_req);
        self.send_message(msg).await?;

        let response = self.wait_for_message(ProtoOaPayloadType::ProtoOaSymbolsListRes).await?;

        if let Some(payload) = response.payload {
            let symbols_res = ProtoOaSymbolsListRes::decode(payload.as_ref())
                .map_err(|e| CTraderError::InvalidResponse(format!("Failed to decode symbols list: {}", e)))?;

        let mut candidates = vec![
            symbol_name.to_string(),
        ];
        candidates.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        let symbol = candidates.iter().find_map(|candidate| {
            symbols_res.symbol.iter().find(|s| {
                s.symbol_name
                    .as_ref()
                    .map(|name| name.eq_ignore_ascii_case(candidate))
                    .unwrap_or(false)
            })
        });

        let symbol = match symbol {
            Some(symbol) => symbol,
            None => {
                let available: Vec<&str> = symbols_res
                    .symbol
                    .iter()
                    .filter_map(|s| s.symbol_name.as_deref())
                    .collect();
                warn!(
                    "Symbol not found. Total symbols available: {}. Full list: {:?}",
                    available.len(),
                    available
                );
                return Err(CTraderError::InvalidResponse(
                    format!("Symbol '{}' not found in broker symbol list", symbol_name),
                )
                .into());
            }
        };

        info!("Resolved '{}' -> symbol ID {}", symbol_name, symbol.symbol_id);
        return Ok(symbol.symbol_id);
        }

        Err(CTraderError::InvalidResponse("Empty symbols list response".into()).into())
    }
}

fn build_tls_config() -> std::result::Result<ClientConfig, CTraderError> {
    let mut root_store = RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs()
        .map_err(|e| CTraderError::ConnectionFailed(format!(
            "Failed to load native root certificates: {}",
            e
        )))?;

    let mut added = 0usize;
    for cert in native_certs {
        if root_store.add(cert).is_ok() {
            added += 1;
        }
    }

    if added == 0 {
        return Err(CTraderError::ConnectionFailed(
            "No native root certificates were added".into(),
        ));
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

fn oauth_redirect_uri() -> String {
    env::var("CTRADER_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:8899".to_string())
}

impl Drop for CTraderClient {
    fn drop(&mut self) {
        debug!("CTraderClient dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CTraderConfig {
        CTraderConfig {
            environment: crate::config::TradingEnvironment::Demo,
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            account_id: "12345".to_string(),
            access_token: Some("demo_token".to_string()),
            server: "demo.ctraderapi.com".to_string(),
            port: 5035,
            client_id_live: None,
            client_secret_live: None,
            account_id_live: None,
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = CTraderClient::new(test_config());
        assert!(!client.is_authenticated().await);
        assert_eq!(client.environment(), CTraderEnvironment::Demo);
    }

    #[tokio::test]
    async fn test_client_with_live_environment() {
        let client = CTraderClient::with_environment(test_config(), CTraderEnvironment::Live);
        assert_eq!(client.environment(), CTraderEnvironment::Live);
        assert!(client.environment().is_live());
    }

    #[test]
    fn test_environment_parsing() {
        assert_eq!("demo".parse::<CTraderEnvironment>().ok(), Some(CTraderEnvironment::Demo));
        assert_eq!("DEMO".parse::<CTraderEnvironment>().ok(), Some(CTraderEnvironment::Demo));
        assert_eq!("live".parse::<CTraderEnvironment>().ok(), Some(CTraderEnvironment::Live));
        assert_eq!("LIVE".parse::<CTraderEnvironment>().ok(), Some(CTraderEnvironment::Live));
        assert_eq!(
            "production".parse::<CTraderEnvironment>().ok(),
            Some(CTraderEnvironment::Live)
        );
        assert_eq!("prod".parse::<CTraderEnvironment>().ok(), Some(CTraderEnvironment::Live));
        assert!("invalid".parse::<CTraderEnvironment>().is_err());
    }

    #[test]
    fn test_environment_endpoints() {
        assert_eq!(CTraderEnvironment::Demo.server_endpoint(), "demo.ctraderapi.com");
        assert_eq!(CTraderEnvironment::Live.server_endpoint(), "live.ctraderapi.com");
        assert_eq!(CTraderEnvironment::Demo.default_port(), 5035);
        assert_eq!(CTraderEnvironment::Live.default_port(), 5035);
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", CTraderEnvironment::Demo), "DEMO");
        assert_eq!(format!("{}", CTraderEnvironment::Live), "LIVE");
    }

    #[tokio::test]
    async fn test_validate_credentials() {
        let client = CTraderClient::new(test_config());
        assert!(client.validate_credentials().is_ok());

        let empty_config = CTraderConfig {
            environment: crate::config::TradingEnvironment::Demo,
            client_id: "".to_string(),
            client_secret: "test".to_string(),
            account_id: "123".to_string(),
            access_token: Some("demo_token".to_string()),
            server: "demo.ctraderapi.com".to_string(),
            port: 5035,
            client_id_live: None,
            client_secret_live: None,
            account_id_live: None,
        };
        let client = CTraderClient::new(empty_config);
        assert!(client.validate_credentials().is_err());
    }

    #[test]
    fn test_verify_credentials_requires_access_token_for_demo() {
        let mut config = test_config();
        config.access_token = None;
        let client = CTraderClient::new(config);

        assert!(client.verify_credentials().is_err());
    }

    #[test]
    fn test_verify_credentials_accepts_valid_token() {
        let mut config = test_config();
        config.access_token = Some("valid_token".to_string());
        let client = CTraderClient::new(config);

        assert!(client.verify_credentials().is_ok());
    }

    #[test]
    fn test_from_config_auto_detect() {
        let mut config = test_config();
        config.server = "demo.ctraderapi.com".to_string();
        let client = CTraderClient::from_config(config.clone());
        assert_eq!(client.environment(), CTraderEnvironment::Demo);

        config.server = "live.ctraderapi.com".to_string();
        let client = CTraderClient::from_config(config);
        assert_eq!(client.environment(), CTraderEnvironment::Live);
    }
}
