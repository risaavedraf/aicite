# Errores Ingest — Pendientes de Fix

> Errores encontrados en la revisión del crate `ingest`.
> Este archivo NO se sube a GitHub.
> Cuando se fixee un error, marcarlo como ✅ y moverlo a completados.

---

## 🔴 CRITICAL

### 1. Panic UTF-8 en truncación de `sanitize_display_name`

**Archivo:** `crates/ingest/src/validator.rs:97-98`

**Problema:** `trimmed[..255]` slicea por bytes, no por caracteres. Si el byte 255 cae dentro de un char multi-byte (emoji, acento, CJK), panic en runtime (`byte index 255 is not a char boundary`).

```rust
// Código actual (bug)
if trimmed.len() > 255 {
    trimmed[..255].to_string()  // PANIC en boundary UTF-8 multi-byte
}
```

**Reproducción:** 254 ASCII + 1 emoji (4 bytes) = 258 bytes, 255 chars. `len()` > 255 → entra al branch → `trimmed[..255]` intenta cortar en medio del emoji → panic.

**Fix:**
```rust
if trimmed.len() > 255 {
    trimmed.chars().take(255).collect::<String>()
} else {
    trimmed.to_string()
}
```

**Nota:** el test `test_sanitize_display_name_truncation` usa `"a".repeat(300)` (ASCII puro) y no detecta el bug. Agregar test con emoji/caracteres multi-byte.

---

### 2. `extract_plain_text` usa `content.len()` (bytes) para `total_chars`

**Archivo:** `crates/ingest/src/extractor.rs:37`

**Problema:** `total_chars = content.len()` retorna el tamaño en bytes, no la cantidad de caracteres Unicode. El campo se llama `total_chars` y el resto del pipeline asume character-based offsets.

```rust
let total_chars = content.len();  // bytes, no chars
```

**Impacto:** para texto con multi-byte chars (acentos, CJK, emoji), `total_chars` es mayor que el conteo real de caracteres. Si algún consumidor usa este valor para validación o allocaciones basadas en char count, obtiene valores incorrectos. Actualmente el valor no se usa críticamente downstream (los chunks usan offsets del chunker, no este total), pero es una bomba semántica.

**Fix:**
```rust
let total_chars = content.chars().count();
```

**Nota:** `extract_pdf_text` tiene el mismo problema en línea equivalente.

---

### 3. `heading_parser.rs` calcula `char_offset` con `line.len()` (bytes)

**Archivo:** `crates/graph/src/heading_parser.rs:17, 35`

**Problema:** La variable se llama `char_offset` pero se incrementa con `line.len()` que es byte length, no char count. El campo `HeadingSpan.char_offset` almacena byte offsets.

```rust
char_offset += line.len() + 1;  // bytes, no caracteres
```

**Impacto directo:** en `lib.rs:130`, `topic_boundaries` se construye a partir de `heading.char_offset` (bytes) y se compara con `c.offset_start` del `chunker.rs` (character offsets). Para texto ASCII puro son idénticos, pero para texto con multi-byte chars, los boundaries están desalineados → chunks se asignan al topic incorrecto o no se asignan.

**Fix:** usar `line.chars().count() + 1` en lugar de `line.len() + 1` para calcular el offset real en caracteres. O alternativamente, renombrar el campo a `byte_offset` y hacer la conversión explícita en `lib.rs`.

---

## 🟠 HIGH

### 4. `sentence_chunker.rs` usa `len()` (bytes) para comparación con `min_chars`

**Archivo:** `crates/ingest/src/sentence_chunker.rs:42`

**Problema:** `current_text.len() < min_chars` compara byte length con un parámetro llamado `min_chars`. Para texto multi-byte, un chunk de 20 caracteres (ej: 20 acentos = 40 bytes) pasaría el threshold de `min_chars=30` incorrectamente.

```rust
} else if current_text.len() < min_chars {  // bytes vs chars
```

**Fix:**
```rust
} else if current_text.chars().count() < min_chars {
```

---

### 5. `sentence_chunker.rs` calcula `offset_end` con `len()` (bytes) en lugar de char count

**Archivo:** `crates/ingest/src/sentence_chunker.rs:47-48, 58-59`

**Problema:** `offset_end = current_offset_start + current_text.len()` usa byte length. Si `current_text` contiene multi-byte chars, `offset_end` sobrepasa el offset real en el texto original.

```rust
let offset_end = current_offset_start + current_text.len();  // bytes
```

**Impacto:** offsets incorrectos para texto no-ASCII. Los offsets se propagan a `ChunkInput.offset_start/end` y luego a `Chunk.offset_start/end` en storage.

**Fix:**
```rust
let offset_end = current_offset_start + current_text.chars().count();
```

---

### 6. Off-by-one en threshold de `min_chars` — fusión incorrecta de chunks

**Archivo:** `crates/ingest/src/sentence_chunker.rs:42`

**Problema:** `current_text.len() < min_chars` usa `<` (strict less-than). Un chunk con exactamente `min_chars` caracteres NO se flush — se mergea con la siguiente oración. La intención documentada dice "shorter than min_chars", pero el comportamiento para `== min_chars` es merge en vez de standalone.

