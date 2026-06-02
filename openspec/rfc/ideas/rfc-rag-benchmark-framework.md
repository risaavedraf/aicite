# RFC: RAG Benchmark Framework

## Status: Draft

This RFC defines a **systematic benchmark methodology** for evaluating CITE's retrieval quality, efficiency, and the impact of the hierarchical graph (v0.2 → v0.3+). It establishes metrics, datasets, evaluation procedures, and acceptance criteria.

## Quick path

1. Create evaluation dataset (40-80 representative queries with ground truth).
2. Implement automated metrics: Context Precision, Context Recall, Latency.
3. Run v0.2 vs v0.3 comparison.
4. Add LLM-as-Judge for Faithfulness and Relevancy.
5. Iterate based on results.

## Problem

CITE is evolving from flat semantic search (v0.2) to hierarchical graph-based retrieval (v0.3+). Without a structured benchmark:

- No objective way to measure if the graph improves retrieval.
- No baseline to compare against.
- Design decisions (chunk size, limit, min_score) are made by intuition.
- No regression detection when changing the engine.

## Goals

1. Define **retrieval quality metrics** (Precision, Recall, Hit Rate).
2. Define **generation quality metrics** (Faithfulness, Relevancy, Correctness).
3. Define **efficiency metrics** (Latency, Token usage, Memory, DB size).
4. Create a **repeatable evaluation procedure** with a standard dataset.
5. Establish **acceptance criteria** for release gates.
6. Enable **version-to-version comparison** (v0.2 vs v0.3 vs future).

## Non-goals

- Build a full evaluation platform (use existing tools like Ragas/DeepEval).
- Evaluate answer generation quality (CITE is retrieval-only by design).
- Benchmark against external RAG systems (separate effort).
- Automate CI-based benchmark runs (future phase).

## Metrics

### Retrieval quality

| Metric | Formula | What it measures |
|---|---|---|
| **Context Precision** | Relevant Retrieved / Total Retrieved | Noise in results |
| **Context Recall** | Relevant Retrieved / Total Relevant (ground truth) | Missed information |
| **Hit Rate @K** | % queries with ≥1 relevant chunk in top K | Minimum viability |

### Generation quality (via LLM-as-Judge)

| Metric | What it measures |
|---|---|
| **Faithfulness** | Does the response only cite retrieved chunks? (hallucination check) |
| **Answer Relevancy** | Does the response directly answer the query? |
| **Answer Correctness** | Does the response match the gold answer? |

### Efficiency

| Metric | Target |
|---|---|
| **Search latency** | < 800ms with thousands of chunks |
| **Token efficiency** | Minimize injected context tokens |
| **Memory (RAM)** | Document baseline |
| **DB size** | MB per document/chunk ratio |

### Acceptance criteria (initial targets)

| Metric | Minimum | Goal |
|---|---|---|
| Context Precision | > 0.60 | > 0.75 |
| Context Recall | > 0.55 | > 0.70 |
| Faithfulness | > 0.85 | > 0.90 |
| Search latency | < 1200ms | < 800ms |
| Hit Rate @5 | > 0.70 | > 0.85 |

## Evaluation dataset

### Structure

```json
{
  "query": "How does JWT token rotation work?",
  "gold_answer": "JWT access tokens rotate every 15 minutes...",
  "relevant_chunk_ids": ["chunk-042", "chunk-043"],
  "category": "factual"
}
```

### Categories to cover

| Category | Description | Example |
|---|---|---|
| **Factual** | Direct fact lookup | "What is the default chunk size?" |
| **Summarization** | Synthesize across chunks | "Summarize the auth architecture" |
| **Comparison** | Contrast two concepts | "Difference between v0.2 and v0.3 retrieval?" |
| **Hierarchical** | Requires graph traversal | "What are all the sub-topics under Authentication?" |

### Dataset size

- **Minimum viable**: 40 queries
- **Recommended**: 60-80 queries
- **Distribution**: ~40% factual, ~25% summarization, ~20% comparison, ~15% hierarchical

## Evaluation procedure

### Step 1: Prepare dataset

Create `evaluation/dataset.json` with queries, gold answers, and relevant chunk IDs.

### Step 2: Run baseline (v0.2)

```bash
cite search --query "..." --limit 5 --output json > results/v0.2/query_001.json
```

### Step 3: Run candidate (v0.3)

Same queries against the hierarchical graph version.

### Step 4: Compute metrics

- **Automated**: Precision, Recall, Hit Rate, Latency (script computes from JSON).
- **LLM-as-Judge**: Faithfulness, Relevancy (send to Claude/GPT-4o/Grok with rubric).

### Step 5: Compare and report

Generate tables and charts (Python + matplotlib/seaborn).

### Variations to test

| Variable | Values to test |
|---|---|
| Chunk size | 256, 512, 1024 |
| Limit (top-K) | 5, 8, 12 |
| Min score threshold | 0.2, 0.3, 0.5 |
| Graph enabled | yes / no |

## Tools

| Tool | Purpose |
|---|---|
| **Ragas** (Python) | RAG evaluation framework — best for structured metrics |
| **DeepEval** | Alternative evaluation framework |
| **pandas + scikit-learn** | Custom metric computation |
| **hyperfine** / **criterion.rs** | Latency benchmarking (Rust) |
| **matplotlib / seaborn** | Visualization |

## Roadmap

| Level | Capabilities |
|---|---|
| **Basic** (current) | Flat semantic search + JSON output |
| **Good** (with graph) | Hybrid search (semantic + structural), parent/child chunks, metadata filters |
| **Advanced** | Multi-hop reasoning, agentic RAG, incremental embedding updates, continuous evaluation |
| **Team** | Multi-user knowledge base, shared evaluation datasets |

## Open questions

1. Should the evaluation dataset live in-repo or as a separate fixture?
2. Should we run LLM-as-Judge locally (Ollama) or via API (Claude/GPT)?
3. Do we need a dedicated `cite benchmark` CLI command?
4. How often should benchmarks run — per PR, weekly, manual?
5. Should we benchmark against external tools (LlamaIndex, GraphRAG) or only self-compare?

## Review plan

- [ ] Confirm acceptance criteria thresholds.
- [ ] Confirm metric definitions and formulas.
- [ ] Decide dataset location and format.
- [ ] Decide LLM-as-Judge provider and rubric.
- [ ] Decide whether to build a `cite benchmark` command.
- [ ] Decide benchmark frequency (manual vs automated).
- [ ] Validate against v0.2 baseline.

## Related docs

- [RFC: CITE-Pi Integration](./rfc-cite-pi-integration.md) — embedding model and latency targets
- [v0.2 Phase Map](../../changes/v0.2-phase-map.md)
- [Phase 6: Evaluation](../../changes/phase-6-evaluation/)
- [System Architecture](../../prd/07-system-architecture.md)
