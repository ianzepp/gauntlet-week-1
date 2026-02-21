//! End-to-end performance harness for realtime frame transport.
//!
//! This crate targets live-server benchmarking from the outside:
//! it acquires WS tickets via HTTP auth endpoints, opens real websocket
//! sessions, exchanges binary frame traffic, and reports latency/throughput
//! metrics.

use std::time::{Duration, Instant};

use frames::{Frame, Status};
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{COOKIE, HeaderMap, HeaderValue};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Runtime configuration for perf tests, loaded from environment variables.
#[derive(Clone, Debug)]
pub struct PerfConfig {
    /// HTTP base URL of the server under test (e.g. `"http://127.0.0.1:3000"`).
    pub base_url: String,
    /// Pre-obtained one-time WS ticket. When set, skips the HTTP ticket-acquisition flow.
    pub ws_ticket: Option<String>,
    /// Session cookie token used to fetch WS tickets programmatically.
    pub session_token: Option<String>,
    /// Number of requests to issue in baseline round-trip tests.
    pub baseline_requests: usize,
    /// Object counts to use for complexity scaling tests.
    pub complexity_counts: Vec<usize>,
    /// Number of simulated concurrent users for mass-load tests.
    pub mass_users: usize,
    /// Number of requests each simulated user sends in mass-load tests.
    pub mass_requests_per_user: usize,
}

