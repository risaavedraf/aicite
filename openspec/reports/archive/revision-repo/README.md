# Revisión del Repositorio — Workflow

> Documentación interna. No subir a GitHub.
> Idioma: español (para el usuario).

## Propósito

Entender cómo funciona el código del proyecto crate por crate. El foco es **comprensión**, no fixeo. Los errores encontrados se persisten en Engram y en el `errores.md` de cada carpeta para fixear después.

## Estructura de Carpetas

```
revision-repo/
├── README.md                              # Este archivo (workflow)
├── cli/                                   # Revisión del crate CLI
│   ├── review.md                          # Reporte de comprensión
│   └── errores.md                         # Errores encontrados en CLI
├── compliance/                            # Revisión de compliance vs PRDs
│   └── review.md
├── code-review-anterior/                  # Items diferidos de code review previo
│   └── items-pendientes.md
├── common/                                # ⏳ Pendiente
├── config/                                # ⏳ Pendiente
├── engine/                                # ⏳ Pendiente
├── storage/                               # ⏳ Pendiente
├── ingest/                                # ⏳ Pendiente
│   └── errores.md                         # Errores encontrados en ingest
├── providers/                             # ⏳ Pendiente
├── retrieval/                             # ⏳ Pendiente
└── graph/                                 # ⏳ Pendiente
```

Cada crate revisado tiene su propia carpeta con:
- `review.md` — reporte de comprensión (cómo funciona el código)
- `errores.md` — errores encontrados para fixear después

## Workflow de Revisión

### 1. Preparación

Antes de arrancar la revisión de un crate:

1. Leer este README para entender el flujo
2. Leer el reporte de compliance (`compliance/review.md`) para contexto de hallazgos conocidos
3. Buscar en Engram observaciones previas del crate a revisar
4. Listar los archivos del crate con `find crates/<nombre> -name "*.rs"`

### 2. Revisión del Crate

Para cada crate, delegar a un subagent reviewer con estas instrucciones:

```
Tarea: Revisión de crates/<nombre> en español
Objetivo: ENTENDER cómo funciona el código, no fixear

Foco:
  - Qué hace cada archivo/módulo
  - Cómo se conectan las piezas
  - Flujo de datos (de input a output)
  - Decisiones de diseño y sus tradeoffs
  - Errores encontrados → NO fixear, solo documentar

Formato del reporte (en español):
  1. Resumen del crate (propósito, estructura)
  2. Flujo principal (paso a paso)
  3. Módulos/archivos clave (qué hace cada uno)
  4. Decisiones de diseño observadas
  5. Errores encontrados (con archivos, líneas, fix sugerido)
  6. Conexiones con otros crates

Outputs:
  - Reporte: openspec/reports/revision-repo/<crate>/review.md
  - Errores: openspec/reports/revision-repo/<crate>/errores.md
```

### 3. Después de la Revisión

1. Guardar el reporte en `revision-repo/<crate>/review.md`
2. Guardar errores en `revision-repo/<crate>/errores.md`
3. Persistir errores en Engram con `mem_save`:
   - type: `bugfix` o `discovery`
   - scope: `project`
   - contenido: qué, por qué, dónde, cómo fixear
4. Actualizar la tabla de progreso abajo

### 4. Plan de Fixeo

Cuando se terminen todas las revisiones, crear un plan de fixeo basado en:
- Errores en `errores.md` de cada carpeta y Engram
- Prioridad (CRITICAL > HIGH > MEDIUM > LOW)
- Dependencias entre fixes

## Crates del Proyecto

| Crate | Propósito | Estado |
|-------|-----------|--------|
| `cli` | Interfaz de línea de comandos, 14 subcomandos | ✅ Revisado |
| `common` | Tipos compartidos, errores, traits | ✅ Revisado |
| `config` | Carga de configuración, runtime modes | ✅ Revisado |
| `engine` | Lógica de negocio: ingest, context, recovery | ✅ Revisado |
| `storage` | Persistencia SQLite, documentos, chunks, traces | ✅ Revisado |
| `ingest` | Pipeline de ingesta: extracción, chunking, validación | ✅ Revisado |
| `providers` | Embedding providers (Gemini, OpenAI-compatible) | ✅ Revisado |
| `retrieval` | Búsqueda vectorial, scoring, ranking | ✅ Revisado |
| `graph` | Grafo de conocimiento (si existe) | ✅ Revisado |

## Convenciones

- **Todo en español** — el usuario necesita entender sin barrera de idioma
- **Foco en comprensión** — explicar QUÉ hace el código, no solo listar bugs
- **Errores separados por crate** — cada carpeta tiene su propio `errores.md`
- **Errores también en Engram** — doble respaldo para no perder nada
- **Una carpeta por crate** — todo el material de un crate va en su carpeta

---

*Creado: 2026-06-02*
*Última actualización: 2026-06-02*
