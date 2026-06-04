# Ingest Hierarchy Specification

## Purpose

Define the sentence-based chunking strategy and ingest pipeline integration for the hierarchical document structure. The ingest crate adds a new `chunk_by_sentence` function and wires the hierarchy builder into the ingest pipeline, gated by configuration flags. This enables topic-scoped chunks (30–200 chars) organized under a Document → Topic → Concept → Chunk hierarchy.

## Requirements

### Requirement: Ingest crate MUST provide `chunk_by_sentence` function (MUST)

The ingest crate MUST expose a public function:
```rust
pub fn chunk_by_sentence(
    text: &str,
    min_chunk_chars: usize,
    max_chunk_chars: usize,
) -> Result<Vec<ChunkInput>, CiteError>
```

The function MUST:
1. Split text on sentence boundaries (`.`, `!`, `?` followed by whitespace or EOF)
2. Merge adjacent sentences if combined length < `min_chunk_chars`
3. Hard cap each chunk at `max_chunk_chars`; if a single sentence exceeds `max_chunk_chars`, split on the nearest clause boundary
4. Return chunks with valid offset tracking

#### Scenario: 2000-char text produces chunks 30-200 chars

- GIVEN a 2000-character text with 15 sentences (average ~133 chars each)
- WHEN `chunk_by_sentence` is called with `min_chunk_chars=30`, `max_chunk_chars=200`
- THEN all returned chunks have length >= 30 chars
- AND all returned chunks have length <= 200 chars
- AND no sentence is split across chunks
- AND the total text coverage is complete (no content lost)

#### Scenario: No sentence is split across chunks

- GIVEN text with sentences: "Sentence one. Sentence two. Sentence three."
- WHEN `chunk_by_sentence` is called
- THEN each chunk contains complete sentences
- AND no chunk starts mid-sentence
- AND no chunk ends mid-sentence

#### Scenario: Short sentences are merged until min_chunk_chars

- GIVEN text with short sentences: "A. B. C. D. E. F. G. H. I. J."
- WHEN `chunk_by_sentence` is called with `min_chunk_chars=30`
- THEN adjacent sentences are merged until combined length >= 30
- AND no chunk contains only "A." (too short)

#### Scenario: Long sentence is split on clause boundary

- GIVEN a sentence with 250 characters containing commas at positions 100 and 200
- WHEN `chunk_by_sentence` is called with `max_chunk_chars=200`
- THEN the sentence is split at the comma nearest to 200 chars
- AND both resulting chunks are complete clauses (not mid-word)

### Requirement: Sentence chunker MUST handle UTF-8 correctly (MUST)

The sentence chunker MUST use `char_indices()` for all offset math to correctly handle multi-byte UTF-8 characters.

#### Scenario: UTF-8 text is chunked without splitting multi-byte chars

- GIVEN text with multi-byte characters: "Café résumé naïve 🎉🎊🎉🎊"
- WHEN `chunk_by_sentence` is called
- THEN all returned chunks are valid UTF-8
- AND no multi-byte character is split across chunks
- AND offset tracking is correct (char-based, not byte-based)

#### Scenario: Emoji in sentences are preserved

- GIVEN text: "Hello 🌍! How are you? I am fine 😊."
- WHEN `chunk_by_sentence` is called
- THEN emoji are preserved in the correct chunks
- AND sentence boundaries are detected correctly around emoji

### Requirement: Sentence chunker MUST handle edge cases (MUST)

The sentence chunker MUST handle edge cases gracefully:
- Empty text
- Text with no sentence boundaries
- Text with only whitespace
- Text with abbreviations (e.g., "Dr.", "U.S.A.")
- Text with multiple consecutive punctuation marks

#### Scenario: Empty text returns empty vec

- GIVEN an empty string
- WHEN `chunk_by_sentence` is called
- THEN an empty Vec is returned
- AND no error occurs

#### Scenario: Text with no sentence boundaries

- GIVEN text with no periods, exclamation marks, or question marks: "This is a long sentence without any punctuation marks"
- WHEN `chunk_by_sentence` is called with `max_chunk_chars=50`
- THEN the text is split at the nearest word boundary to 50 chars
- AND no chunk exceeds 50 chars

#### Scenario: Text with abbreviations handles periods correctly

