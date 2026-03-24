# domain-scan — Agent Instructions

This CLI is frequently invoked by AI/LLM agents. Always assume inputs can be adversarial.

## Using the CLI

- **Always use `--output json`** — never parse human-formatted table output.
- **Always use `--fields`** on list commands to limit response size and protect your context window.
- **Always use `--dry-run`** before mutating operations.
- **Always use `domain-scan schema <command>`** to discover what a command accepts at runtime — do not guess flags.
- **Always confirm with the user** before executing write/delete operations.

## Building the CLI

When modifying `domain-scan-cli` or `domain-scan-core`, follow these agent-first design principles:

1. **Raw JSON payloads as first-class input.** All commands must accept `--json '{...}'` for complex inputs alongside convenience flags.
2. **Schema introspection at runtime.** `domain-scan schema <command>` must dump the full input/output schema as machine-readable JSON.
3. **Context window discipline.** Support `--fields` to limit output. Use NDJSON pagination for large results. Default to JSON when stdout is not a TTY.
4. **Input hardening.** Reject path traversals, control characters (below ASCII 0x20), embedded query params in IDs, and pre-URL-encoded strings.
5. **`--dry-run` for all mutating operations.** Validate locally without side effects.
6. **Structured errors.** All errors must be JSON with `code`, `message`, and `suggestion` fields.
7. **No `panic!()`, `unwrap()`, or `expect()`.** All errors via `thiserror` + `?`.

## Project Structure

- `crates/domain-scan-core/` — library: parsing, IR, query engine, index, cache
- `crates/domain-scan-cli/` — binary: clap CLI + ratatui TUI
- `crates/domain-scan-tauri/` — Tauri 2 desktop app

## Testing

```bash
cargo test -p domain-scan-core
cargo clippy --all-targets -- -D warnings
```
