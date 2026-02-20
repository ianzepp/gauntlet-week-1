use std::collections::HashMap;

use frames::{Frame, Status};
use serde_json::json;

use crate::{
    TraceFilter, TraceSession, build_trace_sessions, compute_metrics, pair_request_spans,
    prefix_display, sub_label, syscall_prefix, tree_depth,
};

fn frame(id: &str, parent: Option<&str>, ts: i64, syscall: &str, status: Status) -> Frame {
    Frame {
        id: id.to_owned(),
        parent_id: parent.map(str::to_owned),
        ts,
        board_id: Some("board-1".to_owned()),
        from: Some("server".to_owned()),
        syscall: syscall.to_owned(),
        status,
        data: json!({}),
    }
}

fn frame_with_board(
    id: &str,
    parent: Option<&str>,
    ts: i64,
    syscall: &str,
    status: Status,
    board_id: Option<&str>,
) -> Frame {
    Frame {
        id: id.to_owned(),
        parent_id: parent.map(str::to_owned),
        ts,
        board_id: board_id.map(str::to_owned),
        from: Some("server".to_owned()),
        syscall: syscall.to_owned(),
        status,
        data: json!({}),
    }
}

#[test]
fn prefix_mapping_matches_spec() {
    let ai = prefix_display("ai:tool_call");
    assert_eq!(ai.letter, "A");
    assert_eq!(ai.color, "#4ad981");

    let other = prefix_display("weird:event");
    assert_eq!(other.letter, "-");
    assert_eq!(other.label, "OTHER");
}

#[test]
fn syscall_prefix_handles_empty_and_no_separator() {
    assert_eq!(syscall_prefix("ai:prompt"), "ai");
    assert_eq!(syscall_prefix("board"), "board");
    assert_eq!(syscall_prefix(""), "");
}

#[test]
fn default_filter_hides_cursor_and_item() {
    let filter = TraceFilter::default();
    assert!(filter.allows(&frame("1", None, 1, "ai:prompt", Status::Request)));
    assert!(filter.allows(&frame(
        "t1",
        Some("1"),
        2,
        "tool:applyChangesYaml",
        Status::Done
    )));
    assert!(!filter.allows(&frame("2", None, 2, "cursor:move", Status::Done)));
    assert!(!filter.allows(&frame("3", None, 3, "chat:message", Status::Item)));
}

#[test]
fn include_all_filter_allows_cursor_item_cancel_and_other() {
    let filter = TraceFilter::include_all();
    assert!(filter.allows(&frame("1", None, 1, "cursor:move", Status::Item)));
    assert!(filter.allows(&frame("2", None, 2, "board:join", Status::Cancel)));
    assert!(filter.allows(&frame("3", None, 3, "", Status::Done)));
}

#[test]
fn filter_toggles_prefixes_and_statuses() {
    let mut filter = TraceFilter::default();
    assert!(filter.allows(&frame("1", None, 1, "ai:prompt", Status::Done)));

    filter.set_prefix_enabled("ai", false);
    assert!(!filter.allows(&frame("2", None, 2, "ai:prompt", Status::Done)));

    filter.set_prefix_enabled("cursor", true);
    filter.set_status_enabled(Status::Item, true);
    assert!(filter.allows(&frame("3", None, 3, "cursor:move", Status::Item)));

    filter.set_status_enabled(Status::Done, false);
    assert!(!filter.allows(&frame("4", None, 4, "board:join", Status::Done)));

    let prefixes = filter.active_prefixes();
    assert!(prefixes.contains(&"cursor".to_owned()));
    assert!(!prefixes.contains(&"ai".to_owned()));
    let statuses = filter.active_statuses();
    assert!(statuses.contains(&Status::Item));
    assert!(!statuses.contains(&Status::Done));
}

#[test]
fn build_trace_sessions_empty_input() {
    assert!(build_trace_sessions(&[]).is_empty());
}

#[test]
fn trace_sessions_group_by_root_parent_chain() {
    let frames = vec![
        frame("root-a", None, 100, "ai:prompt", Status::Request),
        frame(
            "child-a1",
            Some("root-a"),
            110,
            "ai:llm_request",
            Status::Request,
        ),
        frame("root-b", None, 200, "board:join", Status::Done),
        frame(
            "child-a2",
            Some("child-a1"),
            120,
            "ai:tool_call",
            Status::Done,
        ),
    ];

    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 2);

    assert_eq!(sessions[0].root_frame_id, "root-a");
    assert_eq!(sessions[0].frames.len(), 3);
    assert!(sessions[0].ended_at.is_some());

    assert_eq!(sessions[1].root_frame_id, "root-b");
    assert_eq!(sessions[1].frames.len(), 1);
}

