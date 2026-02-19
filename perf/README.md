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

## Auth modes

The harness supports three modes:

1. `PERF_SESSION_TOKEN` (recommended for stable runs):
- Uses `/api/auth/ws-ticket` with `session_token` cookie.

2. `PERF_WS_TICKET` (single-client only):
- Direct one-time ticket for simple single-client runs.
- Not valid for mass-user runs.

3. Dev bootstrap (no token required):
- Server must run with `PERF_TEST_AUTH_BYPASS=true`.
- Harness calls `/api/dev/ws-ticket` to mint ephemeral users + tickets.
- Useful for local perf work without OAuth/login.

## Environment variables

- `PERF_BASE_URL` default: `http://127.0.0.1:3000`
- `PERF_SESSION_TOKEN` optional
- `PERF_WS_TICKET` optional
- `PERF_TEST_AUTH_BYPASS` (server env; set to `true` to enable `/api/dev/ws-ticket`)
- `PERF_BASELINE_REQUESTS` default: `200`
- `PERF_COMPLEXITY_COUNTS` default: `100,500,1000`
- `PERF_MASS_USERS` default: `25`
- `PERF_MASS_REQUESTS_PER_USER` default: `20`
