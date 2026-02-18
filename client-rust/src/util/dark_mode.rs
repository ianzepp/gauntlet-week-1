//! Dark mode initialization and toggle.
//!
//! Reads the user's preference from `localStorage` and applies the
//! `.dark-mode` class to the `<html>` element. Toggle writes back to
//! `localStorage` and updates the class. Requires a browser environment.

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

/// Apply or remove the `.dark-mode` class on the `<html>` element.
pub fn apply(enabled: bool) {
    #[cfg(feature = "hydrate")]
    {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.document_element() {
                let class_list = el.class_list();
                if enabled {
                    let _ = class_list.add_1("dark-mode");
                } else {
                    let _ = class_list.remove_1("dark-mode");
                }
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
