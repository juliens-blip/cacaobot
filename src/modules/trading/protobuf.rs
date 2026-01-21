//! cTrader Open API Protobuf message definitions
//!
//! This module contains the Protobuf message structures for cTrader Open API.
//! Based on: https://help.ctrader.com/open-api/messages/

use bytes::{Buf, BufMut, BytesMut};
use prost::Message;
use std::io::Cursor;

/// Payload types for cTrader messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProtoOAPayloadType {
    ProtoOAApplicationAuthReq = 2100,
    ProtoOAApplicationAuthRes = 2101,
    ProtoOAAccountAuthReq = 2102,
    ProtoOAAccountAuthRes = 2103,
    ProtoOAVersionReq = 2104,
    ProtoOAVersionRes = 2105,
    ProtoOANewOrderReq = 2106,
    ProtoOATrailingSlChangedEvent = 2107,
    ProtoOACancelOrderReq = 2108,
    ProtoOAAmendPositionSLTPReq = 2109,
    ProtoOAAmendOrderReq = 2110,
    ProtoOAClosePositionReq = 2111,
    ProtoOAAssetListReq = 2112,
    ProtoOAAssetListRes = 2113,
    ProtoOASymbolsListReq = 2114,
    ProtoOASymbolsListRes = 2115,
    ProtoOASymbolByIdReq = 2116,
    ProtoOASymbolByIdRes = 2117,
    ProtoOASymbolsForConversionReq = 2118,
    ProtoOASymbolsForConversionRes = 2119,
    ProtoOASymbolChangedEvent = 2120,
    ProtoOATraderReq = 2121,
    ProtoOATraderRes = 2122,
    ProtoOATraderUpdatedEvent = 2123,
    ProtoOAReconcileReq = 2124,
    ProtoOAReconcileRes = 2125,
    ProtoOAExecutionEvent = 2126,
    ProtoOASubscribeSpotsReq = 2127,
    ProtoOAUnsubscribeSpotsReq = 2128,
    ProtoOASpotEvent = 2129,
    ProtoOAOrderErrorEvent = 2130,
    ProtoOADealListReq = 2131,
    ProtoOADealListRes = 2132,
    ProtoOAGetAccountListByAccessTokenReq = 2133,
    ProtoOAGetAccountListByAccessTokenRes = 2134,
    ProtoHeartbeatEvent = 51,
}

impl ProtoOAPayloadType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            2100 => Some(Self::ProtoOAApplicationAuthReq),
            2101 => Some(Self::ProtoOAApplicationAuthRes),
            2102 => Some(Self::ProtoOAAccountAuthReq),
            2103 => Some(Self::ProtoOAAccountAuthRes),
            2127 => Some(Self::ProtoOASubscribeSpotsReq),
            2129 => Some(Self::ProtoOASpotEvent),
            2106 => Some(Self::ProtoOANewOrderReq),
            2126 => Some(Self::ProtoOAExecutionEvent),
            2111 => Some(Self::ProtoOAClosePositionReq),
            51 => Some(Self::ProtoHeartbeatEvent),
            _ => None,
        }
    }
}

/// Order side (Buy/Sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ProtoOATradeSide {
    Buy = 1,
    Sell = 2,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ProtoOAOrderType {
    Market = 1,
    Limit = 2,
    Stop = 3,
    StopLimit = 4,
    MarketRange = 5,
}

/// Application authentication request
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOAApplicationAuthReq {
    #[prost(string, tag = "1")]
    pub client_id: String,
    #[prost(string, tag = "2")]
    pub client_secret: String,
}

/// Application authentication response
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOAApplicationAuthRes {}

/// Account authentication request
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOAAccountAuthReq {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
    #[prost(string, tag = "2")]
    pub access_token: String,
}

/// Account authentication response
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOAAccountAuthRes {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
}

/// Subscribe to spot prices
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOASubscribeSpotsReq {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
    #[prost(int64, repeated, tag = "2")]
    pub symbol_id: Vec<i64>,
}

/// Spot price event
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOASpotEvent {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
    #[prost(int64, tag = "2")]
    pub symbol_id: i64,
    #[prost(uint64, optional, tag = "3")]
    pub bid: Option<u64>,
    #[prost(uint64, optional, tag = "4")]
    pub ask: Option<u64>,
}

/// New order request
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOANewOrderReq {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
    #[prost(int64, tag = "2")]
    pub symbol_id: i64,
    #[prost(enumeration = "i32", tag = "3")]
    pub order_type: i32,
    #[prost(enumeration = "i32", tag = "4")]
    pub trade_side: i32,
    #[prost(int64, tag = "5")]
    pub volume: i64,
    #[prost(double, optional, tag = "6")]
    pub limit_price: Option<f64>,
    #[prost(double, optional, tag = "7")]
    pub stop_price: Option<f64>,
    #[prost(string, optional, tag = "8")]
    pub label: Option<String>,
    #[prost(double, optional, tag = "9")]
    pub stop_loss: Option<f64>,
    #[prost(double, optional, tag = "10")]
    pub take_profit: Option<f64>,
}

/// Order execution event
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOAExecutionEvent {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
    #[prost(int64, tag = "2")]
    pub order_id: i64,
    #[prost(int64, tag = "3")]
    pub position_id: i64,
    #[prost(string, tag = "4")]
    pub execution_type: String,
}

/// Close position request
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOAClosePositionReq {
    #[prost(int64, tag = "1")]
    pub ctid_trader_account_id: i64,
    #[prost(int64, tag = "2")]
    pub position_id: i64,
    #[prost(int64, tag = "3")]
    pub volume: i64,
}

/// Heartbeat event
#[derive(Clone, PartialEq, Message)]
pub struct ProtoHeartbeatEvent {}

/// Wrapper for all messages with client message ID
#[derive(Clone, PartialEq, Message)]
pub struct ProtoMessage {
    #[prost(uint32, tag = "1")]
    pub payload_type: u32,
    #[prost(bytes, optional, tag = "2")]
    pub payload: Option<Vec<u8>>,
    #[prost(string, optional, tag = "3")]
    pub client_msg_id: Option<String>,
}

impl ProtoMessage {
    /// Create a new message
    pub fn new(payload_type: ProtoOAPayloadType, payload: impl Message) -> Self {
        let mut buf = BytesMut::new();
        payload.encode(&mut buf).expect("Failed to encode message");

        ProtoMessage {
            payload_type: payload_type as u32,
            payload: Some(buf.to_vec()),
            client_msg_id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    /// Encode message with length prefix (4 bytes)
    pub fn encode_with_length(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        self.encode(&mut buf).expect("Failed to encode message");

        let msg_len = buf.len() as u32;
        let mut result = BytesMut::with_capacity(4 + buf.len());
        result.put_u32(msg_len);
        result.put(buf);

        result.to_vec()
    }

    /// Decode message from bytes with length prefix
    pub fn decode_with_length(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        if buf.len() < 4 {
            return Err(prost::DecodeError::new("Buffer too short"));
        }

        let mut cursor = Cursor::new(buf);
        let msg_len = cursor.get_u32() as usize;

        if buf.len() < 4 + msg_len {
            return Err(prost::DecodeError::new("Incomplete message"));
        }

        let msg_bytes = &buf[4..4 + msg_len];
        ProtoMessage::decode(msg_bytes)
    }
}

/// Helper to add uuid dependency note
mod uuid {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct Uuid;

    impl Uuid {
        pub fn new_v4() -> UuidValue {
            UuidValue
        }
    }

    pub struct UuidValue;

    impl UuidValue {
        pub fn to_string(&self) -> String {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("{:x}", timestamp)
        }
    }
}
