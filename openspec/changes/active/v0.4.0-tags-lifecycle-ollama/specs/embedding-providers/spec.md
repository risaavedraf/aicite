# Embedding Providers Specification

## Purpose

Support local Ollama embeddings and provider batch capabilities while preserving existing Gemini and OpenAI-compatible embedding behavior.

## Requirements

### Requirement: Batch Embedding Capability

The embedding provider interface MUST support embedding multiple texts through an `embed_batch` capability. Providers that do not implement native batch behavior MUST have a sequential fallback that produces one embedding per input text in input order.

#### Scenario: Sequential fallback preserves order

- GIVEN a provider supports single-text embedding but no native batch endpoint
- WHEN `embed_batch` is called with three input texts
- THEN the provider MUST return three embeddings
- AND the embeddings MUST correspond to the input texts in the same order

### Requirement: Batch Strategy Reporting

The embedding provider interface MUST expose a `BatchStrategy` hint with supported values for native batch, rate-limited batch behavior, and sequential fallback. Provider health or diagnostics MUST be able to report the selected provider's batch strategy.

#### Scenario: Health reports batch strategy

- GIVEN Cite is configured with an embedding provider
- WHEN provider health is requested
- THEN the output MUST identify the provider's batch strategy as native, rate-limited, or sequential

### Requirement: Provider Configuration Fields

The system MUST support provider configuration fields needed by local and cloud embedding providers, including provider name, model, endpoint where applicable, dimensions where applicable, device where applicable, batch size where applicable, and workspace where applicable. Optional fields MUST have safe defaults or clear validation errors.

#### Scenario: Local endpoint default

- GIVEN provider configuration specifies `ollama` and a model
- AND no endpoint is explicitly configured
- WHEN Cite resolves provider configuration
- THEN the endpoint SHOULD default to the local Ollama service

### Requirement: Provider Factory Compatibility

The provider factory MUST support `gemini`, `openai-compatible`, and `ollama` provider selection. Adding Ollama MUST NOT break existing Gemini or OpenAI-compatible provider creation, configuration, or embedding behavior.

#### Scenario: Existing Gemini config still works

- GIVEN an existing valid Gemini provider configuration with an API key
- WHEN Cite creates the embedding provider after Ollama support is added
- THEN provider creation MUST succeed as before
- AND embedding requests MUST continue to use Gemini behavior

#### Scenario: Existing OpenAI-compatible config still works

- GIVEN an existing valid OpenAI-compatible provider configuration with an endpoint and API key
- WHEN Cite creates the embedding provider after Ollama support is added
- THEN provider creation MUST succeed as before
- AND embedding requests MUST continue to use OpenAI-compatible behavior

### Requirement: API Key Requirements by Provider Type

The system MUST validate API key requirements by provider type. Cloud providers that require API keys MUST continue to enforce them; local providers such as Ollama MUST bypass API-key validation.

#### Scenario: Cloud provider still requires key

- GIVEN provider configuration specifies Gemini without an API key
- WHEN Cite creates the provider
- THEN provider creation MUST fail with a configuration error

#### Scenario: Local provider bypasses key validation

- GIVEN provider configuration specifies Ollama without an API key
- WHEN Cite creates the provider
- THEN provider creation MUST proceed to local endpoint/model validation instead of failing on API-key validation

### Requirement: Ollama Provider Configuration

The system MUST support `ollama` as an embedding provider with a model name and an HTTP endpoint. The endpoint SHOULD default to the local Ollama service when not explicitly configured. Local Ollama configuration MUST NOT require an API key.

#### Scenario: Ollama without API key

- GIVEN provider configuration specifies `provider = "ollama"` and a model
- AND no API key is configured
- WHEN Cite creates the embedding provider
- THEN provider creation MUST NOT fail because of the missing API key

### Requirement: Ollama Embedding Requests

The Ollama provider MUST embed text through Ollama's local HTTP embedding API and SHOULD use native batch input when multiple texts are provided. The provider MUST return embeddings associated with the configured model.

#### Scenario: Ollama batch embedding

- GIVEN Ollama is running locally with the configured embedding model
- WHEN Cite requests embeddings for multiple texts
- THEN the Ollama provider MUST send a batch-capable embedding request
- AND MUST return one embedding per input text

### Requirement: Ollama Health Details

The system MUST report Ollama provider health with provider identity, configured model, endpoint, measured connectivity status, latency when measurable, and batch strategy.

#### Scenario: Healthy Ollama service

- GIVEN Ollama is running at the configured endpoint
- WHEN `cite health` checks the provider
- THEN the output MUST identify provider `ollama`
- AND MUST include the configured model and endpoint
- AND SHOULD include measured latency
- AND MUST include native batch strategy

#### Scenario: Unreachable Ollama service

- GIVEN provider configuration specifies `ollama`
- AND the configured endpoint is unreachable
- WHEN provider health is checked
- THEN the output MUST report the provider as unavailable
- AND SHOULD include an actionable message indicating the local service or endpoint should be checked
