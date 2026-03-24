# domain-scan - structural code intelligence via tree-sitter

**domain-scan** extracts a complete structural census of interfaces, services, methods, types, and their relationships from any codebase. Built in Rust with declarative `.scm` tree-sitter query files. Adding a new language = writing `.scm` files, zero Rust code.

## Architecture

- **Rust core** (`domain-scan-core`) - all analysis logic. Shared IR maps tree-sitter ASTs from any language into language-agnostic types via declarative `.scm` query files.
- **CLI** (`domain-scan-cli`) - `domain-scan scan`, `domain-scan interfaces`, `domain-scan services`, `domain-scan methods`, `domain-scan schemas`, `domain-scan impls`, `domain-scan validate`, `domain-scan match`, `domain-scan prompt`. Includes `--interactive` TUI mode via ratatui.
- **Tauri desktop app** (`domain-scan-tauri`) - Three-panel GUI: Entity Tree | Source Preview | Details Panel. Single-click tree navigation. Build status color indicators. Prompt generation scoped to selected entities.

## Crate Layout

```
crates/
  domain-scan-core/       # Library: parsing, IR, query engine, index, cache
    src/               # Rust source
    queries/           # .scm tree-sitter query files (one dir per language)
    tests/             # Integration tests + fixtures
  domain-scan-cli/        # Binary: clap CLI + ratatui TUI
  domain-scan-tauri/      # Tauri 2 desktop app
    src/               # Rust backend (Tauri IPC commands)
    ui/                # React frontend (entity tree, source preview, details panel)
```

## Coding Standards

- **Strict clippy deny wall**: no `.unwrap()`, no `.expect()`, no `panic!()`, no `println!()`
- All errors via `thiserror`, all propagation via `?`
- Co-locate unit tests with implementation (`#[cfg(test)]`)
- Integration tests in `crates/domain-scan-core/tests/`
- Real tree-sitter parsing in tests (no mocks for tree-sitter)
- Property-based tests with `proptest` where appropriate
- Snapshot tests with `insta` for JSON output

## Key Conventions

- Every `.scm` query file MUST have at least one integration test with a real code fixture
- Test fixtures live in `crates/domain-scan-core/tests/fixtures/<language>/`
- Expected output JSON lives in `crates/domain-scan-core/tests/fixtures/<language>/expected/`
- Thread-local parser pools for rayon safety (tree-sitter parsers are !Send)
- Content-addressed caching: SHA-256(path + content) as cache key
- All IR types derive Serialize, Deserialize, Debug, Clone, PartialEq

## Effect.ts — First-Class Framework

Effect.ts is a first-class framework with deep extraction via a dedicated `queries/typescript/effect.scm`. Beyond `Schema.Struct` (handled in `schemas.scm`), the Effect query file extracts:

- **Services**: `Context.Tag`, `Effect.Service`, `Context.GenericTag` → `ServiceDef` with `kind: EffectService`
- **Layers**: `Layer.effect`, `Layer.sync`, `Layer.succeed`, `Layer.merge`, pipe composition → `EffectLayerDef` (provides/requires/composition graph)
- **Errors**: `Data.TaggedError`, `Schema.TaggedError` → `EffectErrorChannelDef` (tag + structured fields)
- **Schemas**: `Schema.Class`, `Schema.TaggedStruct`, `Schema.Union` → `SchemaDef` with `kind: EffectSchema`
- **Pipelines**: `pipe`, `flow`, `Effect.gen`, `.pipe()` → `EffectPipelineDef` (combinator steps + dependency tracking)

The CLI exposes these via `domain-scan effect [services|layers|errors|schemas|pipelines|graph]`.

## Module Build Status Model

Every scanned module/crate/package has a `BuildStatus` that determines how its structural data is treated:

