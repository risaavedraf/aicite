# Spec: Breadcrumb Response

## Overview

Enrich retrieval response types with topic/concept breadcrumb metadata so the agent knows the semantic context of each result.

## Requirements

### REQ-1: Breadcrumb field in Citation

**ID**: REQ-BR-1
**Priority**: Must

`Citation` struct MUST include optional breadcrumb fields:

```rust
pub struct Citation {
    // ... existing fields ...
    pub topic_name: Option<String>,
    pub concept_name: Option<String>,
    pub breadcrumb: Option<String>,  // "doc > topic > concept"
}
```

### REQ-2: Breadcrumb field in SearchHit

**ID**: REQ-BR-2
**Priority**: Must

`SearchHit` struct MUST include optional breadcrumb fields:

```rust
pub struct SearchHit {
    // ... existing fields ...
    pub topic_name: Option<String>,
    pub concept_name: Option<String>,
    pub breadcrumb: Option<String>,
}
```

### REQ-3: Breadcrumb field in RetrieveHit

**ID**: REQ-BR-3
**Priority**: Must

`RetrieveHit` struct MUST include optional breadcrumb fields:

```rust
pub struct RetrieveHit {
    // ... existing fields ...
    pub topic_name: Option<String>,
    pub concept_name: Option<String>,
    pub breadcrumb: Option<String>,
}
```

### REQ-4: Breadcrumb format

**ID**: REQ-BR-4
**Priority**: Must

The `breadcrumb` field MUST follow the format:
```
"{display_name} > {topic_name} > {concept_name}"
```

When topic_name is None: `"{display_name}"`
When concept_name is None: `"{display_name} > {topic_name}"`

### REQ-5: Breadcrumb enrichment in engine

**ID**: REQ-BR-5
**Priority**: Must

After ranking by similarity, the engine MUST enrich each result with breadcrumb data from the `HierarchicalChunkEmbedding` metadata.

### REQ-6: Null breadcrumb for flat retrieval

**ID**: REQ-BR-6
**Priority**: Must

When using flat retrieval (`--flat` or no hierarchy data), breadcrumb fields MUST be `null` in the JSON response.

## Scenarios

### S1: Breadcrumb present in hierarchical mode
```
Given: Chunk belongs to topic "Authentication" > concept "JWT Tokens" in "arch.txt"
When: context query returns this chunk
Then: breadcrumb = "arch.txt > Authentication > JWT Tokens"
```

### S2: Breadcrumb null in flat mode
```
Given: --flat flag used
When: context query returns a chunk
Then: breadcrumb = null, topic_name = null, concept_name = null
```

### S3: Breadcrumb with topic only (no concept)
```
Given: Chunk belongs to topic "Auth" but concept_id is NULL
When: context query returns this chunk
Then: breadcrumb = "arch.txt > Auth"
```
