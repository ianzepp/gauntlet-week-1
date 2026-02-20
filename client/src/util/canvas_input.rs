//! Canvas input mapping and pointer helper utilities.

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

#[cfg(feature = "hydrate")]
pub fn map_tool(tool: ToolType) -> CanvasTool {
    match tool {
        ToolType::Select => CanvasTool::Select,
        ToolType::Sticky | ToolType::Rectangle | ToolType::Frame | ToolType::Youtube => CanvasTool::Select,
        ToolType::Ellipse => CanvasTool::Ellipse,
        ToolType::Line | ToolType::Connector => CanvasTool::Line,
        ToolType::Text => CanvasTool::Text,
        ToolType::Draw | ToolType::Eraser => CanvasTool::Select,
    }
}

#[cfg(feature = "hydrate")]
pub fn map_button(button: i16) -> CanvasButton {
    match button {
        1 => CanvasButton::Middle,
        2 => CanvasButton::Secondary,
        _ => CanvasButton::Primary,
    }
}

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

#[cfg(feature = "hydrate")]
pub fn pointer_event_hits_control(ev: &leptos::ev::PointerEvent, selector: &str) -> bool {
    use wasm_bindgen::JsCast;

    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(selector).ok().flatten())
        .is_some()
}

#[cfg(feature = "hydrate")]
pub fn map_modifiers(shift: bool, ctrl: bool, alt: bool, meta: bool) -> CanvasModifiers {
    CanvasModifiers { shift, ctrl, alt, meta }
}

#[cfg(feature = "hydrate")]
pub fn should_prevent_default_key(key: &str) -> bool {
    matches!(key, "Delete" | "Backspace" | "Escape" | "Enter")
}

#[cfg(feature = "hydrate")]
pub fn pointer_point(ev: &leptos::ev::PointerEvent) -> CanvasPoint {
    CanvasPoint::new(f64::from(ev.offset_x()), f64::from(ev.offset_y()))
}

#[cfg(feature = "hydrate")]
pub fn wheel_point(ev: &leptos::ev::WheelEvent) -> CanvasPoint {
    CanvasPoint::new(f64::from(ev.offset_x()), f64::from(ev.offset_y()))
}