**Reproducción:** `chunk_by_sentence("ABCDE. FGHIJ.", 5)` — "ABCDE." tiene exactamente 5 chars. Con `min_chars=5`, debería ser standalone, pero `< 5` no lo captura → mergea con "FGHIJ.".

**Nota:** el test `test_min_chars_boundary` verifica este caso y pasa actualmente porque `"ABCDE."` tiene 6 bytes (el punto cuenta), así que `6 < 5` es false → flush. Pero `"ABCDE"` (5 chars, sin punto) sería el caso real del bug si el split incluyera el punto en la siguiente oración.

**Fix:**
```rust
} else if current_text.chars().count() <= min_chars {
```

---

## 🟡 MEDIUM

### 7. Match branch inalcanzable en `find_sentence_boundary`

**Archivo:** `crates/ingest/src/chunker.rs:108`

**Problema:** El segundo match arm para `'.' | '!' | '?'` con `i + 1 >= total` es inalcanzable. El primer arm ya matchea todos los casos donde `i + 1 < scan_end` (y `scan_end <= total`). Para que `i + 1 >= total` se cumpla, necesitaría `i + 1 >= scan_end`, pero eso ya fue capturado por el primer arm.

```rust
'.' | '!' | '?' if i + 1 < scan_end => match chars[i + 1] { ... },
'.' | '!' | '?' if i + 1 >= total => Some(i + 1),  // DEAD CODE
```

**Impacto:** funcional es cero (es código muerto), pero confunde a lectores y sugiere que hay un caso edge manejado que en realidad no existe.

**Fix:** eliminar el branch muerto o documentar por qué está ahí como defensa futura.

---

### 8. Test de truncación no detecta bug UTF-8 (test inútil para el caso real)

**Archivo:** `crates/ingest/src/validator.rs:206-210`

**Problema:** `test_sanitize_display_name_truncation` usa `"a".repeat(300)` — ASCII puro. Esto no reproduce el panic de CRITICAL #1. El test pasa pero no valida la corrección del código para texto multi-byte.

```rust
let long_name = "a".repeat(300);  // ASCII: len() == char count
let result = sanitize_display_name(&long_name);
assert_eq!(result.len(), 255);  // Verifica bytes, no chars
```

**Fix:** agregar test con string multi-byte:
```rust
let emoji_name = "🎉".repeat(100);  // 400 bytes, 100 chars
let result = sanitize_display_name(&emoji_name);
assert_eq!(result.chars().count(), 100);  // No debería truncar
```

---

## 🟢 LOW

### 9. `sentence_chunker` no respeta `max_chunk_chars` de IngestConfig

**Archivo:** `crates/ingest/src/sentence_chunker.rs` (todo el módulo), `crates/ingest/src/lib.rs:38-51`

**Problema:** `chunk_by_sentence` solo recibe `min_chars`. El campo `IngestConfig.max_chunk_chars` existe pero no se pasa ni se usa. Una oración muy larga (ej: un párrafo de 5000 chars) producirá un chunk enorme sin división.

**Impacto:** bajo — es un límite conocido del diseño actual. Pero puede causar problemas con modelos de embedding que tienen límites de tokens.

**Fix:** agregar lógica de split dentro de `chunk_by_sentence` para dividir oraciones que excedan `max_chunk_chars`, o al menos documentar la limitación.

---

### 10. Overflow u32 en offsets para documentos muy grandes

**Archivo:** `crates/ingest/src/lib.rs:46-47`

**Problema:** `offset_start: sc.offset_start as u32` y `offset_end: sc.offset_end as u32` — cast silencioso de `usize` a `u32`. Para documentos con >4B caracteres (~16GB ASCII), esto produce overflow silencioso (wrap-around en debug panic en debug mode).

```rust
offset_start: sc.offset_start as u32,
offset_end: sc.offset_end as u32,
```

**Impacto:** muy bajo en la práctica — ningún documento real de texto tendría ese tamaño. Pero es una limitación arquitectural.

**Fix:** cambiar `ChunkInput.offset_start/end` a `u64` o `usize`, o validar que el valor entra en `u32` antes del cast.

---

### 11. Offset tracking drift en `sentence_chunker` por trimming

**Archivo:** `crates/ingest/src/sentence_chunker.rs:36-48`

**Problema:** `offset_end` se calcula como `current_offset_start + current_text.len()` donde `current_text` ya fue trimmeado. Si el texto original de la oración tenía leading/trailing whitespace, el offset_end no refleja la posición real en el texto fuente.

**Ejemplo:** texto `"  Hola mundo. Adiós."` — la primera oración `"Hola mundo."` (trimmeada) tiene offset_start=0, len=11, offset_end=11. Pero en el texto original, "Hola mundo." termina en posición 13 (por los 2 espacios leading).

**Impacto:** bajo — los offsets son aproximados y se usan para hierarchy assignment (que tiene tolerancia). Pero para features futuras que necesiten reconstrucción precisa del texto fuente, sería incorrecto.

---

## ✅ Completados

| # | Fix | Fecha | Notas |
|---|-----|-------|-------|
| - | - | - | - |

---

*Última actualización: 2026-06-02*
*Tests al momento de revisión: 57/57 passing (50 unit + 7 e2e)*
