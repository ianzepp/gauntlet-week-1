#![cfg(not(feature = "hydrate"))]

use super::*;

#[test]
fn read_preference_is_false_in_non_hydrate_tests() {
    assert!(!read_preference());
}

#[test]
fn toggle_flips_boolean_value() {
    assert!(toggle(false));
    assert!(!toggle(true));
}

#[test]
fn apply_is_noop_but_callable() {
    apply(false);
    apply(true);
}
