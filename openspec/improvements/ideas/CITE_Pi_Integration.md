# CITE + Pi.dev Integration Guide

## Objetivo
Guía para integrar **CITE** (tu motor RAG) con **Pi.dev** de forma eficiente, manteniendo el modelo de embeddings cargado en memoria y reduciendo latencia.

---

## 1. Problema Actual
- Gemini Embedding tarda ~1 segundo por query.
- Solución ideal: Modelo local cargado una sola vez.

---

## 2. Consumo de Recursos (Modelos Recomendados)

| Modelo                        | Dimensiones | RAM (aprox.)     | VRAM (GPU)     | Tiempo embedding (CPU) | Tiempo (GPU) | Recomendado para |
|-------------------------------|-------------|------------------|----------------|------------------------|--------------|------------------|
| all-MiniLM-L6-v2              | 384        | 150-300 MB      | -             | 15-40 ms              | -           | Pruebas rápidas |
| **Nomic-embed-text-v1.5**     | 768        | **400-700 MB**  | 300-600 MB    | 30-70 ms              | 10-25 ms    | **Balance ideal** |
| BAAI/bge-m3                   | 1024       | 700-1100 MB     | 500-900 MB    | 50-120 ms             | 15-40 ms    | Alto multilingüe |
| Snowflake-arctic-embed-l      | 1024       | 800-1300 MB     | 600-1000 MB   | 60-150 ms             | 20-50 ms    | Máxima calidad |

**Recomendación principal:** Empieza con **Nomic-embed-text-v1.5** (768 dims).

---

## 3. Arquitectura Recomendada con Pi

### Enfoque: Extensión / Skill de Pi

Pi carga la extensión al iniciar → CITE carga el modelo una sola vez → Las búsquedas son on-demand y rápidas.

**Ventajas:**
- Modelo cargado persistentemente mientras Pi corre.
- Baja latencia (20-80ms por embedding).
- Fácil de usar desde el agente de Pi.

---

## 4. Estructura Sugerida de la Extensión

```
cite-pi-extension/
├── manifest.json
├── src/
│   ├── main.rs              # Registro de la skill
│   ├── cite_engine.rs       # Wrapper de tu engine Rust
│   ├── model_manager.rs     # Carga y maneja el modelo de embedding
│   └── tools/
│       ├── search.rs
│       └── ingest.rs
├── models/                  # (opcional) modelos descargados
└── Cargo.toml
```

**Componentes clave:**

- `model_manager.rs`: Carga el modelo una sola vez al iniciar la extensión.
- `tools/search.rs`: Llama a tu CLI o directamente al engine con el modelo ya cargado.
- Manejo de errores si el modelo no se pudo cargar.

---

## 5. Modos de Operación

1. **Modo Integrado con Pi** (recomendado)
   - Pi inicia → carga modelo → skill `cite-search` disponible.

2. **Modo CLI independiente**
   - Mantienes `cite search` funcionando como antes.

3. **Híbrido**
   - La extensión detecta si el daemon de CITE está corriendo o carga su propio modelo.

---

## 6. Consideraciones Técnicas

- **Carga del modelo**: Hazla lazy (solo cuando se use la skill por primera vez) para no ralentizar el arranque de Pi.
- **Memoria**: Monitorea el consumo. 700MB extra es aceptable en la mayoría de máquinas modernas.
- **Multilingüe**: Prioriza modelos buenos en español si tu base de conocimiento está en español.
- **Re-ingest**: Cuando cambies de modelo de embedding, debes regenerar todos los vectores en la base de datos.

---

## 7. Próximos Pasos Sugeridos

1. Elegir modelo (Nomic recomendado).
2. Crear la extensión mínima que solo exponga `cite search`.
3. Medir latencia antes/después con el benchmark.
4. Agregar soporte para `ingest` y `list documents`.
5. (Opcional) Agregar comando `cite status` para ver si el modelo está cargado.

---

**Notas finales:**
- Esta integración hace que CITE se sienta como una herramienta nativa dentro de Pi.
- Mantén el CLI puro en Rust lo más independiente posible (buena práctica).

¿Quieres que agregue ejemplos de código de estructura o prompts para registrar la tool en Pi?
