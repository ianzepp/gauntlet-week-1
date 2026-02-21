use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Args, Parser, Subcommand};
use frames::{Frame, Status};
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{COOKIE, HeaderMap, HeaderValue};
use serde_json::{Map, Value};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("missing session token; pass --session-token or set COLLAB_SESSION_TOKEN")]
    MissingSessionToken,
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    #[error("websocket connect failed: {0}")]
    WsConnect(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("websocket closed")]
    WsClosed,
    #[error("frame decode failed: {0}")]
    Decode(#[from] frames::CodecError),
    #[error("timed out waiting for websocket frame")]
    Timeout,
    #[error("server returned error for {syscall}: {message}")]
    ServerError { syscall: String, message: String },
    #[error("missing expected field `{0}`")]
    MissingField(&'static str),
    #[error("invalid JSON payload: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

#[derive(Parser, Debug)]
#[command(name = "collab-cli", about = "CollabBoard API and websocket CLI")]
struct Cli {
    #[arg(long, env = "COLLAB_BASE_URL", default_value = "http://127.0.0.1:3000")]
    base_url: String,

    #[arg(long, env = "COLLAB_SESSION_TOKEN")]
    session_token: Option<String>,

    #[arg(long, env = "COLLAB_WS_TICKET")]
    ws_ticket: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone)]
struct CliContext {
    base_url: String,
    session_token: Option<String>,
    ws_ticket: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Api(ApiCommand),
    Ws(WsCommand),
}

#[derive(Args, Debug)]
struct ApiCommand {
    #[command(subcommand)]
    command: ApiSubcommand,
}

#[derive(Subcommand, Debug)]
enum ApiSubcommand {
    Board(BoardCommand),
    Object(ObjectCommand),
}

#[derive(Args, Debug)]
struct BoardCommand {
    #[command(subcommand)]
    command: BoardSubcommand,
}

#[derive(Subcommand, Debug)]
enum BoardSubcommand {
    List,
    Get {
        board_id: Uuid,
    },
}

#[derive(Args, Debug)]
struct ObjectCommand {
    #[command(subcommand)]
    command: ObjectSubcommand,
}

#[derive(Subcommand, Debug)]
enum ObjectSubcommand {
    List {
        board_id: Uuid,
    },
    Create {
        board_id: Uuid,
        #[arg(long)]
        data: String,
    },
    Get {
        board_id: Uuid,
        object_id: Uuid,
    },
    Patch {
        board_id: Uuid,
        object_id: Uuid,
        #[arg(long)]
        data: String,
    },
    Delete {
        board_id: Uuid,
        object_id: Uuid,
    },
}

#[derive(Args, Debug)]
struct WsCommand {
    #[command(subcommand)]
    command: WsSubcommand,
}

#[derive(Subcommand, Debug)]
enum WsSubcommand {
    StreamCreate(StreamCreateArgs),
}

#[derive(Args, Debug)]
struct StreamCreateArgs {
    #[arg(long)]
    board_id: Uuid,

    #[arg(long, default_value_t = 100)]
    count: usize,

    #[arg(long, default_value = "sticky_note")]
    kind: String,

    #[arg(long, default_value_t = 0.0)]
    start_x: f64,

    #[arg(long, default_value_t = 0.0)]
    start_y: f64,

    #[arg(long, default_value_t = 60.0)]
    step_x: f64,

    #[arg(long, default_value_t = 60.0)]
    step_y: f64,

    #[arg(long, default_value_t = 40)]
    columns: usize,

    #[arg(long, default_value_t = 160.0)]
    width: f64,

    #[arg(long, default_value_t = 100.0)]
    height: f64,

    #[arg(long, default_value_t = 0.0)]
    rotation: f64,

    #[arg(long)]
    props: Option<String>,

    #[arg(long, default_value_t = true)]
    wait_for_ack: bool,
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    let ctx = CliContext {
        base_url: cli.base_url,
        session_token: cli.session_token,
        ws_ticket: cli.ws_ticket,
    };

    match cli.command {
        Command::Api(api) => run_api(&ctx, api).await,
        Command::Ws(ws) => run_ws(&ctx, ws).await,
    }
}

async fn run_api(cli: &CliContext, api: ApiCommand) -> Result<(), CliError> {
    match api.command {
        ApiSubcommand::Board(board) => run_api_board(cli, board).await,
        ApiSubcommand::Object(object) => run_api_object(cli, object).await,
    }
}

async fn run_api_board(cli: &CliContext, board: BoardCommand) -> Result<(), CliError> {
    match board.command {
        BoardSubcommand::List => {
            let json = api_request(cli, reqwest::Method::GET, "/api/board", None).await?;
            print_json(&json)?;
            Ok(())
        }
        BoardSubcommand::Get { board_id } => {
            let path = format!("/api/board/{board_id}");
            let json = api_request(cli, reqwest::Method::GET, &path, None).await?;
            print_json(&json)?;
            Ok(())
        }
    }
}

async fn run_api_object(cli: &CliContext, object: ObjectCommand) -> Result<(), CliError> {
    match object.command {
        ObjectSubcommand::List { board_id } => {
            let path = format!("/api/board/{board_id}/objects");
            let json = api_request(cli, reqwest::Method::GET, &path, None).await?;
            print_json(&json)?;
            Ok(())
        }
        ObjectSubcommand::Create { board_id, data } => {
            let body = serde_json::from_str::<Value>(&data)?;
            let path = format!("/api/board/{board_id}/objects");
            let json = api_request(cli, reqwest::Method::POST, &path, Some(body)).await?;
            print_json(&json)?;
            Ok(())
        }
        ObjectSubcommand::Get { board_id, object_id } => {
            let path = format!("/api/board/{board_id}/objects/{object_id}");
            let json = api_request(cli, reqwest::Method::GET, &path, None).await?;
            print_json(&json)?;
            Ok(())
        }
        ObjectSubcommand::Patch {
            board_id,
            object_id,
            data,
        } => {
            let body = serde_json::from_str::<Value>(&data)?;
            let path = format!("/api/board/{board_id}/objects/{object_id}");
            let json = api_request(cli, reqwest::Method::PATCH, &path, Some(body)).await?;
            print_json(&json)?;
            Ok(())
        }
        ObjectSubcommand::Delete {
            board_id,
            object_id,
        } => {
            let path = format!("/api/board/{board_id}/objects/{object_id}");
            let json = api_request(cli, reqwest::Method::DELETE, &path, None).await?;
            print_json(&json)?;
            Ok(())
        }
    }
}

async fn run_ws(cli: &CliContext, ws: WsCommand) -> Result<(), CliError> {
    match ws.command {
        WsSubcommand::StreamCreate(args) => stream_create(cli, args).await,
    }
}

async fn stream_create(cli: &CliContext, args: StreamCreateArgs) -> Result<(), CliError> {
    let ticket = match &cli.ws_ticket {
        Some(ticket) => ticket.clone(),
        None => fetch_ws_ticket(cli).await?,
    };

    let ws_url = ws_url(&cli.base_url, &ticket)?;
    let (mut stream, _) = connect_async(ws_url)
        .await
        .map_err(|error| CliError::WsConnect(Box::new(error)))?;

    wait_for_session_connected(&mut stream).await?;

    let join = request_frame("board:join", Some(args.board_id), Value::Object(Map::new()));
    let join_id = join.id.clone();
    stream
        .send(Message::Binary(frames::encode_frame(&join).into()))
        .await
        .map_err(|error| CliError::WsConnect(Box::new(error)))?;
    wait_for_terminal_response(&mut stream, &join_id, "board:join").await?;

    let base_props = args
        .props
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()?
        .unwrap_or_else(|| Value::Object(Map::new()));

    for index in 0..args.count {
        let col = index % args.columns.max(1);
        let row = index / args.columns.max(1);

        let x = args.start_x + (col as f64 * args.step_x);
        let y = args.start_y + (row as f64 * args.step_y);

        let mut props_obj = match &base_props {
            Value::Object(map) => map.clone(),
            _ => Map::new(),
        };
        props_obj.insert("text".to_owned(), Value::String(format!("stream-{index}")));

        let payload = serde_json::json!({
            "kind": args.kind,
            "x": x,
            "y": y,
            "width": args.width,
            "height": args.height,
            "rotation": args.rotation,
            "props": props_obj,
        });

        let req = request_frame("object:create", Some(args.board_id), payload);
        let req_id = req.id.clone();
        stream
            .send(Message::Binary(frames::encode_frame(&req).into()))
            .await
            .map_err(|error| CliError::WsConnect(Box::new(error)))?;

        if args.wait_for_ack {
            wait_for_terminal_response(&mut stream, &req_id, "object:create").await?;
        }
    }

    eprintln!(
        "stream_create complete: board_id={} count={} wait_for_ack={}",
        args.board_id, args.count, args.wait_for_ack
    );
    Ok(())
}

async fn api_request(
    cli: &CliContext,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, CliError> {
    let session_token = cli
        .session_token
        .as_deref()
        .ok_or(CliError::MissingSessionToken)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("session_token={session_token}"))?,
    );

    let client = reqwest::Client::builder().default_headers(headers).build()?;
    let url = format!("{}{}", cli.base_url.trim_end_matches('/'), path);

    let request = client.request(method, &url);
    let request = if let Some(json) = body {
        request.json(&json)
    } else {
        request
    };

    let response = request.send().await?;
    let status = response.status();
    let value = response.json::<Value>().await.unwrap_or_else(|_| Value::Null);

    if !status.is_success() {
        return Err(CliError::ServerError {
            syscall: format!("HTTP {}", status.as_u16()),
            message: value.to_string(),
        });
    }

    Ok(value)
}

