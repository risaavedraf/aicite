# SDD Spec — Phase 3: Retrieval Pipeline

## 1) Retrieval data contract

### Candidate record (from storage)

Each candidate must include:
- `document_id`
- `display_name`
- `chunk_id`
- `section_id`
- `chunk_index`
- `text`
- `page`
- `offset_start`
- `offset_end`
- `vector: Vec<f32>`

Only candidates from documents with `status='ready'` are eligible.

## 2) Similarity

### Cosine similarity

Given query vector `q` and candidate vector `v`:

`score = dot(q,v) / (||q|| * ||v||)`

Rules:
- If dimensions differ, candidate is skipped.
- If either norm is zero, candidate is skipped.
- Score domain is `[-1, 1]`.

## 3) Top-k selection

- Runtime `k` source:
  1. CLI `--k`
  2. `config.retrieval.top_k`
- Valid range: `1..=10`
- Default: `5`
- Sorting: descending by similarity score
- Result size: `min(k, eligible_candidates)`

## 4) Query validation

- Query must be non-empty after trim.
- Max query length: 4000 chars.
- Violations return `CiteError::InvalidParameter` (empty) or `CiteError::QueryTooLong`.

## 5) Engine behavior

### `search`

Input: query + optional k

Output per hit:
- metadata + `score`
- short preview (first ~160 chars, whitespace normalized)

### `retrieve`

Input: query + optional k

Output per hit:
- same metadata + `score`
- full chunk text

## 6) CLI behavior

### `cite search <query> [--k N]`

- JSON output contains `{ query, top_k, hit_count, results[] }`
- Human output prints ranked concise lines.

### `cite retrieve <query> [--k N]`

- JSON output contains `{ query, top_k, hit_count, results[] }`
- Human output prints ranked hits and chunk text.

## 7) Partial-corpus handling

- Documents in `pending`, `processing`, or `failed` are ignored.
- No eligible chunks must not be treated as an error.

## 8) Tests

Required coverage:
- cosine success, dimension mismatch, zero norm
- top-k clipping
- ready-only filtering at storage layer
- engine validation of query/k
- empty-corpus behavior