impl PerfConfig {
    /// Load perf config from environment with sane defaults.
    #[must_use]
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("PERF_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_owned());
        let ws_ticket = std::env::var("PERF_WS_TICKET")
            .ok()
            .filter(|s| !s.is_empty());
        let session_token = std::env::var("PERF_SESSION_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        let baseline_requests = env_usize("PERF_BASELINE_REQUESTS", 200);
        let complexity_counts = env_usize_list("PERF_COMPLEXITY_COUNTS", &[100, 500, 1000]);
        let mass_users = env_usize("PERF_MASS_USERS", 25);
        let mass_requests_per_user = env_usize("PERF_MASS_REQUESTS_PER_USER", 20);

        Self {
            base_url,
            ws_ticket,
            session_token,
            baseline_requests,
            complexity_counts,
            mass_users,
            mass_requests_per_user,
        }
    }
}

/// Error type for perf harness operations.
#[derive(Debug, thiserror::Error)]
pub enum PerfError {
    /// Neither `PERF_WS_TICKET` nor `PERF_SESSION_TOKEN` was provided.
    #[error("missing auth context: set PERF_WS_TICKET or PERF_SESSION_TOKEN")]
    MissingAuth,
    /// The base URL could not be converted to a WebSocket URL.
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    /// An HTTP request to the server failed.
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// An HTTP header value could not be constructed.
    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    /// The WebSocket connection or handshake failed.
    #[error("websocket connect failed: {0}")]
    WsConnect(Box<tokio_tungstenite::tungstenite::Error>),
    /// The WebSocket connection was closed unexpectedly.
    #[error("websocket closed")]
    WsClosed,
    /// A binary frame could not be decoded.
    #[error("frame decode failed: {0}")]
    Decode(#[from] frames::CodecError),
    /// No response frame arrived before the deadline.
    #[error("timed out waiting for frame")]
    Timeout,
    /// The server returned an error-status frame for the given syscall.
    #[error("server returned error status for {syscall}: {message}")]
    ServerError { syscall: String, message: String },
    /// A required field was absent from the server response payload.
    #[error("missing expected field `{0}`")]
    MissingField(&'static str),
    /// `PERF_WS_TICKET` is a one-time secret and cannot be shared across multiple users.
    #[error("PERF_WS_TICKET is one-time and cannot be reused for {0} users")]
    StaticTicketInsufficient(usize),
}

/// Aggregated latency metrics in milliseconds.
#[derive(Clone, Debug)]
pub struct LatencyMetrics {
    /// Total number of operations measured.
    pub count: usize,
    /// Minimum observed latency in milliseconds.
    pub min_ms: f64,
    /// Maximum observed latency in milliseconds.
    pub max_ms: f64,
    /// Arithmetic mean latency in milliseconds.
    pub avg_ms: f64,
    /// Median (50th percentile) latency in milliseconds.
    pub p50_ms: f64,
    /// 95th percentile latency in milliseconds.
    pub p95_ms: f64,
    /// 99th percentile latency in milliseconds.
    pub p99_ms: f64,
    /// Throughput in operations per second.
    pub ops_per_sec: f64,
}

impl LatencyMetrics {
    /// Build latency metrics from operation durations.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn from_durations(durations: &[Duration]) -> Self {
        if durations.is_empty() {
            return Self {
                count: 0,
                min_ms: 0.0,
                max_ms: 0.0,
                avg_ms: 0.0,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                ops_per_sec: 0.0,
            };
        }

        let mut ms = durations
            .iter()
            .map(|d| d.as_secs_f64() * 1_000.0)
            .collect::<Vec<_>>();
        ms.sort_by(f64::total_cmp);

        let count = ms.len();
        let min_ms = ms[0];
        let max_ms = ms[count - 1];
        let avg_ms = ms.iter().sum::<f64>() / count as f64;
        let total_s = durations
            .iter()
            .map(Duration::as_secs_f64)
            .sum::<f64>()
            .max(1e-9);

        Self {
            count,
            min_ms,
            max_ms,
            avg_ms,
            p50_ms: percentile(&ms, 0.50),
            p95_ms: percentile(&ms, 0.95),
            p99_ms: percentile(&ms, 0.99),
            ops_per_sec: count as f64 / total_s,
        }
    }
}

/// A single WebSocket connection used to issue frames and measure latency in perf scenarios.
pub struct WsPerfClient {
    stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl WsPerfClient {
    /// Connect to `/api/ws` using a one-time WS ticket.
    ///
    /// # Errors
    ///
    /// Returns an error if URL conversion or websocket handshake fails.
    pub async fn connect(base_url: &str, ticket: &str) -> Result<Self, PerfError> {
        let ws_url = ws_url(base_url, ticket)?;
        let (stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| PerfError::WsConnect(Box::new(e)))?;
        Ok(Self { stream })
    }

    /// Send a frame as binary protobuf over websocket.
    ///
    /// # Errors
    ///
    /// Returns an error if socket send fails.
    pub async fn send(&mut self, frame: &Frame) -> Result<(), PerfError> {
        let bytes = frames::encode_frame(frame);
        self.stream
            .send(Message::Binary(bytes.into()))
            .await
            .map_err(|e| PerfError::WsConnect(Box::new(e)))?;
        Ok(())
    }

    /// Read the next binary frame from websocket.
    ///
    /// Ignores non-binary messages and returns timeout error after `deadline`.
    ///
    /// # Errors
    ///
    /// Returns timeout, decode, or websocket transport errors.
    pub async fn recv_next(&mut self, deadline: Duration) -> Result<Frame, PerfError> {
        let fut = async {
            loop {
                let Some(msg) = self.stream.next().await else {
                    return Err(PerfError::WsClosed);
                };
                match msg.map_err(|e| PerfError::WsConnect(Box::new(e)))? {
                    Message::Binary(bytes) => {
                        return frames::decode_frame(&bytes).map_err(PerfError::from);
                    }
                    Message::Close(_) => return Err(PerfError::WsClosed),
                    _ => {}
                }
            }
        };

        tokio::time::timeout(deadline, fut)
            .await
            .map_err(|_| PerfError::Timeout)?
    }

    /// Wait for the initial `session:connected` frame.
    ///
    /// # Errors
    ///
    /// Returns timeout or websocket errors.
    pub async fn wait_connected(&mut self) -> Result<Frame, PerfError> {
        loop {
            let frame = self.recv_next(Duration::from_secs(5)).await?;
            if frame.syscall == "session:connected" {
                return Ok(frame);
            }
        }
    }

    /// Send a request and wait for the matching terminal response.
    ///
    /// # Errors
    ///
    /// Returns timeout/socket/decode errors, or a server error response.
    pub async fn request(&mut self, request: Frame) -> Result<(Frame, Duration), PerfError> {
        let request_id = request.id.clone();
        let request_syscall = request.syscall.clone();

        let started = Instant::now();
        self.send(&request).await?;

        loop {
            let frame = self.recv_next(Duration::from_secs(10)).await?;
            if frame.parent_id.as_deref() != Some(request_id.as_str()) {
                continue;
            }
            if frame.syscall != request_syscall {
                continue;
            }
            if !is_terminal(frame.status) {
                continue;
            }
            if frame.status == Status::Error {
                let syscall = frame.syscall.clone();
                let message = frame_error_message(&frame);
                return Err(PerfError::ServerError { syscall, message });
            }
            return Ok((frame, started.elapsed()));
        }
    }
}

/// Acquire a one-time WS ticket, using either explicit ticket or session cookie.
///
/// # Errors
///
/// Returns [`PerfError::MissingAuth`] if no auth context exists.
pub async fn acquire_ws_ticket(config: &PerfConfig) -> Result<String, PerfError> {
    if let Some(ticket) = &config.ws_ticket {
        return Ok(ticket.clone());
    }

    if let Some(session_token) = config.session_token.clone() {
        return fetch_ticket_with_session(config, &session_token).await;
    }

    // Dev/perf fallback path: requires server-side PERF_TEST_AUTH_BYPASS=true.
    fetch_ticket_via_dev_bootstrap(config).await
}

/// Acquire multiple one-time WS tickets.
///
/// # Errors
///
/// Returns error when ticket creation fails.
pub async fn acquire_many_ws_tickets(
    config: &PerfConfig,
    count: usize,
) -> Result<Vec<String>, PerfError> {
    if config.ws_ticket.is_some() && count > 1 {
        return Err(PerfError::StaticTicketInsufficient(count));
    }
    let mut tickets = Vec::with_capacity(count);
    for _ in 0..count {
        tickets.push(acquire_ws_ticket(config).await?);
    }
    Ok(tickets)
}

/// Build a request frame with generated id.
#[must_use]
pub fn request_frame(syscall: &str, board_id: Option<&str>, data: serde_json::Value) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: board_id.map(ToOwned::to_owned),
        from: None,
        syscall: syscall.to_owned(),
        status: Status::Request,
        data,
    }
}

/// Extract a required `board_id` string from a response frame payload.
///
/// # Errors
///
/// Returns error when field is missing or not a string.
pub fn board_id_from_response(frame: &Frame) -> Result<String, PerfError> {
    frame
        .data
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(PerfError::MissingField("id"))
}

fn is_terminal(status: Status) -> bool {
    matches!(status, Status::Done | Status::Error | Status::Cancel)
}

fn frame_error_message(frame: &Frame) -> String {
    frame
        .data
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("request failed")
        .to_owned()
}

async fn fetch_ticket_with_session(
    config: &PerfConfig,
    session_token: &str,
) -> Result<String, PerfError> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("session_token={session_token}"))?,
    );

    let url = format!(
        "{}/api/auth/ws-ticket",
        config.base_url.trim_end_matches('/')
    );
    let body = client
        .post(url)
        .headers(headers)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    body.get("ticket")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(PerfError::MissingField("ticket"))
}

