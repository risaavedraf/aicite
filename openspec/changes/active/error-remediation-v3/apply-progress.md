# Apply Progress — error-remediation-v3

**Phase:** apply  
**Slices:** CR-1 — verified CodeRabbit code/test fixes; CR-2 — verified documentation/archive corrections; C9-1 — common typed-ID foundation; C9-2 — storage boundary typed-ID pilot; C9-3 — retrieval boundary typed-ID pilot; C9-4 — graph/domain typed IDs; C9-5a — engine/common typed-ID migration; C9-5b — storage test fixup; C9-6 — typed-ID cleanup/public API audit; SNAP-1 — snapshot activation rollback confidence; SNAP-2 — snapshot pointer `updated_at` migration; TIME-1 — `created_at` DateTime<Utc> consistency  
**Status:** ✅ CR-1 completed; ✅ CR-2 completed; ✅ C9-1 completed; ✅ C9-2 completed; ✅ C9-3 completed; ✅ C9-4 completed; ✅ C9-5a completed; ✅ C9-5b completed; ✅ C9-6 completed; ✅ SNAP-1 completed; ✅ SNAP-2 completed; ✅ TIME-1 completed  
**Date:** 2026-06-04

## Scope Guard

Implemented CR-1 code/test fixes, CR-2 documentation/archive corrections, C9-1 common typed-ID foundation, C9-2 storage boundary typed-ID pilot, C9-3 retrieval boundary typed-ID pilot, C9-4 graph/domain typed IDs, C9-5a/b engine/common typed-ID migration with storage test fixup, C9-6 typed-ID cleanup/public API audit, SNAP-1 snapshot activation rollback confidence, SNAP-2 snapshot pointer `updated_at` migration, and TIME-1 `created_at` DateTime consistency for the selected graph/storage records.

## CodeRabbit CR-1 Finding Statuses

| Finding | Status | Evidence |
| --- | --- | --- |
| `crates/cli/src/commands/health.rs` `--json` wording vs live provider checks | `verified-fixed` | Verified `execute()` builds full health output and `check_provider()` calls `provider.embed("test")` for both human and JSON output. User chose to preserve live checks. Updated health documentation and CLI help wording to state JSON is output-only and provider checks may run. |
| `crates/cli/src/commands/setup.rs` provider/model pairing | `verified-fixed` | Verified setup reused `config.embedding.model` while selected provider could change. Added `selected_provider_model()` and used the derived model for connection testing and saved config in non-interactive and interactive setup. Added focused unit tests. |
| `crates/config/src/lib.rs` environment restoration and host config isolation | `verified-fixed` | Verified selected tests mutated `CITE_EMBEDDING_TIMEOUT` / `CITE_TOP_K` without restoration and fallback tests used `Config::load()`. Added `EnvVarGuard` restoration and switched targeted fallback tests to `Config::load_from(Some(isolated_missing_config_path()))`. |
| `crates/retrieval/src/lib.rs` vector clone in `rank_by_similarity` | `verified-fixed` | Verified hot path used `candidate.clone().into()`, cloning `ChunkEmbeddingRecord.vector`. Added `impl From<&ChunkEmbeddingRecord> for ScoredChunk` and changed ranking to convert from a reference. Added metadata preservation coverage. |
| `crates/storage/src/rate_limits.rs` non-positive prune age | `verified-fixed` | Verified `prune_stale_rate_limits` computed cutoff before validating `max_age_seconds`. Added early `InvalidParameter` error for `<= 0` and focused test proving rows are not deleted. |
| Prior artifact count corrections | `verified-deferred` | CR-2 documentation/archive slice; not implemented in CR-1 by scope. |
| Stale archive report corrections | `verified-deferred` | CR-2 documentation/archive slice; not implemented in CR-1 by scope. |

## CodeRabbit CR-2 Finding Statuses

| Finding | Status | Evidence |
| --- | --- | --- |
| `openspec/changes/active/error-remediation-v2/apply-progress.md` PR/wave count | `verified-fixed` | Verified the artifact lists Wave 1 through Wave 7 and seven commits in the chain. Updated status from `ALL 6 PRs APPLIED` to `ALL 7 PRs APPLIED`. |
| `openspec/changes/active/error-remediation/second-pass-prompt.md` T3/T4 total | `verified-fixed` | Verified T3 table sums to 37 and T4 table sums to 38, for 75 total. Updated the objective line from 78 to 75. |
| Runtime guard archive docs (`cli/errores.md`, `compliance/review.md`, `engine/errores.md`, `engine/review.md`) | `verified-fixed` | Verified current CLI `ingest::execute()` calls `engine::runtime_guard::check_ingest_allowed(&config.runtime.mode)`, while engine `ingest`, `ingest_next`, and `ingest_internal` do not re-check runtime mode. Updated docs to distinguish CLI enforcement from engine-internal boundary risk. |
| Graph UTF-8 heading offset archive docs (`graph/errores.md`, `graph/review.md`) | `verified-fixed` | Verified `crates/graph/src/heading_parser.rs` uses `char_offset += line.chars().count() + 1`. Updated docs to mark the byte-offset claim historical/resolved and recommend UTF-8 regression coverage. |
| Ingest UTF-8 archive docs (`ingest/errores.md`, `ingest/review.md`) | `verified-fixed` | Verified `sanitize_display_name` truncates with `trimmed.chars().take(255)`, `extract_plain_text` uses `content.chars().count()`, and `extract_pdf_text` accumulates `text.chars().count()`. Updated docs to reflect char-based behavior. |
| Provider API-key archive docs (`providers/errores.md`) | `verified-fixed` | Verified `create_provider` uses `resolve_api_key(config).ok_or_else(...)`, and both `GeminiProvider::new` and `OpenAICompatibleProvider::new` reject empty keys. Updated provider docs and clarified the archived report is in-repo. |

## Tests Added Or Updated

- `crates/cli/src/commands/health.rs`
  - `health_output_includes_provider_status_for_json_contract`
- `crates/cli/src/commands/setup.rs`
  - `selected_provider_model_uses_provider_default_when_provider_changes`
  - `selected_provider_model_preserves_existing_model_for_same_provider`
- `crates/config/src/lib.rs`
  - updated `test_env_embedding_timeout_overridden`
  - updated `test_env_invalid_top_k_falls_back_to_default`
  - updated `test_invalid_env_values_fall_back_to_defaults`
