# SDD Artifacts — AI Cite CLI

This directory contains Specification-Driven Development (SDD) artifacts organized by status.

📋 **[Roadmap](roadmap.md)** — Full phase plan with deliverables and dependencies

## Structure

```
changes/
├── active/                    # Work in progress
│   ├── error-remediation/     # First-pass error fixes
│   ├── error-remediation-v2/  # Second-pass error fixes
│   └── error-remediation-v3/  # Verification pass ← current
├── completed/                 # Verified and done
│   ├── phase-10-*/
│   ├── phase-11-*/
│   └── phase-12-*/
└── archive/                   # Historical phases (1–9)
```

## Phases

| Phase | Name | Status |
|---|---|---|
| 1 | Scaffold | ✅ Archived |
| 2 | Ingest Pipeline | ✅ Archived |
| 3 | Retrieval Pipeline | ✅ Archived |
| 4 | Context Packs + Citations | ✅ Archived |
| 5 | Durability (locks, rate limits) | ✅ Archived |
| 6 | Evaluation (golden dataset) | ✅ Archived |
| 7 | Packaging + Docs | ✅ Archived |
| 8 | Rename to Cite | ✅ Archived |
| 9 | Installation Experience | ✅ Archived |
| 10 | Hierarchical Graph Foundation | ✅ Completed |
| 11 | Hierarchical Retrieval | ✅ Completed |
| 12 | Agent UX | ✅ Completed |

## Active Work

| Change | Status |
|---|---|
| error-remediation-v3 | 🔄 Active |
