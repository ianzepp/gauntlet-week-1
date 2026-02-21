//! Observability data models and derivation helpers for CollabBoard traces.
//!
//! This crate is UI-framework agnostic so client crates can consume it directly
//! for rendering trace/session views.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use frames::{Frame, Status};
use serde::{Deserialize, Serialize};

/// Display metadata for a syscall prefix, used to colour-code frames in the trace UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefixDisplay {
    /// Single uppercase letter shown in compact/icon views.
    pub letter: &'static str,
    /// Full label shown in expanded views (e.g. `"OBJECT"`, `"AI"`).
    pub label: &'static str,
    /// CSS hex color assigned to this prefix category.
    pub color: &'static str,
}

/// Return the display metadata for the prefix of a syscall string.
///
/// Extracts the prefix via [`syscall_prefix`] and maps it to a fixed colour scheme.
/// Unknown or empty prefixes fall back to a neutral grey `"OTHER"` entry.
#[must_use]
pub fn prefix_display(syscall: &str) -> PrefixDisplay {
    match syscall_prefix(syscall) {
        "board" => PrefixDisplay {
            letter: "B",
            label: "BOARD",
            color: "#5b9bd5",
        },
        "object" => PrefixDisplay {
            letter: "O",
            label: "OBJECT",
            color: "#e6a23c",
        },
        "ai" => PrefixDisplay {
            letter: "A",
            label: "AI",
            color: "#4ad981",
        },
        "tool" => PrefixDisplay {
            letter: "T",
            label: "TOOL",
            color: "#2ec4b6",
        },
        "chat" => PrefixDisplay {
            letter: "C",
            label: "CHAT",
            color: "#888888",
        },
        "cursor" => PrefixDisplay {
            letter: "U",
            label: "CURSOR",
            color: "#b388ff",
        },
        "save" => PrefixDisplay {
            letter: "S",
            label: "SAVE",
            color: "#ff69b4",
        },
        _ => PrefixDisplay {
            letter: "-",
            label: "OTHER",
            color: "#666666",
        },
    }
}

/// Extract the namespace prefix from a `"prefix:operation"` syscall string.
///
/// Returns the part before the first colon, or an empty string when no colon is present.
#[must_use]
pub fn syscall_prefix(syscall: &str) -> &str {
    syscall.split(':').next().unwrap_or_default()
}

/// A live-configurable filter controlling which frames are visible in the trace panel.
///
/// Maintains two independent allow-sets: one for syscall prefixes and one for frame statuses.
/// A frame passes the filter only when both its prefix and status are in their respective sets.
/// The default configuration excludes `cursor` frames (high frequency / low value) and `Item`
/// streaming frames to reduce noise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceFilter {
    include_prefixes: BTreeSet<String>,
    include_statuses: BTreeSet<StatusKey>,
}