- `crates/retrieval/src/lib.rs`
  - `test_scored_chunk_from_record_reference_preserves_metadata`
  - `test_scored_chunk_typed_ids_render_as_strings`
  - existing ranking tests preserved
- `crates/storage/src/rate_limits.rs`
  - `test_prune_rejects_non_positive_age_without_deleting_rows`

## Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p cli health_output_includes_provider_status_for_json_contract` | passed | Health output test passed, confirming provider status remains part of health output contract. |
| `cargo test -p cli selected_provider_model` | passed | 2 setup model-selection tests passed. |
| `cargo test -p retrieval test_scored_chunk_from_record_reference_preserves_metadata` | passed | Reference conversion metadata test passed. |
| `cargo test -p retrieval test_rank_by_similarity_top_k` | passed | Existing ranking behavior test passed. |
| `cargo test -p storage test_prune_rejects_non_positive_age_without_deleting_rows` | passed | Non-positive prune age rejection test passed. |
| `cargo test -p config test_env_embedding_timeout_overridden` | passed | Env timeout override/restoration test passed. |
| `cargo test -p config test_env_invalid_top_k_falls_back_to_default` | passed | Isolated invalid top-k fallback test passed. |
| `cargo test -p config test_invalid_env_values_fall_back_to_defaults` | passed | Isolated invalid env fallback test passed. |
| `cargo fmt && cargo fmt --check` | passed | Formatting applied and check passed. |
| `cargo test` | passed | Workspace tests passed: all listed unit/integration/doc tests passed; provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |

## CR-2 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `grep -n "check_ingest_allowed(&config.runtime.mode)\|pub fn ingest_next\|fn ingest_internal" crates/cli/src/commands/ingest.rs crates/engine/src/ingest.rs` | passed | Verified CLI runtime guard call exists and engine `ingest_next` / `ingest_internal` remain internal boundary risks. |
| `grep -n "line.chars().count() + 1\|trimmed.chars().take(255)\|content.chars().count()\|total_chars += text.chars().count()" crates/graph/src/heading_parser.rs crates/ingest/src/validator.rs crates/ingest/src/extractor.rs` | passed | Verified current UTF-8/character-count implementations behind graph and ingest doc corrections. |
| `grep -n "resolve_api_key(config).ok_or_else\|api_key.is_empty()" crates/cli/src/commands/mod.rs crates/providers/src/gemini.rs crates/providers/src/openai.rs` | passed | Verified provider factory and constructors reject missing/empty API keys. |
| `grep -n "ALL 7 PRs\|Wave 7\|^c329610\|^213ee99\|^48b0ffc\|^f09b06f\|^46d88ac\|^06692e6\|^f6c2a3a" openspec/changes/active/error-remediation-v2/apply-progress.md` | passed | Verified V2 status now matches seven waves/commits. |
| `grep -n "75 errores restantes\|T3: Medium (37 errores)\|T4: Low (38 errores)" openspec/changes/active/error-remediation/second-pass-prompt.md` | passed | Verified second-pass total now matches 37 + 38 = 75. |
| `git diff --numstat -- <CR-2 docs>` | passed | CR-2 tracked docs delta is 43 insertions / 192 deletions across 11 tracked Markdown files, staying under the 400-line review budget. |
| `git diff --check` | passed | No whitespace errors reported; command emitted existing CRLF conversion warnings for touched files. |
| `git diff --stat` | passed | Produced combined CR-1 + CR-2 diff stat for parent review; CR-2 itself remains documentation-only. |

## Diff Summary

CR-1 changed Rust code in CLI health/setup, config tests, retrieval ranking conversion, and storage rate-limit pruning. The slice keeps `health --json` live-provider behavior, fixes provider/model consistency during setup, makes selected config tests environment-safe and host-config isolated, avoids cloning embedding vectors in ranking output construction, and rejects invalid rate-limit prune ages before deletion.

CR-2 changed only documentation/archive artifacts and this apply-progress evidence. It corrected V2 status/count drift, second-pass T3/T4 totals, stale runtime-guard archive claims, stale UTF-8 archive claims, and stale provider API-key archive claims. No Rust code was edited by CR-2.

Current CR-1 tracked-code diff before this artifact: 202 insertions / 25 deletions across 6 Rust files. CR-2 tracked documentation diff is 43 insertions / 192 deletions across 11 tracked Markdown files, plus this V3 apply-progress update. Both slices remain under the 400-line review budget.

## Self-Review

Reviewed the CR-1 diff against the project `code-quality-review` skill, especially Rust ownership/error-handling guidance:

- The retrieval change removes the hot-path record/vector clone and borrows candidates idiomatically.
- The setup model helper keeps provider selection logic local and avoids broad provider abstraction churn.
- The rate-limit guard uses the existing `CiteError::InvalidParameter` vocabulary.
- Config test env restoration uses a `Drop` guard to restore state even if assertions panic.

CR-2 self-review checked documentation factual accuracy against current code and scoped edits against the cognitive-doc-design skill:

- Runtime guard docs now lead with the current outcome: CLI ingest is guarded; engine internals are the remaining boundary risk.
- UTF-8 docs now distinguish resolved char-based implementations from remaining unrelated archive findings.
- Provider docs now reflect both caller-side `resolve_api_key(...).ok_or_else(...)` validation and constructor-side `api_key.is_empty()` checks.
- Count corrections are minimal one-line fixes backed by the visible wave/commit and T3/T4 totals.
- No Rust code was edited during CR-2.

No self-review issues requiring repair remain.

## C9-1 Status — Common Typed-ID Foundation

| Item | Status | Evidence |
| --- | --- | --- |
| `DocumentId`, `ChunkId`, and `TraceId` foundational traits | `completed` | Added consistent `Display`, `From<String>`, `From<&str>`, `FromStr<Err = Infallible>`, `AsRef<str>`, `Deref<Target = str>`, clone/equality/debug/hash, and explicit `#[serde(transparent)]` support through a local macro in `crates/common/src/types.rs`. |
| Common module exports | `completed` | Re-exported `DocumentId`, `ChunkId`, and `TraceId` from `crates/common/src/lib.rs` so downstream slices can import them from the common crate root. |
| Downstream migration boundary | `deferred` | C9-1 intentionally leaves storage/retrieval/graph/engine/CLI fields as raw strings. C9-2+ will choose concrete migration boundaries and record exact fields migrated. |
| External representation | `preserved` | Unit tests prove each typed ID serializes to a JSON string and deserializes from the same string shape. No schema, fixture, or downstream type migration was introduced. |

