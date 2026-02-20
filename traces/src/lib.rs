//! Observability data models and derivation helpers for CollabBoard traces.
//!
//! This crate is UI-framework agnostic so client crates can consume it directly
//! for rendering trace/session views.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use frames::{Frame, Status};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefixDisplay {
    pub letter: &'static str,
    pub label: &'static str,
    pub color: &'static str,
}

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

#[must_use]
pub fn syscall_prefix(syscall: &str) -> &str {
    syscall.split(':').next().unwrap_or_default()
}

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
    #[must_use]
    pub fn include_all() -> Self {
        let include_prefixes = ["board", "object", "ai", "tool", "chat", "cursor", "save", "other"]
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

    pub fn set_prefix_enabled(&mut self, prefix: &str, enabled: bool) {
        if enabled {
            self.include_prefixes.insert(prefix.to_owned());
        } else {
            self.include_prefixes.remove(prefix);
        }
    }

    pub fn set_status_enabled(&mut self, status: Status, enabled: bool) {
        if enabled {
            self.include_statuses.insert(StatusKey::from(status));
        } else {
            self.include_statuses.remove(&StatusKey::from(status));
        }
    }

    #[must_use]
    pub fn active_prefixes(&self) -> Vec<String> {
        self.include_prefixes.iter().cloned().collect()
    }

    #[must_use]
    pub fn active_statuses(&self) -> Vec<Status> {
        self.include_statuses
            .iter()
            .copied()
            .map(Status::from)
            .collect()
    }
}

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSession {
    pub root_frame_id: String,
    pub board_id: Option<String>,
    pub frames: Vec<Frame>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

impl TraceSession {
    #[must_use]
    pub fn total_frames(&self) -> usize {
        self.frames.len()
    }

    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.frames
            .iter()
            .filter(|f| f.syscall.starts_with("ai:") && f.status == Status::Done)
            .filter_map(|f| f.data.get("tokens").and_then(serde_json::Value::as_u64))
            .sum()
    }

    #[must_use]
    pub fn total_cost(&self) -> f64 {
        self.frames
            .iter()
            .filter(|f| f.syscall.starts_with("ai:") && f.status == Status::Done)
            .filter_map(|f| f.data.get("cost_usd").and_then(serde_json::Value::as_f64))
            .sum()
    }

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.frames
            .iter()
            .filter(|f| f.status == Status::Error)
            .count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanTiming {
    pub request_frame_id: Option<String>,
    pub terminal_frame_id: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceMetrics {
    pub total: usize,
    pub errors: usize,
    pub pending_requests: usize,
    pub by_prefix: BTreeMap<String, usize>,
}

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

#[must_use]
pub fn sub_label(frame: &Frame) -> Option<String> {
    match frame.syscall.as_str() {
        "ai:llm_request" => frame
            .data
            .get("model")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        "ai:tool_call" => frame
            .data
            .get("tool")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        _ if frame.syscall.starts_with("tool:") => frame
            .data
            .get("tool")
            .or_else(|| frame.data.get("name"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .or_else(|| frame.syscall.split_once(':').map(|(_, op)| op.to_owned())),
        "object:create" | "object:update" | "object:delete" => frame
            .data
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        "chat:message" => frame
            .data
            .get("from")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

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

fn sort_frames(frames: &[Frame]) -> Vec<Frame> {
    let mut ordered = frames.to_vec();
    ordered.sort_by_key(|f| (f.ts, f.id.clone()));
    ordered
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
