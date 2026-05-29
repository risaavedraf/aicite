# Config Hierarchy Specification

## Purpose

Define the configuration additions required for Phase 10 — Hierarchical Graph Foundation. This includes new fields in the `IngestConfig` struct and corresponding environment variable overrides to enable sentence-based chunking and hierarchical document structure during ingest.

## Requirements

### Requirement: Config crate MUST add `sentence_chunking` field (MUST)

The `IngestConfig` struct MUST include a new field:
```rust
pub sentence_chunking: bool
```

Default value: `false`

This field controls whether the ingest pipeline uses sentence-based chunking (`chunk_by_sentence`) instead of the existing fixed-size chunking (`chunk_text`).

#### Scenario: Default config has sentence_chunking disabled

- GIVEN a default `IngestConfig` instance
- WHEN the config is inspected
- THEN `sentence_chunking` is `false`

#### Scenario: sentence_chunking can be enabled via config

- GIVEN a config file or environment variable setting `sentence_chunking=true`
- WHEN the config is loaded
- THEN `sentence_chunking` is `true`

### Requirement: Config crate MUST add `min_chunk_chars` field (MUST)

The `IngestConfig` struct MUST include a new field:
```rust
pub min_chunk_chars: usize
```

Default value: `30`

This field specifies the minimum chunk length in characters. When using sentence-based chunking, adjacent sentences are merged until combined length >= `min_chunk_chars`.

#### Scenario: Default config has min_chunk_chars=30

- GIVEN a default `IngestConfig` instance
- WHEN the config is inspected
- THEN `min_chunk_chars` is `30`

#### Scenario: min_chunk_chars can be customized

- GIVEN a config setting `min_chunk_chars=50`
- WHEN the config is loaded
- THEN `min_chunk_chars` is `50`

#### Scenario: min_chunk_chars validation

- GIVEN a config setting `min_chunk_chars=0`
- WHEN the config is loaded
- THEN the config is accepted (no validation error)
- AND the sentence chunker will not merge short sentences

### Requirement: Config crate MUST add `max_chunk_chars` field (MUST)

The `IngestConfig` struct MUST include a new field:
```rust
pub max_chunk_chars: usize
```

Default value: `200`

This field specifies the maximum chunk length in characters. When using sentence-based chunking, no chunk exceeds `max_chunk_chars`; if a single sentence exceeds this limit, it is split on the nearest clause boundary.

#### Scenario: Default config has max_chunk_chars=200

- GIVEN a default `IngestConfig` instance
- WHEN the config is inspected
- THEN `max_chunk_chars` is `200`

#### Scenario: max_chunk_chars can be customized

- GIVEN a config setting `max_chunk_chars=150`
- WHEN the config is loaded
- THEN `max_chunk_chars` is `150`

#### Scenario: max_chunk_chars validation

- GIVEN a config setting `max_chunk_chars=10` (less than min_chunk_chars)
- WHEN the config is loaded
- THEN the config is accepted (no validation error at load time)
- AND the sentence chunker will return an error when invoked

### Requirement: Config crate MUST add `build_hierarchy` field (MUST)

The `IngestConfig` struct MUST include a new field:
```rust
pub build_hierarchy: bool
```

Default value: `false`

This field controls whether the ingest pipeline extracts topics and concepts from document structure and creates hierarchy rows in the database.

#### Scenario: Default config has build_hierarchy disabled

- GIVEN a default `IngestConfig` instance
- WHEN the config is inspected
- THEN `build_hierarchy` is `false`

#### Scenario: build_hierarchy can be enabled via config

- GIVEN a config file or environment variable setting `build_hierarchy=true`
- WHEN the config is loaded
- THEN `build_hierarchy` is `true`

### Requirement: Config MUST support environment variable overrides for new fields (MUST)

The config loader MUST support the following environment variables:
- `CITE_SENTENCE_CHUNKING` → `ingest.sentence_chunking` (bool)
- `CITE_MIN_CHUNK_CHARS` → `ingest.min_chunk_chars` (usize)
- `CITE_MAX_CHUNK_CHARS` → `ingest.max_chunk_chars` (usize)
- `CITE_BUILD_HIERARCHY` → `ingest.build_hierarchy` (bool)

#### Scenario: Environment variables override defaults

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

#### Scenario: Partial environment variable overrides

- GIVEN environment variable `CITE_BUILD_HIERARCHY=true` (others unset)
- WHEN `Config::load()` is called
- THEN `config.ingest.build_hierarchy` is `true`
- AND `config.ingest.sentence_chunking` is `false` (default)
- AND `config.ingest.min_chunk_chars` is `30` (default)
- AND `config.ingest.max_chunk_chars` is `200` (default)

#### Scenario: Invalid environment variable values are ignored