## C9-1 Tests Added Or Updated

- `crates/common/src/types.rs`
  - `document_id_has_string_transparent_foundation_traits`
  - `chunk_id_has_string_transparent_foundation_traits`
  - `trace_id_has_string_transparent_foundation_traits`

## C9-1 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p common string_transparent_foundation_traits` | passed | Focused typed-ID tests passed: 3 passed, 0 failed. |
| `cargo fmt --check` | passed | Formatting check completed with no reported formatting changes required. |
| `cargo test -p common` | passed | Common crate tests passed: 14 unit tests and 9 doc tests passed. |
| `cargo test` | passed | Workspace tests passed, including CR-1 and C9-1 tests; provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |

## C9-2 Status — Storage Boundary Typed-ID Pilot

| Item | Status | Evidence |
| --- | --- | --- |
| `TopicRow.document_id` storage row boundary | `completed` | Migrated `crates/storage/src/topics.rs::TopicRow.document_id` from `String` to `DocumentId`. `get_topic()` and `list_topics_by_document()` still bind `&str` query parameters and decode SQLite TEXT through `String -> DocumentId`, preserving persisted strings. |
| `SemanticLinkRow.source_chunk_id` / `target_chunk_id` storage row boundary | `completed` | Migrated `crates/storage/src/semantic_links.rs::SemanticLinkRow.source_chunk_id` and `target_chunk_id` from `String` to `ChunkId`. `insert_semantic_link()`, `get_links_from()`, and `get_links_to()` still bind/query `&str` and decode SQLite TEXT through `String -> ChunkId`. |
| No validation behavior change | `preserved` | C9-2 uses C9-1 string-transparent newtypes and does not add ID format validation, parse-error mapping, or schema changes. |
| Downstream migration boundary | `deferred` | Graph, engine, CLI, common `Document`/`Chunk`/trace records, `ConceptRow`, snapshots, and trace persistence remain raw string boundaries for later C9 slices. |

## C9-2 Exact Boundary Map

Migrated in this slice:

- `crates/storage/src/topics.rs`
  - `TopicRow.document_id: DocumentId`
- `crates/storage/src/semantic_links.rs`
  - `SemanticLinkRow.source_chunk_id: ChunkId`
  - `SemanticLinkRow.target_chunk_id: ChunkId`

Intentionally left as `String` for later slices:

- `common::types::Document.document_id`
- `common::types::Chunk.chunk_id`
- `common::types::Chunk.document_id`
- `common::types::TraceHeaderInput.trace_id`
- `common::types::TraceHeaderRecord.trace_id`
- `common::types::TraceCitationRecord.trace_id`
- `common::types::TraceCitationRecord.document_id`
- `common::types::TraceCitationRecord.chunk_id`
- `storage::snapshots` snapshot/document ID return values
- `storage::traces` trace, document, and chunk ID fields
- All retrieval, graph, engine, and CLI ID fields/callers

## C9-2 Tests Added Or Updated

- `crates/storage/src/topics.rs`
  - updated `test_insert_and_get_topic`
  - updated `test_list_topics_by_document`
  - added `test_topic_row_decodes_document_id_as_typed_id_and_preserves_storage_string`
- `crates/storage/src/semantic_links.rs`
  - updated `test_insert_and_get_links_from`
  - updated `test_get_links_to`
  - added `test_semantic_link_row_decodes_chunk_ids_as_typed_ids_and_preserves_storage_strings`

## C9-2 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p storage typed_id -- --nocapture` | passed | Focused C9-2 typed-ID pilot tests passed: 2 passed, 0 failed. |
| `cargo test -p storage test_topic_row_decodes_document_id_as_typed_id_and_preserves_storage_string test_semantic_link_row_decodes_chunk_ids_as_typed_ids_and_preserves_storage_strings` | failed | Invalid cargo syntax for multiple test filters; superseded by the passing `typed_id` focused test and full storage test. |
| `cargo test -p storage` | passed | Storage crate tests passed: 88 unit tests passed; 10 doc tests ignored. |
| `cargo fmt --check` | passed | Formatting check completed with no output. |
| `cargo test` | passed | Workspace tests passed; provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |

## C9-2 Self-Review

Reviewed the C9-2 diff against the project `code-quality-review` skill and Rust idioms:

- Storage boundary migration is narrow and uses newtypes only where row structs expose document/chunk IDs directly.
- SQLite binding remains string-shaped via existing `&str` function parameters; row decoding converts from `String` into string-transparent newtypes.
- No broad downstream migration was introduced: retrieval, graph, engine, and CLI remain untouched by C9-2.
- No ID validation or parse-error behavior was added; this matches the C9-1 infallible typed-ID decision.
- Tests assert both typed row access (`as_ref()`) and unchanged persisted SQLite TEXT values.

## C9-3 Status — Retrieval Boundary Typed-ID Pilot

| Item | Status | Evidence |
| --- | --- | --- |
| `ChunkEmbeddingRecord` typed ID mirrors | `completed` | Added `chunk_id_typed: ChunkId` and `document_id_typed: DocumentId` to `crates/storage/src/embeddings.rs::ChunkEmbeddingRecord`, decoding SQLite TEXT into string-transparent newtypes without changing persisted values. |
| `ScoredChunk` typed ID mirrors | `completed` | Added `chunk_id_typed` / `document_id_typed` to `retrieval::ScoredChunk`, populated from `ChunkEmbeddingRecord` conversions while keeping string fields unchanged. |
| No validation behavior change | `preserved` | Typed IDs remain string-transparent/infallible; no new parse-error or validation semantics were introduced. |
| Downstream migration boundary | `deferred` | Graph, engine, and CLI still operate on string IDs; string fields remain the primary boundary for `ScoredChunk` and `ChunkEmbeddingRecord`. |

## C9-3 Exact Boundary Map

Migrated in this slice:

- `crates/storage/src/embeddings.rs`
  - `ChunkEmbeddingRecord.chunk_id_typed: ChunkId`
  - `ChunkEmbeddingRecord.document_id_typed: DocumentId`
- `crates/retrieval/src/lib.rs`
  - `ScoredChunk.chunk_id_typed: ChunkId`
  - `ScoredChunk.document_id_typed: DocumentId`

Intentionally left as `String` for compatibility:

- `storage::embeddings::ChunkEmbeddingRecord.chunk_id`
- `storage::embeddings::ChunkEmbeddingRecord.document_id`
- `retrieval::ScoredChunk.chunk_id`
- `retrieval::ScoredChunk.document_id`
- All graph, engine, and CLI ID fields/callers

