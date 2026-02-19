# frames

Shared realtime frame model and protobuf codec for client/server WebSocket transport.

## Scope

This crate defines:

- Canonical frame types (`Frame`, `Status`)
- Binary wire encoding (`encode_frame`)
- Binary wire decoding (`decode_frame`)

HTTP/auth endpoints can continue using JSON outside this crate.

## Wire model

`Frame` fields:

- `id: String` (UUID text)
- `parent_id: Option<String>`
- `ts: i64` (unix millis)
- `board_id: Option<String>`
- `from: Option<String>`
- `syscall: String`
- `status: Status` (`request | done | error | cancel`)
- `data: serde_json::Value`

## Notes

- Transport is protobuf, but `data` remains flexible JSON-like value content (`prost_types::Value`).
- Protobuf numeric values are decoded as JSON numbers (float representation), so integer consumers should accept whole-number floats where needed.
- This crate is transport-focused and contains no business logic.