async fn fetch_ticket_via_dev_bootstrap(config: &PerfConfig) -> Result<String, PerfError> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/dev/ws-ticket",
        config.base_url.trim_end_matches('/')
    );
    let response = client.post(url).send().await?;
    if !response.status().is_success() {
        return Err(PerfError::MissingAuth);
    }

    let body = response.json::<serde_json::Value>().await?;
    body.get("ticket")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(PerfError::MissingField("ticket"))
}

fn ws_url(base_url: &str, ticket: &str) -> Result<String, PerfError> {
    let trimmed = base_url.trim_end_matches('/');

    if let Some(rest) = trimmed.strip_prefix("http://") {
        return Ok(format!("ws://{rest}/api/ws?ticket={ticket}"));
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        return Ok(format!("wss://{rest}/api/ws?ticket={ticket}"));
    }

    Err(PerfError::InvalidBaseUrl(base_url.to_owned()))
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_usize_list(key: &str, default: &[usize]) -> Vec<usize> {
    let Some(raw) = std::env::var(key).ok() else {
        return default.to_vec();
    };

    let values = raw
        .split(',')
        .filter_map(|part| part.trim().parse::<usize>().ok())
        .collect::<Vec<_>>();

    if values.is_empty() {
        default.to_vec()
    } else {
        values
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let index = ((sorted_values.len() - 1) as f64 * p).round() as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

#[cfg(test)]
#[path = "e2e_perf_test.rs"]
mod tests;
