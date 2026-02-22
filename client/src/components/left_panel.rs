//! Collapsible left panel containing a tool rail and inspector panel.
//!
//! ARCHITECTURE
//! ============
//! Keeps creation tools and selection inspector co-located so editing controls
//! remain spatially consistent with canvas interactions.

use leptos::prelude::*;
use crate::components::tool_rail::ToolRail;

/// Left sidebar with a fixed tool rail and expandable inspector panel.
#[component]
pub fn LeftPanel() -> impl IntoView {
    view! {
        <div class="left-panel">
            <ToolRail/>
            <div id="left-dials-mount" class="left-panel__dials-mount"></div>
        </div>
    }
}