#[test]
fn trace_sessions_keep_open_session_without_terminal_frame() {
    let frames = vec![
        frame("root", None, 10, "ai:prompt", Status::Request),
        frame("item", Some("root"), 11, "ai:stream", Status::Item),
    ];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].ended_at, None);
}

#[test]
fn trace_sessions_handle_missing_parent_as_root() {
    let frames = vec![
        frame("orphan", Some("missing"), 5, "chat:message", Status::Done),
        frame("root", None, 8, "board:join", Status::Done),
    ];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].root_frame_id, "orphan");
    assert_eq!(sessions[1].root_frame_id, "root");
}

#[test]
fn trace_sessions_cycle_in_parent_chain_does_not_hang() {
    let frames = vec![
        frame("a", Some("b"), 1, "ai:prompt", Status::Request),
        frame("b", Some("a"), 2, "ai:prompt", Status::Done),
    ];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 2);
}

#[test]
fn span_pairing_handles_request_done_and_done_only() {
    let frames = vec![
        frame("req-1", Some("root"), 1000, "ai:tool_call", Status::Request),
        frame("done-1", Some("root"), 1180, "ai:tool_call", Status::Done),
        frame("done-2", Some("root"), 1300, "chat:message", Status::Done),
    ];

    let spans = pair_request_spans(&frames);
    assert_eq!(spans.len(), 2);

    assert_eq!(spans[0].request_frame_id.as_deref(), Some("req-1"));
    assert_eq!(spans[0].duration_ms, 180);

    assert_eq!(spans[1].request_frame_id, None);
    assert_eq!(spans[1].duration_ms, 0);
}

#[test]
fn span_pairing_fifo_for_multiple_pending_requests() {
    let frames = vec![
        frame("req-1", Some("root"), 100, "ai:tool_call", Status::Request),
        frame("req-2", Some("root"), 110, "ai:tool_call", Status::Request),
        frame("done-1", Some("root"), 140, "ai:tool_call", Status::Done),
        frame("done-2", Some("root"), 170, "ai:tool_call", Status::Done),
    ];
    let spans = pair_request_spans(&frames);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].request_frame_id.as_deref(), Some("req-1"));
    assert_eq!(spans[1].request_frame_id.as_deref(), Some("req-2"));
    assert_eq!(spans[0].duration_ms, 40);
    assert_eq!(spans[1].duration_ms, 60);
}

#[test]
fn span_pairing_scopes_by_parent_and_ignores_item() {
    let frames = vec![
        frame("req-a", Some("a"), 100, "ai:llm_request", Status::Request),
        frame("item-a", Some("a"), 110, "ai:llm_request", Status::Item),
        frame("req-b", Some("b"), 120, "ai:llm_request", Status::Request),
        frame("done-b", Some("b"), 130, "ai:llm_request", Status::Done),
        frame("done-a", Some("a"), 180, "ai:llm_request", Status::Done),
    ];
    let spans = pair_request_spans(&frames);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].request_frame_id.as_deref(), Some("req-b"));
    assert_eq!(spans[1].request_frame_id.as_deref(), Some("req-a"));
}

#[test]
fn span_pairing_treats_cancel_as_terminal() {
    let frames = vec![
        frame("req-1", Some("root"), 100, "board:join", Status::Request),
        frame("cancel-1", Some("root"), 140, "board:join", Status::Cancel),
    ];
    let spans = pair_request_spans(&frames);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].request_frame_id.as_deref(), Some("req-1"));
    assert_eq!(spans[0].duration_ms, 40);
}

#[test]
fn metrics_compute_prefix_counts_errors_and_pending() {
    let frames = vec![
        frame("1", None, 1, "ai:prompt", Status::Request),
        frame("2", None, 2, "ai:prompt", Status::Error),
        frame("3", None, 3, "object:update", Status::Request),
    ];

    let metrics = compute_metrics(&frames);
    assert_eq!(metrics.total, 3);
    assert_eq!(metrics.errors, 1);
    assert_eq!(metrics.pending_requests, 1);
    assert_eq!(metrics.by_prefix.get("ai"), Some(&2));
    assert_eq!(metrics.by_prefix.get("object"), Some(&1));
}

