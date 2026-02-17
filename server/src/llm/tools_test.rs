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
