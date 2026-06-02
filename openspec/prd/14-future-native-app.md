# Future Native App — Companion UI for the CLI Engine

This document describes a future V2 companion product, not the MVP.

## Purpose

The future native app provides a visual shell on top of the CLI/engine so that non-technical users can browse documents, retrieve context, inspect citations, and follow guided workflows without learning the command surface first.

The app does not redefine the product. It consumes the same retrieval/context contract as the CLI and depends on the CLI/engine remaining the source of truth.

## What the V2 app may add

- Document browsing and search.
- Citation inspection with richer visuals.
- Guided ingestion, retrieval, context-pack, and source-read flows.
- Easier onboarding for non-technical users.
- A friendly surface for manual verification and demoing.

## What the V2 app does not change

- The engine remains the source of truth.
- The semantic document corpus remains private and grounded.
- The CLI contract remains stable.
- Retrieval, citations, and traceability stay in the core engine.

## V2 boundary

Do not move any native app work into the MVP scope unless the CLI/engine MVP is already done and stable. The companion app should be treated as a separate product layer that wraps the existing contract.