impl Default for TraceFilter {
    fn default() -> Self {
        let include_prefixes = ["board", "object", "ai", "tool", "chat", "save"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let include_statuses = [Status::Request, Status::Done, Status::Error]
            .into_iter()
            .map(StatusKey::from)
            .collect::<BTreeSet<_>>();

        Self {
            include_prefixes,
            include_statuses,
        }
    }
}

impl TraceFilter {
    /// Construct a filter that allows every known prefix and every status — useful in debug views.
    #[must_use]
    pub fn include_all() -> Self {
        let include_prefixes = [
            "board", "object", "ai", "tool", "chat", "cursor", "save", "other",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
        let include_statuses = [
            Status::Request,
            Status::Done,
            Status::Error,
            Status::Item,
            Status::Cancel,
        ]
        .into_iter()
        .map(StatusKey::from)
        .collect::<BTreeSet<_>>();

        Self {
            include_prefixes,
            include_statuses,
        }
    }

    /// Return `true` if `frame` passes both the prefix and status allow-sets.
    ///
    /// Frames with an empty prefix are treated as belonging to the `"other"` bucket for
    /// allow-set lookup purposes.
    #[must_use]
    pub fn allows(&self, frame: &Frame) -> bool {
        let prefix = syscall_prefix(&frame.syscall);
        let prefix_allowed = self.include_prefixes.contains(prefix)
            || (prefix.is_empty() && self.include_prefixes.contains("other"));
        let status_allowed = self
            .include_statuses
            .contains(&StatusKey::from(frame.status));
        prefix_allowed && status_allowed
    }

    /// Enable or disable a syscall prefix in the filter.
    pub fn set_prefix_enabled(&mut self, prefix: &str, enabled: bool) {
        if enabled {
            self.include_prefixes.insert(prefix.to_owned());
        } else {
            self.include_prefixes.remove(prefix);
        }
    }

    /// Enable or disable a frame status in the filter.
    pub fn set_status_enabled(&mut self, status: Status, enabled: bool) {
        if enabled {
            self.include_statuses.insert(StatusKey::from(status));
        } else {
            self.include_statuses.remove(&StatusKey::from(status));
        }
    }

    /// Return the currently enabled prefixes in sorted order.
    #[must_use]
    pub fn active_prefixes(&self) -> Vec<String> {
        self.include_prefixes.iter().cloned().collect()
    }

    /// Return the currently enabled statuses in sorted order.
    #[must_use]
    pub fn active_statuses(&self) -> Vec<Status> {
        self.include_statuses
            .iter()
            .copied()
            .map(Status::from)
            .collect()
    }
}

/// An `Ord`-compatible wrapper for `Status` so it can be stored in a `BTreeSet`.
///
/// `Status` itself does not implement `Ord`; this newtype assigns a stable u8 ordinal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StatusKey(u8);

impl From<Status> for StatusKey {
    fn from(value: Status) -> Self {
        match value {
            Status::Request => Self(0),
            Status::Item => Self(1),
            Status::Done => Self(2),
            Status::Error => Self(3),
            Status::Cancel => Self(4),
        }
    }
}

impl From<StatusKey> for Status {
    fn from(value: StatusKey) -> Self {
        match value.0 {
            0 => Self::Request,
            1 => Self::Item,
            2 => Self::Done,
            3 => Self::Error,
            _ => Self::Cancel,
        }
    }
}

/// A group of causally related frames that together represent a single user or AI action.
///
/// Sessions are formed by tracing `parent_id` links to their root frame and grouping all
/// descendants together. `ended_at` is `None` while the session is still in flight (its last
/// frame is a `Request` or `Item`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSession {
    /// The ID of the root (parentless) frame in this session.
    pub root_frame_id: String,
    /// Board the session belongs to, if any.
    pub board_id: Option<String>,
    /// All frames in the session, sorted by timestamp then ID.
    pub frames: Vec<Frame>,
    /// Timestamp of the first frame in milliseconds since the Unix epoch.
    pub started_at: i64,
    /// Timestamp of the last terminal frame, or `None` if still in progress.
    pub ended_at: Option<i64>,
}

impl TraceSession {
    /// Return the total number of frames in this session.
    #[must_use]
    pub fn total_frames(&self) -> usize {
        self.frames.len()
    }

    /// Return the sum of token counts from all completed AI frames in this session.
    ///
    /// Reads the `trace.tokens` field from each `ai:*` Done frame.
    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.frames
            .iter()
            .filter(|f| f.syscall.starts_with("ai:") && f.status == Status::Done)
            .filter_map(|f| trace_field(f, "tokens").and_then(serde_json::Value::as_u64))
            .sum()
    }

    /// Return the total USD cost across all completed AI frames in this session.
    ///
    /// Reads the `trace.cost_usd` field from each `ai:*` Done frame.
    #[must_use]
    pub fn total_cost(&self) -> f64 {
        self.frames
            .iter()
            .filter(|f| f.syscall.starts_with("ai:") && f.status == Status::Done)
            .filter_map(|f| trace_field(f, "cost_usd").and_then(serde_json::Value::as_f64))
            .sum()
    }

    /// Return the number of Error-status frames in this session.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.frames
            .iter()
            .filter(|f| f.status == Status::Error)
            .count()
    }
}

