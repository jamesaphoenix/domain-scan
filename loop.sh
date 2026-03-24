#!/usr/bin/env bash
set -euo pipefail

# Minimal Claude loop harness with live streaming of tool calls, thinking, and messages
# Usage: ./loop.sh [prompt-file] [cooldown-seconds]

PROMPT_FILE="${1:-prompt.md}"
COOLDOWN="${2:-5}"
LOG_DIR="/tmp/claude-loop-logs"

mkdir -p "$LOG_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
MAGENTA='\033[1;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

if ! command -v jq &>/dev/null; then
  echo -e "${RED}Error: jq is required (brew install jq)${NC}" >&2
  exit 1
fi

if [[ ! -f "$PROMPT_FILE" ]]; then
  echo -e "${RED}Error: ${PROMPT_FILE} not found${NC}" >&2
  exit 1
fi

# Parse a single stream-json line and print a human-readable version
render_event() {
  local line="$1"
  local type subtype

  type=$(echo "$line" | jq -r '.type // empty' 2>/dev/null) || return 0
  subtype=$(echo "$line" | jq -r '.subtype // empty' 2>/dev/null) || true

  case "$type" in
    system)
      if [[ "$subtype" == "init" ]]; then
        local model session
        model=$(echo "$line" | jq -r '.model // "unknown"')
        session=$(echo "$line" | jq -r '.session_id // "unknown"')
        echo -e "${DIM}  model: ${model}  session: ${session}${NC}"
      elif [[ "$subtype" == "compact_boundary" ]]; then
        echo -e "${DIM}  ── context compacted ──${NC}"
      fi
      ;;

    stream_event)
      # Partial streaming events (with --include-partial-messages)
      local delta_type delta_text
      delta_type=$(echo "$line" | jq -r '.event.delta.type // empty' 2>/dev/null) || true
      case "$delta_type" in
        text_delta)
          delta_text=$(echo "$line" | jq -rj '.event.delta.text // empty' 2>/dev/null) || true
          if [[ -n "$delta_text" ]]; then
            printf '%s' "$delta_text"
          fi
          ;;
        thinking_delta)
          delta_text=$(echo "$line" | jq -rj '.event.delta.thinking // empty' 2>/dev/null) || true
          if [[ -n "$delta_text" ]]; then
            printf "${DIM}%s${NC}" "$delta_text"
          fi
          ;;
      esac
      ;;

    assistant)
      # Complete assistant message — only extract thinking + tool calls.
      # Skip .type=="text" here because it was already streamed via stream_event deltas.
      local output
      output=$(echo "$line" | jq -r '
        .message.content[]? |
        if .type == "thinking" then
          "  \u001b[2m💭 Thinking: \(.thinking | if length > 200 then .[:200] + "..." else . end)\u001b[0m"
        elif .type == "tool_use" then
          if .name == "Skill" then
            "  \u001b[1;35m⚡ Skill: \(.input.skill // "unknown")\u001b[0m" +
            if .input.args then " args: \(.input.args)" else "" end
          elif .name == "Agent" then
            "  \u001b[1;36m🤖 Agent: \(.input.description // "unknown")\u001b[0m" +
            if .input.subagent_type then " [\(.input.subagent_type)]" else "" end
          elif .name == "Bash" then
            "  \u001b[1;33m► Bash\u001b[0m \(.input.command // "" | if length > 150 then .[:150] + "..." else . end)"
          elif .name == "Read" then
            "  \u001b[1;33m► Read\u001b[0m \(.input.file_path // "")"
          elif .name == "Write" then
            "  \u001b[1;33m► Write\u001b[0m \(.input.file_path // "")"
          elif .name == "Edit" then
            "  \u001b[1;33m► Edit\u001b[0m \(.input.file_path // "")"
          elif .name == "Glob" then
            "  \u001b[1;33m► Glob\u001b[0m \(.input.pattern // "")"
          elif .name == "Grep" then
            "  \u001b[1;33m► Grep\u001b[0m \(.input.pattern // "")"
          elif .name == "WebSearch" then
            "  \u001b[1;33m► WebSearch\u001b[0m \(.input.query // "")"
          elif .name == "WebFetch" then
            "  \u001b[1;33m► WebFetch\u001b[0m \(.input.url // "")"
          else
            "  \u001b[1;33m► \(.name)\u001b[0m \(.input | tostring | if length > 150 then .[:150] + "..." else . end)"
          end
        else empty
        end
      ' 2>/dev/null) || true
      if [[ -n "$output" ]]; then
        echo -e "$output"
      fi
      ;;

    user)
      # Tool results come back as user messages with tool_result content
      local tool_results
      tool_results=$(echo "$line" | jq -r '
        .message.content[]? |
        select(.type == "tool_result") |
        if .is_error == true then
          "  \u001b[0;31m✗ \(.tool_use_id // "tool")\u001b[0m"
        else
          "  \u001b[0;32m✓ tool result\u001b[0m"
        end
      ' 2>/dev/null) || true
      if [[ -n "$tool_results" ]]; then
        echo -e "$tool_results"
      fi
      ;;

    tool_result)
      # Standalone tool_result events (verbose mode)
      local tool_name is_error content_preview
      tool_name=$(echo "$line" | jq -r '.tool_name // "tool"' 2>/dev/null) || true
      is_error=$(echo "$line" | jq -r '.is_error // false' 2>/dev/null) || true
      content_preview=$(echo "$line" | jq -r '
        if .content | type == "string" then
          .content | if length > 300 then .[:300] + "..." else . end
        elif .content | type == "array" then
          [.content[]? | .text? // empty] | join("") | if length > 300 then .[:300] + "..." else . end
        else
          ""
        end
      ' 2>/dev/null) || true

      if [[ "$is_error" == "true" ]]; then
        echo -e "  ${RED}✗ ${tool_name}${NC}"
      else
        echo -e "  ${GREEN}✓ ${tool_name}${NC}"
      fi
      if [[ -n "$content_preview" && "$content_preview" != "null" ]]; then
        echo -e "${DIM}$(echo "$content_preview" | head -5)${NC}"
      fi
      ;;

    rate_limit_event)
      local status resets_at
      status=$(echo "$line" | jq -r '.rate_limit_info.status // "unknown"' 2>/dev/null) || true
      if [[ "$status" != "allowed" ]]; then
        resets_at=$(echo "$line" | jq -r '.rate_limit_info.resetsAt // ""' 2>/dev/null) || true
        echo -e "  ${YELLOW}⏳ Rate limited (resets: ${resets_at})${NC}"
      fi
      ;;

    result)
      local cost duration_ms stop turns
      cost=$(echo "$line" | jq -r '.total_cost_usd // 0' 2>/dev/null) || true
      duration_ms=$(echo "$line" | jq -r '.duration_ms // 0' 2>/dev/null) || true
      stop=$(echo "$line" | jq -r '.stop_reason // "unknown"' 2>/dev/null) || true
      turns=$(echo "$line" | jq -r '.num_turns // 0' 2>/dev/null) || true
      echo ""
      echo -e "${DIM}  ── cost: \$${cost}  duration: ${duration_ms}ms  turns: ${turns}  stop: ${stop} ──${NC}"
      ;;
  esac
}