- GIVEN text: "Dr. Smith went to Washington. He arrived at 3 p.m."
- WHEN `chunk_by_sentence` is called
- THEN "Dr." is not treated as a sentence boundary
- AND "Washington." is treated as a sentence boundary
- AND "p.m." is not treated as a sentence boundary

### Requirement: Sentence chunker MUST preserve offset tracking (MUST)

The sentence chunker MUST return `ChunkInput` instances with accurate:
- `offset_start`: char offset from start of input text
- `offset_end`: char offset from start of input text
- `chunk_index`: sequential index starting from 0
- `page`: None (sentence chunking doesn't track pages)

#### Scenario: Chunk offsets are contiguous and non-overlapping

- GIVEN text with 1000 characters
- WHEN `chunk_by_sentence` is called
- THEN chunk 0 has `offset_start=0`
- AND each subsequent chunk has `offset_start` equal to the previous chunk's `offset_end`
- AND the last chunk has `offset_end=1000`
- AND no chunks overlap

#### Scenario: Chunk index is sequential

- GIVEN text that produces 5 chunks
- WHEN `chunk_by_sentence` is called
- THEN chunks have `chunk_index` values: 0, 1, 2, 3, 4

### Requirement: Config crate MUST add hierarchy configuration fields (MUST)

The `IngestConfig` struct MUST be extended with:
- `sentence_chunking: bool` (default: `false`)
- `min_chunk_chars: usize` (default: `30`)
- `max_chunk_chars: usize` (default: `200`)
- `build_hierarchy: bool` (default: `false`)

#### Scenario: New config fields have correct defaults

- GIVEN a default `IngestConfig` instance
- WHEN the config is inspected
- THEN `sentence_chunking` is `false`
- AND `min_chunk_chars` is `30`
- AND `max_chunk_chars` is `200`
- AND `build_hierarchy` is `false`

#### Scenario: Config can be loaded from environment variables

- GIVEN environment variables:
  - `CITE_SENTENCE_CHUNKING=true`
  - `CITE_MIN_CHUNK_CHARS=50`
  - `CITE_MAX_CHUNK_CHARS=150`
  - `CITE_BUILD_HIERARCHY=true`
- WHEN `Config::load()` is called
- THEN `config.ingest.sentence_chunking` is `true`
- AND `config.ingest.min_chunk_chars` is `50`
- AND `config.ingest.max_chunk_chars` is `150`
- AND `config.ingest.build_hierarchy` is `true`

### Requirement: Ingest pipeline MUST support hierarchy mode (MUST)

The ingest pipeline MUST support a hierarchy mode gated by `build_hierarchy` config flag. When enabled:
1. Parse document headings → produce topic/concept skeleton via `graph::build_hierarchy`
2. Chunk text using sentence-based strategy (if `sentence_chunking` is on)
3. Assign each chunk to its parent concept
4. Write topics, concepts, and chunk FKs to storage in a single transaction
5. Generate embeddings per-chunk as before

When `build_hierarchy` is `false`, ingest MUST behave identically to v0.1.0.

#### Scenario: Ingest with hierarchy=true creates topic/concept rows

- GIVEN a markdown document with `## Topic A` and `### Concept A1`
- WHEN ingest is run with `build_hierarchy=true`
- THEN a topic row is created in the `topics` table with name "Topic A"
- AND a concept row is created in the `concepts` table with name "Concept A1"
- AND chunk rows have `topic_id` and `concept_id` set to the created topic/concept

#### Scenario: Ingest with hierarchy=false behaves like v0.1.0

- GIVEN a markdown document with `## Topic A` and `### Concept A1`
- WHEN ingest is run with `build_hierarchy=false` (default)
- THEN no topic or concept rows are created
- AND chunk rows have `topic_id=NULL` and `concept_id=NULL`
- AND chunks are created using the existing fixed-size chunking strategy

#### Scenario: Ingest with sentence_chunking=true uses sentence-based chunking

- GIVEN a markdown document
- WHEN ingest is run with `sentence_chunking=true` and `build_hierarchy=true`
- THEN chunks are created using `chunk_by_sentence`
- AND all chunks have length between `min_chunk_chars` and `max_chunk_chars`
- AND no sentence is split across chunks

#### Scenario: Ingest with sentence_chunking=false uses fixed-size chunking

- GIVEN a markdown document
- WHEN ingest is run with `sentence_chunking=false` and `build_hierarchy=true`
- THEN chunks are created using the existing `chunk_text` function
- AND chunk size is determined by `chunk_size_chars` config

### Requirement: Ingest pipeline MUST handle non-markdown files (MUST)

When `build_hierarchy=true` and the file is not markdown (e.g., `.txt`, `.pdf`):
1. Create a single topic named "Untitled"
2. Create no concepts
3. Assign all chunks to the "Untitled" topic

#### Scenario: Ingest .txt file with hierarchy=true creates single Untitled topic

- GIVEN a plain text file with no markdown headings
- WHEN ingest is run with `build_hierarchy=true`
- THEN a single topic row is created with name "Untitled"
- AND no concept rows are created
- AND all chunk rows have `topic_id` set to the "Untitled" topic
- AND all chunk rows have `concept_id=NULL`

#### Scenario: Ingest .pdf file with hierarchy=true creates single Untitled topic

- GIVEN a PDF file (no markdown structure)
- WHEN ingest is run with `build_hierarchy=true`
- THEN a single topic row is created with name "Untitled"
- AND no concept rows are created
- AND all chunk rows have `topic_id` set to the "Untitled" topic

### Requirement: Ingest pipeline MUST write hierarchy data atomically (MUST)

When creating topics, concepts, and updating chunk FKs, the ingest pipeline MUST:
1. Use a single database transaction
2. Rollback all changes if any insert fails
3. Ensure consistency between topic/concept/chunk relationships

#### Scenario: Transaction rollback on failure

- GIVEN a document with 2 topics and 5 concepts
- WHEN ingest is processing and a concept insert fails (e.g., FK violation)
- THEN the entire transaction is rolled back
- AND no partial topic/concept rows exist
- AND no chunk rows have partial FK assignments

#### Scenario: All-or-nothing hierarchy creation

- GIVEN a document that produces 3 topics and 10 concepts
- WHEN ingest completes successfully
- THEN all 3 topics exist in the database
- AND all 10 concepts exist in the database
- AND all chunks have correct FK assignments
- AND no orphaned rows exist

### Requirement: Ingest pipeline MUST generate embeddings per-chunk (MUST)

When `build_hierarchy=true`, the ingest pipeline MUST generate embeddings for each chunk as before. Topic and concept embeddings are NOT generated in Phase 10 (deferred to Phase 11).

#### Scenario: Chunks get embeddings in hierarchy mode

- GIVEN a document ingested with `build_hierarchy=true`
- WHEN the ingest completes
- THEN all chunk rows have corresponding embedding rows in the `embeddings` table
- AND topic rows have `embedding=NULL`
- AND concept rows have `embedding=NULL`

### Requirement: Ingest pipeline MUST update chunk_count on topics and concepts (SHOULD)

After assigning chunks to topics and concepts, the ingest pipeline SHOULD update the `chunk_count` field on each topic and concept row.

#### Scenario: Topic chunk_count reflects assigned chunks

- GIVEN a document with 2 topics: "Topic A" (5 chunks) and "Topic B" (3 chunks)
- WHEN ingest completes
- THEN `Topic A.chunk_count` is 5
- AND `Topic B.chunk_count` is 3

#### Scenario: Concept chunk_count reflects assigned chunks

- GIVEN a topic with 2 concepts: "Concept A1" (2 chunks) and "Concept A2" (3 chunks)
- WHEN ingest completes
- THEN `Concept A1.chunk_count` is 2
- AND `Concept A2.chunk_count` is 3

### Requirement: Sentence chunker MUST be tested with property-based tests (SHOULD)

The sentence chunker SHOULD include property-based tests (e.g., using `proptest` or `quickcheck`) that verify:
- All chunks are within min/max bounds
- No sentence is split across chunks
- Total text coverage is complete
- Offset tracking is correct

#### Scenario: Property-based tests pass for random inputs

- GIVEN random text inputs with varying sentence structures
- WHEN `chunk_by_sentence` is called
- THEN all invariants hold:
  - chunk length >= min_chunk_chars
  - chunk length <= max_chunk_chars
  - no sentence split
  - offset continuity
