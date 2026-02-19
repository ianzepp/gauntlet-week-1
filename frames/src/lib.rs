//! Shared frame model and protobuf codec for realtime WS transport.
//!
//! This crate owns the wire representation used by both `server` and `client`.
//! It intentionally keeps frame payloads flexible (`serde_json::Value`) while
//! encoding over protobuf for compact binary transport.

use prost::Message;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("failed to decode protobuf frame: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("invalid frame status: {0}")]
    InvalidStatus(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Request,
    Done,
    Error,
    Cancel,
}

impl Status {
    /// Convert status into wire enum integer value.
    #[must_use]
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Request => WireFrameStatus::Request as i32,
            Self::Done => WireFrameStatus::Done as i32,
            Self::Error => WireFrameStatus::Error as i32,
            Self::Cancel => WireFrameStatus::Cancel as i32,
        }
    }

    /// Parse a status from wire enum integer value.
    fn from_i32(value: i32) -> Result<Self, CodecError> {
        match WireFrameStatus::try_from(value) {
            Ok(WireFrameStatus::Request) => Ok(Self::Request),
            Ok(WireFrameStatus::Done) => Ok(Self::Done),
            Ok(WireFrameStatus::Error) => Ok(Self::Error),
            Ok(WireFrameStatus::Cancel) => Ok(Self::Cancel),
            Err(_) => Err(CodecError::InvalidStatus(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub parent_id: Option<String>,
    pub ts: i64,
    pub board_id: Option<String>,
    pub from: Option<String>,
    pub syscall: String,
    pub status: Status,
    pub data: Value,
}

/// Encode a frame into protobuf bytes.
#[must_use]
pub fn encode_frame(frame: &Frame) -> Vec<u8> {
    let wire = frame_to_wire(frame);

    let mut out = Vec::with_capacity(wire.encoded_len());
    wire.encode(&mut out).expect("Vec writes should not fail");
    out
}

/// Decode protobuf bytes into a frame.
///
/// # Errors
///
/// Returns [`CodecError::Decode`] for malformed bytes and
/// [`CodecError::InvalidStatus`] for out-of-range status values.
pub fn decode_frame(bytes: &[u8]) -> Result<Frame, CodecError> {
    let wire = WireFrame::decode(bytes)?;
    wire_to_frame(wire)
}

fn frame_to_wire(frame: &Frame) -> WireFrame {
    WireFrame {
        id: frame.id.clone(),
        parent_id: frame.parent_id.clone(),
        ts: frame.ts,
        board_id: frame.board_id.clone(),
        from: frame.from.clone(),
        syscall: frame.syscall.clone(),
        status: frame.status.as_i32(),
        data: Some(json_to_proto_value(&frame.data)),
    }
}

fn wire_to_frame(wire: WireFrame) -> Result<Frame, CodecError> {
    Ok(Frame {
        id: wire.id,
        parent_id: wire.parent_id,
        ts: wire.ts,
        board_id: wire.board_id,
        from: wire.from,
        syscall: wire.syscall,
        status: Status::from_i32(wire.status)?,
        data: wire
            .data
            .map_or(Value::Object(Map::new()), |v| proto_to_json_value(&v)),
    })
}

fn json_to_proto_value(value: &Value) -> prost_types::Value {
    let kind = match value {
        Value::Null => {
            prost_types::value::Kind::NullValue(prost_types::NullValue::NullValue as i32)
        }
        Value::Bool(v) => prost_types::value::Kind::BoolValue(*v),
        Value::Number(v) => prost_types::value::Kind::NumberValue(v.as_f64().unwrap_or(0.0)),
        Value::String(v) => prost_types::value::Kind::StringValue(v.clone()),
        Value::Array(v) => prost_types::value::Kind::ListValue(prost_types::ListValue {
            values: v.iter().map(json_to_proto_value).collect(),
        }),
        Value::Object(v) => prost_types::value::Kind::StructValue(prost_types::Struct {
            fields: v
                .iter()
                .map(|(k, v)| (k.clone(), json_to_proto_value(v)))
                .collect(),
        }),
    };

    prost_types::Value { kind: Some(kind) }
}

fn proto_to_json_value(value: &prost_types::Value) -> Value {
    let Some(kind) = &value.kind else {
        return Value::Null;
    };

    match kind {
        prost_types::value::Kind::NullValue(_) => Value::Null,
        prost_types::value::Kind::NumberValue(v) => {
            serde_json::Number::from_f64(*v).map_or(Value::Null, Value::Number)
        }
        prost_types::value::Kind::StringValue(v) => Value::String(v.clone()),
        prost_types::value::Kind::BoolValue(v) => Value::Bool(*v),
        prost_types::value::Kind::StructValue(v) => Value::Object(
            v.fields
                .iter()
                .map(|(k, v)| (k.clone(), proto_to_json_value(v)))
                .collect(),
        ),
        prost_types::value::Kind::ListValue(v) => {
            Value::Array(v.values.iter().map(proto_to_json_value).collect())
        }
    }
}

#[derive(Clone, PartialEq, Message)]
struct WireFrame {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, optional, tag = "2")]
    parent_id: Option<String>,
    #[prost(int64, tag = "3")]
    ts: i64,
    #[prost(string, optional, tag = "4")]
    board_id: Option<String>,
    #[prost(string, optional, tag = "5")]
    from: Option<String>,
    #[prost(string, tag = "6")]
    syscall: String,
    #[prost(enumeration = "WireFrameStatus", tag = "7")]
    status: i32,
    #[prost(message, optional, tag = "8")]
    data: Option<prost_types::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
enum WireFrameStatus {
    Request = 0,
    Done = 1,
    Error = 2,
    Cancel = 3,
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
