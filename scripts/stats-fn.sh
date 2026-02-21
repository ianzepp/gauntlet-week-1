#!/usr/bin/env bash
# stats-fn.sh — Count public functions per crate and overall.
#
# Reports: crate, source files, test files, pub fn count, non-pub fn count, total fn count.
# Excludes target/ and *_test.rs from "source" counts; test files counted separately.

set -euo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

CRATES=(server canvas client frames perf traces)

printf "%-12s %6s %6s %8s %8s %8s\n" "Crate" "Src" "Test" "Pub Fn" "Priv Fn" "Total"
printf "%-12s %6s %6s %8s %8s %8s\n" "-----" "---" "----" "------" "-------" "-----"

total_src=0 total_test=0 total_pub=0 total_priv=0

for crate in "${CRATES[@]}"; do
    if [[ ! -d "$crate" ]]; then continue; fi

    src_files=$(find "$crate" -name '*.rs' -not -path '*/target/*' -not -name '*_test.rs' -not -path '*/tests/*' | wc -l | tr -d ' ')
    test_files=$(find "$crate" -name '*_test.rs' -not -path '*/target/*' | wc -l | tr -d ' ')
    test_files=$((test_files + $(find "$crate" -path '*/tests/*.rs' -not -path '*/target/*' 2>/dev/null | wc -l | tr -d ' ')))

    pub_fns=$(grep -r --include='*.rs' -c '^\s*pub\s\+fn\s\|^\s*pub(crate)\s\+fn\s' "$crate" 2>/dev/null | grep -v '/target/' | grep -v '_test\.rs' | grep -v '/tests/' | awk -F: '{s+=$2} END {print s+0}')
    all_fns=$(grep -r --include='*.rs' -c '^\s*\(pub\s\+\|pub(crate)\s\+\)\?fn\s' "$crate" 2>/dev/null | grep -v '/target/' | grep -v '_test\.rs' | grep -v '/tests/' | awk -F: '{s+=$2} END {print s+0}')
    priv_fns=$((all_fns - pub_fns))

    printf "%-12s %6d %6d %8d %8d %8d\n" "$crate" "$src_files" "$test_files" "$pub_fns" "$priv_fns" "$all_fns"

    total_src=$((total_src + src_files))
    total_test=$((total_test + test_files))
    total_pub=$((total_pub + pub_fns))
    total_priv=$((total_priv + priv_fns))
done

total_all=$((total_pub + total_priv))
printf "%-12s %6s %6s %8s %8s %8s\n" "-----" "---" "----" "------" "-------" "-----"
printf "%-12s %6d %6d %8d %8d %8d\n" "TOTAL" "$total_src" "$total_test" "$total_pub" "$total_priv" "$total_all"
