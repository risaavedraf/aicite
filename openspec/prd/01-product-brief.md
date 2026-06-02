# Product Brief — AI Cite CLI for Document Context

Build a production-quality MVP that turns a private document corpus into a semantic context engine exposed through a CLI for agents and integrators.

## Executive summary

The product ingests private documents, organizes them into a semantic structure, and exposes cited retrieval results through CLI commands that any external AI agent can call as a tool. The engine returns context packs, source snippets, citation metadata, and trace data; the external agent remains responsible for interpreting or generating downstream answers.

The native app is not the MVP. It becomes a future V2 companion that wraps the same CLI/engine if a visual shell is later useful.

## Problem

People and small teams keep useful knowledge in PDFs, notes, policies, reports, and project documents. Generic chatbots cannot safely use that private context without opaque prompt stuffing, and raw search still requires too much manual reading.

What is needed is a private document-context cite that an AI agent can query, inspect, and verify without forcing the product to become another assistant or chat app.

## Product goal

Deliver a working MVP where a user or agent can:

1. Load a default corpus.
2. In local/private mode, ingest supported documents into a private corpus; in the public packaged demo, use preloaded sample documents only.
3. Build or refresh the semantic index and minimal source/section/chunk metadata for the corpus.
4. Search and retrieve cited chunks through the CLI.
5. Build an agent-consumable context pack with stable JSON.
6. Read source snippets and inspect retrieval traces.

## Positioning

| Capability | What this project demonstrates |
|---|---|
| Grounded document engine | Ingestion, semantic organization, vector-first retrieval, citations, and no-results behavior |
| Agent-friendly tooling | Stable CLI contract, machine-readable outputs, source inspection, and traceability |
| Production-minded engineering | Durable local persistence, locks, rate limits, logging, error handling, testing, and deployability |
| Privacy-aware design | Private corpus handling, disclosure, data minimization, and local reset/delete paths |

## Success metric

The MVP succeeds when a technical reviewer can run the CLI or connect an agent to it, retrieve useful cited context from a corpus, and verify the supporting snippets without reading the code first.

## Delivery approach

The MVP is CLI-first and powered by a Rust engine. Built-in answer generation, MCP access, full hybrid search, and a native app are post-MVP extensions unless explicitly re-scoped later.

## Non-goals

- Build a general-purpose ChatGPT clone.
- Make a built-in assistant or LLM answerer the center of the MVP.
- Support every file type in the first version.
- Optimize for enterprise multi-tenant security in the MVP.
- Train a custom foundation model.
- Build complex autonomous agent workflows before the grounded retrieval core is reliable.

## MVP promise

> Ask your agent to retrieve cited context from your documents and verify the source trail.

## Related docs

- [Users and Problems](./02-users-and-problems.md)
- [MVP Scope](./03-mvp-scope.md)
- [AI and Retrieval Design](./08-ai-retrieval-design.md)
