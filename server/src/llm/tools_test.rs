use super::*;

#[test]
fn gauntlet_tools_match_legacy_tools() {
    let tools = gauntlet_week_1_tools();
    let legacy = legacy_tools();
    assert_eq!(tools.len(), legacy.len());
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"createStickyNote"));
    assert!(names.contains(&"createShape"));
    assert!(names.contains(&"createFrame"));
    assert!(names.contains(&"createConnector"));
    assert!(names.contains(&"moveObject"));
    assert!(names.contains(&"resizeObject"));
    assert!(names.contains(&"updateText"));
    assert!(names.contains(&"changeColor"));
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
fn legacy_tools_returns_all_ten_tools() {
    let tools = legacy_tools();
    assert_eq!(tools.len(), 10);
}

#[test]
fn legacy_tools_names_are_correct() {
    let tools = legacy_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"batch"));
    assert!(names.contains(&"createStickyNote"));
    assert!(names.contains(&"createShape"));
    assert!(names.contains(&"createFrame"));
    assert!(names.contains(&"createConnector"));
    assert!(names.contains(&"moveObject"));
    assert!(names.contains(&"resizeObject"));
    assert!(names.contains(&"updateText"));
    assert!(names.contains(&"changeColor"));
    assert!(names.contains(&"getBoardState"));
}

#[test]
fn legacy_tools_all_have_object_schemas() {
    let tools = legacy_tools();
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
fn legacy_tools_required_fields_are_arrays() {
    let tools = legacy_tools();
    for tool in &tools {
        if let Some(required) = tool.input_schema.get("required") {
            assert!(required.is_array(), "tool {} required should be array", tool.name);
        }
    }
}

#[test]
fn create_sticky_note_requires_text_x_y() {
    let tools = legacy_tools();
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
    let tools = legacy_tools();
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
    let tools = legacy_tools();
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
    let tools = legacy_tools();
    let tool = tools.iter().find(|t| t.name == "getBoardState").unwrap();
    assert!(tool.input_schema.get("required").is_none());
}
