# AI Ethics and Governance

The product must make AI-adjacent behavior traceable, reviewable, and accountable. The MVP cite returns evidence and context for external agents; it is not itself the final authority or a built-in assistant.

## Governance principle

> AI output is not authority by itself. Every production use of returned context must be traceable to provider/index configuration, source context, citations, timestamp, and a responsible human owner.

## Accountability model

| Actor | Responsibility |
|---|---|
| End user | Reviews downstream answers and citations before acting on the output. |
| Consumer agent | Uses the context pack according to instructions, cites sources, and avoids unsupported claims. |
| Product/operator | Defines acceptable use, privacy controls, provider configuration, and monitoring. |
| Developer/team | Implements safeguards, logging, provider disclosure, tests, and failure behavior. |
| AI/embedding provider | Operates external models under its own terms and data-processing commitments. |
| AI model | Generates embeddings or downstream text; it is not a legal or moral decision-maker. |

## Human responsibility rule

The CLI output and README must not imply that the model, the cite, or a downstream agent is the final authority. For any professional, legal, medical, financial, employment, or high-impact decision, the product must state that a human reviewer is responsible for validating downstream answers against cited sources.

## White-box provider/model registry

This registry documents which AI models/providers are used, what they do, what data they receive, and why they are allowed in the system. It must be updated whenever a provider/model is added, removed, or changes role.

| Registry ID | Model name | Provider | System role | Data sent | Output used for | Current status | Notes / risks |
|---|---|---|---|---|---|---|---|
| `embedding-mock-local` | Deterministic mock embeddings | Local test provider | Tests and local smoke checks | Fixture chunks and fixture queries | Stable retrieval fixtures | Required for tests | Default embedding provider for automated tests; must not require network or secrets. |
| `embedding-configured-default` | Configured embedding model ID | OpenAI-compatible or local provider | Embedding generation for document chunks and retrieval queries | Chunk text and retrieval query text | Vector embeddings stored in local SQLite/index for retrieval | Required for local/private real-provider runs | Model ID, dimensions, and provider are operator configuration; switching provider requires re-indexing all documents and re-calibrating retrieval thresholds. |
| `answer-adapter-post-mvp` | Configured answer model ID | OpenAI-compatible or local provider | Optional post-MVP answer generation over context packs | User query + retrieved document chunks + citation metadata | Grounded natural-language answer with citations | Deferred | Must not be required for MVP. If added later, it must preserve the context-pack contract and provider disclosure. |

## Required context trace

Each retrieval/context request should produce an internal trace record:

| Field | Purpose |
|---|---|
| `trace_id` | Unique identifier for audit/debugging |
| `query_id` | Links the trace to the retrieval query without exposing raw query text in logs |
| `context_pack_id` | Links the trace to the emitted context pack when applicable |
| `timestamp` | When the context was generated |
| `schema_version` | Which context-pack schema was used |
| `embedding_model_registry_id` | Stable ID from the white-box registry when embeddings are used |
| `provider` | Provider or gateway used for embeddings/retrieval support |
| `document_ids` | Documents involved in retrieval |
| `citation_ids` | Source chunks shown to the user or agent |
| `retrieval_top_k` | Retrieval breadth |
| `evidence_floor` | Lower threshold below which `no_results` must use empty citations |
| `confidence_threshold` | Threshold for normal `context` results; weaker or partial evidence becomes `insufficient_context` |
| `ranking_method` | Ranking/scoring strategy used |
| `responsible_owner` | Human/team accountable for production/team operation; optional or `null` in local/private mode |
| `user_visible_disclaimer_shown` | Whether the context was shown with the required caveat |

The trace should avoid storing full prompts, raw query text, or sensitive raw document text unless an explicit production retention policy allows it.

## User-facing transparency

The CLI should show, at minimum:

- provider/model/index used when a provider call occurs;
- context timestamp;
- cited sources;
- trace ID;
- disclaimer that downstream AI answers must be verified against sources;
- warning when the corpus does not contain enough relevant context.

## Ethical safeguards

| Risk | Safeguard |
|---|---|
| Hallucination by downstream agent | Context-pack instructions, citations, no-results behavior, and tests. |
| Hidden model influence | Show provider/model/index metadata and keep provider registry. |
| Over-reliance | User-facing disclaimer and citation-first UX. |
| Sensitive data exposure | Sample-doc public demo, privacy warnings, data minimization. |
| Prompt injection | Treat uploaded documents as data, not instructions. |
| Bias or unfair treatment | Do not use MVP for automated high-impact decisions. |
| Accountability gap | Assign responsible human/team for production use. |

## Prohibited MVP uses

The MVP must not be marketed or used as:

- legal advice;
- medical advice;
- financial advice;
- automated employment decision system;
- automated eligibility/credit/risk decision system;
- replacement for human review in high-impact domains;
- autonomous answer authority without source verification.

## Production governance checklist

Before real production use:

- [ ] Assign a responsible product owner/operator.
- [ ] Review provider terms for every model in the registry.
- [ ] Confirm whether providers retain inputs or use data for training.
- [ ] Define allowed and prohibited use cases.
- [ ] Add context trace storage and retention policy.
- [ ] Add context-pack schema versioning.
- [ ] Add downstream answer feedback and incident reporting if an answer adapter is introduced.
- [ ] Add periodic provider/model review when providers/models change.
- [ ] Add privacy/legal review for personal-data processing.

## Related docs

- [Legal and Privacy Compliance](./12-legal-privacy-compliance.md)
- [AI and Retrieval Design](./08-ai-retrieval-design.md)
- [API Contract](./09-api-contract.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
