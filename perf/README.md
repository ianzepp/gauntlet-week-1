# perf

Live end-to-end performance tests for client-server frame communication.

This crate runs against a running `server` instance and measures:

- WS request/response round-trip latency
- Object-create performance as board complexity grows
- Mass-user concurrent request performance

## Run

From workspace root:

```bash
cargo test -p perf -- --ignored --nocapture
```

## Required auth setup

Provide either:

- `PERF_SESSION_TOKEN` (recommended): used to mint one-time WS tickets via `/api/auth/ws-ticket`
- `PERF_WS_TICKET` (single-client only): direct one-time ticket for simple runs

Mass-user test requires `PERF_SESSION_TOKEN` so it can mint many tickets.

## Environment variables

- `PERF_BASE_URL` default: `http://127.0.0.1:3000`
- `PERF_SESSION_TOKEN` optional
- `PERF_WS_TICKET` optional
- `PERF_BASELINE_REQUESTS` default: `200`
- `PERF_COMPLEXITY_COUNTS` default: `100,500,1000`
- `PERF_MASS_USERS` default: `25`
- `PERF_MASS_REQUESTS_PER_USER` default: `20`
