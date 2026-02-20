use super::assistant_preview_and_has_more;

#[test]
fn preview_for_empty_text_is_empty_without_more() {
    let (preview, has_more) = assistant_preview_and_has_more("   ");
    assert!(preview.is_empty());
    assert!(!has_more);
}

#[test]
fn preview_preserves_multiline_paragraph_and_trims_outer_whitespace() {
    let input = "  First line\nsecond line  \n\nthird paragraph ";
    let (preview, has_more) = assistant_preview_and_has_more(input);
    assert_eq!(preview, "First line\nsecond line\n\nthird paragraph");
    assert!(!has_more);
}

#[test]
fn preview_flags_markdown_table_as_structured() {
    let input = "Summary\n\n| A | B |\n| --- | --- |";
    let (preview, has_more) = assistant_preview_and_has_more(input);
    assert_eq!(preview, "Summary");
    assert!(has_more);
}

#[test]
fn preview_flags_numbered_parenthesis_list_as_structured() {
    let input = "Intro\n\n1) first\n2) second";
    let (preview, has_more) = assistant_preview_and_has_more(input);
    assert_eq!(preview, "Intro");
    assert!(has_more);
}
