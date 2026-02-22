use super::*;

#[test]
fn gauntlet_tools_match_board_tools() {
    let tools = gauntlet_week_1_tools();
    let board = board_tools();
    assert_eq!(tools.len(), board.len());
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"createStickyNote"));
    assert!(names.contains(&"createShape"));
    assert!(names.contains(&"createFrame"));
    assert!(names.contains(&"createConnector"));
    assert!(names.contains(&"createSvgObject"));
    assert!(names.contains(&"updateSvgContent"));
    assert!(names.contains(&"importSvg"));
    assert!(names.contains(&"exportSelectionToSvg"));
    assert!(names.contains(&"deleteObject"));
    assert!(names.contains(&"rotateObject"));
    assert!(names.contains(&"moveObject"));
    assert!(names.contains(&"resizeObject"));
    assert!(names.contains(&"updateText"));
    assert!(names.contains(&"updateTextStyle"));
    assert!(names.contains(&"changeColor"));
    assert!(names.contains(&"createMermaidDiagram"));
    assert!(names.contains(&"getBoardState"));
}

#[test]
fn schema_shape_is_object() {
    let tools = gauntlet_week_1_tools();
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
fn board_tools_returns_all_seventeen_tools() {
    let tools = board_tools();
    assert_eq!(tools.len(), 17);
}

#[test]
fn board_tools_names_are_correct() {
    let tools = board_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"createStickyNote"));
    assert!(names.contains(&"createShape"));
    assert!(names.contains(&"createFrame"));
    assert!(names.contains(&"createConnector"));
    assert!(names.contains(&"createSvgObject"));
    assert!(names.contains(&"updateSvgContent"));
    assert!(names.contains(&"importSvg"));
    assert!(names.contains(&"exportSelectionToSvg"));
    assert!(names.contains(&"deleteObject"));
    assert!(names.contains(&"rotateObject"));
    assert!(names.contains(&"moveObject"));
    assert!(names.contains(&"resizeObject"));
    assert!(names.contains(&"updateText"));
    assert!(names.contains(&"updateTextStyle"));
    assert!(names.contains(&"changeColor"));
    assert!(names.contains(&"createMermaidDiagram"));
    assert!(names.contains(&"getBoardState"));
}

#[test]
fn board_tools_all_have_object_schemas() {
    let tools = board_tools();
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
fn board_tools_required_fields_are_arrays() {
    let tools = board_tools();
    for tool in &tools {
        if let Some(required) = tool.input_schema.get("required") {
            assert!(required.is_array(), "tool {} required should be array", tool.name);
        }
    }
}

#[test]
fn create_sticky_note_requires_text_x_y() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "createStickyNote").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["text", "x", "y"]);
}

#[test]
fn create_shape_requires_type_x_y_width_height() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "createShape").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["type", "x", "y", "width", "height"]);
}

#[test]
fn move_object_requires_object_id_x_y() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "moveObject").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["objectId", "x", "y"]);
}

#[test]
fn get_board_state_requires_nothing() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "getBoardState").unwrap();
    assert!(tool.input_schema.get("required").is_none());
}

#[test]
fn create_svg_object_requires_svg_x_y_width_height() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "createSvgObject").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["svg", "x", "y", "width", "height"]);
}

#[test]
fn update_svg_content_requires_object_id_and_svg() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "updateSvgContent").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["objectId", "svg"]);
}

#[test]
fn import_svg_requires_only_svg() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "importSvg").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["svg"]);
}

#[test]
fn export_selection_to_svg_requires_object_ids() {
    let tools = board_tools();
    let tool = tools
        .iter()
        .find(|t| t.name == "exportSelectionToSvg")
        .unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["objectIds"]);
}

#[test]
fn delete_object_requires_object_id() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "deleteObject").unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["objectId"]);
}

#[test]
fn create_connector_style_enum_matches_supported_values() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "createConnector").unwrap();
    let values: Vec<&str> = tool
        .input_schema
        .get("properties")
        .and_then(|v| v.get("style"))
        .and_then(|v| v.get("enum"))
        .and_then(serde_json::Value::as_array)
        .unwrap()
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(values, vec!["line", "arrow"]);
}

#[test]
fn update_text_field_enum_matches_ui_fields() {
    let tools = board_tools();
    let tool = tools.iter().find(|t| t.name == "updateText").unwrap();
    let values: Vec<&str> = tool
        .input_schema
        .get("properties")
        .and_then(|v| v.get("field"))
        .and_then(|v| v.get("enum"))
        .and_then(serde_json::Value::as_array)
        .unwrap()
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(values, vec!["text", "title"]);
}

#[test]
fn create_mermaid_diagram_requires_mermaid() {
    let tools = board_tools();
    let tool = tools
        .iter()
        .find(|t| t.name == "createMermaidDiagram")
        .unwrap();
    let required: Vec<&str> = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(required, vec!["mermaid"]);
    // Verify optional properties exist.
    let props = tool.input_schema.get("properties").unwrap();
    assert!(props.get("mermaid").is_some());
    assert!(props.get("x").is_some());
    assert!(props.get("y").is_some());
    assert!(props.get("scale").is_some());
}