- **`Built`**: The module compiles/runs successfully. Source code is the **single source of truth**. Interfaces, services, and methods are pulled directly from tree-sitter parsing. LLM enrichment is not needed for interface extraction, but is still needed for higher-level enrichment (e.g. domain classification, subsystem mapping, intent inference).
- **`Unbuilt`**: The module has never been built or has no artifacts. Source code is a **best guess**. Tree-sitter still extracts what it can, but results are marked `confidence: "low"` and should be enriched by LLM sub-agents.
- **`Error`**: The module fails to build (compiler errors, stale artifacts). Confidence: Medium. Valid syntax parses fine, broken code may be incomplete.
- **`Rebuild`**: The module is being actively rebuilt/refactored. Source code is **unreliable**. Do not pull interfaces from source as authoritative. LLM agents should reconcile old vs new definitions and flag conflicts.

The CLI reflects this: `domain-scan scan` auto-detects build status (checks for lock files, build artifacts, compiler errors). The `--build-status` flag overrides detection. The JSON output includes `build_status` per file and `confidence` per extracted entity.

## CLI Tree Navigation

When the CLI outputs hierarchical data (e.g. `domain-scan interfaces --show-methods`), parent nodes (interfaces/classes) must expand to child nodes (methods/properties) on a **single selection**. No double-click or triple-click required. If using a TUI mode, `Enter` on a parent immediately shows children. If piping to a pager, the hierarchy is pre-expanded.

## Entity-to-Subsystem Matching Workflow

domain-scan's end goal is to map every extracted entity to a known subsystem (as defined in a manifest like octospark-visualizer's `system.json`). The workflow:

1. `domain-scan scan` extracts all structural entities (deterministic, tree-sitter)
2. `domain-scan match --manifest system.json` maps entities to subsystems by file path, import graph, and name
3. Unmatched items are flagged. Drizzle schemas, interfaces, services are all exposed but need to be assigned to a subsystem.
4. `domain-scan match --prompt-unmatched` generates LLM sub-agent prompts to propose where unmatched items belong
5. Human reviews all proposals and accepts/rejects
6. Repeat until unmatched count is zero

**For `Built` modules**: source code is truth. Entities are matched deterministically.
**For everything else**: source code is a guess. LLM agents propose matches. Human reviews.

## Specs

All specs live in `specs/`. The index is `specs/readme.md`.

## Build & Test

```bash
cargo build -p domain-scan-core          # Library
cargo build -p domain-scan-cli           # CLI binary
cargo tauri build -p domain-scan-tauri   # Desktop app
cargo test -p domain-scan-core           # All core tests
cargo test -p domain-scan-core -- --test integration  # Integration only
cargo clippy --all-targets -- -D warnings
```

## Agent-First CLI Design

The CLI is the primary interface for AI agents (not a separate MCP server). Follow these principles from [Rewrite Your CLI for AI Agents](https://justin.poehnelt.com/posts/rewrite-your-cli-for-ai-agents/):

1. **Raw JSON payloads as first-class input.** Support `--json '{...}'` for complex inputs alongside convenience flags. Agents generate JSON trivially; custom flag hierarchies are lossy.
2. **Schema introspection at runtime.** `domain-scan schema <command>` dumps the full input/output schema as machine-readable JSON. The CLI is the docs — agents self-serve without pre-stuffed documentation.
3. **Context window discipline.** Support `--fields` to limit output fields. Use NDJSON pagination (`--page-all`) for large results. Agents pay per token — never dump the full blob when a subset suffices.
4. **Input hardening against hallucinations.** Agents are not trusted operators. Validate all inputs:
   - Reject path traversals (`../../.ssh`)
   - Reject control characters (below ASCII 0x20)
   - Reject embedded query params in resource IDs (`fileId?fields=name`)
   - Reject pre-URL-encoded strings that would double-encode (`%2e%2e`)
5. **`--dry-run` for all mutating operations.** Agents "think out loud" before acting. Validate the request locally without side effects.
6. **`--output json` everywhere.** Machine-readable output is table stakes. Default to JSON when stdout is not a TTY.
7. **Ship agent skill files.** Encode invariants agents can't intuit from `--help` (e.g. "always use `--fields` on list calls", "always `--dry-run` before mutating"). These live as structured Markdown with YAML frontmatter.
8. **Structured errors.** Errors must be JSON with an error code, message, and suggested fix. Agents can't parse "Error: something went wrong".
