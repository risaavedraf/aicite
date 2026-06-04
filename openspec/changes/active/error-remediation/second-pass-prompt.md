# Prompt: SDD Second Pass — Error Remediation (T3+T4)

## Contexto

Estamos en el proyecto aiharness (E:\Proyectos\Intento_de_conseguir_pega\aiharness).

Completamos el **first pass** del SDD error-remediation (v0.2.4):
- 3 PRs encadenados, 24+ errores T1+T2 arreglados
- 308 tests pass, 0 clippy warnings, clean format
- Artefactos SDD en `openspec/changes/error-remediation/`
- Tracking en `openspec/reports/error-tracking.md`

**Objetivo:** Arreglar los **78 errores restantes (T3+T4) + 11 casts** descubiertos en verify.

## Última sesión (checkpoint)

- Fix aplicado al PR #7: `crates/cli/src/commands/mod.rs` moviendo `#[cfg(test)] mod tests` al final para pasar `clippy::items_after_test_module`.
- Commit: `06692e6` en `refactor/error-remediation-v2-waves-1-2` (ya push).
- Este archivo fue actualizado para reanudar el second pass en una próxima sesión.

## Errores pendientes

### T3: Medium (37 errores)

| Categoría | Errores | Ejemplos |
|-----------|--------:|---------|
| DRY refactoring | ~14 | Error display ×14 duplicado en CLI |
| Test infrastructure | ~10 | Golden fixtures inconsistentes ×4, tests dependen de network |
| Type consistency | ~5 | `created_at` String vs DateTime<Utc>, offset u32 vs usize vs i64 |
| Misc medium | ~8 | GoldenProvider duplicado, evaluate inconsistencies |

### T4: Low (38 errores)

| Categoría | Errores | Ejemplos |
|-----------|--------:---------|
| Dead code cleanup | ~12 | Engine empty struct, SemanticLink, into_compact_*, Graph unit struct |
| Naming/docs | ~10 | Inconsistent naming, missing doc comments |
| Minor cleanup | ~16 | Unused imports, redundant clones, style issues |

### C9: Newtypes (deferred — ~50 archivos)

`DocumentId`, `ChunkId`, `TraceId` definidos pero no usados. Migración completa toca ~50 archivos.

### Casts fuera de scope (11 errores)

11 unchecked `as u32` casts en `documents.rs`, `traces.rs`, `rate_limits.rs` — mismo patrón que Theme 8 del first pass.

## Fuente de verdad

- Análisis canonical: `openspec/reports/revision-repo/analisis-final-v2.md`
- Error tracking: `openspec/reports/error-tracking.md`
- SDD first pass completo: `openspec/changes/error-remediation/` (proposal, spec, design, tasks, apply-progress, verify-report)

## Requisitos SDD

1. Dividir errores en fases ejecutables (paralelo/async/encadenado)
2. Agrupar por TEMA, no por crate
3. Cada fase independiente o declarar dependencias
4. Priorizar: DRY > test infrastructure > type consistency > dead code > naming > newtypes
5. Review budget < 400 líneas por PR
6. Strategy: ask-always

## SDD Preflight

- execution_mode: interactive
- artifact_store: both (openspec + engram)
- chained_pr_strategy: ask_always
- review_budget_lines: 400

## Instrucciones

1. Leer `openspec/reports/error-tracking.md` para entender el estado actual
2. Leer `openspec/reports/revision-repo/analisis-errores-completo.md` para el inventario completo de errores T3+T4
3. Arrancar SDD: spec → design → tasks → apply → verify
4. Persistir en ambos stores (openspec + engram)
5. Skill de code quality review disponible en `.pi/skills/code-quality-review/SKILL.md`

## Nota sobre newtypes

La migración de newtypes (C9) es el tema más grande (~50 archivos). Tener en cuenta:
- Su propio SDD separado
- Se puede hacer incrementalmente (crate por crate)
- Se agrupa con type consistency para hacerlo en un solo pass

## Notas a tener en cuenta
- Archivos de más de 900 lineas, considerar en este pass refactorizar estos archivos en mas crates, modulos, etc, por separado, considerar dentro del scope archivos de más de 150 lineas

## Expected output

- Artefactos SDD en `openspec/changes/error-remediation-v2/` o similar
- Tracking actualizado en `openspec/reports/error-tracking.md`
- Version bump si amerita (v0.2.5 o v0.3.0 dependiendo del scope)
