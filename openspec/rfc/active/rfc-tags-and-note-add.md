# RFC: Tags, Note Add, and Retrieval Quality

**Status:** Draft
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Related:** EVALUACION_CITE.md, CITE_Pi_Integration.md, rfc-auto-docs-sync.md

---

## Problem

### 1. No metadata on chunks

Chunks have no way to carry semantic metadata beyond basic document structure. We can't mark something as `status:implemented` vs `status:planned`, tag by category, or scope by workspace. This makes `check-docs` report false positives for planned features and prevents filtering by semantic categories.

### 2. No agent knowledge capture

Cite is read-only: it ingests physical documents and retrieves from them. Agents can't write back what they learn. Each session starts from zero — the agent has no institutional memory of past work, decisions, or discoveries.

### 3. Retrieval scores are low

Real query scores top out at 0.63-0.76 (never reaching 0.8). The embedding model (`gemini-embedding-001`) is general-purpose, expensive (rate-limited), and not optimized for technical retrieval. The chunking strategy (1000 chars fixed) splits context mid-concept.

```
Query                              Top score   Spread
"document ingestion"                0.7002     0.68-0.70
"how to configure API keys"         0.6361     0.58-0.64
"acceptance criteria for retrieval"  0.7365     0.69-0.74
"what happens when doc fails"       0.7628     0.70-0.76
```

The spread between rank 1 and rank 5 is only 0.05-0.08, meaning the engine barely discriminates between relevant and irrelevant results.

---

## Proposal

### Part 1: Tag System

A flexible key:value tag system for all entities (chunks, notes, documents, topics, concepts).

#### Schema

```sql
CREATE TABLE tags (
    tag_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,        -- chunk_id, document_id
    entity_type TEXT NOT NULL,      -- 'chunk', 'document'
    key TEXT NOT NULL,              -- 'status', 'tag', 'workspace', 'since', 'type'
    value TEXT NOT NULL,            -- 'implemented', 'auth', 'aiharness', 'v0.3.0'
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_id, key, value)
);

CREATE INDEX idx_tags_entity ON tags(entity_id);
CREATE INDEX idx_tags_key ON tags(key);
CREATE INDEX idx_tags_key_value ON tags(key, value);
```

#### Key design decisions

- **Multiple values per key:** A chunk can have `tag:auth`, `tag:security`, `tag:jwt`
- **Query by key only:** `--tag status` returns everything with any status
- **Query by key:value:** `--tag status:implemented` returns only implemented items
- **Compound filters:** `--tag status:implemented --tag tag:auth` (AND logic)
- **Auto-tag on ingest:** `cite ingest openspec/prd/01.md` → auto-tag `type:prd` from path

#### CLI

```bash
# Set tags
cite tag set <entity_id> status:implemented
cite tag set <entity_id> tag:auth tag:security    # multiple in one call
cite tag set <entity_id> since:v0.3.0 workspace:aiharness

# Get tags for entity
cite tag get <entity_id>

# Remove tag
cite tag rm <entity_id> status:planned

# Search with tag filters
cite search "auth" --tag status:implemented
cite search "auth" --tag tag:security
cite search "auth" --tag status:implemented --tag tag:security

# List by tag
cite list --tag type:prd
cite list --tag status:planned

# Bulk tag from ingest path
cite ingest openspec/prd/01.md --auto-tag    # infers type:prd from path
```

#### Auto-tag rules (path-based)

| Path pattern | Auto-tags |
|---|---|
| `openspec/prd/*` | `type:prd`, `workspace:<repo>` |
| `openspec/specs/*` | `type:spec`, `workspace:<repo>` |
| `openspec/architecture/*` | `type:architecture`, `workspace:<repo>` |
| `openspec/guides/*` | `type:guide`, `workspace:<repo>` |
| `openspec/rfc/*` | `type:rfc`, `workspace:<repo>` |

#### Integration with check-docs

Tags solve the doc sync problem:

```bash
# Blocks tagged status:planned are skipped by check-docs
# Blocks tagged status:implemented are verified
# Blocks with no status tag default to "verify" behavior
```

In markdown, authors can tag code blocks:

```markdown
<!-- tag:status=planned -->
```bash
cite context-batch --json
```
```

`check-docs` reads this tag and reports as `planned` instead of `outdated`.

---

### Part 2: Note Add