## C9-3 Tests Added Or Updated

- `crates/retrieval/src/lib.rs`
  - `test_scored_chunk_from_record_reference_preserves_metadata` (extended)
  - `test_scored_chunk_typed_ids_render_as_strings`

## C9-3 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p retrieval test_scored_chunk_typed_ids_render_as_strings` | passed | Focused typed-ID string rendering test passed (1 passed). |
| `cargo test -p retrieval` | passed | Retrieval crate tests passed: 16 unit tests + 2 doc tests. |
| `cargo fmt --check` | passed | Formatting check completed with no output. |
| `cargo test` | passed | Workspace tests passed; provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |

## C9-3 Self-Review

Reviewed the C9-3 diff against the project `code-quality-review` skill and Rust idioms:

- Retrieval boundary migration is additive and narrow, introducing typed ID mirrors without altering string-shaped public fields.
- Conversions avoid extra vector clones; existing reference-based `ScoredChunk` conversion remains unchanged.
- Typed IDs are populated from already-decoded strings, so SQLite persistence and ranking behavior remain stable.
- No new validation or error-handling semantics were introduced; typed IDs remain infallible wrappers.

## C9-4 Status — Graph/Domain Typed IDs

| Item | Status | Evidence |
| --- | --- | --- |
| `TopicId` and `ConceptId` common foundations | `completed` | Added string-transparent `TopicId` and `ConceptId` newtypes with the same `Display`, `From<String>`, `From<&str>`, `FromStr<Err = Infallible>`, `AsRef<str>`, `Deref<Target = str>`, clone/equality/debug/hash, and `#[serde(transparent)]` contract as the existing C9-1 IDs. Re-exported both from `common`. |
| Graph domain ID fields | `completed` | Migrated `graph::types::Topic.topic_id` to `TopicId`, `Topic.document_id` to `DocumentId`, `Concept.concept_id` to `ConceptId`, and `Concept.topic_id` to `TopicId`. `build_hierarchy()` constructs typed IDs explicitly and tests verify JSON serialization remains string-shaped. |
| Storage graph row ID fields | `completed` | Migrated `storage::topics::TopicRow.topic_id` to `TopicId` and `storage::concepts::ConceptRow.concept_id` / `topic_id` to `ConceptId` / `TopicId`. SQLite query/write parameters remain `&str`; row decoding converts stored TEXT through `String -> typed ID`. |
| Heading/topic and link boundaries | `preserved` | `graph::heading_parser` behavior was not changed. Ingest hierarchy call sites use typed IDs via `Deref<Target = str>` at storage boundaries and convert to owned strings only for internal topic boundary tracking. `storage::SemanticLinkRow` chunk-ID behavior from C9-2 was left unchanged. |
| Out-of-scope work | `deferred` | No `created_at` DateTime changes, engine/CLI ID migrations, SNAP work, or broad `common::Document`/`common::Chunk` migrations were introduced. `storage::embeddings::HierarchicalChunkEmbedding.topic_id` / `concept_id` remain `Option<String>` for retrieval compatibility. |

## C9-4 Exact Boundary Map

Migrated in this slice:

- `crates/common/src/types.rs`
  - `TopicId`
  - `ConceptId`
- `crates/common/src/lib.rs`
  - root exports for `TopicId` and `ConceptId`
- `crates/graph/src/types.rs`
  - `Topic.topic_id: TopicId`
  - `Topic.document_id: DocumentId`
  - `Concept.concept_id: ConceptId`
  - `Concept.topic_id: TopicId`
- `crates/storage/src/topics.rs`
  - `TopicRow.topic_id: TopicId`
- `crates/storage/src/concepts.rs`
  - `ConceptRow.concept_id: ConceptId`
  - `ConceptRow.topic_id: TopicId`

Intentionally left as `String` for compatibility or later slices:

- `common::types::Document.document_id`
- `common::types::Chunk.chunk_id`
- `common::types::Chunk.document_id`
- `storage::embeddings::HierarchicalChunkEmbedding.topic_id`
- `storage::embeddings::HierarchicalChunkEmbedding.concept_id`
- `storage::chunks` hierarchy write parameters
- Engine and CLI ID fields/callers
- All `created_at` fields

## C9-4 Tests Added Or Updated

- `crates/common/src/types.rs`
  - `topic_id_has_string_transparent_foundation_traits`
  - `concept_id_has_string_transparent_foundation_traits`
- `crates/graph/src/hierarchy.rs`
  - updated `test_ids_are_unique`
  - `test_graph_ids_are_typed_and_serialize_as_strings`
- `crates/storage/src/topics.rs`
  - updated `test_insert_and_get_topic`
  - renamed/extended `test_topic_row_decodes_ids_as_typed_ids_and_preserves_storage_strings`
- `crates/storage/src/concepts.rs`
  - updated `test_insert_and_get_concept`
  - updated `test_list_concepts_by_topic`
  - `test_concept_row_decodes_ids_as_typed_ids_and_preserves_storage_strings`

## C9-4 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p common string_transparent_foundation_traits` | passed | Focused common typed-ID tests passed: 5 passed, 0 failed. |
| `cargo test -p graph` | passed | Graph crate tests passed: 16 unit tests passed, including graph typed-ID JSON/string compatibility. |
| `cargo test -p storage typed_id` | passed | Focused storage typed-ID tests passed: 3 passed, 0 failed, covering C9-2 and C9-4 storage row typed IDs. |
| `cargo test` | passed | Workspace tests passed across CLI, common, config, engine, graph, ingest, providers, retrieval, and storage; provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |
| `cargo fmt --check` | passed | Formatting check completed with no output after running `cargo fmt`. |

## C9-4 Self-Review

Reviewed the C9-4 diff against the project `code-quality-review` skill and Rust idioms:

- New graph ID types reuse the existing C9-1 macro, avoiding duplicated trait implementations and keeping behavior consistent.
- Graph/storage conversions are explicit at construction and row-decoding boundaries; storage write/query APIs remain string-shaped.
- The only new dependency is `serde_json` as a graph dev-dependency for serialization tests; production graph dependencies are unchanged.
- Ingest fallout was limited to compatibility conversions for internal topic-boundary strings; no engine/CLI/API behavior was broadened.
- `created_at` fields and retrieval hierarchical metadata remain untouched to preserve C9-4 scope.

