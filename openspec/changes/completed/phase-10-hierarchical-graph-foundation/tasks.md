# Phase 10 Tasks — Hierarchical Graph Foundation

## Slice A — Migration 006

**Goal**: Create the schema migration for hierarchy tables.

**Allowlist**:
- `crates/storage/src/migrations/006_hierarchy.sql` (new)
- `crates/storage/src/migrations/mod.rs` (add migration)
- `crates/storage/src/migrations/` (test file)

**Estimated lines**: ~80

**Changes**:
1. Create `006_hierarchy.sql` with:
   - `CREATE TABLE topics (...)`
   - `CREATE TABLE concepts (...)`
   - `CREATE TABLE semantic_links (...)`
   - `ALTER TABLE chunks ADD COLUMN concept_id TEXT REFERENCES concepts(concept_id)`
   - `ALTER TABLE chunks ADD COLUMN topic_id TEXT REFERENCES topics(topic_id)`
   - 7 indexes (topics, concepts, chunks hierarchy, semantic links)

2. Register migration in `mod.rs`

3. Test: migration applies on a DB with existing data (migrations 001-005); existing chunks have NULL concept_id/topic_id; queries still work.

**Dependencies**: None

**Decision needed**: No

---

## Slice B — Graph Domain Types

**Goal**: Define Rust types for the new hierarchy entities.

**Allowlist**:
- `crates/graph/src/types.rs` (new)
- `crates/graph/src/lib.rs` (re-export)

**Estimated lines**: ~100

**Changes**:
1. Create `types.rs` with:
   ```rust
   pub struct Topic {
       pub topic_id: String,
       pub document_id: String,
       pub name: String,
       pub summary: Option<String>,
       pub embedding: Option<Vec<f32>>,
       pub chunk_count: i64,
       pub created_at: String,
   }

   pub struct Concept {
       pub concept_id: String,
       pub topic_id: String,
       pub name: String,
       pub summary: Option<String>,
       pub embedding: Option<Vec<f32>>,
       pub chunk_count: i64,
       pub created_at: String,
   }

   pub struct SemanticLink {
       pub link_id: String,
       pub source_chunk_id: String,
       pub target_chunk_id: String,
       pub similarity_score: f64,
       pub link_type: String,
       pub created_at: String,
   }

   pub struct HeadingSpan {
       pub level: usize,      // 1=H1, 2=H2, 3=H3...
       pub title: String,
       pub char_offset: usize,
   }
   ```

2. Add `Default`, `Debug`, `Clone` derives
3. Re-export from `lib.rs`

**Dependencies**: None (pure types, no I/O)

**Decision needed**: No

---

## Slice C — Heading Parser

**Goal**: Extract heading structure from markdown content.

**Allowlist**:
- `crates/graph/src/heading_parser.rs` (new)
- `crates/graph/src/lib.rs` (mod declaration)

**Estimated lines**: ~120

**Changes**:
1. Create `heading_parser.rs` with:
   ```rust
   pub fn extract_headings(markdown: &str) -> Vec<HeadingSpan>
   ```
   - Parse lines starting with `#`, `##`, `###`, etc.
   - Track char offset of each heading
   - Return `Vec<HeadingSpan>` ordered by appearance

2. Unit tests:
   - Markdown with H2 and H3 → correct level/title/offset
   - Markdown with no headings → empty vec
   - Headings inside code blocks → ignored
   - Edge cases: empty string, only headings, nested headings

**Dependencies**: Slice B (uses `HeadingSpan` type)

**Decision needed**: No

---

## Slice D — Hierarchy Builder

**Goal**: Build topic/concept tree from headings and chunks.

**Allowlist**:
- `crates/graph/src/hierarchy.rs` (new)
- `crates/graph/src/lib.rs` (re-export)

**Estimated lines**: ~150

**Changes**:
1. Create `hierarchy.rs` with:
   ```rust
   pub struct HierarchyResult {
       pub topics: Vec<TopicWithConcepts>,
   }

   pub struct TopicWithConcepts {
       pub topic: Topic,
       pub concepts: Vec<ConceptWithChunks>,
   }

   pub struct ConceptWithChunks {
       pub concept: Concept,
       pub chunk_indices: Vec<usize>,  // indices into original chunk vec
   }

   pub fn build_hierarchy(
       document_id: &str,
       headings: &[HeadingSpan],
       chunk_count: usize,
       get_chunk_offset: impl Fn(usize) -> usize,
   ) -> HierarchyResult
   ```

