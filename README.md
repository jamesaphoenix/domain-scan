# domain-scan

Structural code intelligence via tree-sitter. Find every interface, service, method, trait, protocol, and type boundary in any codebase. Fast, deterministic, language-agnostic.

## Features

- **12 languages**: TypeScript, Rust, Go, Python, Java, Kotlin, Scala, C#, Swift, C++, PHP, Ruby
- **Declarative**: add a new language by writing `.scm` query files — zero Rust code
- **Parallel**: rayon-powered parsing with thread-local parser pools
- **Incremental**: content-addressed SHA-256 caching skips unchanged files
- **Build-status-aware**: confidence levels adapt to whether modules actually compile
- **Agent-first CLI**: structured JSON errors, `--fields` masks, `--json` input, NDJSON pagination, runtime schema introspection

## Install

```bash
cargo install --path crates/domain-scan-cli
```

## Quick Start

```bash
# Scan a project (auto-detects languages, respects .gitignore)
domain-scan scan

# List all interfaces
domain-scan interfaces

# List services with routes
domain-scan services --show-routes

# Search for anything named "Auth"
domain-scan search Auth

# JSON output (auto-detected when piping)
domain-scan interfaces --output json | jq '.[].name'

# Limit output fields (saves tokens for AI agents)
domain-scan interfaces --output json --fields name,methods

# NDJSON for streaming large results
domain-scan interfaces --page-all
```

## Subcommands

| Command | Description |
|---------|-------------|
| `scan` | Full structural scan of a directory |
| `interfaces` | List interfaces / traits / protocols |
| `services` | List service definitions (Express, NestJS, Spring, FastAPI, etc.) |
| `methods` | List methods (filter by owner, async, visibility) |
| `schemas` | List runtime schema definitions (Zod, Pydantic, Drizzle, etc.) |
| `impls` | List implementations of a trait/interface |
| `search` | Search across all entity names |
| `stats` | Print scan statistics |
| `validate` | Run data quality checks |
| `match` | Match entities to subsystems via a manifest |
| `prompt` | Generate LLM prompts with sub-agent dispatch |
| `cache` | Cache management (stats, clear, prune) |
| `schema` | Dump JSON schema for any subcommand's input/output |

## Examples

### Scan and filter

```bash
# Scan only TypeScript files
domain-scan scan --languages typescript

# Scan with verbose timing
domain-scan scan --verbose

# Override build status for all files
domain-scan scan --build-status built
```

### Query entities

```bash
# Find interfaces matching a pattern
domain-scan interfaces --name Repository

# List async methods
domain-scan methods --async

# Methods owned by a specific class
domain-scan methods --owner UserService

# Schemas from a specific framework
domain-scan schemas --framework zod
```

### Validation

```bash
# Run all validation rules
domain-scan validate

# Strict mode: exit 1 on warnings
domain-scan validate --strict

# Self-test: validate domain-scan's own codebase
domain-scan validate --self-test

# Run specific rules only
domain-scan validate --rules naming-conventions,no-god-interfaces
```

### Entity-to-subsystem matching

```bash
# Match entities to subsystems
domain-scan match --manifest system.json

# Preview what --write-back would do
domain-scan match --manifest system.json --write-back --dry-run

# Fail CI if unmatched items remain
domain-scan match --manifest system.json --fail-on-unmatched
```

### LLM prompt generation

```bash
# Generate a prompt for 5 agents
domain-scan prompt --agents 5

# Focus on auth-related entities
domain-scan prompt --focus "auth" --agents 3

# Include full scan results in the prompt
domain-scan prompt --include-scan
```

### Agent-friendly features

```bash
# Runtime schema introspection
domain-scan schema interfaces
domain-scan schema --all

# Raw JSON input (replaces individual flags)
domain-scan interfaces --json '{"name": "Repo", "show_methods": true}'

# Field masks for context window discipline
domain-scan scan --output json --fields files.path,files.language,stats

# Auto-JSON when piped (non-TTY detection)
domain-scan interfaces | jq .
```

### Cache management

```bash
# Check cache stats
domain-scan cache stats

# Preview what clear would delete
domain-scan cache clear --dry-run

# Actually clear
domain-scan cache clear

# Prune entries for deleted files
domain-scan cache prune
```

### TUI mode

```bash
# Interactive tree navigation
domain-scan interfaces --interactive

# Keyboard: j/k navigate, Enter expand/collapse, / search, q quit
```

## Configuration

Create `.domain-scan.toml` in your project root:

```toml
[project]
name = "my-project"

[scan]
include = ["src/**"]
exclude = ["**/generated/**", "**/vendor/**"]

[cache]
max_size_mb = 100
```

## Build Status Model

| Status | Source Code Is... | Confidence |
|--------|-------------------|------------|
| `Built` | Source of truth | High |
| `Unbuilt` | Best guess | Low |
| `Error` | Partial truth | Medium |
| `Rebuild` | Unreliable | Low |

## Architecture

```
crates/
  domain-scan-core/    # Library: parsing, IR, query engine, index, cache
    queries/           # .scm tree-sitter query files (one dir per language)
  domain-scan-cli/     # Binary: clap CLI + ratatui TUI
  domain-scan-tauri/   # Tauri 2 desktop app
```

## Performance

| Metric | Measured |
|--------|----------|
| Parse throughput (sequential) | ~8,000 files/sec |
| Parse throughput (parallel) | ~53,000 files/sec |
| Cached re-scan | ~95,000 files/sec |
| Full pipeline (66 files) | ~10ms |

Run benchmarks: `cargo bench -p domain-scan-core`

## Development

```bash
cargo build -p domain-scan-core          # Library
cargo build -p domain-scan-cli           # CLI binary
cargo test -p domain-scan-core           # All core tests
cargo test -p domain-scan-cli            # CLI integration tests
cargo clippy --all-targets -- -D warnings
cargo bench -p domain-scan-core          # Benchmarks
```

## License

MIT
