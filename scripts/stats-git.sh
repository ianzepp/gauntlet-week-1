#!/usr/bin/env bash
# stats-git.sh — Git history statistics.
#
# Reports: total commits, commits per day, first/last commit dates,
# and top changed files.

set -uo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

echo "=== Git History ==="
echo ""

total_commits=$(git rev-list --count HEAD)
first_commit=$(git log --reverse --format='%cs' | head -1 || true)
last_commit=$(git log -1 --format='%cs')

# macOS date calculation
first_epoch=$(date -j -f '%Y-%m-%d' "$first_commit" +%s 2>/dev/null || date -d "$first_commit" +%s 2>/dev/null || echo 0)
last_epoch=$(date -j -f '%Y-%m-%d' "$last_commit" +%s 2>/dev/null || date -d "$last_commit" +%s 2>/dev/null || echo 0)

if [[ $first_epoch -gt 0 && $last_epoch -gt 0 ]]; then
    days=$(( (last_epoch - first_epoch) / 86400 + 1 ))
else
    days=1
fi
per_day=$(awk "BEGIN {printf \"%.1f\", $total_commits / $days}")

printf "%-24s %s\n" "Total commits:" "$total_commits"
printf "%-24s %s\n" "First commit:" "$first_commit"
printf "%-24s %s\n" "Last commit:" "$last_commit"
printf "%-24s %s days\n" "Active span:" "$days"
printf "%-24s %s\n" "Commits/day:" "$per_day"

echo ""
echo "=== Commits by Crate (approximate, by path) ==="
echo ""

for crate in server canvas client frames perf traces; do
    count=$(git log --oneline -- "$crate/" 2>/dev/null | wc -l | tr -d ' ')
    printf "%-12s %d commits\n" "$crate" "$count"
done

echo ""
echo "=== Recent Activity (last 7 days) ==="
echo ""

since=$(date -v-7d +%Y-%m-%d 2>/dev/null || date -d '7 days ago' +%Y-%m-%d 2>/dev/null || echo "2000-01-01")
recent=$(git rev-list --count --since="$since" HEAD)
printf "%-24s %d\n" "Commits (last 7d):" "$recent"

echo ""
echo "=== Top 15 Most-Changed Files ==="
echo ""

git log --pretty=format: --name-only 2>/dev/null | grep '\.rs$' | sort | uniq -c | sort -rn | head -15 || true
