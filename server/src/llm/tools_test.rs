use super::*;

#[test]
fn tool_count() {
    let tools = gauntlet_week_1_tools();
    assert_eq!(tools.len(), 10);
}

#[test]
fn tool_names_match_spec() {
    let tools = gauntlet_week_1_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    let expected = [
        "batch",
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
fn create_sticky_note_schema() {
    let tools = gauntlet_week_1_tools();
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

    let properties = tool.input_schema.get("properties").unwrap();
    for key in [
        "backgroundColor",
        "borderColor",
        "borderWidth",
        "fill",
        "stroke",
        "stroke_width",
    ] {
        assert!(properties.get(key).is_some(), "createStickyNote missing property: {key}");
    }
    assert!(
        properties.get("color").is_none(),
        "createStickyNote should not expose deprecated color"
    );
}

#[test]
fn get_board_state_has_no_required_params() {
    let tools = gauntlet_week_1_tools();
    let tool = tools.iter().find(|t| t.name == "getBoardState").unwrap();
    // No "required" key or empty
    let required = tool.input_schema.get("required");
    assert!(required.is_none() || required.unwrap().as_array().unwrap().is_empty());
}

#[test]
fn batch_schema_requires_calls() {
    let tools = gauntlet_week_1_tools();
    let tool = tools.iter().find(|t| t.name == "batch").unwrap();
    let required = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    let req_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(req_strs.contains(&"calls"));
}

#[test]
fn create_shape_schema_exposes_style_fields() {
    let tools = gauntlet_week_1_tools();
    let tool = tools.iter().find(|t| t.name == "createShape").unwrap();
    let properties = tool.input_schema.get("properties").unwrap();
    for key in [
        "backgroundColor",
        "borderColor",
        "borderWidth",
        "fill",
        "stroke",
        "stroke_width",
    ] {
        assert!(properties.get(key).is_some(), "createShape missing property: {key}");
    }
    assert!(
        properties.get("color").is_none(),
        "createShape should not expose deprecated color"
    );
}

#[test]
fn change_color_schema_only_requires_object_id() {
    let tools = gauntlet_week_1_tools();
    let tool = tools.iter().find(|t| t.name == "changeColor").unwrap();
    let required = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    let req_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(req_strs, vec!["objectId"]);
    let properties = tool.input_schema.get("properties").unwrap();
    assert!(
        properties.get("color").is_none(),
        "changeColor should not expose deprecated color"
    );
}