2. Logic:
   - `##` headings → topics
   - `###` headings → concepts within current topic
   - Chunks assigned to their enclosing heading based on offsets
   - If no headings → single "Untitled" topic
   - Generate IDs: `topic_{doc_id}_{index}`, `concept_{doc_id}_{topic_idx}_{idx}`

3. Unit tests:
   - Markdown with ## and ### → correct tree structure
   - No headings → "Untitled" topic with all chunks
   - Chunks correctly assigned by offset
   - Only H2 (no concepts) → topics with empty concepts

**Dependencies**: Slice B (types), Slice C (heading parser)

**Decision needed**: No

---

## Slice E — Sentence Chunker

**Goal**: Implement sentence-based chunking strategy.

**Allowlist**:
- `crates/ingest/src/sentence_chunker.rs` (new)
- `crates/ingest/src/lib.rs` (mod declaration)
- `crates/config/src/lib.rs` (new fields)

**Estimated lines**: ~200

**Changes**:
1. Add config fields:
   ```rust
   pub sentence_chunking: bool,     // default: false
   pub min_chunk_chars: usize,      // default: 30
   pub max_chunk_chars: usize,      // default: 200
   pub build_hierarchy: bool,       // default: false
   ```

2. Create `sentence_chunker.rs`:
   ```rust
   pub struct SentenceChunk {
       pub text: String,
       pub offset_start: usize,
       pub offset_end: usize,
   }

   pub fn chunk_by_sentence(
       text: &str,
       min_chars: usize,
       max_chars: usize,
   ) -> Vec<SentenceChunk>
   ```

3. Algorithm:
   - Split on sentence boundaries (`.`, `!`, `?` followed by whitespace/EOF)
   - Handle abbreviations (`Dr.`, `e.g.`, `i.e.`) — don't split on these
   - Merge short sentences (< min_chars) with next sentence
   - Hard cap at max_chars; if single sentence > max_chars, split on clause boundary
   - Track char offsets accurately (use `char_indices()`)

4. Unit tests:
   - 2000-char text → chunks 30-200 chars
   - No sentence split across chunks
   - UTF-8 multi-byte chars → correct offsets
   - Short sentences merged correctly
   - Very long sentence → split on clause
   - Abbreviations not treated as sentence ends

**Dependencies**: None (independent function)

**Decision needed**: No

---

## Slice F — Storage CRUD for Hierarchy

**Goal**: Add storage layer functions for topics, concepts, semantic_links.

**Allowlist**:
- `crates/storage/src/topics.rs` (new)
- `crates/storage/src/concepts.rs` (new)
- `crates/storage/src/semantic_links.rs` (new)
- `crates/storage/src/lib.rs` (mod declarations)

**Estimated lines**: ~200

**Changes**:
1. `topics.rs`:
   ```rust
   pub fn insert_topic(conn: &Connection, topic: &Topic) -> Result<()>
   pub fn get_topic(conn: &Connection, topic_id: &str) -> Result<Option<Topic>>
   pub fn list_topics_by_document(conn: &Connection, document_id: &str) -> Result<Vec<Topic>>
   pub fn update_chunk_count(conn: &Connection, topic_id: &str) -> Result<()>
   ```

2. `concepts.rs`:
   ```rust
   pub fn insert_concept(conn: &Connection, concept: &Concept) -> Result<()>
   pub fn get_concept(conn: &Connection, concept_id: &str) -> Result<Option<Concept>>
   pub fn list_concepts_by_topic(conn: &Connection, topic_id: &str) -> Result<Vec<Concept>>
   pub fn update_chunk_count(conn: &Connection, concept_id: &str) -> Result<()>
   ```

3. `semantic_links.rs`:
   ```rust
   pub fn insert_link(conn: &Connection, link: &SemanticLink) -> Result<()>
   pub fn get_links_from(conn: &Connection, chunk_id: &str) -> Result<Vec<SemanticLink>>
   pub fn get_links_to(conn: &Connection, chunk_id: &str) -> Result<Vec<SemanticLink>>
   ```

4. Update `chunks.rs` to support setting topic_id/concept_id:
   ```rust
   pub fn set_chunk_hierarchy(conn: &Connection, chunk_id: &str, topic_id: &str, concept_id: Option<&str>) -> Result<()>
   ```

5. Unit tests for each CRUD function.

