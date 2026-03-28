#!/usr/bin/env bash
# nucleo benchmark suite
# Measures token consumption and execution speed across all CLI commands.
#
# Usage:
#   ./benchmarks/run.sh                  # full suite, markdown report
#   ./benchmarks/run.sh --json           # output raw JSON results
#   ./benchmarks/run.sh --quick          # subset of commands (fast smoke test)
#   ./benchmarks/run.sh --formats        # compare all formats for each command
#   ./benchmarks/run.sh --help           # show help
#
# Prerequisites:
#   - nucleo installed to PATH (cargo install --path .)
#   - Authenticated session (nucleo auth login) — for echo command

set -euo pipefail

# ─── Config ──────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
TIMESTAMP=$(date +%Y-%m-%dT%H:%M:%S)
DATE_SLUG=$(date +%Y-%m-%d)

# Token estimation: ~4 chars per token (GPT/Claude average for English + JSON)
CHARS_PER_TOKEN=4

# Resolve nucleo binary — prefer PATH-installed binary
if command -v nucleo &>/dev/null; then
  CLI="nucleo"
elif [[ -x "$ROOT_DIR/target/release/nucleo" ]]; then
  echo "Warning: nucleo not in PATH. Using target/release build." >&2
  CLI="$ROOT_DIR/target/release/nucleo"
elif [[ -x "$ROOT_DIR/target/debug/nucleo" ]]; then
  echo "Warning: nucleo not in PATH. Using target/debug build." >&2
  CLI="$ROOT_DIR/target/debug/nucleo"
else
  echo "Error: nucleo not found. Run 'cargo install --path .' first." >&2
  exit 1
fi

# ─── Flags ───────────────────────────────────────────────────────────────────

OUTPUT_MODE="markdown"
QUICK=false
COMPARE_FORMATS=false

for arg in "$@"; do
  case "$arg" in
    --json) OUTPUT_MODE="json" ;;
    --quick) QUICK=true ;;
    --formats) COMPARE_FORMATS=true ;;
    --help|-h)
      echo "nucleo benchmark suite"
      echo ""
      echo "Usage: ./benchmarks/run.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --json       Output raw JSON results instead of markdown"
      echo "  --quick      Run a small subset of commands"
      echo "  --formats    Compare all output formats for each command"
      echo "  --help       Show this help"
      exit 0
      ;;
  esac
done

# ─── Command Definitions ────────────────────────────────────────────────────

# Each entry: "category|label|command_args"
COMMANDS=(
  "core|status|status --format json"
  "core|config show|config show"
  "core|ping httpbin|ping --url https://httpbin.org/get --format json"
  "core|echo httpbin|echo --data {\"test\":true} --url https://httpbin.org/post --format json"
  "plugins|plugins list|plugins list --format json"
)

QUICK_COMMANDS=(
  "core|status|status --format json"
  "core|config show|config show"
  "core|ping httpbin|ping --url https://httpbin.org/get --format json"
)

FORMATS=("json" "table" "yaml" "csv" "ids")

# ─── Helpers ─────────────────────────────────────────────────────────────────

# Portable nanosecond timestamp (macOS date doesn't support %N)
now_ns() {
  if [[ "$(uname)" == "Darwin" ]]; then
    python3 -c 'import time; print(int(time.time()*1e9))'
  else
    date +%s%N
  fi
}

# Run a single command, capture output size and time
# Returns: "exit_code|time_ms|bytes|chars|est_tokens|lines"
bench_one() {
  local cmd_args="$1"
  local tmpfile
  tmpfile=$(mktemp)

  local start_ns end_ns elapsed_ms
  start_ns=$(now_ns)

  # shellcheck disable=SC2086
  if $CLI $cmd_args > "$tmpfile" 2>/dev/null; then
    local exit_code=0
  else
    local exit_code=$?
  fi

  end_ns=$(now_ns)
  elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))

  local bytes chars lines est_tokens
  bytes=$(wc -c < "$tmpfile" | tr -d ' ')
  chars=$(wc -m < "$tmpfile" | tr -d ' ')
  lines=$(wc -l < "$tmpfile" | tr -d ' ')
  est_tokens=$(( chars / CHARS_PER_TOKEN ))

  rm -f "$tmpfile"
  echo "${exit_code}|${elapsed_ms}|${bytes}|${chars}|${est_tokens}|${lines}"
}

# Right-align a number with commas
fmt_num() {
  printf "%'d" "$1" 2>/dev/null || printf "%d" "$1"
}

# ─── Run Benchmarks ─────────────────────────────────────────────────────────

if $QUICK; then
  ACTIVE_COMMANDS=("${QUICK_COMMANDS[@]}")