## Residual Risks

- C9-4 changes public graph/storage row field types for selected topic/concept IDs; workspace tests pass, but external consumers comparing directly to `String` must use `as_ref()`/`Display` or typed IDs directly.
- `health --json` still performs provider network checks by approved user decision; users needing local-only health would require a future behavior change.
- `selected_provider_model()` intentionally preserves custom model when the selected provider matches existing config; unknown providers fall back to the existing configured model.
- CR-2 intentionally corrected only CodeRabbit-listed documentation/archive findings. Other archived reports may still contain historical first-pass claims outside this slice.
- C9-1/C9-2 keep IDs string-transparent and infallible; if later slices require format validation, that will be a separate design decision because it can change parsing/error behavior.
- C9-2 changes public storage row field types for `TopicRow` and `SemanticLinkRow`; workspace tests pass, but downstream external consumers would need to use `as_ref()`/`Display` or typed IDs directly.
- C9-5a migrates public engine domain types to typed IDs; downstream consumers comparing to `String` must use `.into()`, `as_ref()`, or typed constructors.

## C9-5a Status — Engine/Common Typed-ID Migration

| Item | Status | Evidence |
| --- | --- | --- |
| Common domain types migrated | `completed` | `Document.document_id`, `Chunk.chunk_id`, `Chunk.document_id`, `Citation.document_id`, `Citation.chunk_id`, `TraceHeaderInput.trace_id`, `TraceHeaderRecord.trace_id`, `TraceCitationRecord.trace_id/document_id/chunk_id`, `ContextResponse.trace_id` all use typed IDs. |
| Engine types migrated | `completed` | `IngestResult.document_id` as `DocumentId`, `Hit.chunk_id`/`document_id` as `ChunkId`/`DocumentId`. |
| Out-of-scope types preserved | `n/a` | `ContextMetadata` has no ID fields requiring typed IDs. `ReadResponse.document_id`/`chunk_id`/`trace_id` remain `String`. `Hit` has no `trace_id` field. |
| CLI output boundaries | `completed` | All `output.rs` and command files use `.as_ref()` or `.to_string()` at serialization boundaries. |
| Storage test fixup | `deferred` | 46 storage test compilation errors remain — mechanical `.into()` fixes (C9-5b). |

## C9-5a Exact Boundary Map

Migrated in this slice:

- `crates/common/src/types.rs`
  - `Document.document_id: DocumentId`
  - `Chunk.chunk_id: ChunkId`
  - `Chunk.document_id: DocumentId`
  - `Citation.document_id: DocumentId`
  - `Citation.chunk_id: ChunkId`
  - `TraceHeaderInput.trace_id: TraceId`
  - `TraceHeaderRecord.trace_id: TraceId`
  - `TraceCitationRecord.trace_id: TraceId`
  - `TraceCitationRecord.document_id: DocumentId`
  - `TraceCitationRecord.chunk_id: ChunkId`
  - `ContextResponse.trace_id: TraceId`
- `crates/engine/src/ingest.rs`
  - `IngestResult.document_id: DocumentId`
- `crates/engine/src/retrieve.rs`
  - `Hit.chunk_id: ChunkId`
  - `Hit.document_id: DocumentId`
- `crates/engine/src/context.rs`
  - Context metadata/response typed-ID conversions
- `crates/engine/src/refresh.rs`
  - Refresh result typed-ID conversions
- `crates/engine/src/recovery.rs`
  - Recovery result typed-ID conversions
- `crates/cli/src/output.rs`
  - All JSON output uses `.as_ref()` / `.to_string()` at serialization boundaries
- `crates/cli/src/commands/*.rs`
  - Command output boundaries use typed-ID string accessors

Intentionally left as `String`:

- `ContextMetadata.excluded_non_ready_document_ids: Vec<String>`
- `ReadResponse.document_id`, `ReadResponse.chunk_id`, `ReadResponse.trace_id`
- `storage::embeddings::HierarchicalChunkEmbedding.topic_id`
- `storage::embeddings::HierarchicalChunkEmbedding.concept_id`
- `created_at` fields

## C9-5a Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo check` | passed | Workspace compiles clean with zero errors. |
| `cargo test -p common` | passed | Common crate tests passed. |
| `cargo test -p engine` | passed | Engine crate tests passed. |
| `cargo fmt --check` | passed | Formatting check completed with no output. |

## C9-5b Status — Storage Test Fixup

| Item | Status | Evidence |
| --- | --- | --- |
| 46 storage test `.into()` fixes | `completed` | All `E0308` mismatched types errors in storage tests resolved by adding `.into()` to String expressions that need typed IDs. 8 files fixed: chunks.rs, concepts.rs, documents.rs, embeddings.rs, semantic_links.rs, topics.rs, traces.rs, lib.rs. |
| 1 ingest test `.into()` fix | `completed` | Additional `E0308` in `ingest/src/lib.rs:199` test helper fixed with `.into()`. |
| Full workspace validation | `completed` | `cargo test` passes 318 tests, 0 failures. `cargo clippy -- -D warnings` clean. `cargo fmt --check` clean. |

## C9-5b Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo check -p storage --tests` | passed | Zero compilation errors in storage tests. |
| `cargo test -p storage` | passed | 89 unit tests passed, 0 failed. |
| `cargo test` | passed | Workspace: 318 passed, 0 failed, 13 ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy clean, zero warnings. |
| `cargo fmt --check` | passed | Formatting check clean after `cargo fmt`. |

## C9-5a/b Self-Review

Reviewed the C9-5a/b diff against the project `code-quality-review` skill:

- Engine domain types now carry typed IDs that serialize identically to strings (serde transparent), preserving API compatibility.
- CLI output boundaries use `.as_ref()` / `.to_string()` at serialization edges, keeping JSON output unchanged.
- Storage test fixes are strictly mechanical `.into()` calls in `#[cfg(test)]` modules; no production code altered in C9-5b.
- The ingest test fix (`lib.rs:199`) was caught during full workspace validation, same pattern.
- Simplified redundant `DocumentId::from(hit.document_id.clone())` → `hit.document_id_typed.clone()` in `context.rs` using ScoredChunk's already-typed fields.
- Corrected boundary map: `ContextMetadata` has no ID fields, `ReadResponse` fields remain String, `Hit` has no `trace_id`.
- No new dependencies, no behavior changes, no validation semantics added.

## C9-6 Status — Typed-ID Cleanup And Public API Audit

