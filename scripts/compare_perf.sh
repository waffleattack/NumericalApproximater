#!/usr/bin/env bash
# Compare criterion benchmarks on HEAD vs the commit before the perf improvements.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRE_PERF="${1:-35da054^}"
WORKTREE="${TMPDIR:-/tmp}/na-preperf-bench"
RESULTS_DIR="$ROOT/target/perf-compare"
mkdir -p "$RESULTS_DIR"

bench_infra=(
    src/lib.rs
    benches/hot_path.rs
    Cargo.toml
    src/main.rs
)

run_bench() {
    local label="$1"
    local dir="$2"
    local out="$RESULTS_DIR/${label}.txt"
    echo "Running benchmarks in $dir ($label)..."
    (
        cd "$dir"
        cargo bench --bench hot_path -- --noplot 2>&1
    ) | tee "$out"
    echo "Saved: $out"
}

echo "=== Benchmark: current (HEAD) ==="
run_bench "after" "$ROOT"

echo ""
echo "=== Setting up worktree at $PRE_PERF ==="
git -C "$ROOT" worktree remove -f "$WORKTREE" 2>/dev/null || true
git -C "$ROOT" worktree add "$WORKTREE" "$PRE_PERF" --detach

mkdir -p "$WORKTREE/benches" "$WORKTREE/scripts"
for f in "${bench_infra[@]}"; do
    mkdir -p "$(dirname "$WORKTREE/$f")"
    cp "$ROOT/$f" "$WORKTREE/$f"
done

echo ""
echo "=== Benchmark: before perf ($PRE_PERF) ==="
run_bench "before" "$WORKTREE"

echo ""
echo "=== Summary (median time per iteration) ==="
python3 - "$RESULTS_DIR/before.txt" "$RESULTS_DIR/after.txt" <<'PY'
import re
import sys

def parse(path):
    rows = []
    current = None
    with open(path) as f:
        for line in f:
            m = re.search(r"Benchmarking (\S+)", line)
            if m:
                current = m.group(1)
            m = re.search(r"time:\s+\[([^\]]+)\]\s+([0-9.]+) ([a-z]+)", line)
            if m and current:
                rows.append((current, float(m.group(2)), m.group(3)))
                current = None
    return rows

def to_ns(val, unit):
    scale = {"ns": 1, "us": 1e3, "ms": 1e6, "s": 1e9}
    return val * scale[unit]

before = {k: to_ns(v, u) for k, v, u in parse(sys.argv[1])}
after = {k: to_ns(v, u) for k, v, u in parse(sys.argv[2])}

print(f"{'benchmark':<45} {'before':>12} {'after':>12} {'speedup':>10}")
print("-" * 82)
for name in sorted(set(before) | set(after)):
    b = before.get(name)
    a = after.get(name)
    if b is None or a is None:
        continue
    speedup = b / a
    def fmt(ns):
        if ns >= 1e6:
            return f"{ns/1e6:.2f} ms"
        if ns >= 1e3:
            return f"{ns/1e3:.2f} µs"
        return f"{ns:.1f} ns"
    print(f"{name:<45} {fmt(b):>12} {fmt(a):>12} {speedup:>9.2}x")

PY

echo ""
echo "Full logs: $RESULTS_DIR/before.txt and $RESULTS_DIR/after.txt"
git -C "$ROOT" worktree remove -f "$WORKTREE" 2>/dev/null || true
