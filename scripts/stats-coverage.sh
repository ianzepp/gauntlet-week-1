#!/usr/bin/env bash
# stats-coverage.sh — Line coverage per crate via cargo-llvm-cov.
#
# Requires: cargo-llvm-cov (cargo install cargo-llvm-cov)
#           llvm-tools-preview (rustup component add llvm-tools-preview)
#
# Reports: crate, covered lines, total lines, line coverage %.
# Test files (*_test.rs) are excluded from the coverage denominator.

set -uo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

if ! command -v cargo-llvm-cov &>/dev/null; then
    echo "error: cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov" >&2
    exit 1
fi

CRATES=(server canvas client frames traces)

printf "%-12s %8s %8s %8s\n" "Crate" "Covered" "Total" "Line%"
printf "%-12s %8s %8s %8s\n" "-----" "-------" "-----" "-----"

t_covered=0 t_total=0

for crate in "${CRATES[@]}"; do
    if [[ ! -d "$crate" ]]; then continue; fi

    json=$(cargo llvm-cov -p "$crate" --json --summary-only \
        --ignore-filename-regex '_test\.rs$' 2>/dev/null) || true

    if [[ -z "$json" ]]; then
        printf "%-12s %8s %8s %8s\n" "$crate" "-" "-" "-"
        continue
    fi

    read -r covered total pct < <(echo "$json" | python3 -c "
import sys, json
d = json.load(sys.stdin)
t = d['data'][0]['totals']['lines']
print(f\"{t['covered']} {t['count']} {t['percent']:.1f}\")
" 2>/dev/null) || true

    if [[ -z "${covered:-}" ]]; then
        printf "%-12s %8s %8s %8s\n" "$crate" "-" "-" "-"
        continue
    fi

    printf "%-12s %8d %8d %7s%%\n" "$crate" "$covered" "$total" "$pct"

    t_covered=$((t_covered + covered))
    t_total=$((t_total + total))
done

if [[ $t_total -gt 0 ]]; then
    t_pct=$(awk "BEGIN {printf \"%.1f\", $t_covered / $t_total * 100}")
else
    t_pct="0.0"
fi

printf "%-12s %8s %8s %8s\n" "-----" "-------" "-----" "-----"
printf "%-12s %8d %8d %7s%%\n" "TOTAL" "$t_covered" "$t_total" "$t_pct"