| Item | Status | Evidence |
| --- | --- | --- |
| Workspace raw-ID audit | `completed` | Audited `document_id: String`, `chunk_id: String`, `trace_id: String`, typed ID fields, typed ID mirrors, and typed-ID `.to_string()` call sites across `crates/**/*.rs`. Remaining raw strings are classified below rather than left ambiguous. |
| Redundant conversion cleanup | `completed` | Simplified `engine::retrieve::Hit::from_scored_chunk()` to move `ScoredChunk.chunk_id_typed` / `document_id_typed` directly instead of rebuilding IDs from string mirrors. |
| Public API invariant tests | `no gap found` | Existing C9-1/C9-3/C9-4 tests already cover serde-transparent string representation and typed/string mirror invariants for public migrated types. No new test gap was found for this cleanup-only change. |
| Out-of-scope guard | `preserved` | No SNAP-1, SNAP-2, or TIME-1 files/requirements were implemented in this slice. |

## C9-6 Boundary Map

### Migrated typed-ID boundaries

- `common::types::Document.document_id: DocumentId`
- `common::types::Chunk.chunk_id: ChunkId`
- `common::types::Chunk.document_id: DocumentId`
- `common::types::Citation.document_id: DocumentId`
- `common::types::Citation.chunk_id: ChunkId`
- `common::types::ContextResponse.trace_id: TraceId`
- `common::types::TraceHeaderInput.trace_id: TraceId`
- `common::types::TraceHeaderRecord.trace_id: TraceId`
- `common::types::TraceCitationRecord.trace_id: TraceId`
- `common::types::TraceCitationRecord.document_id: DocumentId`
- `common::types::TraceCitationRecord.chunk_id: ChunkId`
- `graph::types::Topic.topic_id: TopicId`
- `graph::types::Topic.document_id: DocumentId`
- `graph::types::Concept.concept_id: ConceptId`
- `graph::types::Concept.topic_id: TopicId`
- `storage::documents::DocumentRow.document_id: DocumentId`
- `storage::chunks::ChunkRow.chunk_id: ChunkId`
- `storage::chunks::ChunkRow.document_id: DocumentId`
- `storage::topics::TopicRow.topic_id: TopicId`
- `storage::topics::TopicRow.document_id: DocumentId`
- `storage::concepts::ConceptRow.concept_id: ConceptId`
- `storage::concepts::ConceptRow.topic_id: TopicId`
- `storage::semantic_links::SemanticLinkRow.source_chunk_id: ChunkId`
- `storage::semantic_links::SemanticLinkRow.target_chunk_id: ChunkId`
- `storage::traces` row decoding for trace citations and headers returns typed `TraceId` / `DocumentId` / `ChunkId` records.
- `storage::embeddings::ChunkEmbeddingRecord.chunk_id_typed: ChunkId` and `document_id_typed: DocumentId` mirror decoded strings for retrieval compatibility.
- `retrieval::ScoredChunk.chunk_id_typed: ChunkId` and `document_id_typed: DocumentId` mirror public string fields for compatibility.
- `engine::ingest::IngestResult.document_id: DocumentId`
- `engine::retrieve::Hit.chunk_id: ChunkId`
- `engine::retrieve::Hit.document_id: DocumentId`

### Intentionally string-boundary fields

- CLI argument structs in `crates/cli/src/commands/{get,ingest,retrieve,retry,search,trace}.rs` remain `String` because clap parses user-supplied command-line text at the external boundary.
- CLI JSON DTOs in command/output modules keep `String` IDs where they are serialization/output compatibility shapes.
- `common::types::ReadSelector::{Citation.trace_id, Chunk.document_id, Chunk.chunk_id}` remains `String` because it models mutually-exclusive read command selectors from user input.
- `common::types::ReadResponse.{document_id,chunk_id,trace_id}` remains `String` to preserve the read command response contract recorded in C9-5a.
- `common::types::TraceResponse.trace_id` and `document_ids: Vec<String>` remain `String` because trace output stores aggregate ID lists from persisted comma-separated trace metadata.
- `common::error::{DocumentNotFound,DocumentNotReady,TraceNotFound,ChunkNotFound}` keeps `String` payloads because errors serialize/report external identifiers and do not participate in typed domain flow.
- `storage::embeddings::ChunkEmbeddingRecord.{chunk_id,document_id}` and `retrieval::ScoredChunk.{chunk_id,document_id}` remain `String` as compatibility mirrors alongside typed ID fields.
- Local SQLite row extraction variables such as `let document_id: String = row.get(...)` remain raw strings at persistence decode boundaries before conversion.

### Deferred ID-related leftovers

- `storage::embeddings::HierarchicalChunkEmbedding.{topic_id,concept_id}: Option<String>` remains deferred for a future graph/retrieval API cleanup because it would broaden C9 beyond `DocumentId`/`ChunkId`/`TraceId` and affect hierarchy metadata consumers.
- `storage::snapshots` snapshot ID and document member lists remain string-boundary/deferred because SNAP-1/SNAP-2 are separate slices and snapshot IDs do not yet have a dedicated newtype.
- Persisted trace `document_ids` / `citation_ids` aggregate columns remain strings because they are comma-separated audit metadata, not typed row references.
- ID format validation remains deferred by design: all C9 IDs are string-transparent and infallible; adding validation would be a separate compatibility-affecting decision.

## C9-6 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `grep -n "\\b(document_id\\|chunk_id\\|trace_id)\\s*:\\s*String\\b" crates/**/*.rs` | passed | Identified remaining raw ID fields and classified them as CLI/output, error payload, compatibility mirror, row extraction, or deferred boundary. |
| `grep -n "\\b(document_id\\|chunk_id\\|trace_id)\\s*:\\s*\\(DocumentId\\|ChunkId\\|TraceId\\)\\b" crates/**/*.rs` | passed | Confirmed migrated typed-ID boundaries across common, graph, storage, and engine. |
| `grep -n "\\b(document_id\\|chunk_id\\|trace_id)_typed\\s*:" crates/**/*.rs` | passed | Confirmed storage/retrieval typed mirror fields used for compatibility. |
| `cargo test` | passed | Workspace tests passed: CLI 23, common 16, config 11, engine 53 + integration, graph 16, ingest 56 + integration, providers 12 passed/2 ignored, retrieval 16, storage 89, doc tests passed/ignored as configured. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |
| `cargo fmt --check` | passed | Formatting check completed with no output. |

## C9-6 Self-Review

Reviewed C9-6 against the project `code-quality-review` skill:

