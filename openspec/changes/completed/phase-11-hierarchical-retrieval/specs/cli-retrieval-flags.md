# Spec: CLI Retrieval Flags

## Overview

Add CLI flags to `cite context`, `cite search`, and `cite retrieve` for hierarchy control and scoped queries.

## Requirements

### REQ-1: --flat flag

**ID**: REQ-CLF-1
**Priority**: Must

All retrieval commands (`context`, `search`, `retrieve`) MUST accept a `--flat` flag that forces flat retrieval (v0.1.0 behavior).

```bash
cite context "query" --flat
cite search "query" --flat
cite retrieve "query" --flat
```

When `--flat` is used:
- `use_hierarchy` is set to `false`
- Response format matches v0.1.0 exactly (no breadcrumb fields)

### REQ-2: --topic flag

**ID**: REQ-CLF-2
**Priority**: Must

All retrieval commands MUST accept a `--topic` flag that filters results to a specific topic by name or ID.

```bash
cite context "query" --topic "Authentication"
cite search "query" --topic "Authentication"
cite retrieve "query" --topic "topic_auth_001"
```

**Behavior**:
- If the value matches a topic_id exactly, use that topic
- Otherwise, search topics by name (case-insensitive partial match)
- If multiple topics match, return an error listing the matches
- If no topic matches, return an error

### REQ-3: --concept flag

**ID**: REQ-CLF-3
**Priority**: Must

All retrieval commands MUST accept a `--concept` flag that filters results to a specific concept by name or ID.

```bash
cite context "query" --concept "JWT Tokens"
cite search "query" --concept "concept_jwt_001"
```

**Behavior**: Same matching logic as `--topic`.

### REQ-4: --topic and --concept mutual exclusivity

**ID**: REQ-CLF-4
**Priority**: Must

`--topic` and `--concept` MUST NOT be used together. If both are provided, return an error:
```
Error: --topic and --concept cannot be used together. Use --concept to scope within a topic.
```

### REQ-5: --flat and --topic/--concept mutual exclusivity

**ID**: REQ-CLF-5
**Priority**: Must

`--flat` MUST NOT be used with `--topic` or `--concept`. If combined, return an error:
```
Error: --flat cannot be combined with --topic or --concept.
```

### REQ-6: JSON output includes breadcrumb

**ID**: REQ-CLF-6
**Priority**: Must

When using `--json` output with hierarchical retrieval, the JSON MUST include breadcrumb fields:
```json
{
  "results": [
    {
      "chunk_id": "...",
      "text": "JWT tokens with 15-min expiry",
      "score": 0.95,
      "topic_name": "Authentication",
      "concept_name": "JWT Tokens",
      "breadcrumb": "arch.txt > Authentication > JWT Tokens"
    }
  ]
}
```

## Scenarios

### S1: --flat flag
```
Given: DB has hierarchy data
When: cite context "query" --flat --json
Then: response uses flat retrieval, no breadcrumb fields in JSON
```

### S2: --topic filter
```
Given: DB has topic "Authentication" with 5 chunks
When: cite context "JWT expiry" --topic "Authentication" --json
Then: returns only chunks from Authentication topic, ranked by similarity
```

### S3: --topic not found
```
Given: DB has no topic matching "Nonexistent"
When: cite context "query" --topic "Nonexistent"
Then: error: "No topic found matching 'Nonexistent'"
```

### S4: --topic and --concept conflict
```
When: cite context "query" --topic "Auth" --concept "JWT"
Then: error: "--topic and --concept cannot be used together"
```

### S5: --flat and --topic conflict
```
When: cite context "query" --flat --topic "Auth"
Then: error: "--flat cannot be combined with --topic or --concept"
```