/// The measured latency of a single request/response pair.
///
/// Produced by [`pair_request_spans`]. `request_frame_id` is `None` when a terminal frame
/// arrived without a matching request (e.g. the request was emitted before tracing began).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanTiming {
    /// ID of the originating Request frame, or `None` if the request was not observed.
    pub request_frame_id: Option<String>,
    /// ID of the terminal (Done, Error, or Cancel) frame that closed this span.
    pub terminal_frame_id: String,
    /// Timestamp of the request, or of the terminal frame when no request was observed.
    pub started_at: i64,
    /// Timestamp of the terminal frame.
    pub ended_at: i64,
    /// `ended_at - started_at` in milliseconds.
    pub duration_ms: i64,
}

/// Aggregate counts derived from a flat list of frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceMetrics {
    /// Total frame count.
    pub total: usize,
    /// Number of frames with Error status.
    pub errors: usize,
    /// Number of Request frames that have not yet received a terminal response.
    pub pending_requests: usize,
    /// Frame count broken down by syscall prefix (e.g. `"object"`, `"ai"`).
    pub by_prefix: BTreeMap<String, usize>,
}

/// Compute summary metrics from a flat list of frames.
///
/// Counts total frames, errors, per-prefix totals, and open requests. Frames with an empty
/// prefix are counted under the `"other"` key in `by_prefix`.
#[must_use]
pub fn compute_metrics(frames: &[Frame]) -> TraceMetrics {
    let mut by_prefix = BTreeMap::<String, usize>::new();
    let mut total = 0usize;
    let mut errors = 0usize;

    for frame in frames {
        total += 1;
        if frame.status == Status::Error {
            errors += 1;
        }

        let prefix = syscall_prefix(&frame.syscall);
        let key = if prefix.is_empty() { "other" } else { prefix };
        *by_prefix.entry(key.to_owned()).or_insert(0) += 1;
    }

    let pending_requests = count_open_requests(frames);

    TraceMetrics {
        total,
        errors,
        pending_requests,
        by_prefix,
    }
}

/// Group a flat list of frames into causally related [`TraceSession`] instances.
///
/// Each frame is traced to its root (the ancestor with no parent in the given set), and all
/// frames sharing the same root are placed into one session. Sessions are sorted by their
/// first frame's timestamp. A session's `ended_at` is `None` when its last frame is still
/// `Request` or `Item`, indicating an in-progress operation.
#[must_use]
pub fn build_trace_sessions(frames: &[Frame]) -> Vec<TraceSession> {
    if frames.is_empty() {
        return Vec::new();
    }

    let ordered = sort_frames(frames);
    let ids = ordered
        .iter()
        .map(|f| f.id.clone())
        .collect::<HashSet<String>>();
    let parents = ordered
        .iter()
        .map(|f| (f.id.clone(), f.parent_id.clone()))
        .collect::<HashMap<_, _>>();

    let mut groups = BTreeMap::<String, Vec<Frame>>::new();

    for frame in ordered {
        let root = find_root_frame_id(&frame.id, &parents, &ids);
        groups.entry(root).or_default().push(frame);
    }

    let mut sessions = groups
        .into_iter()
        .filter_map(|(root_frame_id, mut grouped_frames)| {
            grouped_frames.sort_by_key(|f| (f.ts, f.id.clone()));
            let first = grouped_frames.first()?;
            let last = grouped_frames.last()?;
            let board_id = first.board_id.clone();
            let started_at = first.ts;

            let ended_at = if matches!(last.status, Status::Request | Status::Item) {
                None
            } else {
                Some(last.ts)
            };

            Some(TraceSession {
                root_frame_id,
                board_id,
                frames: grouped_frames,
                started_at,
                ended_at,
            })
        })
        .collect::<Vec<_>>();

    sessions.sort_by_key(|s| s.started_at);
    sessions
}

