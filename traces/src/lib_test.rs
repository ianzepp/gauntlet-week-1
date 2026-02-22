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
        trace: None,
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
        trace: None,
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
fn sub_label_prefers_top_level_trace_over_legacy_data_trace() {
    let mut frame = frame("3", None, 3, "tool:createStickyNote", Status::Done);
    frame.trace = Some(json!({ "label": "top_level" }));
    frame.data = json!({ "trace": { "label": "legacy" } });
    assert_eq!(sub_label(&frame).as_deref(), Some("top_level"));
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

// =============================================================
// TraceSession: aggregate methods — zero and boundary cases
// =============================================================

#[test]
fn trace_session_total_frames_empty() {
    let session = TraceSession {
        root_frame_id: "r".to_owned(),
        board_id: None,
        frames: vec![],
        started_at: 0,
        ended_at: None,
    };
    assert_eq!(session.total_frames(), 0);
}

#[test]
fn trace_session_total_tokens_no_ai_frames() {
    let session = TraceSession {
        root_frame_id: "r".to_owned(),
        board_id: None,
        frames: vec![frame("1", None, 1, "board:join", Status::Done)],
        started_at: 1,
        ended_at: Some(1),
    };
    assert_eq!(session.total_tokens(), 0);
}

#[test]
fn trace_session_total_cost_no_ai_frames() {
    let session = TraceSession {
        root_frame_id: "r".to_owned(),
        board_id: None,
        frames: vec![frame("1", None, 1, "board:join", Status::Done)],
        started_at: 1,
        ended_at: Some(1),
    };
    assert!((session.total_cost() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn trace_session_error_count_zero_errors() {
    let session = TraceSession {
        root_frame_id: "r".to_owned(),
        board_id: None,
        frames: vec![
            frame("1", None, 1, "ai:prompt", Status::Request),
            frame("2", None, 2, "ai:prompt", Status::Done),
        ],
        started_at: 1,
        ended_at: Some(2),
    };
    assert_eq!(session.error_count(), 0);
}

#[test]
fn trace_session_error_count_multiple_errors() {
    let session = TraceSession {
        root_frame_id: "r".to_owned(),
        board_id: None,
        frames: vec![
            frame("1", None, 1, "ai:prompt", Status::Error),
            frame("2", None, 2, "board:join", Status::Error),
            frame("3", None, 3, "object:update", Status::Done),
        ],
        started_at: 1,
        ended_at: Some(3),
    };
    assert_eq!(session.error_count(), 2);
}

// =============================================================
// prefix_display: all known prefixes
// =============================================================

#[test]
fn prefix_display_board() {
    let d = prefix_display("board:join");
    assert_eq!(d.letter, "B");
    assert_eq!(d.label, "BOARD");
}

#[test]
fn prefix_display_object() {
    let d = prefix_display("object:update");
    assert_eq!(d.letter, "O");
    assert_eq!(d.label, "OBJECT");
}

#[test]
fn prefix_display_tool() {
    let d = prefix_display("tool:applyChanges");
    assert_eq!(d.letter, "T");
    assert_eq!(d.label, "TOOL");
}

#[test]
fn prefix_display_chat() {
    let d = prefix_display("chat:send");
    assert_eq!(d.letter, "C");
    assert_eq!(d.label, "CHAT");
}

#[test]
fn prefix_display_cursor() {
    let d = prefix_display("cursor:move");
    assert_eq!(d.letter, "U");
    assert_eq!(d.label, "CURSOR");
}

#[test]
fn prefix_display_save() {
    let d = prefix_display("save:snapshot");
    assert_eq!(d.letter, "S");
    assert_eq!(d.label, "SAVE");
}

#[test]
fn prefix_display_unknown_prefix() {
    let d = prefix_display("foo:bar");
    assert_eq!(d.letter, "-");
    assert_eq!(d.label, "OTHER");
}

#[test]
fn prefix_display_empty_syscall() {
    let d = prefix_display("");
    assert_eq!(d.letter, "-");
    assert_eq!(d.label, "OTHER");
}

// =============================================================
// syscall_prefix: edge cases
// =============================================================

#[test]
fn syscall_prefix_multiple_colons_returns_first_segment() {
    assert_eq!(syscall_prefix("ai:llm:request"), "ai");
}

#[test]
fn syscall_prefix_only_colon() {
    assert_eq!(syscall_prefix(":"), "");
}

// =============================================================
// compute_metrics: additional cases
// =============================================================

#[test]
fn metrics_all_done_no_pending() {
    let frames = vec![
        frame("1", None, 1, "board:join", Status::Done),
        frame("2", None, 2, "board:leave", Status::Done),
    ];
    let metrics = compute_metrics(&frames);
    assert_eq!(metrics.total, 2);
    assert_eq!(metrics.errors, 0);
    assert_eq!(metrics.pending_requests, 0);
}

#[test]
fn metrics_single_error_frame() {
    let frames = vec![frame("1", None, 1, "ai:prompt", Status::Error)];
    let metrics = compute_metrics(&frames);
    assert_eq!(metrics.total, 1);
    assert_eq!(metrics.errors, 1);
    assert_eq!(metrics.pending_requests, 0);
}

#[test]
fn metrics_item_frames_not_pending() {
    // Item frames are streaming — they should not count as pending requests.
    let frames = vec![
        frame("1", None, 1, "ai:stream", Status::Request),
        frame("2", None, 2, "ai:stream", Status::Item),
        frame("3", None, 3, "ai:stream", Status::Item),
    ];
    let metrics = compute_metrics(&frames);
    assert_eq!(metrics.pending_requests, 1);
}

// =============================================================
// pair_request_spans: edge cases
// =============================================================

#[test]
fn span_pairing_empty_input() {
    let spans = pair_request_spans(&[]);
    assert!(spans.is_empty());
}

#[test]
fn span_pairing_only_items_produces_no_spans() {
    let frames = vec![
        frame("i1", None, 1, "ai:stream", Status::Item),
        frame("i2", None, 2, "ai:stream", Status::Item),
    ];
    let spans = pair_request_spans(&frames);
    assert!(spans.is_empty());
}

#[test]
fn span_pairing_error_status_is_terminal() {
    let frames = vec![
        frame("req", None, 100, "ai:prompt", Status::Request),
        frame("err", None, 150, "ai:prompt", Status::Error),
    ];
    let spans = pair_request_spans(&frames);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].request_frame_id.as_deref(), Some("req"));
    assert_eq!(spans[0].duration_ms, 50);
}

#[test]
fn span_duration_ms_is_zero_when_done_without_request() {
    // A Done frame with no matching Request has started_at == ended_at.
    let frames = vec![frame("done", None, 999, "chat:message", Status::Done)];
    let spans = pair_request_spans(&frames);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].duration_ms, 0);
    assert_eq!(spans[0].started_at, 999);
    assert_eq!(spans[0].ended_at, 999);
}

