use std::fs::File;
use std::io::{self, BufRead, BufReader};
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
    Ping,
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
    Read {
        board_id: Uuid,
    },
    Create {
        #[arg(long, default_value = "Untitled Board")]
        name: String,
    },
    Update {
        board_id: Uuid,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        is_public: Option<bool>,
    },
    Delete {
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
    Read {
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
    Object(WsObjectCommand),
    #[command(hide = true)]
    StreamCreate(StreamCreateArgs),
}

#[derive(Args, Debug)]
struct WsObjectCommand {
    #[command(subcommand)]
    command: WsObjectSubcommand,
}

#[derive(Subcommand, Debug)]
enum WsObjectSubcommand {
    Create(WsObjectCreateArgs),
}

#[derive(Args, Debug)]
struct WsObjectCreateArgs {
    #[arg(long, required_unless_present = "create_board")]
    board_id: Option<Uuid>,

    #[arg(long, default_value_t = false)]
    create_board: bool,

    #[arg(long, default_value = "CLI Stream Board")]
    board_name: String,

    #[arg(long, default_value = "-", help = "Input file path, or - for stdin")]
    input: String,

    #[arg(long, default_value_t = true)]
    wait_for_ack: bool,

    #[arg(long, help = "Stop after this many created objects")]
    max_objects: Option<usize>,

    #[arg(long, default_value_t = 1000)]
    progress_every: usize,
}

#[derive(Args, Debug)]
struct StreamCreateArgs {
    #[arg(long)]
    board_id: Uuid,

    #[arg(long, default_value_t = 100)]
    count: usize,
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
        Command::Ping => run_ping(&ctx).await,
        Command::Api(api) => run_api(&ctx, api).await,
        Command::Ws(ws) => run_ws(&ctx, ws).await,
    }
}

async fn run_ping(cli: &CliContext) -> Result<(), CliError> {
    let client = reqwest::Client::new();
    let url = format!("{}/healthz", cli.base_url.trim_end_matches('/'));
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(CliError::ServerError {
            syscall: format!("HTTP {}", status.as_u16()),
            message: "health check failed".to_owned(),
        });
    }
    println!("ok");
    Ok(())
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
        BoardSubcommand::Read { board_id } => {
            let path = format!("/api/board/{board_id}");
            let json = api_request(cli, reqwest::Method::GET, &path, None).await?;
            print_json(&json)?;
            Ok(())
        }
        BoardSubcommand::Create { name } => {
            let json = api_request(
                cli,
                reqwest::Method::POST,
                "/api/board",
                Some(serde_json::json!({ "name": name })),
            )
            .await?;
            print_json(&json)?;
            Ok(())
        }
        BoardSubcommand::Update {
            board_id,
            name,
            is_public,
        } => {
            let mut body = Map::new();
            if let Some(name) = name {
                body.insert("name".to_owned(), Value::String(name));
            }
            if let Some(is_public) = is_public {
                body.insert("is_public".to_owned(), Value::Bool(is_public));
            }
            let path = format!("/api/board/{board_id}");
            let json = api_request(
                cli,
                reqwest::Method::PATCH,
                &path,
                Some(Value::Object(body)),
            )
            .await?;
            print_json(&json)?;
            Ok(())
        }
        BoardSubcommand::Delete { board_id } => {
            let path = format!("/api/board/{board_id}");
            let json = api_request(cli, reqwest::Method::DELETE, &path, None).await?;
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
        ObjectSubcommand::Read {
            board_id,
            object_id,
        } => {
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
        WsSubcommand::Object(command) => match command.command {
            WsObjectSubcommand::Create(args) => ws_object_create(cli, args).await,
        },
        WsSubcommand::StreamCreate(args) => stream_create_legacy(cli, args).await,
    }
}

