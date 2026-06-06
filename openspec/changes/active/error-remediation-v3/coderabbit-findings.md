Verify each finding against current code. Fix only still-valid issues, skip the
rest with a brief reason, keep changes minimal, and validate.

In `@crates/cli/src/commands/health.rs` around lines 53 - 54, The help text claims
`--json` is local-only but execute() still runs provider network checks via
check_provider and embed("test"); either update the wording or implement a true
local-only path: in execute() (the command entry) detect the json/local-only
flag and if set skip invoking check_provider and any embed("test") provider
calls, instead populate or return only local fields, and update the command
help/comment to reflect the actual behavior; reference the execute(),
check_provider, and embed("test") calls to locate and change the logic or the
documentation accordingly.


In `@crates/cli/src/commands/setup.rs` around lines 59 - 63, The code always uses
config.embedding.model (variable model) regardless of the selected provider,
causing invalid provider/model pairings; update the logic in the setup flow so
that before calling test_provider_connection and before persisting the config
you determine the model from the selected provider (e.g., derive a
provider-specific model variable from the chosen provider selection or provider
metadata instead of using config.embedding.model), replace usages of
config.embedding.model with that derived model when invoking
test_provider_connection and when saving (refer to the variable model, the
provider variable, the test_provider_connection call, and the code paths that
persist embedding model), and ensure the same derived model is used consistently
in all places currently reusing config.embedding.model.


In `@crates/config/src/lib.rs` around lines 505 - 511, The tests
test_env_embedding_timeout_overridden (and the other test covering lines
~629-638) mutate CITE_* environment variables without restoring prior values;
update each test to capture the original values (e.g., via Option<String> =
std::env::var("CITE_...").ok()), set the env var for the test, then in a
finally/teardown block restore the original with std::env::set_var when Some or
std::env::remove_var when None, matching the pattern used by other tests in this
file so existing environment state is preserved.


In `@crates/config/src/lib.rs` around lines 531 - 537, The tests (e.g.,
test_env_invalid_top_k_falls_back_to_default and the other test at 629-636) call
Config::load() which merges a host default TOML and makes assertions brittle;
change these tests to call Config::load_from(Some(nonexistent_path)) or
load_from a temp file to isolate from host config, keep the ENV_MUTEX and the
CITE_TOP_K env var manipulation as-is (save/restore orig), and assert
retrieval.top_k == 5 against the isolated config loader so the fallback behavior
is deterministic.


In `@crates/retrieval/src/lib.rs` around lines 123 - 142, The hot path is cloning
ChunkEmbeddingRecord.vector because the code uses candidate.clone().into() and
the existing impl From<ChunkEmbeddingRecord> for ScoredChunk consumes/duplicates
the whole record; to fix, add impl From<&ChunkEmbeddingRecord> for ScoredChunk
that copies only the scalar fields (leave out or ignore vector) and update
rank_by_similarity to convert from a reference (use candidate_ref.into() or
&candidate.into()) instead of cloning the whole record; ensure the new impl
references the same field names (ChunkEmbeddingRecord.vector, chunk_id,
document_id, display_name, section_id, chunk_index, text, page, offset_start,
offset_end) and remove the candidate.clone() call so embeddings are not
duplicated on the hot path.


In `@crates/storage/src/rate_limits.rs` around lines 96 - 106,
prune_stale_rate_limits currently accepts non-positive max_age_seconds which can
make cutoff >= now and delete active windows; add an early validation in
prune_stale_rate_limits to reject max_age_seconds <= 0 and return an appropriate
CiteError (e.g. an InvalidArgument/BadRequest-style error) instead of performing
the DELETE, so only positive ages are allowed; reference the function name
prune_stale_rate_limits, the parameter max_age_seconds, and the cutoff
calculation when adding the check and error return.



In `@openspec/changes/active/error-remediation-v2/apply-progress.md` at line 5,
Update the status line "✅ ALL 6 PRs APPLIED" to reflect the correct number of
PRs by cross-checking the waves listed (Wave 1 through Wave 7) and the commit
list (lines 237-245) — if there are 7 commits/waves, change the status to "✅ ALL
7 PRs APPLIED" (or adjust the commit list/wave headings to match 6 if that is
correct); ensure the summary, the "Status:" line, and the commit list are
consistent.


In `@openspec/changes/active/error-remediation/second-pass-prompt.md` around lines
13 - 38, The summary count "78 errores restantes (T3+T4) + 11 casts" is
inconsistent with the T3/T4 breakdown (≈37 + ≈38 = ≈75); update the document so
totals match by recalculating the T3 and T4 counts and either (a) change the
header string "78 errores restantes (T3+T4) + 11 casts" to the correct total, or
(b) adjust the detailed counts in the T3/T4 tables to sum to 78; make the edit
in this file around the header and the "T3: Medium" / "T4: Low" sections (search
for the exact text "78 errores restantes (T3+T4) + 11 casts", "T3: Medium (37
errores)", and "T4: Low (38 errores)") so the summary and breakdown are
consistent.



In `@openspec/reports/archive/revision-repo/cli/errores.md` around lines 11 - 39,
The archived report incorrectly states that
engine::runtime_guard::check_ingest_allowed is never invoked; in fact
crates/cli/src/commands/ingest.rs calls check_ingest_allowed in execute() and
the runtime guard is enforced. Update errores.md to mark this item as resolved
(add a timestamped note or move it to the "Completados" table) and clarify that
check_ingest_allowed is invoked by the CLI's execute() path and also mention the
engine-side guard location (engine::ingest::ingest_internal) if applicable so
the archive reflects the current code.