Agent-facing command to capture knowledge directly into Cite without a physical file.

#### CLI

```bash
# Basic note
cite note add "Fixed the auth bug: JWT expiry was in seconds, not milliseconds"

# With tags (agent assigns whatever key:value pairs make sense)
cite note add "Cosine similarity returns NaN for zero vectors" \
  --tag topic:"Retrieval Pipeline" \
  --tag concept:"Vector Search"

# With status and category tags
cite note add "Rate limit de Gemini es el bottleneck principal" \
  --tag status:decision --tag since:v0.3.1 \
  --tag tag:architecture --tag tag:embedder

# With workspace scope
cite note add "Decided to use nomic-embed-text for local embeddings" \
  --workspace aiharness \
  --tag status:decision

# Notes are searchable like any other content
cite search "auth bug" --tag source_kind:note
cite context "what decisions were made" --tag status:decision
```

#### Storage model

Notes are chunks with `source_kind:note` tag, stored in a virtual document per workspace:

```sql
-- source_kind is a reserved tag on the document (not a column)
-- source_kind:document = ingested from physical file
-- source_kind:note = created via note add

-- Notes get their own virtual document per workspace
-- document_id: 'notes:<workspace>'
-- Tags: source_kind:note, workspace:<name>, plus any agent-assigned tags
```

#### Note document auto-creation

When `--workspace` is specified, Cite creates (or reuses) a virtual document:

```
document_id: notes:aiharness
display_name: "[notes] aiharness"
file_path: null (virtual)
status: ready
```

All notes for that workspace live under this document, organized by agent-assigned tags.

#### Tag handling

- Tags are key:value pairs assigned by the agent (e.g. `tag:jwt`, `status:solved`, `priority:high`)
- No topic/concept hierarchy — tags replace it entirely
- If the agent wants grouping, they use tags: `--tag topic:Authentication`, `--tag area:backend`
- Multiple tags per chunk, multiple values per key allowed
- All tags are searchable via `--tag key:value` filters

#### Session scoping

Notes inherit the current session context:

```bash
# Session-aware: automatically tagged with session metadata
cite note add "Found that FTS5 needs special char escaping" \
  --tag session:2026-06-06-v031
```

---

### Part 3: Retrieval Quality Roadmap

#### Current state (v0.3.0)

| Aspect | Value | Issue |
|--------|-------|-------|
| Embedding model | gemini-embedding-001 | General-purpose, rate-limited, expensive |
| Embedding dims | 3072 | Large, slow |
| Latency | ~1000ms+ per query | API round-trip |
| Top score | 0.63-0.76 | Never reaches 0.8 |
| Discrimination | 0.05-0.08 spread | Can't separate relevant from irrelevant |
| Chunking | 1000 chars fixed | Splits mid-concept |
| Re-ranking | None | Pure cosine similarity |
| Hybrid search | None | No keyword component |

#### v0.3.2/0.3.3: Local Embedder

**Replace Gemini with nomic-embed-text-v1.5:**

| Aspect | Before | After |
|--------|--------|-------|
| Model | gemini-embedding-001 | nomic-embed-text-v1.5 |
| Dims | 3072 | 768 |
| Latency | ~1000ms | 30-70ms CPU |
| Rate limits | Yes (429 errors) | None |
| Quality | General-purpose | Optimized for retrieval |
| Cost | Per-query API call | One-time model load |

**Migration path:**
1. Add `nomic` as a new provider in `providers/`
2. `cite ingest --reembed` to regenerate all vectors
3. Keep Gemini as fallback option
4. Update `check-docs` expected outputs

**Reference:** CITE_Pi_Integration.md — Nomic recommended as balance of quality/resources.

#### v0.4.0: Semantic Chunking + Re-ranking

**Semantic chunking:**
- Respect heading boundaries (H2/H3 as split points)
- Don't split mid-sentence (sentence-boundary detection)
- Variable chunk sizes: 300-800 chars based on content structure
- Metadata-aware: preserve code blocks as single chunks

**Re-ranking (two-stage retrieval):**
```
Stage 1: cosine similarity → top 20 candidates (fast)
Stage 2: cross-encoder re-rank → final top 5 (precise)
```

Cross-encoder options:
- `ms-marco-MiniLM-L-6-v2`: 33M params, ~50ms CPU, good quality
- `bge-reranker-v2-m3`: 568M params, better quality, slower