// =============================================================
// TraceFilter: additional toggle and active_* edge cases
// =============================================================

#[test]
fn filter_active_prefixes_sorted() {
    let filter = TraceFilter::default();
    let prefixes = filter.active_prefixes();
    // Should be sorted alphabetically (BTreeSet ordering).
    let mut sorted = prefixes.clone();
    sorted.sort();
    assert_eq!(prefixes, sorted);
}

#[test]
fn filter_active_statuses_stable() {
    let filter = TraceFilter::default();
    let statuses = filter.active_statuses();
    // Default includes Request, Done, Error.
    assert!(statuses.contains(&Status::Request));
    assert!(statuses.contains(&Status::Done));
    assert!(statuses.contains(&Status::Error));
    assert!(!statuses.contains(&Status::Item));
    assert!(!statuses.contains(&Status::Cancel));
}

#[test]
fn filter_set_same_prefix_twice_is_idempotent() {
    let mut filter = TraceFilter::default();
    filter.set_prefix_enabled("ai", true);
    filter.set_prefix_enabled("ai", true);
    let prefixes = filter.active_prefixes();
    let count = prefixes.iter().filter(|p| p.as_str() == "ai").count();
    assert_eq!(count, 1);
}

#[test]
fn filter_allows_empty_prefix_as_other() {
    let mut filter = TraceFilter::default();
    // "other" is not in the default set; add it.
    filter.set_prefix_enabled("other", true);
    assert!(filter.allows(&frame("1", None, 1, "", Status::Done)));
}

// =============================================================
// build_trace_sessions: ordering by start timestamp
// =============================================================

#[test]
fn trace_sessions_sorted_by_start_timestamp() {
    let frames = vec![
        frame("late", None, 1000, "board:join", Status::Done),
        frame("early", None, 100, "board:join", Status::Done),
    ];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].started_at < sessions[1].started_at);
}

#[test]
fn trace_sessions_single_frame_is_its_own_session() {
    let frames = vec![frame("solo", None, 5, "object:update", Status::Done)];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].root_frame_id, "solo");
    assert_eq!(sessions[0].frames.len(), 1);
    assert_eq!(sessions[0].ended_at, Some(5));
}

#[test]
fn trace_session_ended_at_none_when_last_status_is_request() {
    let frames = vec![
        frame("root", None, 10, "ai:prompt", Status::Request),
        frame("child", Some("root"), 20, "ai:llm_request", Status::Done),
        frame("root2", Some("root"), 30, "ai:result", Status::Request),
    ];
    let sessions = build_trace_sessions(&frames);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].ended_at, None);
}

// =============================================================
// tree_depth: additional cases
// =============================================================

#[test]
fn tree_depth_parent_not_in_map_stops_walk() {
    let by_id = HashMap::from([
        ("b".to_owned(), frame("b", Some("a"), 2, "x", Status::Done)),
        ("c".to_owned(), frame("c", Some("b"), 3, "x", Status::Done)),
    ]);
    // "c" -> "b" -> "a" (not in map) => depth 2 (two parent hops)
    assert_eq!(tree_depth("c", &by_id), 2);
}

#[test]
fn tree_depth_single_hop() {
    let by_id = HashMap::from([
        ("root".to_owned(), frame("root", None, 1, "x", Status::Done)),
        (
            "child".to_owned(),
            frame("child", Some("root"), 2, "x", Status::Done),
        ),
    ]);
    assert_eq!(tree_depth("root", &by_id), 0);
    assert_eq!(tree_depth("child", &by_id), 1);
}
