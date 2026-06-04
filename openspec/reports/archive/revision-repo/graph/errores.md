# Errores Graph — Pendientes de Fix

> Errores encontrados en la revisión del crate `graph`.
> Este archivo NO se sube a GitHub.

---

## 🔴 CRITICAL

### 1. `heading_parser.rs:17,35` — `char_offset` usa `line.len()` (bytes) en vez de `line.chars().count()`

**Archivo:** `crates/graph/src/heading_parser.rs`
**Líneas:** 17 (`char_offset += line.len() + 1`) y 35 (ídem, en el bloque de heading)

**Problema:** `line.len()` devuelve la cantidad de **bytes** UTF-8, no la cantidad de caracteres. El campo se llama `char_offset` y los consumidores (chunkers) producen offsets basados en **caracteres** (`combined.chars().skip(start)`). Para texto no-ASCII (emojis, acentos, CJK, etc.), los offsets de headings divergen de los offsets de chunks.

**Impacto:** En `ingest/src/lib.rs:94`, `chunk_offsets` provienen del chunker (char-based). En `ingest/src/lib.rs:161`, se comparan con `heading.char_offset` (byte-based). Para texto con multi-byte chars, los chunks se asignan al **tópico/concepto incorrecto**.

**Ejemplo concreto:**
```
## Intro
café résumé 🎉🎊🎉🎊🎉🎊🎉🎊
## Details
```
- Heading "Details" tiene `char_offset` = `len("## Intro\n") + len("café résumé 🎉🎊...\n") + 1`
- `line.len()` de la línea con emojis = ~44 bytes, pero ~22 chars
- El heading "Details" reporta offset ~55 (bytes), pero el chunker reporta offsets en chars (~33)
- → El chunk que debería caer en "Details" cae en "Intro"

**Fix sugerido:** Cambiar `line.len()` por `line.chars().count()` en ambas líneas:
```rust
// Línea 17 (code block):
char_offset += line.chars().count() + 1;

// Línea 35 (end of loop):
char_offset += line.chars().count() + 1;
```

**Nota:** El test `test_char_offsets` usa solo texto ASCII, por lo que no detecta este bug. Agregar un test con texto UTF-8 multi-byte.

---

## 🟠 HIGH

### 2. `hierarchy.rs:128-148` — Boundary matching por título duplicado es frágil

**Archivo:** `crates/graph/src/hierarchy.rs`
**Líneas:** 128-148 (construcción de boundaries)

**Problema:** El código busca headings con `find(|h| h.level == 2 && h.title == topic.name)`. Si un documento tiene dos secciones con el mismo título (ej: dos `## Overview`), `find()` siempre retorna la primera ocurrencia. El segundo Topic tendría el boundary del primero, causando asignación incorrecta de chunks.

**Escenario:**
```markdown
## Overview        ← offset 0
contenido A
## Implementation  ← offset 100
contenido B
## Overview        ← offset 200
contenido C
```
Ambos topics "Overview" obtendrían `char_offset = 0` como boundary, y todos los chunks caerían en el primer "Overview".

**Fix sugerido:** Iterar headings con un índice consumible en vez de `find()`. Al construir boundaries, usar el heading en la posición correspondiente al orden de aparición, no por nombre:
```rust
// En vez de find por título, trackear posición en headings con un cursor
let mut h_idx = 0;
for (t_idx, topic) in topics.iter().enumerate() {
    while h_idx < headings.len() {
        if headings[h_idx].level == 2 {
            boundaries.push((headings[h_idx].char_offset, t_idx, None));
            h_idx += 1;
            break;
        }
        h_idx += 1;
    }
    // ... similar para H3/concepts
}
```

### 3. `heading_parser.rs:14-16` — Detección de code blocks es frágil

**Archivo:** `crates/graph/src/heading_parser.rs`
**Líneas:** 14-16

**Problema:** `trimmed.starts_with("```")` tiene múltiples fragilidades:

a) **Code fences indentados**: CommonMark permite hasta 3 espacios antes del fence. `   ```python` no se detectaría como inicio de bloque de código, y `python` no empieza con `#`, así que no causaría un falso heading, pero `   ```\n## Fake` dentro de un bloque indentado SÍ parsearía como heading.

b) **Fences de 4+ backticks**: ```` ```python ```` abre un bloque que solo ```` ``` ```` (exactamente 3) debería cerrar. Pero `starts_with("```")` también matchea ```` ```` ```` (4 backticks), así que un fence de cierre de 4 backticks se trataría como apertura de otro bloque.

**Impacto medio**: Documentos con code fences indentados o fences de 4+ backticks tendrían headings falsos o headings reales ignorados.

**Fix sugerido:** Regex más preciso o al menos `trimmed.starts_with("```") && !trimmed.starts_with("````")` para el caso de 4+ backticks. Para indentación, usar `line.starts_with("```") || line.starts_with("   ```")` (hasta 3 espacios).

---

## 🟡 MEDIUM

### 4. `hierarchy.rs:110-120` — Topic sin H2 crea nodo sin conceptos, chunks no se asignan

**Archivo:** `crates/graph/src/hierarchy.rs`
**Líneas:** 110-120 (fallback "no H2")

