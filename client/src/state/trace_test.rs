use super::*;
use frames::{Frame, Status};
use serde_json::json;

fn make_frame(id: &str, syscall: &str, status: Status) -> Frame {
    Frame {
        id: id.to_owned(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: syscall.to_owned(),
        status,
        data: json!({}),
    }
}

// =============================================================
// Default state
// =============================================================

#[test]
fn trace_state_default_empty() {
    let state = TraceState::default();
    assert_eq!(state.total_frames(), 0);
    assert_eq!(state.error_count(), 0);
    assert!(!state.paused);
    assert!(state.selected_session_id.is_none());
    assert!(state.selected_frame_id.is_none());
}

// =============================================================
// push_frame
// =============================================================

#[test]
fn push_frame_appends() {
    let mut state = TraceState::default();
    state.push_frame(make_frame("a", "board:join", Status::Done));
    assert_eq!(state.total_frames(), 1);
}

#[test]
fn push_frame_when_paused_is_noop() {
    let mut state = TraceState::default();
    state.paused = true;
    state.push_frame(make_frame("a", "board:join", Status::Done));
    assert_eq!(state.total_frames(), 0);
}

#[test]
fn push_frame_evicts_oldest_at_cap() {
    let mut state = TraceState::default();
    for i in 0..TRACE_BUFFER_CAP {
        state.push_frame(make_frame(&i.to_string(), "board:join", Status::Done));
    }
    assert_eq!(state.total_frames(), TRACE_BUFFER_CAP);

    // Push one more — should evict index 0 ("0") and add new frame.
    state.push_frame(make_frame("overflow", "board:join", Status::Done));
    assert_eq!(state.total_frames(), TRACE_BUFFER_CAP);
    assert_eq!(state.frames[0].id, "1");
    assert_eq!(state.frames[TRACE_BUFFER_CAP - 1].id, "overflow");
}

// =============================================================
// error_count
// =============================================================

#[test]
fn error_count_counts_only_errors() {
    let mut state = TraceState::default();
    state.push_frame(make_frame("a", "ai:prompt", Status::Done));
    state.push_frame(make_frame("b", "ai:prompt", Status::Error));
    state.push_frame(make_frame("c", "ai:prompt", Status::Error));
    assert_eq!(state.error_count(), 2);
}

#[test]
fn error_count_zero_when_no_errors() {
    let mut state = TraceState::default();
    state.push_frame(make_frame("a", "board:join", Status::Request));
    state.push_frame(make_frame("b", "board:join", Status::Done));
    assert_eq!(state.error_count(), 0);
}

// =============================================================
// visible_frames
// =============================================================

#[test]
fn visible_frames_excludes_filtered_prefix() {
    let mut state = TraceState::default();
    // Default filter excludes "cursor" prefix.
    state.push_frame(make_frame("a", "cursor:move", Status::Done));
    state.push_frame(make_frame("b", "board:join", Status::Done));
    let visible = state.visible_frames();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, "b");
}

#[test]
fn visible_frames_excludes_filtered_status() {
    let mut state = TraceState::default();
    // Default filter excludes Status::Item.
    state.push_frame(make_frame("a", "ai:prompt", Status::Item));
    state.push_frame(make_frame("b", "ai:prompt", Status::Done));
    let visible = state.visible_frames();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, "b");
}

#[test]
fn visible_frames_all_included_with_include_all_filter() {
    let mut state = TraceState::default();
    state.filter = traces::TraceFilter::include_all();
    state.push_frame(make_frame("a", "cursor:move", Status::Done));
    state.push_frame(make_frame("b", "ai:prompt", Status::Item));
    assert_eq!(state.visible_frames().len(), 2);
}
