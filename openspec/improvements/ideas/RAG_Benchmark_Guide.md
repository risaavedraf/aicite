# Guía de Benchmark para tu RAG CLI (Rust + SQLite)

## Objetivo del Benchmark

Evaluar de forma sistemática la calidad, eficiencia y mejora que trae el grafo jerárquico (v0.2 vs v0.3+) en tu motor de conocimiento.

---

## 1. Métricas Clave

### Retrieval Metrics (Calidad de búsqueda)

- **Context Precision**  
  = (Chunks relevantes recuperados) / (Total chunks inyectados)  
  *Mide cuánto ruido le mandas a la IA.*

- **Context Recall**  
  = (Chunks relevantes recuperados) / (Chunks relevantes según ground truth)  
  *Mide si estás perdiendo información importante.*

- **Hit Rate @K**  
  Porcentaje de queries donde al menos un chunk relevante aparece en los top K resultados.

- **Context Relevancy (LLM-as-Judge)**  
  Usar un LLM para puntuar cuán relevante es cada chunk devuelto (0-1).

### Generation / End-to-End Metrics

- **Faithfulness (Groundedness)**  
  ¿La respuesta final solo contiene información presente en los chunks recuperados? (evita alucinaciones)

- **Answer Relevancy**  
  ¿La respuesta responde directamente a la pregunta del usuario?

- **Answer Correctness**  
  Comparación con respuesta "gold" (ideal).

### Efficiency Metrics

- **Latencia**  
  - Tiempo de búsqueda semántica  
  - Tiempo total (retrieval + generación)

- **Token Efficiency**  
  Promedio de tokens de contexto inyectados por query.

- **Consumo de recursos**  
  Memoria RAM y CPU durante búsquedas.

- **Tamaño de BD** (MB por cantidad de documentos/chunks).

---

## 2. Cómo Medir Relevancia (tu pregunta)

Tu idea es correcta y se llama **Context Precision**.

**Fórmula recomendada:**
```text
Context Precision = Relevant Retrieved / Total Retrieved
Context Recall    = Relevant Retrieved / Total Relevant (ground truth)
```

Necesitas **ground truth** por query: lista de chunks que deberían aparecer.

---

## 3. Metodología Recomendada

### Paso 1: Dataset de Evaluación
Crea un archivo `evaluation_dataset.json` o `.csv` con:

- `query`: string
- `gold_answer`: string (respuesta ideal)
- `relevant_chunk_ids`: array de IDs
- `category`: (ej: factual, summarization, comparison, hierarchical)

**Cantidad ideal inicial:** 40-80 queries representativas de tu uso real.

### Paso 2: Versiones a comparar
- v0.2 (sin grafo)
- v0.3 (con grafo jerárquico)
- Diferentes tamaños de chunk (256, 512, 1024)
- Diferentes `limit` (5, 8, 12)
- Diferentes `min_score`

### Paso 3: Ejecución
- Script que corre tu CLI en batch.
- Guardar todo: query, chunks devueltos, tiempos, JSON de respuesta, etc.

### Paso 4: Evaluación
- Métricas automáticas (Precision, Recall)
- LLM-as-Judge para Faithfulness y Relevancy (Claude 3.5, GPT-4o, Grok, etc.)

---

## 4. Herramientas Útiles

- **Ragas** (Python) → Mejor framework para RAG evaluation.
- **DeepEval**
- Scripts propios en Python (pandas + scikit-learn para métricas clásicas).
- `hyperfine` o `criterion.rs` para medir tiempos en Rust.

---

## 5. Comparaciones Posibles

### Herramientas similares:

- **LlamaIndex** (Python)
- **LangChain** + vector stores
- **Haystack** (deepset)
- **GraphRAG** (Microsoft) → Muy similar a lo que estás haciendo con grafo jerárquico.
- **PrivateGPT** / **AnythingLLM** / **Memex**
- **Obsidian + Copilot** (si usas notas locales)
- Soluciones locales: **LocalAI + RAG**, **Ollama + Continue.dev**

### Contra qué comparar:

1. **Versión anterior de tu propia herramienta** (lo más importante)
2. Búsqueda naive (solo keyword + SQL)
3. Embeddings sin chunking jerárquico
4. Herramientas open-source mencionadas arriba (puedes correr pruebas con las mismas queries)

---

## 6. Hacia dónde apuntar (Roadmap de madurez)

**Nivel Básico (actual):**  
Búsqueda semántica plana + JSON output

**Nivel Bueno (con grafo):**  
- Búsqueda híbrida (semántica + estructural)
- Parent/Child chunks
- Filtros por metadata y jerarquía

**Nivel Avanzado:**
- Multi-hop reasoning
- Agentic RAG (el agente decide cuándo y cómo consultar)
- Actualización incremental de embeddings
- Evaluación automática continua
- Soporte multi-usuario / team knowledge base

**Objetivo ideal:**
Tener un sistema donde:
- Context Precision > 0.75
- Context Recall > 0.70
- Latencia de búsqueda < 800ms (incluso con miles de chunks)
- Faithfulness > 0.90

---

## 7. Checklist para tu Benchmark

- [ ] Dataset de evaluación creado (mínimo 40 queries)
- [ ] Ground truth de chunks relevantes
- [ ] Script de ejecución batch
- [ ] Medición antes/después del grafo jerárquico
- [ ] Reporte con tablas y gráficos (Python + matplotlib/seaborn)
- [ ] Comparación de latencia y token usage
- [ ] Tests con diferentes tamaños de chunk y overlap

---

**Consejo final:**  
Empieza simple. Haz un primer benchmark con solo Context Precision, Recall y Latencia. Eso ya te va a dar muchísimo valor para decidir cómo evolucionar el grafo jerárquico.

Guarda este archivo en la raíz de tu repo como referencia.