- Cleanup stayed local and avoided broad public API churn; only one redundant conversion in `engine::retrieve` was removed.
- Remaining `String` fields are either external DTO/CLI/error contracts, row-boundary raw values, compatibility mirrors, or explicitly deferred leftovers.
- No tests were added because existing typed-ID serde/string-invariant tests already cover the public invariants this slice audits.
- No SNAP or TIME files were edited.

## SNAP-1 Status — Snapshot Activation Rollback Confidence

| Item | Status | Evidence |
| --- | --- | --- |
| Partial activation rollback regression | `completed` | Added `storage::snapshots::tests::test_activate_snapshot_rolls_back_after_pointer_update_failure`, which simulates a failure after superseding the previous snapshot, marking the new snapshot active, and upserting `snapshot_pointer` inside an uncommitted SQLite transaction. Dropping the transaction on the injected error rolls back all partial writes. |
| Previous active pointer remains current | `completed` | After the injected failure, `get_active_snapshot_id()` still returns `snap-1` and `snap-1` remains in `active` state. |
| Failed/new snapshot is not visible as current | `completed` | After the injected failure, `snap-2` remains in `building` state and `snapshot_pointer` does not reference it. |
| Successful activation still commits atomically | `completed` | The same test retries normal `activate_snapshot("snap-2")` after the rollback and verifies `snap-2` becomes active while `snap-1` becomes superseded. Existing `test_activate_snapshot_atomic_no_mixed_visibility` also remains in place. |
| Production transaction change | `not needed` | Current SQLite transaction behavior passed the regression test. SNAP-1 added test-only failure simulation and did not change production activation code. |
| Out-of-scope guard | `preserved` | No `snapshot_pointer.updated_at` migration, timestamp type migration, or TIME-1 files were implemented in this slice. |

## SNAP-1 Tests Added Or Updated

- `crates/storage/src/snapshots.rs`
  - `test_activate_snapshot_rolls_back_after_pointer_update_failure`
  - test-only helpers: `simulate_activation_failure_after_pointer_update`, `snapshot_state`

## SNAP-1 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p storage test_activate_snapshot_rolls_back_after_pointer_update_failure` | passed | Initial focused rollback regression passed: injected failure after pointer update rolled back and successful retry committed atomically. |
| `cargo fmt && cargo test -p storage snapshots::tests::test_activate_snapshot_rolls_back_after_pointer_update_failure` | passed | Formatting applied, then focused namespaced storage snapshot rollback test passed. |
| `cargo test` | passed | Workspace tests passed; storage now reports 90 unit tests, including the SNAP-1 rollback regression; provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |
| `cargo fmt --check` | passed | Formatting check completed with no output. |

## SNAP-1 Structured Status And Action Context

Consumed task status for this delegated slice:

- `changeName`: `error-remediation-v3`
- `changeRoot`: `openspec/changes/active/error-remediation-v3`
- `artifactStore`: `both` requested; OpenSpec updated in this executor, Engram tool unavailable here.
- `applyState`: ready for assigned work-unit slice SNAP-1 by explicit parent/user approval.
- `actionContext.mode`: repo-local
- `workspaceRoot` / allowed edit root: `E:/Proyectos/Intento_de_conseguir_pega/aiharness`
- Edited files are inside the authoritative workspace.
- Review workload gate: full V3 has high 400-line risk and chained PRs are recommended; this executor implemented only the assigned SNAP-1 slice.

## SNAP-1 Workload / PR Boundary

SNAP-1 is a test-only storage slice plus OpenSpec tracking updates. It stays within the planned 400-line review budget and deliberately does not include SNAP-2 or TIME-1.

## SNAP-2 Status — Snapshot Pointer `updated_at` Migration

| Item | Status | Evidence |
| --- | --- | --- |
| Additive schema migration | `completed` | Added migration 008 (`crates/storage/src/migrations/008_snapshot_pointer_updated_at.sql`) to `ALTER TABLE snapshot_pointer ADD COLUMN updated_at TEXT`, then backfill existing rows with `datetime('now')`. The timestamp format matches `storage::util::format_dt` / `parse_dt` (`%Y-%m-%d %H:%M:%S`). |
| Old-schema compatibility | `completed` | Added `test_snapshot_pointer_old_schema_migration_adds_parseable_updated_at`, which creates a version-7 database with the old two-column `snapshot_pointer`, opens it through `Database::open()`, verifies the active pointer is preserved, and parses the migrated `updated_at` value. |
| Pointer activation/update refresh | `completed` | Updated `activate_snapshot()` to insert or update `snapshot_pointer.updated_at` using the same activation timestamp as snapshot state changes. Added `test_activate_snapshot_refreshes_pointer_updated_at`, which forces a stale pointer timestamp and verifies a later activation refreshes it to a parseable non-stale value. |
| Rollback/downgrade note | `documented` | Migration is additive and nullable to avoid a table rebuild for existing databases. Code rollback tolerates the extra column because production reads select named columns; database downgrade is not automatic. |
| Out-of-scope guard | `preserved` | No TIME-1 `created_at` `DateTime<Utc>` migration was implemented in this slice. |

## SNAP-2 Tests Added Or Updated

- `crates/storage/src/snapshots.rs`
  - `test_snapshot_pointer_old_schema_migration_adds_parseable_updated_at`
  - `test_activate_snapshot_refreshes_pointer_updated_at`
  - test-only helpers: `snapshot_pointer_updated_at`, `unique_temp_dir`
- `crates/storage/src/migrations/mod.rs`
  - registered migration 008
- `crates/storage/src/migrations/008_snapshot_pointer_updated_at.sql`
  - additive `updated_at` column and backfill

## SNAP-2 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p storage snapshot_pointer -- --nocapture` | failed as RED evidence, then failed once during test cleanup | First failed before the migration existed with `no such column: updated_at`, proving the old schema lacked the required column. After implementation, the same filter exposed a Windows file-lock cleanup issue from deleting the temp database while `Database` was still open; fixed by dropping the database before `remove_dir_all`. |
| `cargo test -p storage snapshots::tests::test_snapshot_pointer_old_schema_migration_adds_parseable_updated_at snapshots::tests::test_activate_snapshot_refreshes_pointer_updated_at` | failed | Invalid Cargo syntax for multiple test filters; superseded by the passing `snapshots::tests` focused run. |
| `cargo test -p storage snapshots::tests -- --nocapture` | passed | All 14 snapshot tests passed, including old-schema migration, pointer timestamp refresh, and SNAP-1 rollback coverage. |
| `cargo test` | passed | Workspace tests passed; storage now reports 92 unit tests including SNAP-1/SNAP-2 snapshot coverage. Provider network tests remained ignored. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |
| `cargo fmt --check` | passed | Formatting check completed with no output after `cargo fmt`. |

