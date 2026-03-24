---
name: domain-scan-query
version: 1.0.0
description: How to query structural entities — interfaces, services, methods, schemas, impls, search. Always use --fields and --page-all for large result sets.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Querying Entities

## When to use

Use the entity query commands (`interfaces`, `services`, `methods`, `schemas`, `impls`, `search`) to find and filter structural entities after a scan. These are your primary tools for understanding a codebase's architecture.

## Key commands

```bash
# List all interfaces with limited fields
domain-scan interfaces --output json --fields name,file,methods,extends

# Filter interfaces by name pattern
domain-scan interfaces --name "Repository" --output json --fields name,methods

# List services filtered by kind
domain-scan services --kind http_controller --output json --fields name,routes,file

# Show service routes and dependencies
domain-scan services --show-routes --show-deps --output json

# Find async methods owned by a specific class
domain-scan methods --owner UserService --async --output json --fields name,parameters,return_type

# List schemas by framework
domain-scan schemas --framework zod --output json --fields name,fields,file

# Show schema fields inline
domain-scan schemas --show-fields --output json --fields name,fields

# Find all implementations of an interface
domain-scan impls EventHandler --output json --fields target,methods,file

# List all implementations across codebase
domain-scan impls --all --output json --fields target,trait_name,file

# Search across all entity types
domain-scan search "Handler" --output json --fields name,kind,file

# Search filtered by entity kind
domain-scan search "User" --kind interface,service --output json

# Stream large result sets as NDJSON
domain-scan interfaces --page-all --fields name,file

# Combine field masks with NDJSON
domain-scan methods --page-all --fields name,owner,is_async,return_type
```

## Rules

- Always use `--fields` to limit output fields. You pay per token — never request fields you won't use.
- Use `--page-all` for result sets larger than ~100 entities. This emits NDJSON (one JSON object per line) which you can stream-process without buffering.
- Use `domain-scan schema <command>` to discover available filter flags and output fields before constructing queries.
- Prefer specific query commands over `search` when you know the entity kind. `domain-scan interfaces --name Repo` is faster and more precise than `domain-scan search Repo --kind interface`.
- Use `--json` for complex filter combinations instead of multiple flags: `domain-scan interfaces --json '{"name": ".*Repo", "show_methods": true}'`.
- `--fields` only applies with `--output json`. It is ignored for `table` and `compact` formats.

## Common mistakes

- Not using `--fields` → receiving every field on every entity, blowing context window. Always specify the fields you need.
- Using `search` when a specific command exists → `search` returns EntitySummary (less detail). Use `interfaces`/`services`/`methods`/`schemas` for full entity definitions.
- Mixing `--json` with individual filter flags → these are mutually exclusive. Use one or the other.
- Forgetting `--show-methods` on `interfaces`/`impls` → methods are not included by default in table/compact output. In JSON output, methods are always present — use `--fields` to exclude them if unwanted.
- Requesting `--page-all` with `--output table` → NDJSON only works with JSON output.