- GIVEN environment variable `CITE_MIN_CHUNK_CHARS=invalid`
- WHEN `Config::load()` is called
- THEN the invalid value is ignored
- AND `config.ingest.min_chunk_chars` is `30` (default)

### Requirement: Config MUST support file-based configuration for new fields (SHOULD)

The config loader SHOULD support loading new fields from a TOML config file:
```toml
[ingest]
sentence_chunking = true
min_chunk_chars = 50
max_chunk_chars = 150
build_hierarchy = true
```

#### Scenario: Config file overrides defaults

- GIVEN a config file with:
  ```toml
  [ingest]
  build_hierarchy = true
  sentence_chunking = true
  ```
- WHEN `Config::load()` is called
- THEN `config.ingest.build_hierarchy` is `true`
- AND `config.ingest.sentence_chunking` is `true`

#### Scenario: Environment variables override config file

- GIVEN a config file with `build_hierarchy=false`
- AND environment variable `CITE_BUILD_HIERARCHY=true`
- WHEN `Config::load()` is called
- THEN `config.ingest.build_hierarchy` is `true` (env overrides file)

### Requirement: Config merge order MUST be: defaults → file → env (MUST)

The config merge order MUST be:
1. Defaults (hardcoded)
2. File config (TOML)
3. Environment variables

Environment variables have highest priority and override file config, which overrides defaults.

#### Scenario: Merge order is correct

- GIVEN defaults: `build_hierarchy=false`
- AND file config: `build_hierarchy=true`
- AND environment variable: `CITE_BUILD_HIERARCHY=false`
- WHEN `Config::load()` is called
- THEN `config.ingest.build_hierarchy` is `false` (env wins)

#### Scenario: File config overrides defaults

- GIVEN defaults: `min_chunk_chars=30`
- AND file config: `min_chunk_chars=50`
- AND no environment variable set
- WHEN `Config::load()` is called
- THEN `config.ingest.min_chunk_chars` is `50` (file wins over default)

### Requirement: Config MUST NOT break existing config loading (MUST)

Adding new fields to `IngestConfig` MUST NOT break existing config loading. Existing config files without the new fields MUST still load correctly, with new fields using their default values.

#### Scenario: Old config file loads without errors

- GIVEN an old config file without `sentence_chunking`, `min_chunk_chars`, `max_chunk_chars`, `build_hierarchy`
- WHEN `Config::load()` is called
- THEN the config loads without errors
- AND new fields use default values (`false`, `30`, `200`, `false`)

#### Scenario: Existing environment variables continue to work

- GIVEN existing environment variables:
  - `CITE_CHUNK_SIZE=1000`
  - `CITE_CHUNK_OVERLAP=200`
- AND new environment variables:
  - `CITE_BUILD_HIERARCHY=true`
- WHEN `Config::load()` is called
- THEN `config.ingest.chunk_size_chars` is `1000` (existing)
- AND `config.ingest.chunk_overlap_chars` is `200` (existing)
- AND `config.ingest.build_hierarchy` is `true` (new)

### Requirement: Config MUST validate min_chunk_chars < max_chunk_chars (SHOULD)

The config loader SHOULD validate that `min_chunk_chars < max_chunk_chars` when both are set. If invalid, the loader SHOULD return an error or log a warning.

#### Scenario: Validation catches min >= max

- GIVEN config with `min_chunk_chars=200` and `max_chunk_chars=100`
- WHEN `Config::load()` is called
- THEN an error is returned or a warning is logged
- AND the config is not used with invalid values

#### Scenario: Validation passes for valid values

- GIVEN config with `min_chunk_chars=30` and `max_chunk_chars=200`
- WHEN `Config::load()` is called
- THEN no error occurs
- AND the config is loaded successfully

### Requirement: Config MUST document new fields (SHOULD)

The config crate SHOULD include documentation for new fields:
- Doc comments on each field explaining purpose and default value
- Example TOML config showing new fields
- Example environment variables in README or docs

#### Scenario: Config fields have doc comments

- GIVEN the `IngestConfig` struct
- WHEN the source code is inspected
- THEN each new field has a doc comment explaining:
  - Purpose of the field
  - Default value
  - Valid values or constraints

### Requirement: Config defaults MUST align with proposal (MUST)

Config defaults MUST match the proposal:
- `sentence_chunking: false`
- `min_chunk_chars: 30`
- `max_chunk_chars: 200`
- `build_hierarchy: false`

#### Scenario: Default config matches proposal

- GIVEN a default `IngestConfig` instance
- WHEN compared to proposal table
- THEN all defaults match:
  - `sentence_chunking` = `false`
  - `min_chunk_chars` = `30`
  - `max_chunk_chars` = `200`
  - `build_hierarchy` = `false`
