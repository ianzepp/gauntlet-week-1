//! Hygiene — enforces coding standards at test time
//!
//! These tests scan the canvas crate source tree for antipatterns that violate
//! project standards. Each has a budget (ideally zero). If you must add one,
//! you have to fix an existing one first — the budget never grows.
#![allow(clippy::absurd_extreme_comparisons)]

use std::fs;
use std::path::Path;

// Panics — these crash the process.
const MAX_UNWRAP: usize = 0;
const MAX_EXPECT: usize = 0;
const MAX_PANIC: usize = 0;
const MAX_UNREACHABLE: usize = 0;
const MAX_TODO: usize = 0;
const MAX_UNIMPLEMENTED: usize = 0;

// Silent loss — discards errors without inspecting.
const MAX_SILENT_DISCARD: usize = 0;
const MAX_DOT_OK: usize = 0;

// Style / structure.
const MAX_ALLOW_DEAD_CODE: usize = 0;

struct SourceFile {
    path: String,
    content: String,
}

/// Collect production `.rs` files from `canvas/src/`, excluding test files.
fn source_files() -> Vec<SourceFile> {
    let mut files = Vec::new();
    collect_rs_files(Path::new("src"), &mut files);
    files
}

fn collect_rs_files(dir: &Path, out: &mut Vec<SourceFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == "target" || name == "tests" {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let path_str = path.to_string_lossy().to_string();
            // Skip test files
            if path_str.ends_with("_test.rs") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                out.push(SourceFile { path: path_str, content });
            }
        }
    }
}

fn count_in_source(files: &[SourceFile], pattern: &str) -> Vec<(String, usize)> {
    files
        .iter()
        .filter_map(|file| {
            let count = file
                .content
                .lines()
                .filter(|line| line.contains(pattern))
                .count();
            if count > 0 {
                Some((file.path.clone(), count))
            } else {
                None
            }
        })
        .collect()
}

fn total(hits: &[(String, usize)]) -> usize {
    hits.iter().map(|(_, c)| c).sum()
}

fn format_hits(hits: &[(String, usize)]) -> String {
    hits.iter()
        .map(|(path, count)| format!("  {path}: {count}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn unwrap_budget() {
    let files = source_files();
    let hits = count_in_source(&files, ".unwrap()");
    let count = total(&hits);
    assert!(
        count <= MAX_UNWRAP,
        ".unwrap() budget exceeded: found {count}, max {MAX_UNWRAP}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn expect_budget() {
    let files = source_files();
    let hits = count_in_source(&files, ".expect(");
    let count = total(&hits);
    assert!(
        count <= MAX_EXPECT,
        ".expect() budget exceeded: found {count}, max {MAX_EXPECT}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn panic_budget() {
    let files = source_files();
    let hits = count_in_source(&files, "panic!(");
    let count = total(&hits);
    assert!(
        count <= MAX_PANIC,
        "panic!() budget exceeded: found {count}, max {MAX_PANIC}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn unreachable_budget() {
    let files = source_files();
    let hits = count_in_source(&files, "unreachable!(");
    let count = total(&hits);
    assert!(
        count <= MAX_UNREACHABLE,
        "unreachable!() budget exceeded: found {count}, max {MAX_UNREACHABLE}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn todo_budget() {
    let files = source_files();
    let hits = count_in_source(&files, "todo!(");
    let count = total(&hits);
    assert!(
        count <= MAX_TODO,
        "todo!() budget exceeded: found {count}, max {MAX_TODO}. Ratchet down as stubs are implemented.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn unimplemented_budget() {
    let files = source_files();
    let hits = count_in_source(&files, "unimplemented!(");
    let count = total(&hits);
    assert!(
        count <= MAX_UNIMPLEMENTED,
        "unimplemented!() budget exceeded: found {count}, max {MAX_UNIMPLEMENTED}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn silent_discard_budget() {
    let files = source_files();
    let hits = count_in_source(&files, "let _ =");
    let count = total(&hits);
    assert!(
        count <= MAX_SILENT_DISCARD,
        "let _ = budget exceeded: found {count}, max {MAX_SILENT_DISCARD}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn dot_ok_budget() {
    let files = source_files();
    let hits = count_in_source(&files, ".ok()");
    let count = total(&hits);
    assert!(
        count <= MAX_DOT_OK,
        ".ok() budget exceeded: found {count}, max {MAX_DOT_OK}.\n{}",
        format_hits(&hits)
    );
}

#[test]
fn allow_dead_code_budget() {
    let files = source_files();
    let hits = count_in_source(&files, "#[allow(dead_code)]");
    let count = total(&hits);
    assert!(
        count <= MAX_ALLOW_DEAD_CODE,
        "#[allow(dead_code)] budget exceeded: found {count}, max {MAX_ALLOW_DEAD_CODE}.\n{}",
        format_hits(&hits)
    );
}
