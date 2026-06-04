# Spec: Compact/Full Response Mode

## Overview

Add compact/full mode to CLI JSON output. Default compact returns only essential fields (~200-250 tokens). `--full` returns the complete contract (~645-1500 tokens).

## Requirements

### REQ-1: --full flag on all retrieval commands

**ID**: REQ-CF-1
**Priority**: Must

All retrieval commands (`context`, `search`, `retrieve`) MUST accept a `--full` flag.

```bash
cite context "query" --json           # compact (default)
cite context "query" --json --full    # full contract
```

When `--json` is NOT used (human-readable output), `--full` has no effect.

### REQ-2: Compact ContextResponse

**ID**: REQ-CF-2
**Priority**: Must

Compact context output MUST contain only:
```json
{
  "result_kind": "context",
  "citations": [
    {
      "id": "c1",
      "source": "arch.txt",
      "snippet": "JWT tokens with 15-min expiry",
      "score": 0.95
    }
  ],
  "trace_id": "trace_xxx"
}
```

Field mapping from full:
| Compact field | Full field | Transform |
|---------------|------------|-----------|
| `result_kind` | `result_kind` | direct |
| `citations[].id` | `citations[].citation_id` | rename |
| `citations[].source` | `citations[].display_name` | rename |
| `citations[].snippet` | `citations[].text` | truncate to 200 chars max |
| `citations[].score` | `citations[].score` | direct |
| `trace_id` | `trace_id` | direct |

### REQ-3: Compact SearchOutput

**ID**: REQ-CF-3
**Priority**: Must

Compact search output:
```json
{
  "results": [
    {
      "id": "chunk_xxx",
      "source": "arch.txt",
      "score": 0.95,
      "preview": "JWT tokens with 15-min..."
    }
  ]
}
```

### REQ-4: Compact RetrieveOutput

**ID**: REQ-CF-4
**Priority**: Must

Compact retrieve output:
```json
{
  "results": [
    {
      "id": "chunk_xxx",
      "source": "arch.txt",
      "score": 0.95,
      "text": "JWT tokens with 15-min expiry"
    }
  ]
}
```

### REQ-5: Breadcrumb included in full mode

**ID**: REQ-CF-5
**Priority**: Must

When `--full` is used, search and retrieve output MUST include `topic_name`, `concept_name`, `breadcrumb` fields (currently discarded by CLI output structs).

### REQ-6: Default behavior

**ID**: REQ-CF-6
**Priority**: Must

- `--json` without `--full` → compact output
- `--json --full` → full output
- No `--json` flag → human-readable output (unchanged, `--full` ignored)

## Scenarios

### S1: Compact context (default)
```
Given: agent calls cite context "query" --json
Then: returns compact JSON with result_kind, citations (id/source/snippet/score), trace_id
```

### S2: Full context
```
Given: agent calls cite context "query" --json --full
Then: returns full ContextResponse with all metadata
```

### S3: Compact search
```
Given: agent calls cite search "query" --json
Then: returns compact JSON with results (id/source/score/preview)
```

### S4: Breadcrumb in full search
```
Given: hierarchy data exists
When: cite search "query" --json --full
Then: results include topic_name, concept_name, breadcrumb
```
