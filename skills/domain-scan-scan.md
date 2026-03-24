---
name: domain-scan-scan
version: 1.0.0
description: How to scan a codebase with domain-scan — scope by language, use --fields on large codebases, always use --output json.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Scanning a Codebase

## When to use

Use `domain-scan scan` to get a full structural census of a codebase — interfaces, services, classes, methods, schemas, imports, exports, and implementations extracted per file via tree-sitter.

## Key commands

```bash
# Basic scan (auto-detects JSON output when piped)
domain-scan scan --root /path/to/project

# Scope to specific languages
domain-scan scan --root /path/to/project --languages typescript,rust

# Get only stats (much smaller output)
domain-scan scan --output json --fields stats

# Get file paths and languages only
domain-scan scan --output json --fields files.path,files.language,stats

# Override build status detection
domain-scan scan --build-status built

# Skip cache for fresh results
domain-scan scan --no-cache

# Write output to file
domain-scan scan --output json -o scan-results.json
```

## Rules

- Always use `--output json` (or pipe stdout so auto-detection kicks in). Never parse table output programmatically.
- Always use `--fields` on codebases with more than ~50 files. The full scan output includes every entity in every file — this will blow your context window.
- Use `--fields stats` first to understand the codebase size before requesting full entity data.
- Use `--languages` to limit scope when you only care about specific languages. Supported: `typescript`, `python`, `rust`, `go`, `java`, `kotlin`, `csharp`, `swift`, `php`, `ruby`, `scala`, `cpp`.
- Use `--no-cache` when debugging stale results or after modifying `.scm` query files.
- Check `build_status` and `confidence` fields per file — `unbuilt` and `rebuild` files have lower confidence extractions.

## Common mistakes

- Dumping full scan output into context without `--fields` → context window overflow. Use `--fields stats` or `--fields files.path,files.language` first.
- Ignoring `build_status` field → treating low-confidence extractions as authoritative. Always check `confidence` before acting on entity data from `unbuilt`/`rebuild` files.
- Scanning the entire monorepo when you only need one module → use `--root` to scope to the relevant subdirectory.
- Running scan repeatedly without cache → use the default cache. Only add `--no-cache` for debugging.
