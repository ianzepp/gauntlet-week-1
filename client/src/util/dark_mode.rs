//! Dark mode initialization and toggle.
//!
//! Reads the user's preference from `localStorage` and applies a
//! `data-theme` attribute to the `<html>` element. Toggle writes back to
//! `localStorage` and updates that attribute. Requires a browser environment.
//!
//! TRADE-OFFS
//! ==========
//! Preference persistence is best-effort browser-only behavior; SSR paths
//! safely no-op to keep server rendering deterministic.

#[cfg(feature = "hydrate")]
const STORAGE_KEY: &str = "gauntlet_week_1_dark";

/// Read the dark mode preference from localStorage.
///
/// Returns `true` if the user previously enabled dark mode, or if the system
/// prefers dark mode and no preference is stored.
pub fn read_preference() -> bool {
    #[cfg(feature = "hydrate")]
    {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return false,
        };

        // Check localStorage first.
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(val)) = storage.get_item(STORAGE_KEY) {
                return val == "true";
            }
        }

        // Fall back to system preference.
        window
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
            .map_or(false, |mq| mq.matches())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        false
    }
}

/// Apply the `data-theme` attribute on the `<html>` element.
pub fn apply(enabled: bool) {
    #[cfg(feature = "hydrate")]
    {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.document_element() {
                let _ = el.set_attribute("data-theme", if enabled { "dark" } else { "light" });
            }
        }
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = enabled;
    }
}

/// Toggle dark mode and persist the new preference to localStorage.
pub fn toggle(current: bool) -> bool {
    let next = !current;
    apply(next);
    #[cfg(feature = "hydrate")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(STORAGE_KEY, if next { "true" } else { "false" });
            }
        }
    }
    next
}
