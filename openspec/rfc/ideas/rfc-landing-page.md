# RFC: Landing Page (Leptos + GitHub Pages)

## Status: Draft

This RFC proposes a minimal landing page for Cite using **Leptos CSR** and **GitHub Pages**, optimized for **agent discoverability** rather than classic SEO.

## Quick path

1. Decide site location (repo subfolder vs separate repo).
2. Confirm base path for GitHub Pages (`/repo-name/`).
3. Approve minimal sections (hero, features, demo, CTA).
4. Decide whether to add `llms.txt` and README tweaks.

## Problem

Cite needs a small public landing page so **AI agents and developers can discover it** when searching for citation‑oriented document tools. Full SEO is not the priority; fast shipping and clear, indexable text is.

## Goals

1. Ship a **static landing** on GitHub Pages.
2. Keep the stack in **Rust (Leptos)**.
3. Ensure **indexable text** for agents/crawlers.
4. Keep deployment simple and reproducible.

## Non‑goals

- Full SEO optimization or SSR complexity.
- Dynamic backend features.
- Marketing site with heavy analytics.

## Proposed approach

### Rendering strategy

- **Leptos CSR (WASM)** for UI.
- Provide **static HTML fallback text** in `index.html` so crawlers can read content even if WASM is not executed.

### Deployment

- Use **GitHub Pages**.
- Build with `trunk build --release`.
- Publish `/dist` to `gh-pages`.
- Configure **base path** (e.g., `/aicite/`) using `--public-url` or `<base href>`.

### Content (minimal)

- Hero: “Cite is the Evidence.”
- 3‑4 bullets (citations, context packs, CLI‑first, local‑first).
- Demo snippet (`cite context --json`).
- CTA links: GitHub, docs, install.

### Agent discoverability extras (optional)

- Add `llms.txt` at repo root with a short summary + links.
- Strengthen README keywords and examples.
- GitHub topics: `rag`, `citations`, `evidence`, `retrieval`, `vector-search`, `agent-tools`, `rust`.

## Open questions

1. **Site location**: `/site` inside repo or separate `aicite.dev` repo?
2. **Custom domain** or default GitHub Pages URL?
3. **Required sections** beyond the minimal list?
4. Should we add **`llms.txt`** now or later?

## Review plan

- [ ] Confirm Leptos CSR + static fallback is acceptable.
- [ ] Confirm GitHub Pages base path and deployment approach.
- [ ] Approve minimal content sections.
- [ ] Decide on `llms.txt` and README updates.

## Related docs

- [README](../../../README.md)
- [API Contract](../../prd/09-api-contract.md)
- [System Architecture](../../prd/07-system-architecture.md)