else
  ACTIVE_COMMANDS=("${COMMANDS[@]}")
fi

declare -a RESULTS=()
TOTAL_TOKENS=0
TOTAL_MS=0
MAX_TOKENS=0
MAX_MS=0
PASS_COUNT=0
FAIL_COUNT=0

echo "⏱  nucleo benchmark — $(date '+%Y-%m-%d %H:%M:%S')" >&2
echo "   Binary: $CLI" >&2
echo "   Commands: ${#ACTIVE_COMMANDS[@]}" >&2
echo "" >&2

for entry in "${ACTIVE_COMMANDS[@]}"; do
  IFS='|' read -r category label cmd_args <<< "$entry"
  printf "   %-35s " "$label" >&2

  result=$(bench_one "$cmd_args")
  IFS='|' read -r exit_code time_ms bytes chars est_tokens lines <<< "$result"

  if [[ $exit_code -eq 0 ]]; then
    printf "✓  %5d ms  %6d tokens  %6d bytes\n" "$time_ms" "$est_tokens" "$bytes" >&2
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    printf "✗  (exit %d)\n" "$exit_code" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi

  RESULTS+=("${category}|${label}|${cmd_args}|${exit_code}|${time_ms}|${bytes}|${chars}|${est_tokens}|${lines}")

  TOTAL_TOKENS=$((TOTAL_TOKENS + est_tokens))
  TOTAL_MS=$((TOTAL_MS + time_ms))
  (( est_tokens > MAX_TOKENS )) && MAX_TOKENS=$est_tokens
  (( time_ms > MAX_MS )) && MAX_MS=$time_ms
done

# ─── Format Comparison (optional) ───────────────────────────────────────────

declare -a FORMAT_RESULTS=()

if $COMPARE_FORMATS; then
  echo "" >&2
  echo "📊 Format comparison" >&2
  echo "" >&2

  FORMAT_COMMANDS=(
    "status|status"
    "ping httpbin|ping --url https://httpbin.org/get"
  )

  for entry in "${FORMAT_COMMANDS[@]}"; do
    IFS='|' read -r label base_cmd <<< "$entry"
    printf "   %s\n" "$label" >&2

    for fmt in "${FORMATS[@]}"; do
      result=$(bench_one "$base_cmd --format $fmt")
      IFS='|' read -r exit_code time_ms bytes chars est_tokens lines <<< "$result"
      printf "     %-8s %6d tokens  %6d bytes  %5d ms\n" "$fmt" "$est_tokens" "$bytes" "$time_ms" >&2
      FORMAT_RESULTS+=("${label}|${fmt}|${exit_code}|${time_ms}|${bytes}|${chars}|${est_tokens}|${lines}")
    done
  done
fi

# ─── Output: JSON ───────────────────────────────────────────────────────────

