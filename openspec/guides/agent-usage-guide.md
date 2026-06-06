# Agent Usage Guide — Cite CLI from an AI Agent Perspective

This document describes the Cite CLI from the perspective of an AI agent consuming it as a tool. It covers what works well, what has trade-offs, and what could be improved.

## How an agent uses the CLI

### Real-world invocation

Use one of the canonical pathways depending on your environment:

```bash
# Dev run (no release build required)
cargo run --bin cite -- context "what are the acceptance criteria?" --json

# Local built binary
cargo build --release
./target/release/cite context "what are the acceptance criteria?" --json

# Installed release binary in PATH
cite context "what are the acceptance criteria?" --json
```

The agent calls the CLI as a subprocess, parses the JSON output, and uses the citations to answer questions.

### Typical agent workflow

```
1. cite context "<question>" --json
2. Parse result_kind → decide if answer is possible
3. Parse citations[] → extract text + scores
4. Build response using ONLY cited text
5. Include citation IDs in response
6. If result_kind == "no_results" → say "documents don't contain this information"
```

## What works well

### Grounded retrieval with citations

Every claim the agent makes can be traced back to a specific document, chunk, and score. This is the core value proposition — no hallucination because the source text is right there.

```json
{
  "citation_id": "c1",
  "display_name": "architecture.txt",
  "text": "The API gateway routes all external requests...",
  "score": 0.725
}
```

### Honest no-results behavior

When the corpus doesn't contain relevant information, the CLI returns `result_kind: "no_results"` with zero citations. The agent knows to say "I don't have that information" instead of fabricating an answer.

### Result-kind decision table

The `result_kind` field (`context`, `insufficient_context`, `no_results`) tells the agent how much to trust the results:

| Result kind | Agent behavior |
|---|---|
| `context` | High confidence — answer with citations |
| `insufficient_context` | Low confidence — answer cautiously, flag uncertainty |
| `no_results` | No relevant data — say so honestly |

### Traceability

Every response includes a `trace_id` that can be used to audit the full retrieval chain: provider, model, ranking method, thresholds, latency. Useful for debugging and compliance.

### Provider disclosure

Automatic banner when using real providers. The agent doesn't need to manage this — the CLI handles it.

### Deterministic evaluation

The `evaluate` command uses a mock provider (no API key needed) and runs 8 golden fixtures. Good for CI and regression testing.

## Trade-offs and considerations

### Token usage per query

Each citation returns the full chunk text (~500-1000 characters). With default `top-k: 5`, a single query can add 2500-5000 characters to the agent's context window.

**Impact**: In long conversations with many queries, context fills up fast.

**Mitigation strategies**:
- Use `search` instead of `context` when you only need scores, not full text
- Reduce `top-k` for queries where fewer results suffice
- Process JSON output and extract only the relevant snippet, not the full chunk
- Cache results — don't re-query the same information

### Response latency

Typical response: 800-1500ms per query (embedding generation + vector search).

**Impact**: Acceptable for single queries. Adds up if the agent needs to make multiple queries to answer a complex question.

**Mitigation**: Batch related questions when possible, or use broader queries that return more results in one call.

### Citation text vs. snippet

The citation `text` field contains the full chunk. Sometimes the relevant information is a single sentence in a 500-word chunk.

**Ideal**: A `snippet` field with just the most relevant portion (100-200 chars) alongside the full `text`.

### Search vs. Context

| Command | Returns | Best for |
|---|---|---|
| `search` | Scores + short snippets | Quick relevance check, finding which docs to read |
| `context` | Full chunks + instructions + metadata | Building a complete answer with citations |

Use `search` first to explore, `context` when you need to answer.

## What could be improved

### Already implemented ✅

The following features from the original wishlist are now working:

- **Compact/Full mode** — `search`, `retrieve`, `context` all support `--full` flag. Default is compact (liviano en tokens). `--full` adds metadata completa.
- **Snippet field** — Compact mode returns `snippet` (~200 chars) instead of full chunk text.
- **Hierarchical filtering** — `--topic` and `--concept` filters work with hierarchy data.
- **Breadcrumb** — `--full` mode shows `document > topic > concept` breadcrumb.

### Still pending

### 1. Max characters flag

A `--max-chars` option to limit total context size:

