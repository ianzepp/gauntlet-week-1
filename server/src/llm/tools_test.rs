use super::*;

#[test]
fn yaml_only_mode_exposes_single_tool() {
    let tools = gauntlet_week_1_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "applyChangesYaml");
}

#[test]
fn apply_changes_yaml_schema_requires_yaml_string() {
    let tools = gauntlet_week_1_tools();
    let tool = &tools[0];
    let required = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    let req_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(req_strs, vec!["yaml"]);
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
