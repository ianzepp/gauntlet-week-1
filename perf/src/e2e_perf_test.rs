use std::sync::Arc;
use std::time::Duration;

use super::*;

fn print_metrics(name: &str, metrics: &LatencyMetrics) {
    println!(
        "[{name}] count={} min={:.2}ms p50={:.2}ms p95={:.2}ms p99={:.2}ms avg={:.2}ms max={:.2}ms ops/s={:.2}",
        metrics.count,
        metrics.min_ms,
        metrics.p50_ms,
        metrics.p95_ms,
        metrics.p99_ms,
        metrics.avg_ms,
        metrics.max_ms,
        metrics.ops_per_sec
    );
}

async fn connect_client(config: &PerfConfig) -> Result<WsPerfClient, PerfError> {
    let ticket = acquire_ws_ticket(config).await?;
    let mut client = WsPerfClient::connect(&config.base_url, &ticket).await?;
    let _ = client.wait_connected().await?;
    Ok(client)
}

async fn create_and_join_board(client: &mut WsPerfClient) -> Result<String, PerfError> {
    let create = request_frame(
        "board:create",
        None,
        serde_json::json!({ "name": "perf-board" }),
    );
    let (create_resp, _) = client.request(create).await?;
    let board_id = board_id_from_response(&create_resp)?;

    let join = request_frame("board:join", Some(&board_id), serde_json::json!({}));
    let _ = client.request(join).await?;

    Ok(board_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live perf test; run manually with PERF_SESSION_TOKEN and --ignored"]
async fn ws_round_trip_latency_test() -> Result<(), PerfError> {
    let config = PerfConfig::from_env();
    let mut client = connect_client(&config).await?;

    let mut latencies = Vec::with_capacity(config.baseline_requests);
    for _ in 0..config.baseline_requests {
        let req = request_frame("board:list", None, serde_json::json!({}));
        let (_, elapsed) = client.request(req).await?;
        latencies.push(elapsed);
    }

    let metrics = LatencyMetrics::from_durations(&latencies);
    print_metrics("ws_round_trip_latency", &metrics);

    assert!(metrics.count > 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live perf test; run manually with PERF_SESSION_TOKEN and --ignored"]
async fn board_complexity_object_create_perf_test() -> Result<(), PerfError> {
    let config = PerfConfig::from_env();
    let mut client = connect_client(&config).await?;
    let board_id = create_and_join_board(&mut client).await?;

    for &count in &config.complexity_counts {
        let mut latencies = Vec::with_capacity(count);

        for idx in 0..count {
            let req = request_frame(
                "object:create",
                Some(&board_id),
                serde_json::json!({
                    "kind": "sticky_note",
                    "x": (idx % 100) as f64 * 12.0,
                    "y": (idx / 100) as f64 * 12.0,
                    "width": 160.0,
                    "height": 100.0,
                    "rotation": 0.0,
                    "props": {"text": format!("obj-{idx}")}
                }),
            );

            let (_, elapsed) = client.request(req).await?;
            latencies.push(elapsed);
        }

        let metrics = LatencyMetrics::from_durations(&latencies);
        print_metrics(&format!("board_complexity_objects_{count}"), &metrics);
        assert_eq!(metrics.count, count);
    }

    Ok(())
}

async fn run_mass_user_worker(
    base_url: String,
    ticket: String,
    board_id: String,
    requests_per_user: usize,
    barrier: Arc<tokio::sync::Barrier>,
) -> Result<Vec<Duration>, PerfError> {
    let mut client = WsPerfClient::connect(&base_url, &ticket).await?;
    let _ = client.wait_connected().await?;

    let join = request_frame("board:join", Some(&board_id), serde_json::json!({}));
    let _ = client.request(join).await?;

    barrier.wait().await;

    let mut latencies = Vec::with_capacity(requests_per_user);
    for _ in 0..requests_per_user {
        let req = request_frame("board:users:list", Some(&board_id), serde_json::json!({}));
        let (_, elapsed) = client.request(req).await?;
        latencies.push(elapsed);
    }

    Ok(latencies)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "live perf test; run manually with PERF_SESSION_TOKEN and --ignored"]
async fn mass_user_concurrent_perf_test() -> Result<(), PerfError> {
    let config = PerfConfig::from_env();

    let mut bootstrap = connect_client(&config).await?;
    let board_id = create_and_join_board(&mut bootstrap).await?;

    let tickets = acquire_many_ws_tickets(&config, config.mass_users).await?;
    let barrier = Arc::new(tokio::sync::Barrier::new(config.mass_users));

    let mut handles = Vec::with_capacity(config.mass_users);
    for ticket in tickets {
        let base_url = config.base_url.clone();
        let board_id = board_id.clone();
        let barrier = Arc::clone(&barrier);
        let requests = config.mass_requests_per_user;

        handles.push(tokio::spawn(async move {
            run_mass_user_worker(base_url, ticket, board_id, requests, barrier).await
        }));
    }

    let mut all_latencies = Vec::new();
    for handle in handles {
        let worker_latencies = handle.await.map_err(|_| PerfError::Timeout)??;
        all_latencies.extend(worker_latencies);
    }

    let metrics = LatencyMetrics::from_durations(&all_latencies);
    print_metrics(
        &format!(
            "mass_user_concurrent_users_{}_requests_{}",
            config.mass_users, config.mass_requests_per_user
        ),
        &metrics,
    );

    assert_eq!(
        metrics.count,
        config.mass_users * config.mass_requests_per_user
    );
    Ok(())
}
