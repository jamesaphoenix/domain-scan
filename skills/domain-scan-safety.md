---
name: domain-scan-safety
version: 1.0.0
description: Input safety rules — validate paths, always --dry-run before mutating, never pipe raw scan output without --fields.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Input Safety

## When to use

Follow these rules whenever interacting with domain-scan, especially when constructing commands from user input or dynamically generated parameters. The CLI includes input hardening, but defense in depth requires safe usage patterns.

## Key commands

```bash
# Always dry-run before mutating
domain-scan match --manifest system.json --write-back --dry-run
domain-scan cache clear --dry-run
domain-scan cache prune --dry-run

# Always limit output fields to protect context window
domain-scan scan --output json --fields stats
domain-scan interfaces --output json --fields name,file

# Use schema introspection to discover valid inputs
domain-scan schema interfaces
domain-scan schema match
```

## Rules

- **Never pass user-supplied paths without validation.** The CLI rejects path traversals (`../../.ssh`), control characters (below ASCII 0x20), embedded query params (`fileId?fields=name`), and pre-URL-encoded strings (`%2e%2e`). But don't rely solely on CLI-side validation — sanitize inputs before passing them.
- **Always use `--dry-run` before any mutating operation.** This includes `match --write-back`, `cache clear`, and `cache prune`. Review the structured JSON dry-run output before proceeding.
- **Never pipe raw scan output into LLM prompts without `--fields`.** Full scan output can be megabytes. Use `--fields` to select only the fields the LLM needs. For entity lists, use `--fields name,file,kind` at minimum.
- **Never trust `--json` input from untrusted sources without size/depth checks.** The CLI enforces 1 MB max and 32 levels max nesting, but validate on your side too.
- **Always use `--output json` for programmatic consumption.** Table and compact formats are for human reading only — they are not stable across versions.
- **Check exit codes.** `0` = success, `1` = error/failure. Never assume success without checking.

## Input hardening (built into CLI)

The CLI automatically rejects:

| Pattern | Example | Reason |
|---------|---------|--------|
| Path traversal | `../../.ssh/id_rsa` | Directory escape |
| Control characters | `\x00`, `\x1f` | Injection vectors |
| Embedded query params | `file.ts?fields=name` | Parameter confusion |
| Pre-URL-encoded strings | `%2e%2e%2f` | Double-encoding attacks |
| Oversized JSON input | >1 MB `--json` payload | Resource exhaustion |
| Deeply nested JSON | >32 levels in `--json` | Stack overflow |

## Common mistakes

- Constructing file paths from user input without sanitization → even though the CLI validates, path traversal attempts waste time and produce error output.
- Piping full `domain-scan scan` output into an LLM context → context window overflow. Always use `--fields` to limit output size.
- Skipping `--dry-run` on mutating commands → unintended manifest writes or cache deletions. The cost of a dry-run is near zero; the cost of an unintended mutation is high.
- Assuming `--json` input is safe because it's JSON → JSON can contain control characters, deeply nested structures, and oversized payloads. The CLI validates, but sanitize upstream too.
- Parsing error messages as strings → errors are structured JSON with `code`, `message`, and `suggestion` fields. Parse the `code` field for programmatic handling.
