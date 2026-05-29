# Spec: Evaluation Improvements

## Overview

Consolidate duplicate eval providers and add hierarchical evaluation fixtures.

## Requirements

### REQ-1: Consolidate eval providers

**ID**: REQ-EI-1
**Priority**: Should

`EvalProvider` (CLI) and `GoldenProvider` (engine tests) implement the same 8-dim topic-based algorithm. They SHOULD be consolidated into a single shared implementation.

**Location**: Move to `crates/providers/src/eval.rs` as a shared `EvalProvider`, re-export for both CLI and engine tests.

### REQ-2: Hierarchical fixtures

**ID**: REQ-EI-2
**Priority**: Must

At least 2 new evaluation fixtures MUST test hierarchical retrieval:

1. **hier-001**: Query that matches better with small hierarchical chunks (should score higher than flat)
2. **hier-002**: Query scoped to a specific topic via breadcrumb

### REQ-3: Fixture count

**ID**: REQ-EI-3
**Priority**: Must

Total fixtures after Phase 12: 10 (8 existing + 2 hierarchical).

### REQ-4: Existing fixtures unchanged

**ID**: REQ-EI-4
**Priority**: Must

All 8 existing fixtures MUST continue to pass with identical behavior.

## Scenarios

### S1: Hierarchical fixture passes
```
Given: evaluation corpus with hierarchy data
When: run evaluation
Then: hier-001 and hier-002 pass with expected result_kind and breadcrumb
```

### S2: Existing fixtures unaffected
```
When: run evaluation
Then: all 8 original fixtures pass (df-001 through pi-001)
```
