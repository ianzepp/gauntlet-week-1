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
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move || {
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                return;
            };
            if let (Some(host), Some(mount)) = (
                document.get_element_by_id("left-dials-host"),
                document.get_element_by_id("left-dials-mount"),
            ) {
                let _ = mount.append_child(&host);
            }
        });
    }

    view! {
        <div class="left-panel">
            <ToolRail/>
            <div id="left-dials-mount" class="left-panel__dials-mount"></div>
        </div>
    }
}
