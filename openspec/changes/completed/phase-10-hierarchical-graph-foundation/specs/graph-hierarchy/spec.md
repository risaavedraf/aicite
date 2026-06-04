# Graph Hierarchy Specification

## Purpose

Define the domain types and hierarchy builder for the hierarchical document structure (Document → Topic → Concept → Chunk). The graph crate owns the Rust structs (`Topic`, `Concept`, `SemanticLink`) and the logic to extract a topic/concept tree from parsed document sections. This is a pure domain layer with no I/O — the ingest crate calls graph, then writes to storage.

## Requirements

### Requirement: Graph crate MUST define `Topic` domain type (MUST)

The graph crate MUST define a `Topic` struct with the following fields:
- `topic_id: String`
- `document_id: String`
- `name: String`
- `summary: Option<String>`
- `embedding: Option<Vec<f32>>`
- `chunk_count: u32`
- `created_at: String`
- `concepts: Vec<Concept>`

The `Topic` struct MUST derive or implement:
- `Debug`
- `Clone`
- `PartialEq`
- `Serialize` / `Deserialize` (serde)
- `Default` (defaulting `chunk_count` to 0, `concepts` to empty vec)

#### Scenario: Topic struct can be constructed and serialized

- GIVEN a valid `Topic` struct with all required fields
- WHEN the struct is serialized to JSON
- THEN the JSON contains all fields with correct types
- AND `concepts` is serialized as an array
- AND `embedding` is serialized as an array of floats (or null if None)

#### Scenario: Topic struct can be deserialized from storage row

- GIVEN a JSON object representing a topic row from storage
- WHEN the JSON is deserialized to a `Topic` struct
- THEN all fields are populated correctly
- AND `concepts` is an empty vec (populated separately from storage)

### Requirement: Graph crate MUST define `Concept` domain type (MUST)

The graph crate MUST define a `Concept` struct with the following fields:
- `concept_id: String`
- `topic_id: String`
- `name: String`
- `summary: Option<String>`
- `embedding: Option<Vec<f32>>`
- `chunk_count: u32`
- `created_at: String`

The `Concept` struct MUST derive or implement:
- `Debug`
- `Clone`
- `PartialEq`
- `Serialize` / `Deserialize` (serde)
- `Default` (defaulting `chunk_count` to 0)

#### Scenario: Concept struct can be constructed and serialized

- GIVEN a valid `Concept` struct with all required fields
- WHEN the struct is serialized to JSON
- THEN the JSON contains all fields with correct types
- AND `embedding` is serialized as an array of floats (or null if None)

#### Scenario: Concept struct can be deserialized from storage row

- GIVEN a JSON object representing a concept row from storage
- WHEN the JSON is deserialized to a `Concept` struct
- THEN all fields are populated correctly

### Requirement: Graph crate MUST define `SemanticLink` domain type (MUST)

The graph crate MUST define a `SemanticLink` struct with the following fields:
- `link_id: String`
- `source_chunk_id: String`
- `target_chunk_id: String`
- `similarity_score: f64`
- `link_type: String`
- `created_at: String`

The `SemanticLink` struct MUST derive or implement:
- `Debug`
- `Clone`
- `PartialEq`
- `Serialize` / `Deserialize` (serde)
- `Default` (defaulting `link_type` to `"semantic"`)

#### Scenario: SemanticLink struct can be constructed and serialized

- GIVEN a valid `SemanticLink` struct with all required fields
- WHEN the struct is serialized to JSON
- THEN the JSON contains all fields with correct types
- AND `link_type` defaults to `"semantic"`

#### Scenario: SemanticLink struct can be deserialized from storage row

- GIVEN a JSON object representing a semantic link row from storage
- WHEN the JSON is deserialized to a `SemanticLink` struct
- THEN all fields are populated correctly

### Requirement: Graph crate MUST expose `build_hierarchy` function (MUST)

The graph crate MUST expose a public function:
```rust
pub fn build_hierarchy(sections: &[DocumentSection]) -> Vec<Topic>
```

Where `DocumentSection` is a struct with:
- `heading_level: u32` (1 = `#`, 2 = `##`, 3 = `###`, etc.)
- `heading_text: String`
- `content: String`

The function MUST:
1. Group sections by `##` headings into topics
2. Group sections by `###` headings within a topic into concepts
3. Assign remaining content (before first `##` or without `###`) to a default topic/concept

#### Scenario: Markdown with ## and ### produces correct topic/concept tree

- GIVEN a markdown document with:
  ```
  # Title (ignored)

  ## Topic A
  ### Concept A1
  Content for A1
  ### Concept A2
  Content for A2

  ## Topic B
  ### Concept B1
  Content for B1
  ```
- WHEN `build_hierarchy` is called with the parsed sections
- THEN the result contains 2 topics: "Topic A" and "Topic B"
- AND "Topic A" has 2 concepts: "Concept A1" and "Concept A2"
- AND "Topic B" has 1 concept: "Concept B1"
- AND each concept's content is preserved

#### Scenario: Markdown with only ## headings (no ###)

- GIVEN a markdown document with:
  ```
  ## Topic A
  Content for Topic A
  ## Topic B
  Content for Topic B
  ```
- WHEN `build_hierarchy` is called with the parsed sections
- THEN the result contains 2 topics: "Topic A" and "Topic B"
- AND each topic has 0 concepts
- AND topic content is preserved

#### Scenario: Markdown with ### before first ##

- GIVEN a markdown document with:
  ```
  ### Concept before topic
  Content
  ## Topic A
  Content
  ```
- WHEN `build_hierarchy` is called
- THEN a default topic is created for orphaned concepts
- AND "Topic A" is created normally