```bash
cite context "query" --max-chars 2000
```

The CLI would return the most relevant citations that fit within the character budget.

### 2. Multi-query batching

A `cite context-batch` command that accepts multiple queries in one call:

```bash
cite context-batch --json << 'EOF'
["query 1", "query 2", "query 3"]
EOF
```

This would reduce overhead for agents that need to ask multiple related questions.

### 3. Relevance filtering

A `--min-score` flag to only return citations above a certain threshold:

```bash
cite context "query" --min-score 0.7
```

This would filter out low-relevance results automatically.

### 4. Document filtering

A `--doc` flag to search within specific documents:

```bash
cite context "query" --doc architecture.txt --doc api-reference.md
```

### 5. Snippet length control (`--max-snippet-chars`)

Limit the text returned per citation:

```bash
# Return only 200 chars per citation instead of full chunk
cite context "query" --max-snippet-chars 200
```

**Impact**: Reduces token usage by 50-70% while keeping the most relevant content.

### 6. Field selection (`--fields`)

Let the agent specify exactly which fields it needs:

```bash
cite context "query" --fields result_kind,citations.id,citations.snippet,citations.score
```

This is the most flexible approach but adds CLI complexity.

### 7. Streaming output

For long-running queries, streaming the results as they're computed would improve perceived latency.

### 8. Summarized context

A `--mode summary` that returns a brief synthesized answer with citation IDs, instead of raw chunks:

```json
{
  "summary": "The MVP must support ingestion, retrieval, and context pack assembly (FR-001 through FR-005).",
  "citations": ["c1", "c2"],
  "result_kind": "context"
}
```

## Known issue: `insufficient_context` with large chunks

When chunks are large (800-1000+ chars), the vector similarity score tends to drop because the query only matches a small portion of the chunk text. This can cause `result_kind: "insufficient_context"` even when the relevant information is present.

**Example**: A query about "functional requirements" might match a 900-char chunk that contains 100 chars of relevant text and 800 chars of other content. The cosine similarity averages over the whole chunk, diluting the score.

**Observed behavior**: Queries about project requirements returned scores of 0.62-0.69 (below the 0.70 confidence threshold), even though the correct documents were retrieved.

**Potential solutions**:
- Smaller chunks (300-500 chars) for more precise matching
- Hybrid scoring: vector similarity + keyword overlap boost
- Score normalization based on chunk length
- Adaptive thresholds per document type
- Reranking pass that scores query-relevant portions of chunks

## Performance characteristics

| Metric | Current (Gemini) | Planned (Ollama local) |
|---|---|---|
| Query latency | 800-1500ms | 15-50ms |
| Query latency (eval/mock) | <50ms | <50ms |
| Chunks per query | 5 (default top-k) | 5 (default top-k) |
| Characters per chunk | 500-1000 | 500-1000 |
| Total context per query | 2500-5000 chars | 2500-5000 chars |
| Embedding dimensions | 3072 (Gemini) | 2560 (qwen3) or 768 (nomic) |
| Rate limits | Yes (429 errors) | None |
| Cost | $0.15/1M tokens | $0 |

**Roadmap:** See `openspec/rfc/active/rfc-embedding-providers.md` for provider system and migration path.

## Comparison with alternatives

| Approach | Pros | Cons |
|---|---|---|
| **Cite CLI** | Grounded, cited, auditable, local | Per-query latency, token usage |
| **Direct file reading** | No latency, no API calls | Agent must read entire docs, context explosion |
| **Grep/ripgrep** | Fast, no API calls | Not semantic, no ranking |
| **ChatGPT/Claude with docs** | Conversational, easy | Hallucination risk, no citations, docs sent to cloud |
| **RAG framework (LangChain)** | Flexible, many integrations | Complex setup, overkill for CLI use case |

## Recommendation for agent developers

1. **Start with `search`** to explore the corpus and find relevant documents
2. **Use `context`** when building the final answer with citations
3. **Parse `result_kind`** before deciding how to respond
4. **Always cite sources** — include citation IDs in your response
5. **Respect `no_results`** — don't fabricate when the corpus doesn't have the answer
6. **Monitor token usage** — be strategic about how many queries you make per conversation
7. **Use `--json`** for machine-readable output
