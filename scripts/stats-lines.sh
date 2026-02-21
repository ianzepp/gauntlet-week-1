#!/usr/bin/env bash
# stats-lines.sh — Lines of code per crate (source vs test).
#
# Reports: crate, source LOC, test LOC, total LOC, test%.

set -euo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

CRATES=(server canvas client frames perf traces)

printf "%-12s %8s %8s %8s %6s\n" "Crate" "Source" "Test" "Total" "Test%"
printf "%-12s %8s %8s %8s %6s\n" "-----" "------" "----" "-----" "-----"

t_src=0 t_test=0

for crate in "${CRATES[@]}"; do
    if [[ ! -d "$crate" ]]; then continue; fi

    src_loc=$(find "$crate" -name '*.rs' -not -path '*/target/*' -not -name '*_test.rs' -not -path '*/tests/*' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    test_loc=$(find "$crate" \( -name '*_test.rs' -o -path '*/tests/*.rs' \) -not -path '*/target/*' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    total=$((src_loc + test_loc))

    if [[ $total -gt 0 ]]; then
        pct=$(awk "BEGIN {printf \"%.1f\", $test_loc / $total * 100}")
    else
        pct="0.0"
    fi

    printf "%-12s %8d %8d %8d %5s%%\n" "$crate" "$src_loc" "$test_loc" "$total" "$pct"

    t_src=$((t_src + src_loc))
    t_test=$((t_test + test_loc))
done

t_total=$((t_src + t_test))
if [[ $t_total -gt 0 ]]; then
    t_pct=$(awk "BEGIN {printf \"%.1f\", $t_test / $t_total * 100}")
else
    t_pct="0.0"
fi

printf "%-12s %8s %8s %8s %6s\n" "-----" "------" "----" "-----" "-----"
printf "%-12s %8d %8d %8d %5s%%\n" "TOTAL" "$t_src" "$t_test" "$t_total" "$t_pct"