async fn fetch_ws_ticket(cli: &CliContext) -> Result<String, CliError> {
    let body = api_request(cli, reqwest::Method::POST, "/api/auth/ws-ticket", Some(Value::Object(Map::new())))
        .await?;

    body.get("ticket")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(CliError::MissingField("ticket"))
}

fn ws_url(base_url: &str, ticket: &str) -> Result<String, CliError> {
    if let Some(rest) = base_url.strip_prefix("http://") {
        return Ok(format!("ws://{rest}/api/ws?ticket={ticket}"));
    }
    if let Some(rest) = base_url.strip_prefix("https://") {
        return Ok(format!("wss://{rest}/api/ws?ticket={ticket}"));
    }

    Err(CliError::InvalidBaseUrl(base_url.to_owned()))
}

async fn wait_for_session_connected(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Result<(), CliError> {
    loop {
        let frame = recv_next(stream, Duration::from_secs(5)).await?;
        if frame.syscall == "session:connected" {
            return Ok(());
        }
    }
}

async fn wait_for_terminal_response(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    request_id: &str,
    syscall: &str,
) -> Result<Frame, CliError> {
    loop {
        let frame = recv_next(stream, Duration::from_secs(15)).await?;
        if frame.parent_id.as_deref() != Some(request_id) {
            continue;
        }
        if frame.syscall != syscall {
            continue;
        }
        if !matches!(frame.status, Status::Done | Status::Error | Status::Cancel) {
            continue;
        }
        if frame.status == Status::Error {
            return Err(CliError::ServerError {
                syscall: frame.syscall,
                message: frame
                    .data
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown websocket error")
                    .to_owned(),
            });
        }
        return Ok(frame);
    }
}

async fn recv_next(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    timeout: Duration,
) -> Result<Frame, CliError> {
    let fut = async {
        loop {
            let Some(message) = stream.next().await else {
                return Err(CliError::WsClosed);
            };
            match message.map_err(|error| CliError::WsConnect(Box::new(error)))? {
                Message::Binary(bytes) => return frames::decode_frame(&bytes).map_err(CliError::from),
                Message::Close(_) => return Err(CliError::WsClosed),
                _ => {}
            }
        }
    };

    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| CliError::Timeout)?
}

fn request_frame(syscall: &str, board_id: Option<Uuid>, data: Value) -> Frame {
    Frame {
        id: Uuid::new_v4().to_string(),
        parent_id: None,
        ts: now_ms(),
        board_id: board_id.map(|value| value.to_string()),
        from: None,
        syscall: syscall.to_owned(),
        status: Status::Request,
        data,
    }
}

fn now_ms() -> i64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(duration.as_millis()).unwrap_or(0)
}

fn print_json(value: &Value) -> Result<(), CliError> {
    let rendered = serde_json::to_string_pretty(value)?;
    println!("{rendered}");
    Ok(())
}
