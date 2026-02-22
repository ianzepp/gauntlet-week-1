//! CollabBoard-specific tool definitions for the AI agent.
//!
//! Tool names match the G4 Week 1 spec exactly (issue #19).
//!
//! DESIGN
//! ======
//! Definitions are provider-agnostic and converted by adapters, keeping the
//! command surface stable even when LLM backend implementations change.

use super::types::Tool;

/// Build the set of tools available to the `CollabBoard` AI agent.
///
/// Returns the standard board tools.
#[must_use]
pub fn gauntlet_week_1_tools() -> Vec<Tool> {
    board_tools()
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub(crate) fn board_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "createStickyNote".into(),
            description: "Create a sticky note on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Optional sticky note title" },
                    "text": { "type": "string", "description": "Text content of the sticky note" },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "fill": { "type": "string", "description": "Fill color (hex, e.g. #FFEB3B)" },
                    "stroke": { "type": "string", "description": "Canvas stroke color (hex)" },
                    "strokeWidth": { "type": "number", "description": "Stroke width in pixels" }
                },
                "required": ["text", "x", "y"]
            }),
        },
        Tool {
            name: "createShape".into(),
            description: "Create a shape on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["rectangle", "ellipse", "text", "line", "arrow", "youtube_embed"],
                        "description": "Shape type"
                    },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "width": { "type": "number", "description": "Width in pixels" },
                    "height": { "type": "number", "description": "Height in pixels" },
                    "text": { "type": "string", "description": "Text content (used when type is text)" },
                    "fontSize": { "type": "number", "description": "Text font size in pixels (type=text)" },
                    "textColor": { "type": "string", "description": "Text color hex (type=text)" },
                    "video_id": { "type": "string", "description": "YouTube URL or video ID (type=youtube_embed)" },
                    "title": { "type": "string", "description": "Title label (type=youtube_embed)" },
                    "fill": { "type": "string", "description": "Fill color (hex)" },
                    "stroke": { "type": "string", "description": "Canvas stroke color (hex)" },
                    "strokeWidth": { "type": "number", "description": "Stroke width in pixels" }
                },
                "required": ["type", "x", "y", "width", "height"]
            }),
        },
        Tool {
            name: "createFrame".into(),
            description: "Create a frame — a titled rectangular region that groups content on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Frame title displayed at the top" },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "width": { "type": "number", "description": "Width in pixels" },
                    "height": { "type": "number", "description": "Height in pixels" }
                },
                "required": ["title", "x", "y", "width", "height"]
            }),
        },
        Tool {
            name: "createConnector".into(),
            description: "Create a connector line/arrow between two objects.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "fromId": { "type": "string", "format": "uuid", "description": "Source object ID" },
                    "toId": { "type": "string", "format": "uuid", "description": "Target object ID" },
                    "style": { "type": "string", "enum": ["line", "arrow", "dashed"], "description": "Connector visual style" }
                },
                "required": ["fromId", "toId"]
            }),
        },
        Tool {
            name: "rotateObject".into(),
            description: "Rotate an object to an absolute angle in degrees.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to rotate" },
                    "rotation": { "type": "number", "description": "Clockwise rotation in degrees" }
                },
                "required": ["objectId", "rotation"]
            }),
        },
        Tool {
            name: "moveObject".into(),
            description: "Move an object to a new position.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to move" },
                    "x": { "type": "number", "description": "New X position" },
                    "y": { "type": "number", "description": "New Y position" }
                },
                "required": ["objectId", "x", "y"]
            }),
        },
        Tool {
            name: "resizeObject".into(),
            description: "Resize an object to new dimensions.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to resize" },
                    "width": { "type": "number", "description": "New width in pixels" },
                    "height": { "type": "number", "description": "New height in pixels" }
                },
                "required": ["objectId", "width", "height"]
            }),
        },
        Tool {
            name: "updateText".into(),
            description: "Update a text field on an object (text/title/head/foot).".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to update" },
                    "newText": { "type": "string", "description": "New text content" },
                    "field": {
                        "type": "string",
                        "enum": ["text", "title", "head", "foot"],
                        "description": "Which text field to update (default: text)"
                    }
                },
                "required": ["objectId", "newText"]
            }),
        },
        Tool {
            name: "updateTextStyle".into(),
            description: "Update text style properties (textColor and/or fontSize) on an object.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to update" },
                    "textColor": { "type": "string", "description": "Text color hex" },
                    "fontSize": { "type": "number", "description": "Font size in pixels" }
                },
                "required": ["objectId"]
            }),
        },
        Tool {
            name: "changeColor".into(),
            description: "Change the appearance of an object (fill/background, border/stroke, and border width)."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to recolor" },
                    "fill": { "type": "string", "description": "New canvas fill color (hex)" },
                    "stroke": { "type": "string", "description": "New canvas stroke color (hex)" },
                    "strokeWidth": { "type": "number", "description": "New stroke width in pixels" },
                    "textColor": { "type": "string", "description": "New text color hex" }
                },
                "required": ["objectId"]
            }),
        },
        Tool {
            name: "getBoardState".into(),
            description: "Retrieve the current state of all objects on the board. Use this to understand \
                          what's on the board before making changes."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

#[cfg(test)]
#[path = "tools_test.rs"]
mod tests;
