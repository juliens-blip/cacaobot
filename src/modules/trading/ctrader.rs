//! cTrader Open API client
//!
//! This module provides a Rust client for the cTrader Open API using Protobuf over TCP.
//! Documentation: https://help.ctrader.com/open-api/

use crate::config::CTraderConfig;
use crate::error::{CTraderError, Result};
use prost::Message as ProstMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

use super::protobuf::*;

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
    pub side: ProtoOATradeSide,
    pub volume: i64, // in lots * 100 (e.g., 0.1 lot = 10)
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub label: Option<String>,
}

/// cTrader API client
pub struct CTraderClient {
    config: CTraderConfig,
    stream: Arc<Mutex<Option<TcpStream>>>,
    access_token: Arc<RwLock<Option<String>>>,
    prices: Arc<RwLock<HashMap<i64, Price>>>,
    authenticated: Arc<RwLock<bool>>,
    message_tx: mpsc::UnboundedSender<ProtoMessage>,
    reader_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl CTraderClient {
    /// Create a new cTrader client
    pub fn new(config: CTraderConfig) -> Self {
        let (message_tx, _message_rx) = mpsc::unbounded_channel();

        CTraderClient {
            config,
            stream: Arc::new(Mutex::new(None)),
            access_token: Arc::new(RwLock::new(None)),
            prices: Arc::new(RwLock::new(HashMap::new())),
            authenticated: Arc::new(RwLock::new(false)),
            message_tx,
            reader_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to cTrader server
    pub async fn connect(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server, self.config.port);
        info!("Connecting to cTrader at {}", addr);

        let stream = timeout(Duration::from_secs(10), TcpStream::connect(&addr))
            .await
            .map_err(|_| CTraderError::Timeout)?
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        info!("Connected to cTrader");
        *self.stream.lock().await = Some(stream);

        Ok(())
    }

    /// Authenticate with cTrader API
    pub async fn authenticate(&self) -> Result<()> {
        info!("Authenticating with cTrader API");

        // Step 1: Application authentication
        let app_auth_req = ProtoOAApplicationAuthReq {
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
        };

        let msg = ProtoMessage::new(ProtoOAPayloadType::ProtoOAApplicationAuthReq, app_auth_req);
        self.send_message(msg).await?;

        // Wait for application auth response
        let _response =
            self.wait_for_message(ProtoOAPayloadType::ProtoOAApplicationAuthRes).await?;
        debug!("Application authenticated");

        // For demo accounts, we use the client_id as access token
        let access_token = self.config.client_id.clone();
        *self.access_token.write().await = Some(access_token.clone());

        // Step 2: Account authentication
        let account_id = self.config.account_id.parse::<i64>()
            .map_err(|e| CTraderError::AuthFailed(format!("Invalid account ID: {}", e)))?;

        let account_auth_req = ProtoOAAccountAuthReq {
            ctid_trader_account_id: account_id,
            access_token,
        };

        let msg = ProtoMessage::new(ProtoOAPayloadType::ProtoOAAccountAuthReq, account_auth_req);
        self.send_message(msg).await?;

        // Wait for account auth response
        let _response = self.wait_for_message(ProtoOAPayloadType::ProtoOAAccountAuthRes).await?;
        info!("Account authenticated: {}", account_id);

        *self.authenticated.write().await = true;

        // Start heartbeat and message handling
        self.start_background_tasks().await;

        Ok(())
    }

    /// Subscribe to price updates for a symbol
    pub async fn subscribe_to_symbol(&self, symbol_id: i64) -> Result<()> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        let account_id = self.config.account_id.parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let subscribe_req = ProtoOASubscribeSpotsReq {
            ctid_trader_account_id: account_id,
            symbol_id: vec![symbol_id],
        };

        let msg = ProtoMessage::new(ProtoOAPayloadType::ProtoOASubscribeSpotsReq, subscribe_req);
        self.send_message(msg).await?;

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

        let account_id = self.config.account_id.parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let order_req = ProtoOANewOrderReq {
            ctid_trader_account_id: account_id,
            symbol_id: ticket.symbol_id,
            order_type: ProtoOAOrderType::Market as i32,
            trade_side: ticket.side as i32,
            volume: ticket.volume,
            limit_price: None,
            stop_price: None,
            label: ticket.label.clone(),
            stop_loss: ticket.stop_loss,
            take_profit: ticket.take_profit,
        };

        let msg = ProtoMessage::new(ProtoOAPayloadType::ProtoOANewOrderReq, order_req);
        self.send_message(msg).await?;

        info!("Order placed: {:?}", ticket);

        // Wait for execution event
        let response = self.wait_for_message(ProtoOAPayloadType::ProtoOAExecutionEvent).await?;

        // Parse execution event to get order ID and position ID
        if let Some(payload) = response.payload {
            if let Ok(exec_event) = ProtoOAExecutionEvent::decode(payload.as_ref()) {
                let order_id = exec_event.order_id;
                let position_id = exec_event.position_id; // position_id is required field
                info!("Order executed: order_id={} position_id={}", order_id, position_id);
                return Ok((order_id, position_id));
            }
        }

        Err(CTraderError::InvalidResponse("Failed to parse execution event".into()).into())
    }

    /// Get open positions
    pub async fn get_positions(&self) -> Result<Vec<Position>> {
        // Note: In a full implementation, we'd send ProtoOAReconcileReq
        // For now, return empty list as we need to track positions from execution events
        warn!("get_positions not fully implemented - tracking positions from execution events");
        Ok(Vec::new())
    }

    /// Close a position
    pub async fn close_position(&self, position_id: i64, volume: i64) -> Result<()> {
        if !*self.authenticated.read().await {
            return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
        }

        let account_id = self.config.account_id.parse::<i64>()
            .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

        let close_req = ProtoOAClosePositionReq {
            ctid_trader_account_id: account_id,
            position_id,
            volume,
        };

        let msg = ProtoMessage::new(ProtoOAPayloadType::ProtoOAClosePositionReq, close_req);
        self.send_message(msg).await?;

        info!("Position closed: {}", position_id);
        Ok(())
    }

    /// Start continuous reader task to process incoming messages
    pub async fn start_reader(&self) -> Result<()> {
        let stream_arc = self.stream.clone();
        let prices_arc = self.prices.clone();
        let message_tx = self.message_tx.clone();
        
        let task = tokio::spawn(async move {
            info!("cTrader reader task started");
            
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
                
                // Read message header (8 bytes)
                let mut header = [0u8; 8];
                match timeout(Duration::from_secs(30), stream.read_exact(&mut header)).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        error!("Reader: Failed to read header: {}", e);
                        break;
                    }
                    Err(_) => {
                        debug!("Reader: Heartbeat timeout - connection still alive");
                        continue;
                    }
                }
                
                let payload_type = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
                let payload_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
                
                // Read payload
                let mut payload = vec![0u8; payload_len];
                if let Err(e) = stream.read_exact(&mut payload).await {
                    error!("Reader: Failed to read payload: {}", e);
                    break;
                }
                
                drop(stream_guard);
                
                // Process message based on type
                if let Some(msg_type) = ProtoOAPayloadType::from_u32(payload_type) {
                    match msg_type {
                        ProtoOAPayloadType::ProtoOASpotEvent => {
                            if let Ok(spot_event) = ProtoOASpotEvent::decode(payload.as_ref()) {
                                Self::handle_spot_event(spot_event, &prices_arc).await;
                            }
                        }
                        ProtoOAPayloadType::ProtoOAExecutionEvent => {
                            debug!("Reader: Execution event received");
                            let _ = message_tx.send(ProtoMessage { payload_type, payload: Some(payload), client_msg_id: None });
                        }
                        _ => {
                            debug!("Reader: Message type {:?} queued", msg_type);
                            let _ = message_tx.send(ProtoMessage { payload_type, payload: Some(payload), client_msg_id: None });
                        }
                    }
                }
            }
            
            warn!("cTrader reader task stopped");
        });
        
        *self.reader_task.write().await = Some(task);
        info!("Reader task spawned");
        Ok(())
    }
    
    /// Handle spot event (price update)
    async fn handle_spot_event(event: ProtoOASpotEvent, prices: &Arc<RwLock<HashMap<i64, Price>>>) {
        let symbol_id = event.symbol_id;
        // bid/ask are u64 (price in pips), convert to f64
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

        let encoded = message.encode_with_length();
        stream.write_all(&encoded).await
            .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

        debug!("Sent message type: {}", message.payload_type);
        Ok(())
    }

    /// Wait for a specific message type
    async fn wait_for_message(&self, msg_type: ProtoOAPayloadType) -> Result<ProtoMessage> {
        let timeout_duration = Duration::from_secs(30);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout_duration {
                return Err(CTraderError::Timeout.into());
            }

            let mut stream_guard = self.stream.lock().await;
            let stream = stream_guard.as_mut()
                .ok_or(CTraderError::Disconnected)?;

            // Read message length (4 bytes)
            let mut len_buf = [0u8; 4];
            if let Err(e) = timeout(Duration::from_secs(5), stream.read_exact(&mut len_buf)).await {
                warn!("Timeout reading message length: {}", e);
                continue;
            }

            let msg_len = u32::from_be_bytes(len_buf) as usize;
            if msg_len > 1_000_000 {
                return Err(CTraderError::Protocol(format!("Message too large: {}", msg_len)).into());
            }

            // Read message payload
            let mut msg_buf = vec![0u8; msg_len];
            stream.read_exact(&mut msg_buf).await
                .map_err(|e| CTraderError::ConnectionFailed(e.to_string()))?;

            let message = ProtoMessage::decode(msg_buf.as_ref())
                .map_err(|e| CTraderError::Protocol(e.to_string()))?;

            debug!("Received message type: {}", message.payload_type);

            // Handle spot events
            if message.payload_type == ProtoOAPayloadType::ProtoOASpotEvent as u32 {
                if let Some(ref payload) = message.payload {
                    if let Ok(spot_event) = ProtoOASpotEvent::decode(payload.as_ref()) {
                        Self::handle_spot_event(spot_event, &self.prices).await;
                    }
                }
            }

            if message.payload_type == msg_type as u32 {
                return Ok(message);
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

                let heartbeat = ProtoMessage::new(
                    ProtoOAPayloadType::ProtoHeartbeatEvent,
                    ProtoHeartbeatEvent {},
                );

                let mut stream_guard = stream_clone.lock().await;
                if let Some(stream) = stream_guard.as_mut() {
                    let encoded = heartbeat.encode_with_length();
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
}

impl Drop for CTraderClient {
    fn drop(&mut self) {
        // Ensure cleanup
        debug!("CTraderClient dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = CTraderConfig {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            account_id: "12345".to_string(),
            server: "demo.ctraderapi.com".to_string(),
            port: 5035,
        };

        let client = CTraderClient::new(config);
        assert!(!client.is_authenticated().await);
    }
}
