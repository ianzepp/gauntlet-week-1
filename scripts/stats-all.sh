#!/usr/bin/env bash
# stats-all.sh — Run all stat scripts and print a combined report.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "========================================"
echo "  Field Board Project Stats"
echo "  $(date +%Y-%m-%d)"
echo "========================================"
echo ""

echo "--- Lines of Code ---"
"$SCRIPT_DIR/stats-lines.sh"
echo ""

echo "--- Functions ---"
"$SCRIPT_DIR/stats-fn.sh"
echo ""

echo "--- Documentation Coverage ---"
"$SCRIPT_DIR/stats-comments.sh"
echo ""

echo "--- Tests ---"
"$SCRIPT_DIR/stats-tests.sh"
echo ""

echo "--- Code Coverage ---"
"$SCRIPT_DIR/stats-coverage.sh"
echo ""

echo "--- Dependencies ---"
"$SCRIPT_DIR/stats-deps.sh"
echo ""

echo "--- Git ---"
"$SCRIPT_DIR/stats-git.sh"
echo ""