async fn ws_object_create(cli: &CliContext, args: WsObjectCreateArgs) -> Result<(), CliError> {
    let board_id = if args.create_board {
        let created = api_request(
            cli,
            reqwest::Method::POST,
            "/api/board",
            Some(serde_json::json!({ "name": args.board_name })),
        )
        .await?;
        let id = created
            .get("id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or(CliError::MissingField("id"))?;
        eprintln!("created board: {id}");
        id
    } else {
        args.board_id.ok_or(CliError::MissingField("board_id"))?
    };

    let ticket = match &cli.ws_ticket {
        Some(ticket) => ticket.clone(),
        None => fetch_ws_ticket(cli).await?,
    };

    let ws_url = ws_url(&cli.base_url, &ticket)?;
    let (mut stream, _) = connect_async(ws_url)
        .await
        .map_err(|error| CliError::WsConnect(Box::new(error)))?;

    wait_for_session_connected(&mut stream).await?;

    let join = request_frame("board:join", Some(board_id), Value::Object(Map::new()));
    let join_id = join.id.clone();
    stream
        .send(Message::Binary(frames::encode_frame(&join).into()))
        .await
        .map_err(|error| CliError::WsConnect(Box::new(error)))?;
    wait_for_terminal_response(&mut stream, &join_id, "board:join").await?;

    let mut sent = 0_usize;
    let mut skipped = 0_usize;
    let mut reader: Box<dyn BufRead> = if args.input == "-" {
        Box::new(BufReader::new(io::stdin()))
    } else {
        let file = File::open(&args.input).map_err(|error| CliError::ServerError {
            syscall: "open input".to_owned(),
            message: error.to_string(),
        })?;
        Box::new(BufReader::new(file))
    };

    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| CliError::ServerError {
                syscall: "read input".to_owned(),
                message: error.to_string(),
            })?;
        if bytes == 0 {
            break;
        }

        let Some(payload) = parse_jsonl_object_line(&line)? else {
            skipped = skipped.saturating_add(1);
            continue;
        };

        let req = request_frame("object:create", Some(board_id), payload);
        let req_id = req.id.clone();
        stream
            .send(Message::Binary(frames::encode_frame(&req).into()))
            .await
            .map_err(|error| CliError::WsConnect(Box::new(error)))?;

        if args.wait_for_ack {
            wait_for_terminal_response(&mut stream, &req_id, "object:create").await?;
        }

        sent = sent.saturating_add(1);
        if args.progress_every > 0 && sent.is_multiple_of(args.progress_every) {
            eprintln!("streamed {sent} objects...");
        }
        if args.max_objects.is_some_and(|limit| sent >= limit) {
            break;
        }
    }

    eprintln!(
        "ws object create complete: board_id={} created={} skipped={} wait_for_ack={}",
        board_id, sent, skipped, args.wait_for_ack
    );
    Ok(())
}

async fn stream_create_legacy(cli: &CliContext, args: StreamCreateArgs) -> Result<(), CliError> {
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

    for index in 0..args.count {
        let payload = serde_json::json!({
            "kind": "sticky_note",
            "x": index as f64,
            "y": index as f64,
            "width": 160.0,
            "height": 100.0,
            "rotation": 0.0,
            "props": { "text": format!("stream-{index}") },
        });

        let req = request_frame("object:create", Some(args.board_id), payload);
        let req_id = req.id.clone();
        stream
            .send(Message::Binary(frames::encode_frame(&req).into()))
            .await
            .map_err(|error| CliError::WsConnect(Box::new(error)))?;
        wait_for_terminal_response(&mut stream, &req_id, "object:create").await?;
    }

    eprintln!(
        "legacy stream-create complete: board_id={} count={}",
        args.board_id, args.count
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

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    let url = format!("{}{}", cli.base_url.trim_end_matches('/'), path);

    let request = client.request(method, &url);
    let request = if let Some(json) = body {
        request.json(&json)
    } else {
        request
    };

    let response = request.send().await?;
    let status = response.status();
    let value = response
        .json::<Value>()
        .await
        .unwrap_or_else(|_| Value::Null);

    if !status.is_success() {
        return Err(CliError::ServerError {
            syscall: format!("HTTP {}", status.as_u16()),
            message: value.to_string(),
        });
    }

    Ok(value)
}

async fn fetch_ws_ticket(cli: &CliContext) -> Result<String, CliError> {
    let body = api_request(
        cli,
        reqwest::Method::POST,
        "/api/auth/ws-ticket",
        Some(Value::Object(Map::new())),
    )
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
                Message::Binary(bytes) => {
                    return frames::decode_frame(&bytes).map_err(CliError::from);
                }
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

fn parse_jsonl_object_line(line: &str) -> Result<Option<Value>, CliError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut value = serde_json::from_str::<Value>(trimmed)?;
    let Some(map) = value.as_object_mut() else {
        return Ok(None);
    };

    if let Some(line_type) = map.get("type").and_then(Value::as_str) {
        if line_type == "board_export_meta" {
            return Ok(None);
        }
        if line_type != "object" {
            return Ok(None);
        }
        map.remove("type");
    }

    map.remove("id");
    map.remove("board_id");
    map.remove("created_by");
    map.remove("version");
    map.remove("z_index");

    if !map.contains_key("kind") {
        return Ok(None);
    }
    Ok(Some(value))
}
