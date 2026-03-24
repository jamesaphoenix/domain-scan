---
name: domain-scan-cli
version: 1.0.0
description: Agent skill for working with the domain-scan CLI. Encodes invariants agents can't intuit from --help.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# domain-scan CLI — Agent Skill

## Always

- **Always use `--output json`** for machine-readable output. Never parse table/human output.
- **Always use `--fields`** on list commands (`interfaces`, `services`, `methods`, `schemas`) to limit output fields and protect your context window.
- **Always use `--dry-run`** before any mutating operation (`match --apply`, `validate --fix`).
- **Always confirm with the user** before executing write/delete commands.
- **Always use `domain-scan schema <command>`** to introspect what a command accepts at runtime, rather than guessing flags.

## Never

- Never pass unvalidated file paths — the CLI rejects path traversals, but don't rely on it.
- Never dump the full scan output into context — use `--fields` or scope with `--focus`.
- Never assume flag names — use `domain-scan schema <command>` to discover them.

## Common Patterns

### Scan a repo and get a structural summary
```bash
domain-scan scan --output json | jq '.stats'
```

### List interfaces with limited fields
```bash
domain-scan interfaces --output json --fields "name,file,methods_count,build_status"
```

### Search for entities by name
```bash
domain-scan search "Handler" --output json --fields "name,kind,file"
```

### Check what a command accepts
```bash
domain-scan schema interfaces
```

### Validate before acting
```bash
domain-scan validate --dry-run --output json
```

### Focus on a subsystem
```bash
domain-scan interfaces --focus "auth" --output json --fields "name,methods_count"
```

### Generate LLM sub-agent prompts
```bash
domain-scan prompt --agents 3 --focus "auth" --output json
```

## JSON Input

For commands that accept complex input, prefer `--json` over individual flags:
```bash
domain-scan match --json '{"manifest": "system.json", "filters": {"language": "rust", "build_status": "built"}}'
```

## Error Handling

All errors are structured JSON with `code`, `message`, and `suggestion` fields:
```json
{"code": "NO_SCAN", "message": "No scan loaded", "suggestion": "Run domain-scan scan first"}
```

Parse `code` for programmatic handling, not `message`.