/// Pair Request frames with their terminal responses to produce latency spans.
///
/// Frames are matched by `(parent_id, syscall)` in FIFO order — the first pending request for
/// a given key is matched to the first terminal frame with the same key. Items (`Status::Item`)
/// are ignored because they are intermediate streaming frames, not terminals.
#[must_use]
pub fn pair_request_spans(frames: &[Frame]) -> Vec<SpanTiming> {
    let ordered = sort_frames(frames);
    let mut pending = HashMap::<(Option<String>, String), VecDeque<(String, i64)>>::new();
    let mut spans = Vec::<SpanTiming>::new();

    for frame in ordered {
        let key = (frame.parent_id.clone(), frame.syscall.clone());
        match frame.status {
            Status::Request => {
                pending
                    .entry(key)
                    .or_default()
                    .push_back((frame.id.clone(), frame.ts));
            }
            Status::Done | Status::Error | Status::Cancel => {
                let maybe_request = pending.get_mut(&key).and_then(VecDeque::pop_front);
                let (request_frame_id, started_at) = if let Some((req_id, req_ts)) = maybe_request {
                    (Some(req_id), req_ts)
                } else {
                    (None, frame.ts)
                };

                spans.push(SpanTiming {
                    request_frame_id,
                    terminal_frame_id: frame.id.clone(),
                    started_at,
                    ended_at: frame.ts,
                    duration_ms: frame.ts - started_at,
                });
            }
            Status::Item => {}
        }
    }

    spans
}

/// Count how many ancestors separate `frame_id` from the root of its causal tree.
///
/// Walks the `parent_id` chain through `by_id` until a frame without a parent is reached.
/// A cycle-detection set prevents infinite loops on malformed data. Returns 0 for root frames.
#[must_use]
pub fn tree_depth(frame_id: &str, by_id: &HashMap<String, Frame>) -> usize {
    let mut depth = 0usize;
    let mut current = frame_id;
    let mut seen = HashSet::<String>::new();

    while let Some(frame) = by_id.get(current) {
        let Some(parent) = frame.parent_id.as_deref() else {
            break;
        };

        if !seen.insert(parent.to_owned()) {
            break;
        }

        depth += 1;
        current = parent;
    }

    depth
}

/// Extract the human-readable sub-label from a frame's `trace.label` field, if present.
///
/// Sub-labels are short strings (e.g. a tool name or model identifier) attached by the server
/// for display alongside the syscall name in the trace panel.
#[must_use]
pub fn sub_label(frame: &Frame) -> Option<String> {
    trace_field(frame, "label")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// Read a value from the `frame.data.trace` object by key.
fn trace_field<'a>(frame: &'a Frame, key: &str) -> Option<&'a serde_json::Value> {
    frame
        .data
        .get("trace")
        .and_then(serde_json::Value::as_object)
        .and_then(|trace| trace.get(key))
}

/// Count how many Request frames in `frames` have not yet received a terminal response.
///
/// Uses the same `(parent_id, syscall)` keying as [`pair_request_spans`].
fn count_open_requests(frames: &[Frame]) -> usize {
    let mut pending = HashMap::<(Option<String>, String), usize>::new();

    for frame in sort_frames(frames) {
        let key = (frame.parent_id.clone(), frame.syscall.clone());
        match frame.status {
            Status::Request => {
                *pending.entry(key).or_insert(0) += 1;
            }
            Status::Done | Status::Error | Status::Cancel => {
                if let Some(count) = pending.get_mut(&key)
                    && *count > 0
                {
                    *count -= 1;
                }
            }
            Status::Item => {}
        }
    }

    pending.values().sum()
}

/// Walk the `parent_id` chain for `frame_id` to find the topmost ancestor present in `valid_ids`.
///
/// Stops and returns `current` when the parent is absent from `valid_ids` (indicating the root
/// was not observed in the current frame set) or when a cycle is detected.
fn find_root_frame_id(
    frame_id: &str,
    parents: &HashMap<String, Option<String>>,
    valid_ids: &HashSet<String>,
) -> String {
    let mut current = frame_id.to_owned();
    let mut visited = HashSet::<String>::new();

    loop {
        if !visited.insert(current.clone()) {
            return current;
        }

        let next = parents.get(&current).and_then(|parent| parent.clone());
        match next {
            Some(parent) if valid_ids.contains(&parent) => current = parent,
            _ => return current,
        }
    }
}

/// Return a copy of `frames` sorted by timestamp ascending, with frame ID as a tiebreaker.
fn sort_frames(frames: &[Frame]) -> Vec<Frame> {
    let mut ordered = frames.to_vec();
    ordered.sort_by_key(|f| (f.ts, f.id.clone()));
    ordered
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