Expected improvement: +15-25% in ranking precision.

#### v0.5.0: Hybrid Search

Combine vector search with keyword search (SQLite FTS5):

```sql
-- FTS5 index on chunk text
CREATE VIRTUAL TABLE chunks_fts USING fts5(text, content=chunks, content_rowid=rowid);
```

```rust
// Hybrid scoring
let vector_score = cosine_similarity(&query_vec, &chunk_vec);
let keyword_score = fts5_bm25_rank(query_terms, chunk_text);
let final_score = vector_score * 0.7 + keyword_score * 0.3;
```

**Why hybrid:**
- Technical queries often use exact terms (`JWT`, `cosine_similarity`, `FTS5`)
- Vector search is good for semantic similarity but misses exact matches
- Keyword search catches exact terms but misses synonyms
- Combined: best of both worlds

---

## Folder Structure vs Tags

### Current state

```
openspec/
  prd/           ← 17 docs, manually organized
  specs/         ← 2 docs
  guides/        ← 3 docs
  architecture/  ← 4 docs
  rfc/active/    ← 2 docs
```

### Proposed: Hybrid approach

**Folders** stay for human organization in the repo (VS Code, GitHub).

**Tags** replace folder-based classification in Cite:

```bash
# Folder determines auto-tags on ingest
cite ingest openspec/prd/01.md --auto-tag
# → type:prd, workspace:aiharness

# But tags are the search axis
cite context "requirements" --tag type:prd
cite context "architecture" --tag type:architecture
cite context "what's planned" --tag status:planned

# Tags can cross folder boundaries
cite search "auth" --tag type:prd --tag type:spec  # search across both
```

**The rule:** Folders are for humans. Tags are for Cite. Don't require Cite to understand folder structure — it understands tags.

---

## Implementation Plan

### Phase 1: Tags (v0.3.2)

- [ ] `tags` table + migration
- [ ] CLI: `tag set`, `tag get`, `tag rm`
- [ ] Integration: `--tag` filter on `search`, `retrieve`, `context`, `list`
- [ ] Auto-tag on `ingest` from path patterns
- [ ] `check-docs` reads `<!-- tag:status=planned -->` from markdown

### Phase 2: Note Add (v0.3.3)

- [ ] `source_type` column on chunks
- [ ] `note add` command with tags
- [ ] Virtual documents per workspace
- [ ] Auto-create document on first note per workspace
- [ ] `--source-type` filter on search/retrieve/context

### Phase 3: Local Embedder (v0.3.3)

- [ ] nomic-embed-text provider
- [ ] `ingest --reembed` migration command
- [ ] Benchmark: old vs new model on golden dataset
- [ ] Update `check-docs` expected outputs

### Phase 4: Semantic Chunking (v0.4.0)

- [ ] Heading-boundary-aware chunker
- [ ] Sentence-boundary detection
- [ ] Variable chunk sizes (300-800 chars)
- [ ] Re-ranking stage (cross-encoder)

### Phase 5: Hybrid Search (v0.5.0)

- [ ] FTS5 index on chunk text
- [ ] Hybrid scoring (vector * 0.7 + keyword * 0.3)
- [ ] Benchmark: pure vector vs hybrid

---

## Open Questions

1. **Tag value types:** Should tags support non-string values? (`priority:1` vs `priority:high`)
2. **Tag namespaces:** Should `workspace:` be a reserved key or just a convention?
3. **Note retention:** Should agent notes expire or persist forever?
4. **Embedding migration:** When switching models, do we keep old vectors or replace?
5. **Hybrid weights:** Should the 0.7/0.3 split be configurable or tuned automatically?

---

## References

- EVALUACION_CITE.md — Current evaluation with score data
- CITE_Pi_Integration.md — Local embedder analysis (nomic recommendation)
- rfc-auto-docs-sync.md — check-docs implementation (resolved)
- openspec/guides/agent-usage-guide.md — Example of doc desync (tags solve this)

---

## Review Notes

Updated to align with D9: tags replace Topic/Concept hierarchy. `topic_id`/`concept_id` removed as retrieval axes; use tags for any grouping (e.g. `--tag topic:Authentication`, `--tag concept:VectorSearch`). Storage model updated: `source_kind` is a reserved tag, not a column. Hierarchy handling section replaced with tag handling.
