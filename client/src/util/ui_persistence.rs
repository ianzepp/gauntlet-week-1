//! Browser localStorage helpers for transient UI draft persistence.
//!
//! SYSTEM CONTEXT
//! ==============
//! These helpers centralize hydrate-only read/write behavior so pages and
//! components can persist modal/input drafts without repeating web-sys glue.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Load a JSON value from `localStorage` for `key`.
pub fn load_json<T: DeserializeOwned>(key: &str) -> Option<T> {
    #[cfg(feature = "hydrate")]
    {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten())?;
        let raw = storage.get_item(key).ok().flatten()?;
        serde_json::from_str(&raw).ok()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = key;
        None
    }
}

/// Save a JSON value to `localStorage` for `key`.
pub fn save_json<T: Serialize>(key: &str, value: &T) {
    #[cfg(feature = "hydrate")]
    {
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        let Ok(raw) = serde_json::to_string(value) else {
            return;
        };
        let _ = storage.set_item(key, &raw);
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (key, value);
    }
}

