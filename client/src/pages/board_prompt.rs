//! Prompt preview parsing helpers for board AI responses.

/// Build the inline assistant preview and indicate if there is hidden content.
pub fn assistant_preview_and_has_more(text: &str) -> (String, bool) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (String::new(), false);
    }

    let paragraphs = split_paragraphs(trimmed);
    let mut preview: Vec<String> = Vec::new();
    let mut has_more = false;

    for para in &paragraphs {
        if paragraph_is_structured(para) {
            if para.trim_end().ends_with(':') && preview.len() < 3 {
                preview.push(para.clone());
            }
            has_more = true;
            break;
        }

        if preview.len() < 3 {
            preview.push(para.clone());
        } else {
            has_more = true;
            break;
        }
    }

    if preview.is_empty() {
        if let Some(first) = paragraphs.first() {
            preview.push(first.clone());
        }
    }

    if !has_more && paragraphs.len() > preview.len() {
        has_more = true;
    }

    (preview.join("\n\n"), has_more)
}

fn split_paragraphs(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                out.push(current.join("\n").trim().to_owned());
                current.clear();
            }
            continue;
        }
        current.push(line.trim_end());
    }
    if !current.is_empty() {
        out.push(current.join("\n").trim().to_owned());
    }
    out.into_iter().filter(|p| !p.is_empty()).collect()
}

fn paragraph_is_structured(para: &str) -> bool {
    let trimmed = para.trim();
    if trimmed.ends_with(':') {
        return true;
    }
    para.lines().any(line_is_structured)
}

fn line_is_structured(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        return true;
    }
    if starts_with_markdown_numbered_list(t) {
        return true;
    }
    if t.starts_with('|') {
        return true;
    }
    t.contains('|') && (t.contains("---") || t.contains(":---") || t.contains("---:"))
}

fn starts_with_markdown_numbered_list(text: &str) -> bool {
    let mut saw_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if (ch == '.' || ch == ')') && saw_digit {
            return text
                .chars()
                .skip_while(|c| c.is_ascii_digit())
                .nth(1)
                .is_some_and(char::is_whitespace);
        }
        break;
    }
    false
}