#[test]
fn metrics_empty_input_is_zeroed() {
    let metrics = compute_metrics(&[]);
    assert_eq!(metrics.total, 0);
    assert_eq!(metrics.errors, 0);
    assert_eq!(metrics.pending_requests, 0);
    assert!(metrics.by_prefix.is_empty());
}

#[test]
fn metrics_counts_other_prefix_and_ignores_unmatched_done_underflow() {
    let frames = vec![
        frame("done-only", None, 1, "ai:prompt", Status::Done),
        frame("other", None, 2, "", Status::Request),
    ];
    let metrics = compute_metrics(&frames);
    assert_eq!(metrics.pending_requests, 1);
    assert_eq!(metrics.by_prefix.get("other"), Some(&1));
}

#[test]
fn tree_depth_walks_parent_chain() {
    let by_id = HashMap::from([
        (
            "a".to_owned(),
            frame("a", None, 1, "board:join", Status::Done),
        ),
        (
            "b".to_owned(),
            frame("b", Some("a"), 2, "ai:prompt", Status::Request),
        ),
        (
            "c".to_owned(),
            frame("c", Some("b"), 3, "ai:tool_call", Status::Done),
        ),
    ]);

    assert_eq!(tree_depth("a", &by_id), 0);
    assert_eq!(tree_depth("b", &by_id), 1);
    assert_eq!(tree_depth("c", &by_id), 2);
}

#[test]
fn tree_depth_for_missing_frame_is_zero() {
    let by_id = HashMap::new();
    assert_eq!(tree_depth("missing", &by_id), 0);
}

#[test]
fn tree_depth_breaks_on_cycle() {
    let by_id = HashMap::from([
        ("a".to_owned(), frame("a", Some("b"), 1, "x", Status::Done)),
        ("b".to_owned(), frame("b", Some("a"), 2, "x", Status::Done)),
    ]);
    assert_eq!(tree_depth("a", &by_id), 2);
}

#[test]
fn sub_label_reads_trace_label_only() {
    let mut tool = frame("1", None, 1, "tool:applyChangesYaml", Status::Done);
    tool.data = json!({ "trace": { "label": "yaml_apply" } });
    assert_eq!(sub_label(&tool).as_deref(), Some("yaml_apply"));

    let mut legacy = frame("2", None, 2, "ai:llm_request", Status::Done);
    legacy.data = json!({"model": "legacy-without-trace"});
    assert_eq!(sub_label(&legacy), None);
}

#[test]
fn sub_label_missing_or_unknown_payload_returns_none() {
    let llm = frame("4", None, 4, "ai:llm_request", Status::Done);
    assert_eq!(sub_label(&llm), None);

    let unknown = frame("5", None, 5, "unknown:event", Status::Done);
    assert_eq!(sub_label(&unknown), None);
}

#[test]
fn trace_session_aggregate_methods_sum_ai_done_values_only() {
    let mut done_ai = frame("1", None, 1, "ai:prompt", Status::Done);
    done_ai.data = json!({"trace": {"tokens": 10_u64, "cost_usd": 0.001_f64}});
    let mut item_ai = frame("2", None, 2, "ai:prompt", Status::Item);
    item_ai.data = json!({"trace": {"tokens": 100_u64, "cost_usd": 1.0_f64}});
    let err = frame("3", None, 3, "ai:prompt", Status::Error);

    let session = TraceSession {
        root_frame_id: "1".to_owned(),
        board_id: Some("board-1".to_owned()),
        frames: vec![done_ai, item_ai, err],
        started_at: 1,
        ended_at: Some(3),
    };

    assert_eq!(session.total_frames(), 3);
    assert_eq!(session.total_tokens(), 10);
    assert!((session.total_cost() - 0.001).abs() < f64::EPSILON);
    assert_eq!(session.error_count(), 1);
}

#[test]
fn trace_session_board_id_is_taken_from_earliest_frame() {
    let frames = vec![
        frame_with_board("r1", None, 100, "board:join", Status::Request, None),
        frame_with_board(
            "r2",
            Some("r1"),
            110,
            "board:join",
            Status::Done,
            Some("board-x"),
        ),
    ];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].board_id, None);
}
