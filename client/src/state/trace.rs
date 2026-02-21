//! Observability trace state — live frame buffer and selection for the trace view.
//!
//! DESIGN
//! ======
//! All received frames are buffered here so the observability view can render
//! them without re-querying the server. The buffer is bounded to prevent
//! unbounded memory growth.

#[cfg(test)]
#[path = "trace_test.rs"]
mod trace_test;

use frames::Frame;
use traces::TraceFilter;

/// Maximum number of frames retained in the live buffer.
pub const TRACE_BUFFER_CAP: usize = 2000;

/// Live trace buffer and selection state for the observability view.
#[derive(Clone, Debug, Default)]
pub struct TraceState {
    /// All frames received this session, ordered by arrival, bounded by
    /// [`TRACE_BUFFER_CAP`].
    pub frames: Vec<Frame>,
    /// The `root_frame_id` of the currently selected trace session.
    pub selected_session_id: Option<String>,
    /// The `frame.id` of the event selected in the event log (drives Col 3).
    pub selected_frame_id: Option<String>,
    /// Active display filter (prefixes + statuses).
    pub filter: TraceFilter,
    /// When `true`, new frames are no longer appended (freeze the view).
    pub paused: bool,
}

impl TraceState {
    /// Append a frame to the buffer, evicting the oldest frame when the cap is
    /// reached. Silently no-ops when paused.
    pub fn push_frame(&mut self, frame: Frame) {
        if self.paused {
            return;
        }
        if self.frames.len() >= TRACE_BUFFER_CAP {
            self.frames.remove(0);
        }
        self.frames.push(frame);
    }

    /// Total frames currently buffered.
    #[must_use]
    pub fn total_frames(&self) -> usize {
        self.frames.len()
    }

    /// Error frame count in the current buffer.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.frames
            .iter()
            .filter(|f| f.status == frames::Status::Error)
            .count()
    }

    /// Frames that pass the current filter.
    #[must_use]
    pub fn visible_frames(&self) -> Vec<&Frame> {
        self.frames
            .iter()
            .filter(|f| self.filter.allows(f))
            .collect()
    }

    /// Frames belonging to the currently selected session, in arrival order.
    ///
    /// Returns all buffered frames when no session is selected (live mode).
    #[must_use]
    pub fn session_frames(&self) -> Vec<&Frame> {
        match &self.selected_session_id {
            None => self.frames.iter().collect(),
            Some(session_id) => {
                // Build a session by walking parent chains to find which
                // buffered frames belong to this root.
                let id_to_parent: std::collections::HashMap<&str, Option<&str>> = self
                    .frames
                    .iter()
                    .map(|f| (f.id.as_str(), f.parent_id.as_deref()))
                    .collect();

                self.frames
                    .iter()
                    .filter(|f| frame_root_id(f.id.as_str(), &id_to_parent).as_deref() == Some(session_id.as_str()))
                    .collect()
            }
        }
    }
}

/// Walk the parent chain to find the root frame ID.
fn frame_root_id(id: &str, id_to_parent: &std::collections::HashMap<&str, Option<&str>>) -> Option<String> {
    let mut current = id;
    let mut visited = std::collections::HashSet::new();
    loop {
        if !visited.insert(current) {
            return Some(current.to_owned());
        }
        match id_to_parent.get(current) {
            Some(Some(parent)) if id_to_parent.contains_key(*parent) => {
                current = parent;
            }
            _ => return Some(current.to_owned()),
        }
    }
}