if [[ "$OUTPUT_MODE" == "json" ]]; then
  mkdir -p "$RESULTS_DIR"
  JSON_FILE="$RESULTS_DIR/bench-${DATE_SLUG}.json"

  {
    echo "{"
    echo "  \"timestamp\": \"$TIMESTAMP\","
    echo "  \"binary\": \"$CLI\","
    echo "  \"chars_per_token\": $CHARS_PER_TOKEN,"
    echo "  \"summary\": {"
    echo "    \"total_commands\": ${#ACTIVE_COMMANDS[@]},"
    echo "    \"passed\": $PASS_COUNT,"
    echo "    \"failed\": $FAIL_COUNT,"
    echo "    \"total_tokens\": $TOTAL_TOKENS,"
    echo "    \"total_ms\": $TOTAL_MS,"
    echo "    \"avg_tokens\": $(( TOTAL_TOKENS / ${#ACTIVE_COMMANDS[@]} )),"
    echo "    \"avg_ms\": $(( TOTAL_MS / ${#ACTIVE_COMMANDS[@]} ))"
    echo "  },"
    echo "  \"results\": ["

    for i in "${!RESULTS[@]}"; do
      IFS='|' read -r category label cmd_args exit_code time_ms bytes chars est_tokens lines <<< "${RESULTS[$i]}"
      comma=","
      [[ $i -eq $(( ${#RESULTS[@]} - 1 )) ]] && comma=""
      echo "    {\"category\":\"$category\",\"label\":\"$label\",\"command\":\"nucleo $cmd_args\",\"exit_code\":$exit_code,\"time_ms\":$time_ms,\"bytes\":$bytes,\"chars\":$chars,\"est_tokens\":$est_tokens,\"lines\":$lines}${comma}"
    done

    echo "  ]"

    if [[ ${#FORMAT_RESULTS[@]} -gt 0 ]]; then
      echo "  ,\"format_comparison\": ["
      for i in "${!FORMAT_RESULTS[@]}"; do
        IFS='|' read -r label fmt exit_code time_ms bytes chars est_tokens lines <<< "${FORMAT_RESULTS[$i]}"
        comma=","
        [[ $i -eq $(( ${#FORMAT_RESULTS[@]} - 1 )) ]] && comma=""
        echo "    {\"label\":\"$label\",\"format\":\"$fmt\",\"exit_code\":$exit_code,\"time_ms\":$time_ms,\"bytes\":$bytes,\"chars\":$chars,\"est_tokens\":$est_tokens,\"lines\":$lines}${comma}"
      done
      echo "  ]"
    fi

    echo "}"
  } > "$JSON_FILE"

  echo "" >&2
  echo "✅ JSON results: $JSON_FILE" >&2
  cat "$JSON_FILE"
  exit 0
fi

# ─── Output: Markdown Report ────────────────────────────────────────────────

mkdir -p "$RESULTS_DIR"
REPORT_FILE="$RESULTS_DIR/bench-${DATE_SLUG}.md"

{
cat <<'HEADER'
# nucleo Benchmark Results

> Measures token consumption and execution speed for every CLI command.
> Token estimates use ~4 chars/token (standard for GPT/Claude with English + JSON).

HEADER

echo "**Date:** $TIMESTAMP"
echo "**Binary:** \`$CLI\`"
echo ""

# ── Summary card ──

echo "## Summary"
echo ""
echo "| Metric | Value |"
echo "|--------|------:|"
echo "| Commands tested | ${#ACTIVE_COMMANDS[@]} |"
echo "| Passed | $PASS_COUNT |"
echo "| Failed | $FAIL_COUNT |"
echo "| Total tokens (all commands) | $(fmt_num $TOTAL_TOKENS) |"
echo "| Avg tokens / command | $(fmt_num $(( TOTAL_TOKENS / ${#ACTIVE_COMMANDS[@]} ))) |"
echo "| Total time | ${TOTAL_MS} ms |"
echo "| Avg time / command | $(( TOTAL_MS / ${#ACTIVE_COMMANDS[@]} )) ms |"
echo ""

# ── Token consumption table ──

echo "## Token Consumption"
echo ""
echo "| Command | Tokens | Bytes | Lines | Status |"
echo "|---------|-------:|------:|------:|:------:|"

for entry in "${RESULTS[@]}"; do
  IFS='|' read -r category label cmd_args exit_code time_ms bytes chars est_tokens lines <<< "$entry"
  if [[ $exit_code -eq 0 ]]; then
    status="✓"
  else
    status="✗"
  fi
  printf "| %-35s | %6d | %6d | %5d | %s |\n" "$label" "$est_tokens" "$bytes" "$lines" "$status"
done

echo ""

# ── Speed table ──

echo "## Execution Speed"
echo ""
echo "| Command | Time (ms) | Tokens | Status |"
echo "|---------|----------:|-------:|:------:|"

for entry in "${RESULTS[@]}"; do
  IFS='|' read -r category label cmd_args exit_code time_ms bytes chars est_tokens lines <<< "$entry"
  if [[ $exit_code -eq 0 ]]; then
    status="✓"
  else
    status="✗"
  fi
  printf "| %-35s | %6d | %6d | %s |\n" "$label" "$time_ms" "$est_tokens" "$status"
done

echo ""

# ── Category breakdown ──

echo "## By Category"
echo ""

CAT_NAMES=()
CAT_TOKENS_VAL=()
CAT_MS_VAL=()
CAT_COUNT_VAL=()

_cat_index() {
  local needle="$1" i
  for i in "${!CAT_NAMES[@]}"; do
    [[ "${CAT_NAMES[$i]}" == "$needle" ]] && echo "$i" && return
  done
  echo "-1"
}

for entry in "${RESULTS[@]}"; do
  IFS='|' read -r category label cmd_args exit_code time_ms bytes chars est_tokens lines <<< "$entry"
  idx=$(_cat_index "$category")
  if [[ $idx -eq -1 ]]; then
    CAT_NAMES+=("$category")
    CAT_TOKENS_VAL+=($est_tokens)
    CAT_MS_VAL+=($time_ms)
    CAT_COUNT_VAL+=(1)
  else
    CAT_TOKENS_VAL[$idx]=$(( ${CAT_TOKENS_VAL[$idx]} + est_tokens ))
    CAT_MS_VAL[$idx]=$(( ${CAT_MS_VAL[$idx]} + time_ms ))
    CAT_COUNT_VAL[$idx]=$(( ${CAT_COUNT_VAL[$idx]} + 1 ))
  fi
done

echo "| Category | Commands | Total Tokens | Avg Tokens | Total Time (ms) |"
echo "|----------|:--------:|-------------:|-----------:|----------------:|"

for i in "${!CAT_NAMES[@]}"; do
  avg_tok=$(( ${CAT_TOKENS_VAL[$i]} / ${CAT_COUNT_VAL[$i]} ))
  printf "| %-12s | %d | %6d | %6d | %6d |\n" "${CAT_NAMES[$i]}" "${CAT_COUNT_VAL[$i]}" "${CAT_TOKENS_VAL[$i]}" "$avg_tok" "${CAT_MS_VAL[$i]}"
done

echo ""

# ── Format comparison (if run) ──

if [[ ${#FORMAT_RESULTS[@]} -gt 0 ]]; then
  echo "## Format Comparison"
  echo ""
  echo "Token cost of the same data in different output formats:"
  echo ""
  echo "| Command | json | table | yaml | csv | ids |"
  echo "|---------|-----:|------:|-----:|----:|----:|"

  FMT_KEYS=()
  FMT_VALS=()
  SEEN_LABELS=()

  for entry in "${FORMAT_RESULTS[@]}"; do
    IFS='|' read -r label fmt exit_code time_ms bytes chars est_tokens lines <<< "$entry"
    FMT_KEYS+=("${label}|${fmt}")
    FMT_VALS+=($est_tokens)
    found=false
    for seen in "${SEEN_LABELS[@]}"; do
      [[ "$seen" == "$label" ]] && found=true && break
    done
    $found || SEEN_LABELS+=("$label")
  done

  _fmt_lookup() {
    local needle="$1" i
    for i in "${!FMT_KEYS[@]}"; do
      [[ "${FMT_KEYS[$i]}" == "$needle" ]] && echo "${FMT_VALS[$i]}" && return
    done
    echo "-"
  }

  for label in "${SEEN_LABELS[@]}"; do
    json_t=$(_fmt_lookup "${label}|json")
    table_t=$(_fmt_lookup "${label}|table")
    yaml_t=$(_fmt_lookup "${label}|yaml")
    csv_t=$(_fmt_lookup "${label}|csv")
    ids_t=$(_fmt_lookup "${label}|ids")
    printf "| %-20s | %5s | %5s | %5s | %5s | %5s |\n" "$label" "$json_t" "$table_t" "$yaml_t" "$csv_t" "$ids_t"
  done

  echo ""
  echo "### Compression Ratio (vs JSON)"
  echo ""
  echo "| Command | table | yaml | csv | ids |"
  echo "|---------|------:|-----:|----:|----:|"

  for label in "${SEEN_LABELS[@]}"; do
    json_t=$(_fmt_lookup "${label}|json")
    [[ "$json_t" == "-" || "$json_t" -eq 0 ]] && json_t=1
    table_t=$(_fmt_lookup "${label}|table")
    yaml_t=$(_fmt_lookup "${label}|yaml")
    csv_t=$(_fmt_lookup "${label}|csv")
    ids_t=$(_fmt_lookup "${label}|ids")
    table_r="—"; [[ "$table_t" != "-" ]] && table_r="$(( table_t * 100 / json_t ))%"
    yaml_r="—";  [[ "$yaml_t" != "-" ]]  && yaml_r="$(( yaml_t * 100 / json_t ))%"
    csv_r="—";   [[ "$csv_t" != "-" ]]   && csv_r="$(( csv_t * 100 / json_t ))%"
    ids_r="—";   [[ "$ids_t" != "-" ]]   && ids_r="$(( ids_t * 100 / json_t ))%"
    printf "| %-20s | %5s | %5s | %5s | %5s |\n" "$label" "$table_r" "$yaml_r" "$csv_r" "$ids_r"
  done

  echo ""
fi

# ── Footer ──

cat <<'FOOTER'
---

## Methodology

- **Token estimation**: `output_chars / 4` (industry standard for English + structured data with GPT/Claude tokenizers)
- **Timing**: Wall-clock time via nanosecond timestamps, includes network round-trip + CLI startup + formatting
- **Each command is run once** (not averaged) — network variance is expected; re-run for stable numbers

## Reproduce

```bash
# Full suite
./benchmarks/run.sh

# Quick smoke test
./benchmarks/run.sh --quick

# With format comparison
./benchmarks/run.sh --formats

# Raw JSON output
./benchmarks/run.sh --json
```
FOOTER

} > "$REPORT_FILE"

echo "" >&2
echo "✅ Report: $REPORT_FILE" >&2
echo "" >&2
