# Sample Markdown Document

This is a **markdown** document for testing the AI Harness CLI.

## Overview

The ingest pipeline processes documents through several stages:
1. File validation
2. Text extraction
3. Chunking with overlap
4. Embedding generation

## Configuration

Configuration is loaded from multiple sources:
- CLI flags (highest precedence)
- Environment variables
- Config file (TOML)
- Runtime defaults (lowest precedence)

### Environment Variables

| Variable | Purpose | Default |
|---|---|---|
| `HARNESS_DATA_DIR` | Data directory | OS-specific |
| `HARNESS_EMBEDDING_PROVIDER` | Provider ID | `openai-compatible` |
| `HARNESS_EMBEDDING_MODEL` | Model ID | `text-embedding-3-small` |

## Error Handling

All errors follow a consistent format with machine-readable codes.
Exit codes map to specific error categories for agent consumption.
