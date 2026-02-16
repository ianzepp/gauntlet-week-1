//! CollabBoard-specific tool definitions for the AI agent.

use super::types::Tool;

/// Build the set of tools available to the `CollabBoard` AI agent.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn collaboard_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "create_objects".into(),
            description: "Create one or more objects (sticky notes, shapes, text) on the board.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objects": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": { "type": "string", "enum": ["sticky_note", "rectangle", "ellipse", "text"] },
                                "x": { "type": "number" },
                                "y": { "type": "number" },
                                "props": {
                                    "type": "object",
                                    "properties": {
                                        "text": { "type": "string" },
                                        "color": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["kind", "x", "y"]
                        }
                    }
                },
                "required": ["objects"]
            }),
        },
        Tool {
            name: "move_objects".into(),
            description: "Reposition objects by their IDs to new x,y coordinates.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "moves": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string", "format": "uuid" },
                                "x": { "type": "number" },
                                "y": { "type": "number" }
                            },
                            "required": ["id", "x", "y"]
                        }
                    }
                },
                "required": ["moves"]
            }),
        },
        Tool {
            name: "update_objects".into(),
            description: "Change properties (color, text, size) of objects by their IDs.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "updates": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string", "format": "uuid" },
                                "props": { "type": "object" },
                                "width": { "type": "number" },
                                "height": { "type": "number" }
                            },
                            "required": ["id"]
                        }
                    }
                },
                "required": ["updates"]
            }),
        },
        Tool {
            name: "delete_objects".into(),
            description: "Remove objects from the board by their IDs.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": { "type": "string", "format": "uuid" }
                    }
                },
                "required": ["ids"]
            }),
        },
        Tool {
            name: "organize_layout".into(),
            description: "Arrange objects in a grid, cluster, or tree layout.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "layout": { "type": "string", "enum": ["grid", "cluster", "tree", "circle"] },
                    "ids": {
                        "type": "array",
                        "items": { "type": "string", "format": "uuid" },
                        "description": "Object IDs to arrange. If empty, arranges all objects."
                    },
                    "spacing": { "type": "number", "description": "Pixels between objects" }
                },
                "required": ["layout"]
            }),
        },
        Tool {
            name: "summarize_board".into(),
            description: "Read all text content on the board and produce a summary as a new sticky note.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "position": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "number" },
                            "y": { "type": "number" }
                        }
                    }
                }
            }),
        },
        Tool {
            name: "group_by_theme".into(),
            description: "Cluster objects by semantic similarity and color-code them by group.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "num_groups": { "type": "integer", "minimum": 2, "maximum": 10, "description": "Number of groups to create" }
                }
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
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn tool_names() {
        let tools = collaboard_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"create_objects"));
        assert!(names.contains(&"move_objects"));
        assert!(names.contains(&"update_objects"));
        assert!(names.contains(&"delete_objects"));
        assert!(names.contains(&"organize_layout"));
        assert!(names.contains(&"summarize_board"));
        assert!(names.contains(&"group_by_theme"));
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
    fn create_objects_schema_requires_objects_array() {
        let tools = collaboard_tools();
        let create = tools.iter().find(|t| t.name == "create_objects").unwrap();
        let required = create.input_schema.get("required").unwrap();
        assert!(required.as_array().unwrap().iter().any(|v| v == "objects"));
        let items = create
            .input_schema
            .get("properties")
            .unwrap()
            .get("objects")
            .unwrap()
            .get("items")
            .unwrap();
        assert_eq!(items.get("type").and_then(|v| v.as_str()), Some("object"));
    }
}