iteration=0

while true; do
  iteration=$((iteration + 1))
  start_ts=$(date +%s)
  log_file="${LOG_DIR}/run-${iteration}.log"

  echo ""
  echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BOLD}${GREEN}  Loop #${iteration}  $(date '+%Y-%m-%d %H:%M:%S')${NC}"
  echo -e "${DIM}  Prompt: ${PROMPT_FILE}  Log: ${log_file}${NC}"
  echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  PROMPT="$(cat "$PROMPT_FILE")"

  set +e
  claude \
    --print \
    --verbose \
    --output-format stream-json \
    --include-partial-messages \
    --dangerously-skip-permissions \
    -p "$PROMPT" \
    2>&1 | tee "$log_file" | while IFS= read -r line; do
      render_event "$line"
    done
  exit_code=${PIPESTATUS[0]}
  set -e

  end_ts=$(date +%s)
  duration=$((end_ts - start_ts))

  echo ""
  if [[ $exit_code -eq 0 ]]; then
    echo -e "${GREEN}  Done: loop #${iteration} in ${duration}s${NC}"
  else
    echo -e "${RED}  Failed: loop #${iteration} exit=${exit_code} after ${duration}s${NC}"
  fi

  echo -e "${DIM}  Cooldown ${COOLDOWN}s... (Ctrl+C to stop)${NC}"
  sleep "$COOLDOWN"
done