**Problema:** Cuando hay headings pero ninguno es H2 (ej: solo H1 o H3), se crea un Topic con `concepts: Vec::new()`. En la fase de asignación (líneas 154-177), los chunks que caen en este topic no se asignan a ningún concepto porque:
```rust
if let Some(c_idx) = current_c_idx { ... }
else if !topic.concepts.is_empty() { ... }  // ← concepts está vacío, no-op
```

El `topic.chunk_count` se incrementa, pero ningún chunk aparece en `concept.chunk_indices`.

**Inconsistencia**: El path "sin headings en absoluto" (líneas 42-72) SÍ crea un concepto "Default" y asigna chunks. El path "headings sin H2" no.

**Fix sugerido:** En el fallback de "no H2", crear un concepto "Default" igual que en el path de "sin headings":
```rust
if topics.is_empty() {
    // ... crear topic ...
    let concept = Concept { name: "Default".to_string(), ... };
    topics.last_mut().unwrap().concepts.push(ConceptWithChunks {
        concept, chunk_indices: Vec::new(),
    });
}
```

### 5. `types.rs:10,18` — `created_at` como `String` en vez de `DateTime<Utc>`

**Archivo:** `crates/graph/src/types.rs`
**Líneas:** 10 (Topic), 18 (Concept)

**Problema:** `created_at` se almacena como `String` formateado con `"%Y-%m-%d %H:%M:%S"`, mientras que `common::types::Chunk` usa `chrono::DateTime<Utc>`. Esto:
- Impide comparaciones temporales tipadas
- Requiere parsing manual para filtros por fecha
- Inconsiste con el patrón del resto del proyecto

**Impacto bajo-medio**: No causa bugs activos, pero dificulta queries temporales futuras y rompe consistencia del modelo.

**Fix sugerido:** Cambiar a `chrono::DateTime<Utc>` con `#[serde(with = "...")]` si se necesita serialización legible, o usar el mismo patrón que `Chunk`.

### 6. `types.rs:44-51` — `SemanticLink` es código muerto

**Archivo:** `crates/graph/src/types.rs`
**Líneas:** 44-51

**Problema:** `SemanticLink` se define y exporta desde `lib.rs:19`, pero ninguna función del crate lo crea, procesa, ni consume. No hay tests para este tipo.

**Impacto:** Dead code que confunde al lector y aumenta superficie de compilación. Si es forward declaration para una feature futura, debería tener un `// TODO` o `#[allow(dead_code)]` con explicación.

---

## 🟢 LOW

### 7. `lib.rs:21-29` — `Graph` unit struct sin funcionalidad

**Archivo:** `crates/graph/src/lib.rs`
**Líneas:** 21-29

**Problema:** `Graph` es un unit struct sin métodos ni estado. El doc example solo lo crea y descarta. No aporta valor actualmente.

**Sugerencia:** Eliminar o agregar la funcionalidad prometida en el doc comment ("future iterations may hold an in-memory graph index").

### 8. `heading_parser.rs` tests — `test_char_offsets` solo usa ASCII

**Archivo:** `crates/graph/src/heading_parser.rs`
**Líneas:** 107-116

**Problema:** El test de offsets usa `"## First\n\nSome text\n\n## Second"` (todo ASCII), por lo que `line.len() == line.chars().count()` y el bug CRITICAL #1 no se detecta.

**Sugerencia:** Agregar test con texto multi-byte:
```rust
#[test]
fn test_char_offsets_utf8() {
    let md = "## café\n\nrésumé 🎉\n\n## naïve";
    let headings = extract_headings(md);
    // "## café" = 7 chars + 1 newline = 8
    assert_eq!(headings[0].char_offset, 0);
    // "résumé 🎉" = 9 chars + 1 newline = 10 → 8 + 10 = 18
    assert_eq!(headings[1].char_offset, 18);
}
```

### 9. `types.rs:36-38` — `HeadingSpan` sin `Serialize/Deserialize`

**Archivo:** `crates/graph/src/types.rs`
**Líneas:** 36-38

**Problema:** Los otros 3 tipos (`Topic`, `Concept`, `SemanticLink`) derivan `Serialize, Deserialize`. `HeadingSpan` solo tiene `Debug, Clone`. Esto limita la capacidad de serializar el output del parser para debugging, caching, o testing con fixtures.

**Sugerencia:** Agregar `Serialize, Deserialize` si se anticipa necesidad de serialización. Bajo impacto actualmente.

### 10. `hierarchy.rs:128-148` — Boundary lookup es O(T × H)

**Archivo:** `crates/graph/src/hierarchy.rs`
**Líneas:** 128-148

**Problema:** Para cada topic/concept, se itera `headings.iter().find(...)` linealmente. Complejidad O(topics × headings). No es un problema para documentos típicos (<100 headings), pero es innecesario dado que los headings ya están ordenados.

**Sugerencia:** Usar un cursor o HashMap para lookup O(1). Bajo impacto práctico.

---

## ✅ Completados

| # | Archivo | Línea | Descripción | Estado |
|---|---------|-------|-------------|--------|
| — | — | — | — | — |
