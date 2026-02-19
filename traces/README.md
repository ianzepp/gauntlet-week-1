# traces

Shared trace/event primitives for CollabBoard observability UIs.

This crate intentionally avoids UI framework dependencies so it can be used by
`client` (Leptos) or any other renderer.

## Included now

- Syscall prefix-to-display mapping (`board`, `object`, `ai`, etc.)
- Default trace filtering policy (hides `cursor:*` + `item` by default)
- Frame -> trace session grouping by parent chain
- Request/done span pairing for waterfall timing
- Aggregate trace metrics (counts, errors, pending)
- Sub-label extraction helpers for common syscall payloads

## Usage

Add dependency from a UI crate:

```toml
traces = { path = "../traces" }
```

Then consume helpers from `traces::*`.