In `@openspec/reports/archive/revision-repo/compliance/review.md` around lines 14
- 24, Update the archived compliance note to correct the factual error: state
that engine::runtime_guard::check_ingest_allowed is invoked by the CLI ingest
path (the ingest command calls engine::runtime_guard::check_ingest_allowed with
the runtime mode and handles errors), so production/public-demo ingestion is
blocked at runtime; replace the “never invoked” claim with a brief note pointing
to the ingest command's runtime-mode check and optionally add a cross-reference
to the check_ingest_allowed tests to show it is exercised.


In `@openspec/reports/archive/revision-repo/engine/errores.md` around lines 10 -
40, Summary: The doc incorrectly states the CLI doesn't call
check_ingest_allowed; it does, but engine internals don't re-check it. Update
errores.md to state that check_ingest_allowed (function check_ingest_allowed) is
invoked by the CLI (crates/cli/src/commands/ingest.rs) before
engine::ingest::ingest, but engine internal entry points (ingest_next /
ingest_internal in the engine crate) do not perform the guard; note the risk of
bypassing the CLI and recommend either adding runtime checks in
ingest_next/ingest_internal or documenting the intended API boundary. Reference
the symbols check_ingest_allowed, ingest_next, ingest_internal, and
engine::ingest::ingest in the updated text.


In `@openspec/reports/archive/revision-repo/engine/review.md` around lines 312 -
316, Update the review text to correct the inaccuracy: state that the CLI
(crates/cli/src/commands/ingest.rs) does call
engine::runtime_guard::check_ingest_allowed(&config.runtime.mode) before
enqueueing ingests, but the engine worker functions ingest_next and
ingest_internal in engine/src/ingest.rs do not perform the runtime guard check
themselves; keep note that runtime_guard is otherwise used (is_real_provider) in
main.rs for the provider disclosure banner.


In `@openspec/reports/archive/revision-repo/graph/errores.md` around lines 10 -
40, The documentation incorrectly claims the bug (using line.len() for char
offsets) still exists in crates/graph/src/heading_parser.rs; instead update
errores.md to reflect that heading_parser.rs now uses char_offset +=
line.chars().count() + 1 (so the fix has been applied), remove or change the
references to lines 17 and 35 (or mark them as historical), and add a note
recommending adding a UTF-8 multi-byte test (e.g., extend or add
test_char_offsets) to prevent regressions; ensure you reference the symbol
char_offset and the file heading_parser.rs in the updated text.


In `@openspec/reports/archive/revision-repo/graph/review.md` around lines 77 - 82,
Update the documentation to match the implementation: change the statement that
`char_offset` is accumulated using `line.len() + 1` (bytes + newline) to state
it uses `line.chars().count() + 1` (character count + newline) as implemented in
heading_parser (see symbol `char_offset` update in heading_parser.rs); ensure
the wording clarifies it counts Unicode scalar characters rather than raw bytes.


In `@openspec/reports/archive/revision-repo/ingest/errores.md` around lines 11 -
36, The documentation entry claiming a UTF-8 panic in sanitize_display_name is
stale: the current implementation in crates/ingest/src/validator.rs already uses
trimmed.chars().take(255).collect() (safe char-based truncation) rather than
byte slicing; update or remove the errored doc block in
openspec/reports/archive/revision-repo/ingest/errores.md to reflect that
sanitize_display_name implements safe character truncation, and optionally
add/adjust a test (e.g., test_sanitize_display_name_truncation) to include
multi-byte characters (emoji/CJK) to prevent regression.


In `@openspec/reports/archive/revision-repo/ingest/errores.md` around lines 39 -
57, The documentation incorrectly states that extract_plain_text uses
content.len() for total_chars; update the errores.md entry to reflect that in
crates/ingest/src/extractor.rs the total_chars variable is already computed with
content.chars().count() (and verify/mention that extract_pdf_text uses the same
chars().count() fix if applicable), removing or correcting the stale claim about
bytes vs chars for total_chars and any suggested code change.


In `@openspec/reports/archive/revision-repo/ingest/errores.md` around lines 60 -
73, The documentation entry in errores.md incorrectly reports a bug where
heading_parser.rs uses line.len() for char offsets; in fact heading_parser.rs
already uses line.chars().count() to update char_offset (see the char_offset
update and HeadingSpan.char_offset) so the doc entry is obsolete—remove or
update that entry to state the issue is fixed (or rename to a historical note),
and if keeping a note, mention the corrected code path and that lib.rs uses
HeadingSpan.char_offset when building topic_boundaries so no further changes to
heading_parser.rs are required.


In `@openspec/reports/archive/revision-repo/ingest/review.md` around lines 98 -
101, The documentation incorrectly states that extract_plain_text uses
content.len() (byte count) for total_chars; update the doc to match the
implementation by noting that extract_plain_text computes total_chars via
content.chars().count() (Unicode character count) and returns a single PageText
(page=1) with full file content; reference the extract_plain_text function and
the total_chars calculation (content.chars().count()) when updating the wording.


In `@openspec/reports/archive/revision-repo/providers/errores.md` around lines 11
- 50, The documented bug in errores.md is outdated: it claims create_provider
uses resolve_api_key(...).unwrap_or_default() causing empty API keys, but the
actual code in create_provider already validates via
resolve_api_key(...).ok_or_else(...); update errores.md to remove or correct
that claim (reference create_provider and resolve_api_key) and adjust
severity/description accordingly; also remove or explain the contradictory note
"Este archivo NO se sube a GitHub" since this file is included in the PR,
ensuring the document matches the current code state and points readers to the
real defensive checks in the providers (e.g., GeminiProvider::new /
OpenAICompatibleProvider::new) if relevant.