## SNAP-2 Structured Status And Action Context

Consumed task status for this delegated slice:

- `changeName`: `error-remediation-v3`
- `changeRoot`: `openspec/changes/active/error-remediation-v3`
- `artifactStore`: `both` requested; OpenSpec updated in this executor, Engram tool unavailable here.
- `applyState`: ready for assigned work-unit slice SNAP-2 by explicit parent/user approval.
- `actionContext.mode`: repo-local
- `workspaceRoot` / allowed edit root: `E:/Proyectos/Intento_de_conseguir_pega/aiharness`
- Edited files are inside the authoritative workspace.
- Review workload gate: full V3 has high 400-line risk and chained PRs are recommended; this executor implemented only the assigned SNAP-2 slice.

## SNAP-2 Workload / PR Boundary

SNAP-2 changes are limited to snapshot storage migration/activation code, focused snapshot tests, and V3 tracking artifacts. The slice is intended as its own review unit and remains under the 400-line budget.

## TIME-1 Status — `created_at` DateTime<Utc> Consistency

| Item | Status | Evidence |
| --- | --- | --- |
| `graph::types::Topic.created_at` | `completed` | Migrated from `String` to `chrono::DateTime<Utc>`. Added local serde helpers that serialize/deserialize the existing SQLite-style external format (`%Y-%m-%d %H:%M:%S`) instead of chrono's RFC3339 default. |
| `graph::types::Concept.created_at` | `completed` | Migrated from `String` to `chrono::DateTime<Utc>` with the same explicit serde format boundary. `build_hierarchy()` now stores `Utc::now()` directly. |
| `storage::semantic_links::SemanticLinkRow.created_at` | `completed` | Migrated from `String` to `chrono::DateTime<Utc>` and centralized semantic-link row decoding through `row_to_semantic_link()`, which parses stored SQLite TEXT with `storage::util::parse_dt()`. |
| Invalid timestamp rejection | `completed` | Added graph JSON deserialization coverage for invalid topic timestamps and storage row decoding coverage proving an invalid semantic-link `created_at` returns a storage parse error. |
| External timestamp format | `preserved` | Graph JSON and semantic-link SQLite storage continue to use `%Y-%m-%d %H:%M:%S`; no schema migration was required. |
| CLI/output path discovery | `no direct touch needed` | `created_at` grep in CLI paths showed document/list output and fixtures, not graph `Topic`/`Concept` or semantic-link output. No CLI output path consumes the migrated selected records in this slice. |
| Scope guard | `preserved` | Did not migrate unrelated storage `TopicRow` / `ConceptRow` `created_at` fields or broader document/chunk/backlog timestamps. |

## TIME-1 Tests Added Or Updated

- `crates/graph/src/types.rs`
  - `topic_created_at_serializes_in_sqlite_timestamp_format`
  - `concept_created_at_deserializes_valid_sqlite_timestamp`
  - `topic_created_at_rejects_invalid_timestamp`
- `crates/storage/src/semantic_links.rs`
  - `test_semantic_link_row_decodes_created_at_as_datetime`
  - `test_semantic_link_row_rejects_invalid_created_at`

## TIME-1 Command Evidence

| Command | Result | Summary |
| --- | --- | --- |
| `cargo test -p graph created_at -- --nocapture` | passed | Focused graph timestamp tests passed: valid deserialize, invalid rejection, and stable SQLite-format JSON serialization. |
| `cargo test -p storage semantic_link_row -- --nocapture` | passed | Focused semantic-link storage tests passed, including typed chunk IDs plus valid/invalid `created_at` DateTime decoding. |
| `cargo test` | passed | Workspace tests passed: CLI 23, common 16, config 11, engine 53 + integration, graph 19, ingest 56 + integration, providers 12 passed/2 ignored, retrieval 16, storage 94, doc tests passed/ignored as configured. |
| `cargo clippy -- -D warnings` | passed | Workspace clippy completed with no warnings. |
| `cargo fmt --check` | passed | Formatting check completed with no output after `cargo fmt`. |

## TIME-1 Structured Status And Action Context

Consumed task status for this delegated slice:

- `changeName`: `error-remediation-v3`
- `changeRoot`: `openspec/changes/active/error-remediation-v3`
- `artifactStore`: `both` requested; OpenSpec updated in this executor. Engram persistence was not available in this subagent toolset, so memory save is reported inline.
- `applyState`: ready for assigned work-unit slice TIME-1 by explicit parent/user approval.
- `actionContext.mode`: repo-local
- `workspaceRoot` / allowed edit root: `E:/Proyectos/Intento_de_conseguir_pega/aiharness`
- Edited files are inside the authoritative workspace.
- Review workload gate: full V3 has high 400-line risk and chained PRs are recommended; this executor implemented only the assigned TIME-1 slice.

## TIME-1 Workload / PR Boundary

TIME-1 is limited to graph domain timestamp fields, semantic-link storage row timestamp decoding, focused tests, and V3 tracking artifacts. The code diff before OpenSpec tracking updates was 228 insertions / 52 deletions across three Rust files, staying under the 400-line review budget.

## Remaining Tasks After TIME-1

- No implementation tasks remain unchecked in `tasks.md` for the approved V3 slices. Final verify/sync/archive remain separate SDD phases.

## Residual Risks (updated)

- TIME-1 intentionally leaves storage `TopicRow.created_at` and `ConceptRow.created_at` as `String`; those row types were outside the selected scope and can be migrated separately if needed.
- C9-5a migrates public engine domain types to typed IDs; downstream consumers comparing to `String` must use `.into()`, `as_ref()`, or typed constructors.
- `health --json` still performs provider network checks by approved user decision; users needing local-only health would require a future behavior change.
- `selected_provider_model()` intentionally preserves custom model when the selected provider matches existing config; unknown providers fall back to the existing configured model.
- CR-2 intentionally corrected only CodeRabbit-listed documentation/archive findings. Other archived reports may still contain historical first-pass claims outside this slice.
- C9-1/C9-2 keep IDs string-transparent and infallible; if later slices require format validation, that will be a separate design decision because it can change parsing/error behavior.
