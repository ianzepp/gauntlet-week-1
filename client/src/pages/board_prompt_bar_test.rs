use super::PromptBarStatus;

#[test]
fn prompt_bar_status_default_is_idle() {
    assert_eq!(PromptBarStatus::default(), PromptBarStatus::Idle);
}

#[test]
fn prompt_bar_status_variants_are_distinct() {
    assert_ne!(PromptBarStatus::Idle, PromptBarStatus::Loading);
    assert_ne!(PromptBarStatus::Loading, PromptBarStatus::Success);
    assert_ne!(PromptBarStatus::Success, PromptBarStatus::Error);
}
