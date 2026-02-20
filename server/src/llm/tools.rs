//! CollabBoard-specific tool definitions for the AI agent.
//!
//! Tool names match the G4 Week 1 spec exactly (issue #19).
//!
//! DESIGN
//! ======
//! Definitions are provider-agnostic and converted by adapters, keeping the
//! command surface stable even when LLM backend implementations change.

use super::types::Tool;

/// Temporary switch: force the model to emit YAML mutation plans only.
/// Set to `false` to restore the full legacy tool surface.
const YAML_ONLY_MODE: bool = true;

fn apply_changes_yaml_tool() -> Tool {
    Tool {
        name: "applyChangesYaml".into(),
        description: "Apply a YAML mutation plan with create/update/delete blocks in one call.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "yaml": { "type": "string", "description": "YAML document containing a top-level `changes` map" }
            },
            "required": ["yaml"]
        }),
    }
}

/// Build the set of tools available to the `CollabBoard` AI agent.
///
/// Returns the standard board tools plus convenience orchestration helpers.
#[must_use]
pub fn gauntlet_week_1_tools() -> Vec<Tool> {
    if YAML_ONLY_MODE {
        return vec![apply_changes_yaml_tool()];
    }
    legacy_tools()
}

#[must_use]
#[allow(clippy::too_many_lines)]
fn legacy_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "batch".into(),
            description:
                "Execute multiple non-batch tool calls in parallel. Each call contains a tool name and input object."
                    .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "calls": {
                        "type": "array",
                        "description": "Array of non-batch tool calls to execute in parallel",
                        "items": {
                            "type": "object",
                            "properties": {
                                "tool": { "type": "string", "description": "Tool name (must not be batch)" },
                                "input": { "type": "object", "description": "Input payload for the tool" }
                            },
                            "required": ["tool", "input"]
                        }
                    }
                },
                "required": ["calls"]
            }),
        },
        Tool {
            name: "createStickyNote".into(),
            description: "Create a sticky note on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text content of the sticky note" },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "backgroundColor": { "type": "string", "description": "Background color (hex, e.g. #FFEB3B)" },
                    "fill": { "type": "string", "description": "Canvas fill color (hex)" },
                    "borderColor": { "type": "string", "description": "Border color (hex)" },
                    "stroke": { "type": "string", "description": "Canvas stroke color (hex)" },
                    "borderWidth": { "type": "number", "description": "Border width in pixels" },
                    "stroke_width": { "type": "number", "description": "Canvas stroke width in pixels" }
                },
                "required": ["text", "x", "y"]
            }),
        },
        Tool {
            name: "createShape".into(),
            description: "Create a shape (rectangle or ellipse) on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["rectangle", "ellipse"], "description": "Shape type" },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "width": { "type": "number", "description": "Width in pixels" },
                    "height": { "type": "number", "description": "Height in pixels" },
                    "backgroundColor": { "type": "string", "description": "Background color (hex)" },
                    "fill": { "type": "string", "description": "Canvas fill color (hex)" },
                    "borderColor": { "type": "string", "description": "Border color (hex)" },
                    "stroke": { "type": "string", "description": "Canvas stroke color (hex)" },
                    "borderWidth": { "type": "number", "description": "Border width in pixels" },
                    "stroke_width": { "type": "number", "description": "Canvas stroke width in pixels" }
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
            description: "Update the text content of an object (sticky note, frame title, etc).".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to update" },
                    "newText": { "type": "string", "description": "New text content" }
                },
                "required": ["objectId", "newText"]
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
                    "backgroundColor": { "type": "string", "description": "New background color (hex)" },
                    "fill": { "type": "string", "description": "New canvas fill color (hex)" },
                    "borderColor": { "type": "string", "description": "New border color (hex)" },
                    "stroke": { "type": "string", "description": "New canvas stroke color (hex)" },
                    "borderWidth": { "type": "number", "description": "New border width in pixels" },
                    "stroke_width": { "type": "number", "description": "New canvas stroke width in pixels" }
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
        Tool {
            name: "applyChangesYaml".into(),
            description: "Apply a YAML mutation plan with create/update/delete blocks in one call.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "yaml": { "type": "string", "description": "YAML document containing a top-level `changes` map" }
                },
                "required": ["yaml"]
            }),
        },
    ]
}

#[cfg(test)]
#[path = "tools_test.rs"]
mod tests;