**Dependencies**: Slice A (migration), Slice B (types)

**Decision needed**: No

---

## Slice G — Ingest Pipeline Integration

**Goal**: Wire hierarchy building and sentence chunking into the ingest pipeline.

**Allowlist**:
- `crates/ingest/src/pipeline.rs` (or relevant ingest file)
- `crates/ingest/src/lib.rs` (re-export)
- Integration test files

**Estimated lines**: ~250

**Changes**:
1. Modify ingest pipeline to conditionally use sentence chunker:
   ```rust
   let chunks = if config.sentence_chunking {
       chunk_by_sentence(&text, config.min_chunk_chars, config.max_chunk_chars)
   } else {
       chunk_by_fixed_size(&text, config.chunk_size_chars, config.chunk_overlap_chars, config.min_chunk_size_chars)
   };
   ```

2. If `build_hierarchy` is true:
   ```rust
   if config.build_hierarchy {
       // For markdown files
       if file_type == "md" || file_type == "markdown" {
           let headings = extract_headings(&text);
           let hierarchy = build_hierarchy(&document_id, &headings, chunks.len(), |i| chunks[i].offset_start);
           
           // Insert topics and concepts
           for topic_with_concepts in hierarchy.topics {
               storage::topics::insert_topic(&conn, &topic_with_concepts.topic)?;
               for concept_with_chunks in topic_with_concepts.concepts {
                   storage::concepts::insert_concept(&conn, &concept_with_chunks.concept)?;
                   for chunk_idx in concept_with_chunks.chunk_indices {
                       storage::chunks::set_chunk_hierarchy(&conn, &chunks[chunk_idx].chunk_id, &topic_with_concepts.topic.topic_id, Some(&concept_with_chunks.concept.concept_id))?;
                   }
               }
           }
       } else {
           // Non-markdown: single "Untitled" topic
           let topic = Topic { name: "Untitled".to_string(), ... };
           storage::topics::insert_topic(&conn, &topic)?;
           for chunk in &chunks {
               storage::chunks::set_chunk_hierarchy(&conn, &chunk.chunk_id, &topic.topic_id, None)?;
           }
       }
   }
   ```

3. Integration test:
   - Ingest markdown file with `build_hierarchy=true` → topics/concepts created in DB
   - Ingest txt file with `build_hierarchy=true` → single "Untitled" topic
   - Ingest with `build_hierarchy=false` → no hierarchy (identical to v0.1.0)
   - Ingest with `sentence_chunking=true` → chunks 30-200 chars

**Dependencies**: All previous slices

**Decision needed**: No

---

## Slice H — Verification + Closeout

**Goal**: Full test suite, backward compat verification, documentation.

**Allowlist**:
- All test files
- `docs/sdd/phase-10-hierarchical-graph-foundation/` notes
- `docs/v0.2.0-hierarchical-graph.md` (update status)

**Estimated lines**: ~100 (test additions + docs)

**Changes**:
1. Run full test suite: `cargo test`
2. Run lint: `cargo clippy -- -D warnings`
3. Run format: `cargo fmt --check`
4. Verify backward compat:
   - Existing corpus (v0.1.0 data) still works with flat retrieval
   - `build_hierarchy=false` (default) → identical behavior
5. Update verification evidence in SDD artifacts
6. Document known limitations:
   - Sentence chunking off by default
   - Hierarchy building off by default
   - semantic_links table created but empty
   - Non-markdown files get "Untitled" topic

**Dependencies**: All previous slices

**Decision needed**: No

---

## Summary

| Slice | Description | Est. Lines | Depends On |
|-------|-------------|------------|------------|
| A | Migration 006 | ~80 | — |
| B | Graph domain types | ~100 | — |
| C | Heading parser | ~120 | B |
| D | Hierarchy builder | ~150 | B, C |
| E | Sentence chunker + config | ~200 | — |
| F | Storage CRUD | ~200 | A, B |
| G | Ingest pipeline integration | ~250 | All |
| H | Verification + closeout | ~100 | All |
| **Total** | | **~1200** | |

**Review workload**: All slices ≤ 250 lines, well under 400-line budget. No chained PRs needed — each slice is independently reviewable.

**Parallelization possible**:
- Slices A, B, E can start immediately (no deps on each other)
- Slices C, D depend on B
- Slice F depends on A, B
- Slice G depends on all
- Slice H is final

**Implementation order**: A → B → E → C → D → F → G → H
