# Prompt: SDD Verify & Review — error-remediation-v2

## Contexto

Estamos en el proyecto aiharness (E:\Proyectos\Intento_de_conseguir_pega\aiharness).

Completamos el **second pass** del SDD error-remediation-v2:
- 6 PRs encadenados aplicados sobre la branch `refactor/error-remediation-v2-waves-1-2`
- 297 tests pass, 0 clippy warnings, clean format
- Artefactos SDD en `openspec/changes/error-remediation-v2/`
- Tracking actualizado en `openspec/reports/error-tracking.md`

## Objetivo

1. **Review fresco** de todos los commits del second pass (7 commits)
2. **Verificar** que no quedaron regresiones o items sin resolver
3. **Decidir** si la branch está lista para PR a main
4. **Identificar** cualquier issue antes de merge

## Commits a revisar

```
c329610 fix: setup saves model, improve config docs and deprecation warning
213ee99 chore: remove dead code, unused structs, and stale dependency
48b0ffc fix(storage): rate-limit pruning, shared row mapper, corrupt blob errors
f09b06f fix(storage): replace unchecked as u32 casts with checked helpers
46d88ac test: add deterministic edge-case tests and ignore network tests
06692e6 fix(cli): move tests after command helpers
f6c2a3a refactor(error-remediation): apply v2 remediation waves
```

## Checklist de verificación

### Correctness
- [ ] `cargo test` — todos los tests pasan
- [ ] `cargo clippy -- -D warnings` — sin warnings
- [ ] `cargo fmt --check` — formato limpio
- [ ] `grep -rn "as u32" crates/storage/src/` — cero casts sin checked helpers fuera de tests
- [ ] No hay `unwrap()` nuevos en production code paths
- [ ] Los `#[ignore]` tests tienen reason strings

### PR-by-PR review
- [ ] **PR-1 (CLI DRY):** `exit_for_error` y `validate_retrieval_scope` son helpers correctos, no cambian behavior del usuario
- [ ] **PR-2a (Golden fixtures):** fixture source es canonical (engine), no duplicado
- [ ] **PR-2b (Deterministic tests):** tests de red están `#[ignore]`, edge cases son correctos
- [ ] **PR-3 (Cast safety):** `i64_to_u32` / `usize_to_u32` propagan errores correctamente
- [ ] **PR-4 (Storage correctness):** pruning es best-effort (no falla la operación principal), corrupt blobs retornan error
- [ ] **PR-5 (Dead code):** solo se eliminó code no utilizado, SemanticLinkRow preservado
- [ ] **PR-6 (Setup/UX):** setup ahora guarda el modelo testeado, deprecation warning es claro

### API / Breaking changes
- [ ] No hay cambios en public API signatures de storage, engine, o retrieval
- [ ] Los commands de CLI producen el mismo output para el usuario
- [ ] `to_compact_*` functions siguen intactas (solo se eliminaron `into_compact_*`)

### Tracking
- [ ] `openspec/reports/error-tracking.md` refleja el estado actual
- [ ] `openspec/changes/error-remediation-v2/apply-progress.md` tiene los 6 PRs documentados
- [ ] Items deferred están documentados (C9 newtypes, H7 rollback, H19 ScoredChunk)

## Decisiones pendientes

1. **Merge strategy:** ¿PR a main directo, o squash por tema?
2. **Version bump:** ¿v0.2.5 o v0.3.0? (scope significativo pero sin breaking changes)
3. **Changelog:** Actualizar CHANGELOG.md con los cambios del second pass

## Archivos relevantes

- `openspec/changes/error-remediation-v2/apply-progress.md` — estado de los 6 PRs
- `openspec/changes/error-remediation-v2/tasks.md` — task breakdown completo
- `openspec/changes/error-remediation-v2/design.md` — diseño del second pass
- `openspec/reports/error-tracking.md` — tracking consolidado
- `openspec/changes/error-remediation/verify-report.md` — verify del first pass

## Expected output

- Review fresco con verdict (approve / request changes / needs discussion)
- Lista de issues encontrados (si los hay)
- Recomendación de merge strategy
- Confirmación de que la branch está lista para PR a main
