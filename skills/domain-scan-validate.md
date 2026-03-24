---
name: domain-scan-validate
version: 1.0.0
description: How to run validation — use --strict in CI, scope with --rules, interpret violation severities, use --fail-on-unmatched with match.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Validating Code Structure

## When to use

Use `domain-scan validate` to run data quality checks on extracted structural entities — naming conventions, completeness, god-object detection, and implementation coverage.

## Key commands

```bash
# Run all validation rules
domain-scan validate --output json

# Run specific rules only
domain-scan validate --rules interfaces-pascal-case,no-god-interfaces --output json

# Strict mode: treat warnings as failures (for CI)
domain-scan validate --strict --output json

# Validate against a subsystem manifest
domain-scan validate --manifest system.json --output json

# Scope validation to specific languages
domain-scan validate --languages typescript,rust --output json
```

## Rules

- Use `--strict` in CI pipelines. Without it, warnings (WARN severity) don't cause a non-zero exit code — only failures (FAIL) do.
- Use `--rules` to scope checks when you only care about specific conventions. Running all 10 rules on a large codebase produces noisy output.
- Check exit codes: `0` = all pass, `1` = at least one FAIL (or WARN with `--strict`).
- Parse the `violations` array in JSON output for programmatic handling. Each violation has `rule`, `severity`, `message`, `entity_name`, `file`, and `line`.

## Available rules

| Rule | Severity | What it checks |
|------|----------|---------------|
| `interfaces-pascal-case` | WARN | Interface names must be PascalCase |
| `methods-naming-convention` | WARN | Methods follow language conventions (camelCase for TS/Java, snake_case for Rust/Python) |
| `no-duplicate-interface-names` | FAIL | No duplicate interface names within a file |
| `no-duplicate-method-names` | FAIL | No duplicate method names within an interface |
| `interfaces-have-methods` | WARN | Interfaces must have at least one method |
| `services-have-methods` | WARN | Services must have at least one method |
| `schema-fields-have-types` | WARN | Schema fields must have type annotations |
| `no-god-interfaces` | WARN | Flags interfaces with >10 methods |
| `no-god-services` | WARN | Flags services with >10 methods |
| `interfaces-have-implementors` | WARN | Flags interfaces with 0 implementors |

## Common mistakes

- Running `validate` without `--strict` in CI → warnings silently pass. Always use `--strict` in automated pipelines.
- Ignoring the `severity` field → treating all violations equally. FAIL violations indicate real structural problems; WARN violations are style suggestions.
- Running all rules on a partially-built codebase → `interfaces-have-implementors` will flag everything in `unbuilt` modules. Scope with `--rules` or `--build-status built`.
- Not using `--output json` → parsing human-readable output is fragile. Always use structured JSON.
