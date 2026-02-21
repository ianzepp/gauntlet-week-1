#!/usr/bin/env bash
# stats-tests.sh — Count #[test] functions per crate and run the test suite.
#
# Reports: crate, #[test] count, #[ignore] count, pass/fail from cargo test.

set -uo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

CRATES=(server canvas client frames perf traces)

printf "%-12s %8s %8s %8s %8s\n" "Crate" "#[test]" "#[ignore]" "Passed" "Failed"
printf "%-12s %8s %8s %8s %8s\n" "-----" "-------" "---------" "------" "------"

t_tests=0 t_ignored=0 t_passed=0 t_failed=0

for crate in "${CRATES[@]}"; do
    if [[ ! -d "$crate" ]]; then continue; fi

    test_count=$(grep -r --include='*.rs' '#\[test\]' "$crate" 2>/dev/null | grep -v '/target/' | wc -l | tr -d ' ')
    ignore_count=$(grep -r --include='*.rs' '#\[ignore\]' "$crate" 2>/dev/null | grep -v '/target/' | wc -l | tr -d ' ')

    # Run tests for this crate and parse output
    test_output=$(cargo test -p "$crate" 2>&1 || true)
    passed=$(echo "$test_output" | grep '^test result:' | awk '{s+=$4} END {print s+0}')
    failed=$(echo "$test_output" | grep '^test result:' | awk '{s+=$6} END {print s+0}')

    printf "%-12s %8d %8d %8d %8d\n" "$crate" "$test_count" "$ignore_count" "$passed" "$failed"

    t_tests=$((t_tests + test_count))
    t_ignored=$((t_ignored + ignore_count))
    t_passed=$((t_passed + passed))
    t_failed=$((t_failed + failed))
done

printf "%-12s %8s %8s %8s %8s\n" "-----" "-------" "---------" "------" "------"
printf "%-12s %8d %8d %8d %8d\n" "TOTAL" "$t_tests" "$t_ignored" "$t_passed" "$t_failed"
