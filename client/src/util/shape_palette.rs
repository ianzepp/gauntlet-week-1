//! Tool placement defaults and preview helpers.

#[cfg(test)]
#[path = "shape_palette_test.rs"]
mod shape_palette_test;

use crate::state::ui::ToolType;

pub fn placement_shape(tool: ToolType) -> Option<(&'static str, f64, f64, serde_json::Value)> {
    match tool {
        ToolType::Sticky => Some((
            "sticky_note",
            120.0,
            120.0,
            serde_json::json!({
                "title": "New note",
                "text": "",
                "color": "#FFEB3B",
                "backgroundColor": "#FFEB3B",
                "borderColor": "#FFEB3B",
                "borderWidth": 0
            }),
        )),
        ToolType::Rectangle => Some((
            "rectangle",
            160.0,
            100.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 0
            }),
        )),
        ToolType::Frame => Some((
            "frame",
            520.0,
            320.0,
            serde_json::json!({
                "title": "Frame",
                "color": "#9AA3AD",
                "backgroundColor": "rgba(154,163,173,0.08)",
                "borderColor": "#1F1A17",
                "borderWidth": 0,
                "stroke": "#1F1A17",
                "stroke_width": 0
            }),
        )),
        ToolType::Ellipse => Some((
            "ellipse",
            120.0,
            120.0,
            serde_json::json!({
                "color": "#3B82F6",
                "backgroundColor": "#3B82F6",
                "borderColor": "#3B82F6",
                "borderWidth": 0
            }),
        )),
        ToolType::Youtube => Some((
            "youtube_embed",
            320.0,
            220.0,
            serde_json::json!({
                "video_id": "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=RDdQw4w9WgXcQ&start_radio=1",
                "title": "YouTube",
                "stroke": "#1F1A17",
                "stroke_width": 2
            }),
        )),
        ToolType::Line => Some((
            "line",
            180.0,
            0.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 0
            }),
        )),
        ToolType::Connector => Some((
            "arrow",
            180.0,
            0.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 0
            }),
        )),
        ToolType::Text => Some((
            "text",
            220.0,
            56.0,
            serde_json::json!({
                "text": "Text",
                "fontSize": 24,
                "textColor": "#1F1A17"
            }),
        )),
        _ => None,
    }
}

pub fn placement_preview(tool: ToolType) -> Option<(f64, f64, &'static str)> {
    match tool {
        ToolType::Sticky => Some((120.0, 120.0, "rgba(255, 235, 59, 0.55)")),
        ToolType::Rectangle => Some((160.0, 100.0, "rgba(217, 75, 75, 0.5)")),
        ToolType::Frame => Some((520.0, 320.0, "rgba(154, 163, 173, 0.20)")),
        ToolType::Ellipse => Some((120.0, 120.0, "rgba(59, 130, 246, 0.5)")),
        ToolType::Youtube => Some((320.0, 220.0, "rgba(217, 75, 75, 0.45)")),
        ToolType::Line => Some((180.0, 2.0, "rgba(217, 75, 75, 0.65)")),
        ToolType::Connector => Some((180.0, 2.0, "rgba(217, 75, 75, 0.65)")),
        ToolType::Text => Some((220.0, 56.0, "rgba(217, 75, 75, 0.22)")),
        _ => None,
    }
}

pub fn materialize_shape_props(
    kind: &str,
    x: f64,
    y: f64,
    width: f64,
    _height: f64,
    props: serde_json::Value,
) -> serde_json::Value {
    if kind != "line" && kind != "arrow" {
        return props;
    }
    let mut map = match props {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    map.insert("a".to_owned(), serde_json::json!({ "x": x, "y": y }));
    map.insert("b".to_owned(), serde_json::json!({ "x": x + width, "y": y }));
    serde_json::Value::Object(map)
}
