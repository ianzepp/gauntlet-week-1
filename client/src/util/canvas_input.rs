//! Canvas input mapping and pointer helper utilities.
//!
//! Translates browser pointer/keyboard events and UI state into the typed inputs expected by
//! the canvas engine. Keeping the translation layer here means engine types stay free of any
//! browser or Leptos dependencies, and the mapping decisions are testable in isolation.

#[cfg(feature = "hydrate")]
use crate::state::ui::ToolType;
#[cfg(feature = "hydrate")]
use crate::util::dial_math::{
    ZOOM_DIAL_MAX_ANGLE_DEG, ZOOM_DIAL_MIN_ANGLE_DEG, apply_zoom_tick_tension, normalize_degrees_360,
};

#[cfg(feature = "hydrate")]
use canvas::camera::Point as CanvasPoint;
#[cfg(feature = "hydrate")]
use canvas::input::{Button as CanvasButton, Modifiers as CanvasModifiers, Tool as CanvasTool};

/// Map a UI `ToolType` to the canvas engine's `Tool` enum.
///
/// Several UI tools (`Sticky`, `Rectangle`, `Frame`, `Youtube`, `Draw`, `Eraser`) do not have
/// a dedicated canvas engine tool because object creation for those types is handled by the
/// server-side placement path rather than the canvas drag-to-draw gesture. They are mapped to
/// `Select` so that the canvas engine remains in select mode while the UI layer handles the
/// placement preview and click-to-place logic.
#[cfg(feature = "hydrate")]
pub fn map_tool(tool: ToolType) -> CanvasTool {
    match tool {
        ToolType::Select => CanvasTool::Select,
        ToolType::Hand => CanvasTool::Hand,
        ToolType::Sticky | ToolType::Rectangle | ToolType::Frame | ToolType::Youtube => CanvasTool::Select,
        ToolType::Ellipse => CanvasTool::Ellipse,
        ToolType::Line | ToolType::Connector => CanvasTool::Line,
        ToolType::Text => CanvasTool::Text,
        ToolType::Draw | ToolType::Eraser => CanvasTool::Select,
    }
}

/// Map a browser `PointerEvent.button` integer to the canvas engine's `Button` enum.
///
/// The browser reports `0` for primary (left), `1` for middle (wheel), `2` for secondary (right).
/// Any unknown value is treated as primary to match the most common case.
#[cfg(feature = "hydrate")]
pub fn map_button(button: i16) -> CanvasButton {
    match button {
        1 => CanvasButton::Middle,
        2 => CanvasButton::Secondary,
        _ => CanvasButton::Primary,
    }
}

/// Compute the dial angle of the pointer relative to the centre of `element`.
///
/// Uses polar coordinates: `dx`/`dy` from element centre to pointer, converted to an angle
/// via `atan2`. The angle is rotated 90° so that 0° is at the top of the dial (12 o'clock)
/// rather than the right (3 o'clock), then clamped to the dial arc
/// [`ZOOM_DIAL_MIN_ANGLE_DEG`]..=[`ZOOM_DIAL_MAX_ANGLE_DEG`] and softened by tick tension.
///
/// Returns `None` when the pointer is exactly at the element centre (degenerate case).
#[cfg(feature = "hydrate")]
pub fn zoom_angle_from_pointer(ev: &leptos::ev::PointerEvent, element: &web_sys::HtmlDivElement) -> Option<f64> {
    let rect = element.get_bounding_client_rect();
    let cx = rect.x() + (rect.width() * 0.5);
    let cy = rect.y() + (rect.height() * 0.5);
    let dx = f64::from(ev.client_x()) - cx;
    let dy = f64::from(ev.client_y()) - cy;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return None;
    }

    let raw_top_based = normalize_degrees_360(dy.atan2(dx).to_degrees() + 90.0);
    let signed = if raw_top_based > 180.0 {
        raw_top_based - 360.0
    } else {
        raw_top_based
    };
    let clamped = signed.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    Some(apply_zoom_tick_tension(clamped))
}

/// Compute the compass angle of the pointer relative to the centre of `element`.
///
/// Like [`zoom_angle_from_pointer`] but without dial-arc clamping or tick tension — the full
/// [0°, 360°) range is returned. Used for the rotation compass control where the user can spin
/// freely around the full circle. Returns `None` when the pointer is at the element centre.
#[cfg(feature = "hydrate")]
pub fn compass_angle_from_pointer(ev: &leptos::ev::PointerEvent, element: &web_sys::HtmlDivElement) -> Option<f64> {
    let rect = element.get_bounding_client_rect();
    let cx = rect.x() + (rect.width() * 0.5);
    let cy = rect.y() + (rect.height() * 0.5);
    let dx = f64::from(ev.client_x()) - cx;
    let dy = f64::from(ev.client_y()) - cy;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return None;
    }

    Some(normalize_degrees_360(dy.atan2(dx).to_degrees() + 90.0))
}

/// Return `true` if the event target is inside an element matching the CSS `selector`.
///
/// Used to determine whether a global pointer event should be captured by a specific control
/// (e.g. a dial overlay) rather than forwarded to the canvas.
#[cfg(feature = "hydrate")]
pub fn pointer_event_hits_control(ev: &leptos::ev::PointerEvent, selector: &str) -> bool {
    use wasm_bindgen::JsCast;

    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(selector).ok().flatten())
        .is_some()
}

/// Pack four modifier key booleans into the canvas engine's `Modifiers` struct.
#[cfg(feature = "hydrate")]
pub fn map_modifiers(shift: bool, ctrl: bool, alt: bool, meta: bool) -> CanvasModifiers {
    CanvasModifiers { shift, ctrl, alt, meta }
}

/// Return `true` if the canvas host should call `preventDefault` for the given key name.
///
/// Prevents browser defaults (scrolling on arrow keys, browser find on `a`, etc.) while the
/// canvas is focused.
#[cfg(feature = "hydrate")]
pub fn should_prevent_default_key(key: &str) -> bool {
    matches!(
        key,
        "Delete"
            | "Backspace"
            | "Escape"
            | "Enter"
            | "ArrowUp"
            | "ArrowDown"
            | "ArrowLeft"
            | "ArrowRight"
            | " "
            | "a"
            | "A"
            | "g"
            | "G"
    )
}

/// Extract the pointer position from a `PointerEvent` as a canvas engine `Point`.
///
/// Uses `offset_x`/`offset_y` (relative to the target element) rather than `client_x`/`client_y`
/// so that the point is already in canvas-element-local coordinates matching the engine's screen
/// space.
#[cfg(feature = "hydrate")]
pub fn pointer_point(ev: &leptos::ev::PointerEvent) -> CanvasPoint {
    CanvasPoint::new(f64::from(ev.offset_x()), f64::from(ev.offset_y()))
}

/// Extract the wheel event position as a canvas engine `Point`.
///
/// Uses `offset_x`/`offset_y` for the same reason as [`pointer_point`].
#[cfg(feature = "hydrate")]
pub fn wheel_point(ev: &leptos::ev::WheelEvent) -> CanvasPoint {
    CanvasPoint::new(f64::from(ev.offset_x()), f64::from(ev.offset_y()))
}
