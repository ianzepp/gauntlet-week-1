#!/usr/bin/env bash
# stats-deps.sh — List direct dependencies per crate from Cargo.toml.
#
# Reports: crate, number of direct [dependencies] and [dev-dependencies].

set -euo pipefail
cd "$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"

CRATES=(server canvas client frames perf traces)

printf "%-12s %8s %8s %8s\n" "Crate" "Deps" "DevDeps" "Total"
printf "%-12s %8s %8s %8s\n" "-----" "----" "-------" "-----"

t_deps=0 t_dev=0

for crate in "${CRATES[@]}"; do
    toml="$crate/Cargo.toml"
    if [[ ! -f "$toml" ]]; then continue; fi

    # Count lines between [dependencies] and next section header (excluding workspace deps section)
    deps=$(awk '/^\[dependencies\]/{f=1; next} /^\[/{f=0} f && /^[a-zA-Z]/{c++} END{print c+0}' "$toml")
    dev_deps=$(awk '/^\[dev-dependencies\]/{f=1; next} /^\[/{f=0} f && /^[a-zA-Z]/{c++} END{print c+0}' "$toml")
    total=$((deps + dev_deps))

    printf "%-12s %8d %8d %8d\n" "$crate" "$deps" "$dev_deps" "$total"

    t_deps=$((t_deps + deps))
    t_dev=$((t_dev + dev_deps))
done

t_total=$((t_deps + t_dev))
printf "%-12s %8s %8s %8s\n" "-----" "----" "-------" "-----"
printf "%-12s %8d %8d %8d\n" "TOTAL" "$t_deps" "$t_dev" "$t_total"
