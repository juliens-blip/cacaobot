//! cTrader Open API Protobuf message definitions
//!
//! This module includes the auto-generated Protobuf types from the official
//! cTrader Open API .proto files and provides helper methods for message framing.

use bytes::{BufMut, BytesMut};
use prost::Message;
use std::io::Cursor;
use bytes::Buf;

// Include the generated protobuf code
mod generated {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

// Re-export all generated types
pub use generated::*;

// Type aliases for backward compatibility with existing code that uses ProtoOA* naming
pub type ProtoOAPayloadType = ProtoOaPayloadType;
pub type ProtoOATradeSide = ProtoOaTradeSide;
pub type ProtoOAOrderType = ProtoOaOrderType;
pub type ProtoOAApplicationAuthReq = ProtoOaApplicationAuthReq;
pub type ProtoOAApplicationAuthRes = ProtoOaApplicationAuthRes;
pub type ProtoOAAccountAuthReq = ProtoOaAccountAuthReq;
pub type ProtoOAAccountAuthRes = ProtoOaAccountAuthRes;
pub type ProtoOASubscribeSpotsReq = ProtoOaSubscribeSpotsReq;
pub type ProtoOASpotEvent = ProtoOaSpotEvent;
pub type ProtoOANewOrderReq = ProtoOaNewOrderReq;
pub type ProtoOAExecutionEvent = ProtoOaExecutionEvent;
pub type ProtoOAClosePositionReq = ProtoOaClosePositionReq;
pub type ProtoOAReconcileReq = ProtoOaReconcileReq;
pub type ProtoOAReconcileRes = ProtoOaReconcileRes;
pub type ProtoOAPosition = ProtoOaPosition;
pub type ProtoOASymbolsListReq = ProtoOaSymbolsListReq;
pub type ProtoOASymbolsListRes = ProtoOaSymbolsListRes;
pub type ProtoOALightSymbol = ProtoOaLightSymbol;
pub type ProtoOAErrorRes = ProtoOaErrorRes;
pub type ProtoOAOrderErrorEvent = ProtoOaOrderErrorEvent;

/// Helper to create a ProtoMessage envelope wrapping a payload
pub fn new_proto_message(payload_type: ProtoOaPayloadType, payload: impl Message) -> ProtoMessage {
    let mut buf = BytesMut::new();
    payload.encode(&mut buf).expect("Failed to encode message");

    ProtoMessage {
        payload_type: payload_type as i32 as u32,
        payload: Some(buf.to_vec()),
        client_msg_id: Some(generate_msg_id()),
    }
}

/// Encode a ProtoMessage with a 4-byte big-endian length prefix (cTrader wire format)
pub fn encode_with_length(msg: &ProtoMessage) -> Vec<u8> {
    let mut buf = BytesMut::new();
    msg.encode(&mut buf).expect("Failed to encode message");

    let msg_len = buf.len() as u32;
    let mut result = BytesMut::with_capacity(4 + buf.len());
    result.put_u32(msg_len);
    result.put(buf);

    result.to_vec()
}

/// Decode a ProtoMessage from bytes with a 4-byte big-endian length prefix
pub fn decode_with_length(buf: &[u8]) -> Result<ProtoMessage, prost::DecodeError> {
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

/// Generate a simple message ID
fn generate_msg_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

/// Helper to get payload type as u32 from the generated enum
pub fn payload_type_to_u32(pt: ProtoOaPayloadType) -> u32 {
    pt as i32 as u32
}

/// Helper to convert u32 to ProtoOaPayloadType
pub fn payload_type_from_u32(value: u32) -> Option<ProtoOaPayloadType> {
    ProtoOaPayloadType::try_from(value as i32).ok()
}
