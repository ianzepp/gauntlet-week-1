//! CollabBoard-specific tool definitions for the AI agent.
//!
//! Tool names match the G4 Week 1 spec exactly (issue #19).

use super::types::Tool;

/// Build the set of tools available to the `CollabBoard` AI agent.
///
/// Returns the 9 spec-required tools with exact names evaluators will test.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn collaboard_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "createStickyNote".into(),
            description: "Create a sticky note on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text content of the sticky note" },
                    "x": { "type": "number", "description": "X position on canvas" },
                    "y": { "type": "number", "description": "Y position on canvas" },
                    "color": { "type": "string", "description": "Background color (hex, e.g. #FFEB3B)" }
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
                    "color": { "type": "string", "description": "Fill color (hex)" }
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
            description: "Change the color of an object.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objectId": { "type": "string", "format": "uuid", "description": "ID of the object to recolor" },
                    "color": { "type": "string", "description": "New color (hex, e.g. #FF5722)" }
                },
                "required": ["objectId", "color"]
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
mod tests {
    use super::*;

    #[test]
    fn tool_count() {
        let tools = collaboard_tools();
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn tool_names_match_spec() {
        let tools = collaboard_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let expected = [
            "createStickyNote",
            "createShape",
            "createFrame",
            "createConnector",
            "moveObject",
            "resizeObject",
            "updateText",
            "changeColor",
            "getBoardState",
        ];
        for name in &expected {
            assert!(names.contains(name), "missing tool: {name}");
        }
    }

    #[test]
    fn schema_shape_is_object() {
        let tools = collaboard_tools();
        for tool in &tools {
            assert_eq!(
                tool.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "tool {} schema should be type=object",
                tool.name
            );
        }
    }

    #[test]
    fn create_sticky_note_schema() {
        let tools = collaboard_tools();
        let tool = tools.iter().find(|t| t.name == "createStickyNote").unwrap();
        let required = tool
            .input_schema
            .get("required")
            .unwrap()
            .as_array()
            .unwrap();
        let req_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(req_strs.contains(&"text"));
        assert!(req_strs.contains(&"x"));
        assert!(req_strs.contains(&"y"));
    }

    #[test]
    fn get_board_state_has_no_required_params() {
        let tools = collaboard_tools();
        let tool = tools.iter().find(|t| t.name == "getBoardState").unwrap();
        // No "required" key or empty
        let required = tool.input_schema.get("required");
        assert!(required.is_none() || required.unwrap().as_array().unwrap().is_empty());
    }
}
