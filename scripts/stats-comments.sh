#!/usr/bin/env bash
# stats-comments.sh — Measure documentation coverage per crate.
#
# Reports: crate, lines of code, doc comment lines (///), module doc lines (//!),
# inline comment lines (//), blank lines, and doc ratio (doc lines / code lines).

set -euo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

CRATES=(server canvas client frames perf traces)

count_pattern() {
    local pattern="$1" file="$2"
    local n
    n=$(grep -c "$pattern" "$file" 2>/dev/null) || true
    echo "${n:-0}"
}

printf "%-12s %7s %7s %7s %7s %7s %7s\n" "Crate" "Code" "/// Doc" "//! Mod" "// Cmt" "Blank" "Doc%"
printf "%-12s %7s %7s %7s %7s %7s %7s\n" "-----" "----" "-------" "-------" "------" "-----" "----"

t_code=0 t_doc=0 t_mod=0 t_cmt=0 t_blank=0

for crate in "${CRATES[@]}"; do
    if [[ ! -d "$crate" ]]; then continue; fi

    doc_lines=0 mod_lines=0 cmt_lines=0 blank_lines=0 total_lines=0

    while IFS= read -r f; do
        doc_lines=$((doc_lines + $(count_pattern '^\s*///' "$f")))
        mod_lines=$((mod_lines + $(count_pattern '^\s*//!' "$f")))
        cmt_lines=$((cmt_lines + $(count_pattern '^\s*//[^/!]' "$f")))
        blank_lines=$((blank_lines + $(count_pattern '^\s*$' "$f")))
        total_lines=$((total_lines + $(wc -l < "$f")))
    done < <(find "$crate" -name '*.rs' -not -path '*/target/*' -not -name '*_test.rs' -not -path '*/tests/*')

    code_lines=$((total_lines - doc_lines - mod_lines - cmt_lines - blank_lines))
    if [[ $code_lines -lt 0 ]]; then code_lines=0; fi

    if [[ $code_lines -gt 0 ]]; then
        doc_pct=$(awk "BEGIN {printf \"%.1f\", ($doc_lines + $mod_lines) / $code_lines * 100}")
    else
        doc_pct="0.0"
    fi

    printf "%-12s %7d %7d %7d %7d %7d %6s%%\n" "$crate" "$code_lines" "$doc_lines" "$mod_lines" "$cmt_lines" "$blank_lines" "$doc_pct"

    t_code=$((t_code + code_lines))
    t_doc=$((t_doc + doc_lines))
    t_mod=$((t_mod + mod_lines))
    t_cmt=$((t_cmt + cmt_lines))
    t_blank=$((t_blank + blank_lines))
done

if [[ $t_code -gt 0 ]]; then
    t_pct=$(awk "BEGIN {printf \"%.1f\", ($t_doc + $t_mod) / $t_code * 100}")
else
    t_pct="0.0"
fi

printf "%-12s %7s %7s %7s %7s %7s %7s\n" "-----" "----" "-------" "-------" "------" "-----" "----"
printf "%-12s %7d %7d %7d %7d %7d %6s%%\n" "TOTAL" "$t_code" "$t_doc" "$t_mod" "$t_cmt" "$t_blank" "$t_pct"
