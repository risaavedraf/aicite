# Legal and Privacy Compliance

The MVP must be designed with Chilean privacy compliance in mind from the beginning, even if the public demo only uses sample documents.

> This document is a product and engineering compliance guide, not legal advice. Before handling real personal data in production, the project should be reviewed by a qualified legal professional.

## Legal baseline

| Law / framework | Relevance to the product |
|---|---|
| Chile Ley 19.628 | Current baseline for protection of private life and personal data processing in Chile. |
| Chile Ley 21.719 | Modernizes Chile's personal data protection regime, creates the Personal Data Protection Agency, raises compliance standards, and has deferred entry into force reported for December 2026. |

## Compliance principle

> If documents may contain personal data, the system must treat ingestion, embeddings, minimal graph/source metadata, retrieval context, context packs, traces, logs, and citations as personal-data processing surfaces.

## Personal data surfaces

| Surface | Data risk |
|---|---|
| Uploaded files | May contain personal, sensitive, business, contractual, or confidential data. |
| Extracted text | Duplicates document content in another storage layer. |
| Chunks, embeddings, and minimal graph/source metadata | May encode personal data or confidential information. |
| Retrieval context/context packs | Exposes selected document content to the caller and may be sent by an external agent to another provider. |
| Downstream AI answers | External agents may reveal or transform personal data using context returned by the harness. |
| Citations | Expose raw source snippets to the user or agent. |
| Traces | Can include document IDs, citation IDs, provider/index metadata, and timing data. |
| Logs | Can accidentally store filenames, query text, errors, or identifiers. |
| Query text | User queries may contain personal, sensitive, or confidential information sent to external embedding providers. |
| Filenames | Raw file names may contain personal data. They may be stored internally for file management, but user-facing CLI/API fields should use sanitized `display_name` values and logs must use `document_id` instead. |

## MVP privacy decisions

| Decision | Requirement |
|---|---|
| Public packaged demo | Uses bundled sample documents only; uploads are disabled. |
| Local/private demo | Uploads are allowed into the private corpus with explicit warning and provider disclosure; original files are stored in CLI-managed local storage together with extracted text, chunks, embeddings, minimal graph/source metadata, indexes, durable state, and metadata. The MVP protects against public/demo upload exposure, not against a compromised local device or filesystem. |
| Production | Real uploads are blocked until the production compliance checklist is complete. |
| Sensitive data | MVP must warn users not to upload sensitive personal data. |
| Data retention | Local/private data must be manually resettable/deletable via documented steps; automated retention policy is post-MVP. |
| Provider disclosure | README and CLI output must disclose when document snippets, query text, or embeddings may be sent to configured AI providers. |
| Logs | Logs must avoid storing full document text, full prompts, secrets, raw filenames/display names, provider error payloads, query text, citation text, chunk text, or raw personal data. |

## Production compliance checklist

Before enabling real production uploads, the following must be addressed:

- [ ] Identify the data controller/responsible party and any processors/sub-processors.
- [ ] Define lawful basis for processing personal data.
- [ ] Publish a privacy notice explaining purpose, data categories, providers, retention, rights, and contact channel.
- [ ] Limit processing to the stated purpose: document ingestion, semantic retrieval, context-pack assembly, source inspection, and trace generation.
- [ ] Minimize data sent to model/providers: only retrieved chunks or embeddings needed for the requested context.
- [ ] Document international data transfer implications when using external AI providers.
- [ ] Add a data deletion workflow for documents, chunks, embeddings, minimal graph/source metadata, indexes, traces, and derived metadata.
- [ ] Add access, correction, deletion/blocking, and opposition request handling where applicable.
- [ ] Define retention periods for uploads, minimal graph/source metadata, embeddings, logs, retrieval traces, and query history.
- [ ] Add incident response procedure for data exposure or provider misuse.
- [ ] Review contracts / terms with AI and infrastructure providers.
- [ ] Run a privacy impact review before processing sensitive or high-risk data.

## Privacy-by-design requirements

| Area | Requirement |
|---|---|
| Purpose limitation | Use uploaded content only for retrieval, context-pack assembly, citation display, source inspection, and trace generation. |
| Data minimization | Return and, when applicable, send only top relevant chunks and source/citation metadata needed for the requested context, not whole documents. |
| Transparency | Show which provider/model/index was used and which sources support the returned context. |
| User control | User should be able to know what documents are indexed. Full deletion API is post-MVP, but documented manual reset/delete steps are required for local/private MVP use. |
| Security | Keep secrets out of source control, restrict file access to CLI-managed local paths, and document that local/private MVP storage relies on operator-controlled device, OS account, and filesystem protections unless encryption at rest is added later. |
| Auditability | Record provider, timestamp, document IDs, citation IDs, and trace IDs for each context-pack request without logging raw sensitive content. |

## Legal-safe runtime policy

The product has three runtime modes with different rules.

| Mode | Uploads | Privacy rule |
|---|---|---|
| Public packaged demo | Disabled | Use preloaded sample documents only; show “Demo uses sample documents. Public uploads are disabled. Do not enter personal or confidential information.” |
| Local/private demo | Enabled | Show no-sensitive-data warning, disclose provider behavior, document local reset/delete steps, keep uploads scoped to the private corpus, and avoid claiming protection from local-device compromise or provider misuse. |
| Production | Blocked | Do not enable real user uploads until auth, deletion, retention, privacy notice, provider review, and legal review are complete. |

### Local/private cleanup requirement

README must document:

1. CLI-managed local storage paths for original files, extracted text, chunks, embeddings, minimal graph/source metadata, indexes, traces, durable locks, rate-limit state, and metadata.
2. How to stop active CLI commands and manually remove local imported data.
3. How to rebuild the sample/default knowledge base after reset.
4. That logs must not contain full document text, full prompts, secrets, raw filenames/display names, provider error payloads, query text, citation text, chunk text, or raw personal data.
5. That local/private data is not encrypted by the MVP unless the operator's OS/filesystem provides it; sensitive documents require an operator-controlled environment and provider configuration.

### Demo copy requirements

- Public packaged demo banner: “Demo uses sample documents. Public uploads are disabled. Do not enter personal or confidential information.”
- Local/private upload warning: “Do not upload personal, sensitive, or confidential information unless you control the environment and provider configuration.”
- Provider disclosure: retrieved snippets, query text, or embeddings may be sent to the configured AI provider when that provider is enabled.
- Compliance caveat: do not claim legal compliance as certified; claim “designed with Chilean privacy requirements in mind.”

## Related docs

- [AI Ethics and Governance](./13-ai-ethics-governance.md)
- [Non-Functional Requirements](./05-non-functional-requirements.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
- [Risks and Open Questions](./11-risks-open-questions.md)
