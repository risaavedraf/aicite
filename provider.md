# OpenAI-Compatible Embedding Provider — Implementation Report

## Status: ✅ Complete

## Changes Made

### 1. `Cargo.toml` (workspace root)
- Added `"blocking"` feature to workspace `reqwest` dependency.
- Required because the CLI is synchronous (no `#[tokio::main]`) and the `EmbeddingProvider` trait defines sync methods.
- Ran `cargo update` to resolve the lockfile after the feature change.

### 2. `crates/providers/src/openai.rs` (new file)
- **`OpenAICompatibleProvider`** struct implementing `EmbeddingProvider`.
- Fields: `client` (reqwest blocking), `endpoint`, `model`, `provider_id`.
- **`new(endpoint, model, api_key)`** — validates HTTPS, builds a blocking reqwest `Client` with 30s timeout and `Authorization: Bearer` default header.
- **`embed(text)`** — POSTs `{"input": text, "model": model}` to the endpoint. Maps HTTP errors, timeouts, network failures, and parse errors to `HarnessError::EmbeddingProviderError`.
- **`model_id()`** / **`provider_id()`** — return the stored model ID and `"openai-compatible"` respectively.
- `#[derive(Debug)]` on the struct for testability (`Result::unwrap_err` requires `T: Debug`).

### 3. `crates/providers/src/lib.rs`
- Added `pub mod openai;` to expose the new module.

## Validation

```
cargo check -p providers  → ✅ clean
cargo test -p providers   → ✅ 5/5 passed
```

### Test Coverage

| Test | What it verifies |
|------|-----------------|
| `test_provider_creation_valid` | HTTPS URL accepted |
| `test_provider_creation_rejects_http` | HTTP URL → `ConfigError` with "HTTPS" message |
| `test_provider_model_id` | `model_id()` returns the configured model string |
| `test_provider_provider_id` | `provider_id()` returns `"openai-compatible"` |
| `test_embed_invalid_endpoint_returns_error` | Network failure → `EmbeddingProviderError` |

## Design Decisions

1. **Blocking client** over async — the trait is sync and the CLI has no tokio runtime. `reqwest::blocking::Client` is the correct fit.
2. **HTTPS-only enforcement** — rejects `http://` URLs at construction time using `ConfigError`, not at request time.
3. **No mock HTTP server** — avoided adding a dev-dependency (`mockito`/`wiremock`). The `test_embed_invalid_endpoint_returns_error` test exercises the error path by targeting a non-existent localhost endpoint.
4. **Timeout detection** — uses `reqwest::Error::is_timeout()` to distinguish timeouts from other errors.
5. **`provider_id` field** — stored as a field rather than a constant, making it easy to extend later with custom provider IDs.

## Open Risks / Questions

- **Response format**: assumes standard OpenAI format `{"data": [{"embedding": [...]}]}`. Providers with a different response shape will fail at parse time with a clear error message.
- **Endpoint path**: the caller must pass the full URL (e.g. `https://api.openai.com/v1/embeddings`). No path is appended automatically.
- **API versioning**: no `/v1` prefix is enforced or appended. This is intentional for flexibility with non-OpenAI providers.

## Recommended Next Step

Wire `OpenAICompatibleProvider` into the config layer so users can configure embedding providers via `config.toml` or CLI flags.