### Requirement: Heading parser MUST extract markdown headings (MUST)

The graph crate MUST include a heading parser that extracts `##` and `###` headings from markdown content. The parser MUST:
1. Identify lines starting with `##` or `###`
2. Extract the heading text (stripping `#` markers and whitespace)
3. Associate content following a heading with that heading until the next heading of equal or higher level

#### Scenario: Parser extracts ## headings correctly

- GIVEN markdown content:
  ```
  ## Topic A
  Some content here

  ## Topic B
  More content
  ```
- WHEN the heading parser is called
- THEN it returns sections with heading_level=2 for both
- AND heading_text is "Topic A" and "Topic B"
- AND content is "Some content here" and "More content" respectively

#### Scenario: Parser extracts ### headings correctly

- GIVEN markdown content:
  ```
  ### Concept A1
  Content for A1

  ### Concept A2
  Content for A2
  ```
- WHEN the heading parser is called
- THEN it returns sections with heading_level=3 for both
- AND heading_text is "Concept A1" and "Concept A2"
- AND content is "Content for A1" and "Content for A2" respectively

#### Scenario: Parser handles mixed heading levels

- GIVEN markdown content:
  ```
  # Title (ignored)
  ## Topic A
  ### Concept A1
  Content
  ## Topic B
  Content
  ```
- WHEN the heading parser is called
- THEN it returns 3 sections: "Topic A" (level 2), "Concept A1" (level 3), "Topic B" (level 2)
- AND "Concept A1" is nested under "Topic A" by the hierarchy builder

### Requirement: Non-markdown fallback MUST create single "Untitled" topic (MUST)

When `build_hierarchy` is called with content that has no `##` or `###` headings, the function MUST return a single `Topic` with:
- `name: "Untitled"`
- `concepts: []`
- Content from the entire document

#### Scenario: Plain text produces single Untitled topic

- GIVEN a plain text document with no markdown headings
- WHEN `build_hierarchy` is called
- THEN the result contains exactly 1 topic
- AND the topic name is "Untitled"
- AND the topic has 0 concepts
- AND the topic content is the entire document text

#### Scenario: Empty document produces single Untitled topic

- GIVEN an empty document
- WHEN `build_hierarchy` is called
- THEN the result contains exactly 1 topic
- AND the topic name is "Untitled"
- AND the topic has 0 concepts
- AND the topic content is empty

### Requirement: Hierarchy builder MUST handle edge cases (SHOULD)

The hierarchy builder SHOULD handle edge cases gracefully:
- Headings with extra whitespace
- Headings with special characters
- Multiple consecutive headings with no content
- Very long content sections

#### Scenario: Headings with extra whitespace are normalized

- GIVEN markdown content:
  ```
  ##   Topic A   
  Content
  ```
- WHEN `build_hierarchy` is called
- THEN the topic name is "Topic A" (whitespace trimmed)

#### Scenario: Multiple consecutive headings with no content

- GIVEN markdown content:
  ```
  ## Topic A
  ## Topic B
  Content for B
  ```
- WHEN `build_hierarchy` is called
- THEN "Topic A" has empty content
- AND "Topic B" has "Content for B"

### Requirement: Graph crate MUST NOT perform I/O (MUST)

The graph crate MUST NOT:
- Open database connections
- Read/write files
- Make network requests
- Call the storage crate

The graph crate is a pure domain layer. The ingest crate calls graph functions and writes results to storage.

#### Scenario: Graph crate functions are pure

- GIVEN a `DocumentSection` input
- WHEN `build_hierarchy` is called
- THEN the function returns a `Vec<Topic>` without side effects
- AND no database queries are executed
- AND no file I/O occurs

### Requirement: Graph crate MUST be thread-safe (SHOULD)

The graph crate types SHOULD be `Send + Sync` to support potential future parallel processing.

#### Scenario: Topic and Concept types are Send + Sync

- GIVEN a `Topic` or `Concept` instance
- WHEN the instance is moved to another thread
- THEN the move succeeds without compilation errors
- AND the instance can be shared across threads (if Arc-wrapped)

### Requirement: Graph crate MUST provide builder pattern for Topic (SHOULD)

The graph crate SHOULD provide a builder pattern for constructing `Topic` instances:
```rust
Topic::builder()
    .document_id("doc-1")
    .name("Topic A")
    .build()
```

#### Scenario: Builder pattern constructs valid Topic

- GIVEN a Topic builder with document_id and name set
- WHEN `.build()` is called
- THEN a valid `Topic` struct is returned
- AND `topic_id` is auto-generated (UUID or similar)
- AND `chunk_count` defaults to 0
- AND `concepts` defaults to empty vec

### Requirement: Graph crate MUST handle UTF-8 content correctly (MUST)

The graph crate MUST correctly handle UTF-8 encoded content, including:
- Multi-byte characters in headings
- Emoji in headings and content
- Right-to-left text

#### Scenario: UTF-8 headings are parsed correctly

- GIVEN markdown content with UTF-8 headings:
  ```
  ## Überblick
  ### 概要
  Content with émojis 🎉
  ```
- WHEN `build_hierarchy` is called
- THEN the topic name is "Überblick"
- AND the concept name is "概要"
- AND content preserves "Content with émojis 🎉"

### Requirement: Graph crate MUST validate input (SHOULD)

The graph crate SHOULD validate input and return errors for:
- Empty section content (warning, not error)
- Invalid heading levels (e.g., level 0 or > 6)
- Very long heading text (> 1000 chars)

#### Scenario: Invalid heading level returns error or warning

- GIVEN a section with `heading_level: 0`
- WHEN `build_hierarchy` is called
- THEN the function returns an error or skips the section
- AND logs a warning
