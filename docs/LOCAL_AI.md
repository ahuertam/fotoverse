# Local AI — embeddings on-device (Modo 5 semántico)

Creado: 2026-10-28 · Estado: **plan, sin implementar** · Decisión: [ADR-010](ARCHITECTURE.md#adr-010--on-device-embeddings-candle--clip-sobre-llamacpp)

> **Qué es esto.** El plan de implementación del embedding semántico local. La decisión de stack y sus alternativas están en ADR-010; este documento es **cómo se ejecuta**.
>
> **Alcance de este documento:** el **Paso 2** — un binario standalone que demuestra la inferencia local fuera de la app. El **Paso 3** (integración en el pipeline, Modo 5, búsqueda por texto) solo se describe al final, y no se empieza hasta que el Paso 2 haya dado números reales.

---

## 1. El problema que resuelve

fotoverse tiene hoy cuatro modos de detección:

| Modo | Algoritmo | Qué encuentra | Qué se le escapa |
|---|---|---|---|
| 1 — Exacto | GROUP BY por metadatos | Mismo nombre + tamaño + fecha | Contenido distinto con el mismo nombre |
| 2 — Perceptual | pHash (DCT, Hamming ≤ 10) | Misma imagen recomprimida o reescalada | **Misma escena, distinto encuadre** |
| 3 — Visual | Comparativa lado a lado | Lo que el ojo ve | Escala: un par por vez |
| 4 — Vídeo y peso | Metadatos + estimación | Duplicados de vídeo por nombre/tamaño | Todo lo demás |

**El Modo 2 no cubre el caso más común de una biblioteca real.** El pHash mide la distribución de energía de baja frecuencia de la imagen completa. Un recorte —el mismo sitio dos días distintos, la misma persona con otro encuadre— redistribuye esa energía y la distancia de Hamming se dispara. No son "imágenes parecidas": son **escenas distintas para el pHash y la misma escena para una persona**.

Un embedding semántico sí las agrupa, porque no compara píxeles sino significado.

---

## 2. Por qué un binario standalone y no la app

Ponytail: **lo mínimo que demuestra el requisito, sin tocar la app.**

El objetivo del Paso 2 no es una feature: es **obtener cuatro números reales** que el ADR-010 declara pendientes, y tener un artefacto que enseñar.

Si el resultado del spike es negativo (el modelo no carga, el coste es inviable, las fotos del mismo sitio no se agrupan), se dice y se para. Integrarlo en la app antes de tener los números sería trabajo que se puede tirar.

**Qué NO incluye el Paso 2:**

- Nada de UI. Ni una página, ni un botón, ni un router.
- Nada de SQLite. No toca la base de datos.
- Nada de Tauri. Es un binario de consola, sin `tauri::Builder`.
- Nada de la CI todavía (ver §7).

**Qué sí incluye:**

1. Cargar CLIP ViT-B/32 cuantizado en CPU.
2. Recorrer una carpeta y sacar el embedding de cada imagen.
3. Comparar por similitud coseno y sacar los k-vecinos más cercanos.
4. Admitir una query de texto y buscar contra los embeddings de imagen.
5. Imprimir el resultado en texto legible, y en JSON con `--json`.

---

## 3. Diseño técnico

### 3.1 Dónde vive el código

```
src-tauri/
├── Cargo.toml          # + [features] embed, + [[bin]] embed
└── src/
    ├── bin/
    │   └── embed.rs    # el binario (nuevo)
    └── lib.rs          # sin tocar
```

**El crate `fotoverse_lib` no cambia.** El binario es un target de Cargo aparte, así que la app sigue compilando sin las dependencias de ML.

### 3.2 Cargo

```toml
[features]
embed = ["dep:candle-core", "dep:candle-transformers"]

[dependencies]
candle-core         = { version = "0.9", optional = true }
candle-transformers = { version = "0.9", optional = true }

[[bin]]
name = "embed"
path = "src/bin/embed.rs"
required-features = ["embed"]
```

Tres razones de esta forma:

- **`required-features`** es lo que mantiene `candle` fuera del bundle de la app. El día que el Modo 5 entre en el producto, el feature pasa a ser normal; mientras tanto, `cargo build` de la app no toca nada de ML.
- **La feature `quantized` de candle** es la que permite cargar pesos cuantizados. Va en la dependencia, no en la feature del proyecto: `candle-core = { version = "0.9", optional = true, features = ["quantized"] }`.
- **La versión se fija al implementar.** No se pone aquí una versión que no se ha comprobado que exista.

> `image` ya es dependencia del crate y sirve para el preprocesado. No hace falta nada más.

### 3.3 El modelo

| | |
|---|---|
| Modelo | **CLIP ViT-B/32** (OpenAI), torre de visión + torre de texto |
| Formato | Cuantizado **int8** (a confirmar: candle soporta `quantized`, pero el formato exacto de pesos hay que verificarlo) |
| Origen | HuggingFace → conversión a un formato que candle pueda leer |
| Dimensiones | 512 |
| Dónde vive | `./models/clip-vit-b32-int8/` junto al binario, o `$FOTOVERSE_MODEL_DIR` |
| Descarga | **Fuera de este paso.** El binario espera a que el modelo esté en disco y dice exactamente qué falta y de dónde bajarlo. |

**El binario no descarga nada.** Red dentro de la app es exactamente lo que el producto promete no hacer (ADR-010 / §Seguridad). El Paso 2 no lo rompe ni por sorpresa.

### 3.4 Pipeline de inferencia

Este es el punto donde los embeddings se rompen en silencio. Los pasos **no son** negociables si se quiere comparar con las métricas publicadas de CLIP:

```
1. decode          image::open → DynamicImage → RGB8
2. resize          lado corto → 224, FilterType::CatmullRom
3. center crop     224×224
4. escala          / 255.0            → [0, 1]
5. normaliza       (x - mean) / std
                   mean = [0.48145466, 0.4578275, 0.40821073]
                   std  = [0.26862954, 0.26130258, 0.27577711]
6. tensor          NCHW, f32
7. encode          vision tower → vector de 512 dims
8. normaliza L2    (si no, el coseno no es el coseno)
```

**El center crop es una decisión, no un detalle.** Para una biblioteca de fotos personales, recortar al centro **tira información**: lo que diferencia dos fotos del mismo sitio puede estar justo en el borde.

| Opción | A favor | En contra |
|---|---|---|
| **(a) Centro, estándar** | Es lo que hizo el entrenamiento. Los embeddings son comparables con los publicados | Pierde los bordes |
| **(b) Squash a 224×224, sin recorte** | Conserva toda la imagen | Fuera de la distribución de entrenamiento → embeddings algo peores |

**El spike empieza con (a)** y comprueba (b) si (a) falla con fotos panorámicas o con el sujeto descentrado. Es una de las cuatro preguntas de §6.

**Batch = 1 al principio.** Se mide así y se decide. Meter batching es una optimización que solo se justifica con el número real en la mano, y complica el preprocesado (padding). Si el número dice que hace falta, se añade después.

### 3.5 Coseno y k-NN

```
cos(a, b) = (a·b) / (‖a‖ · ‖b‖)
```

Con los vectores ya normalizados a L2, el coseno es el producto escalar. Ojo: **CLIP no normaliza la salida** — hay que normalizar explícitamente o los "cosenos" no son cosenos.

**Búsqueda: fuerza bruta, O(n²).** Para las 27-200 imágenes del spike es instantáneo, y para 50k son 1,25B productos de 512 dims ≈ 640 GFLOPs, o sea segundos con `rayon`. No hace falta índice ANN todavía.

> Consistente con cómo se докуmentó el techo del clustering de pHash (`detect/perceptual.rs`): el O(n²) actual es **deliberado y está nombrado**, y el sustituto —un índice ANN— se decide solo cuando el número lo pida. **No se pre-construye el ANN.**

### 3.6 Salida

Por defecto, texto legible:

```
$ embed ./mis-fotos --top 3
  127 imágenes · modelo clip-vit-b32-int8 · device cpu · 0.00 MB de red

  foto_14.jpg  ↔  foto_27.jpg   cos=0.941
  foto_14.jpg  ↔  foto_31.jpg   cos=0.887
  ...
```

Y con `--text` (usa la torre de texto):

```
$ embed ./mis-fotos --text "playas al atardecer" --top 3
  foto_14.jpg   cos=0.883
  foto_27.jpg   cos=0.851
```

Y `--json` para consumo máquina:

```json
{ "model": "clip-vit-b32-int8", "dim": 512, "images": 127,
  "pairs": [{"a": "foto_14.jpg", "b": "foto_27.jpg", "cos": 0.941}] }
```

**El JSON incluye la versión del modelo.** Es el requisito del ADR-010: sin la versión guardada, no se puede saber si un embedding es comparable con otro.

---

## 4. El fixture: por qué `trial/` no sirve aquí

**La fixture `trial/` es sintética.** `tests/fixture.rs` genera JPEG de 256×192 con un gradiente determinista a partir de un seed. Sirve perfectamente para las regresiones de pHash (hash exacto, mismo contenido → mismo hash).

**Para CLIP no sirve.** CLIP no ve "gradiente con seed 7"; ve textura, y 27 gradientes distintos son 27 imágenes igualmente incomparables. Las distancias saldrán todas parecidas y no significarán nada.

**El fixture que hace falta:** fotos **reales** con ground truth conocido.

La forma barata de construirlo, sin subir fotos personales a un repo:

1. Coger ~8 fotos reales de una carpeta cualquiera (hay de sobra en `~/Pictures` o `~/Desktop`).
2. De cada una, sacar 3 recortes distintos (cuadrado, vertical, centro-shift).
3. El resultado son 24-32 fotos donde **se sabe** qué grupos son la misma imagen.

**El ground truth son las 8 originales**, y los recortes son sus "duplicados" con distinta relación de aspecto.

Y el segundo ground truth, el que de verdad importa: **dos fotos del mismo sitio con distinta luz o distinto encuadre**, tomadas con semanas de diferencia. Eso no se puede fabricar, hay que fotografiarlo. Con 5-10 pares así, el umbral de coseno se calibra contra casos reales y no contra gradientes.

**Estructura del fixture (local, no en git):**

```
~/fotoverse-ai-fixture/
├── groundtruth.tsv      # imagen_a<TAB>imagen_b<TAB>relacion
├── originals/           # 8 fotos reales
└── crops/               # 24 recortes derivados
```

`groundtruth.tsv` es lo que permite **medir precisión/recall** en vez de mirar números en un terminal.

> El fixture **no se sube al repo**: son fotos personales. Lo que sí se sube es `groundtruth.tsv` con nombres de archivo genéricos, o mejor, el **generador del fixture** (el script que hace los recortes), que es lo reproducible.

---

## 5. Tareas, con criterio de "hecho"

| # | Tarea | Criterio de "hecho" | Effort |
|---|---|---|---|
| 1 | Fixture local | `~/fotoverse-ai-fixture/` con originales, recortes y `groundtruth.tsv`. Script de recortes versionado | 1 h |
| 2 | Feature + target Cargo | `cargo build --features embed` compila un binario `embed`; `cargo build` (sin feature) **no** compila candle | 30 min |
| 3 | Carga del modelo | El binario carga CLIP int8 en CPU y falla con un mensaje accionable si el modelo no está | 2-3 h |
| 4 | Embeddings de imagen | `embed <carpeta>` produce 1 vector de 512 dims por imagen, y el count coincide con el de la carpeta | 2-3 h |
| 5 | Coseno + k-NN | `embed <carpeta> --top k` lista los pares, y los recortes de la misma original quedan agrupados | 1 h |
| 6 | Búsqueda por texto | `embed <carpeta> --text "..."` devuelve imágenes coherentes con la query | 1 h |
| 7 | Salida JSON | `--json` emite el formato de §3.6, con la versión del modelo | 30 min |
| 8 | Las 4 preguntas | §6 contestadas **con números medidos**, no con estimaciones | 1 h |
| 9 | Umbral calibrado | Umbral de coseno elegido contra `groundtruth.tsv`, **con los falsos positivos y falsos negativos documentados** | 1-2 h |

**Total estimado: 2-3 días.** Si la tarea 3 se atasca (candle no carga el int8), parar ahí y replantear: el plan B de ADR-010 es `ort` + `fastembed`.

---

## 6. Las cuatro preguntas del spike

Las mismas cuatro que ADR-010 declara pendientes. **Sin estos cuatro números, el ADR no se puede revisar a "implementado"**:

1. **¿Carga candle un CLIP ViT-B/32 cuantizado a int8?** Si no → el plan B (`ort` + `fastembed`) o un formato de pesos distinto.
2. **¿Cuánto ocupa el modelo cuantizado?** Y por tanto, ¿la descarga explícita del usuario es asumible?
3. **¿Cuántos ms por imagen?** En el hardware de un portátil, no en un bench de servidor. Con el número: ¿50k imágenes son 15 min o son 2 horas?
4. **¿Lee candle pesos GGUF cuantizados?** Si sí, cambiar de runtime más adelante es barato.

Y una quinta que no está en el ADR y que sale de §3.4:

5. **¿(a) center crop o (b) squash?** Con ground truth real, no con intuición.

---

## 7. Qué pasa con la CI

**No se añade `cargo test` ni `cargo build --features embed` a la CI en este paso.** Motivos:

- La CI actual **no ejecuta los tests** (ver `tasks` en el repo): `cargo check`, `clippy` y `fmt` solamente. Añadir el binario sin arreglar eso sería añadir una cosa más a una suite que ya está en rojo.
- El feature `embed` es **opcional** por diseño. La CI principal debe seguir compilando sin candle — eso es justamente lo que garantiza la feature.

**Orden correcto cuando llegue:**

1. Arreglar la suite de tests (fixture fuera de rutas absolutas, `#[ignore]` en el regenerador, `cargo test` en CI).
2. **Entonces** añadir un job de CI con `--features embed` que solo haga `cargo check` (no benchmarks, no inferencia).
3. La inferencia en CI no tiene sentido: descarga de cientos de MB y CPU variable.

---

## 8. Paso 3 — integración (solo después)

**Precondición:** el Paso 2 produjo números y el umbral está calibrado.

Qué implicaría:

| Pieza | Nota |
|---|---|
| `files.emb BLOB` | 512 × f16 = 1 KB por imagen. Migración de columna, como las de `db/migrations.rs` |
| `populate_embeddings()` | Paralelo a `populate_session()`, con `rayon` y pool acotado (ADR-007) |
| Modo 5 — Semántico | Clusteriza por coseno, mismo esquema que `perceptual_groups` |
| Búsqueda por texto | Una query → embedding de texto → coseno contra los embeddings de imagen |
| Gestión del modelo | Descarga explícita por el usuario a `app_cache_dir()/models/`, con progreso y tamaño en la UI. **Nunca en background** |
| Versionado del vector | Nombre + versión del modelo guardados con cada embedding, para poder re-embeber |
| Errores | Reutilizan `hash_status` / `hash_error`, igual que pHash |

**El punto donde más se puede romper todo es el último de la tabla:** la app no puede romper su promesa de "100% offline", y el modelo son cientos de MB. La UI tiene que decirlo claro antes de la descarga.

---

## 9. Fuentes

- [CLIP (paper, OpenAI)](https://github.com/openai/CLIP) — entrenamiento, preprocessing y limitaciones documentadas
- [candle](https://github.com/candle-repo/candle) · [candle-transformers](https://github.com/candle-repo/candle/tree/main/candle-transformers)
- [CLIP ViT-B/32 en HuggingFace](https://huggingface.co/sentence-transformers/clip-ViT-B-32)
- [llama.cpp multimodal (mtmd)](https://github.com/ggml-org/llama.cpp/blob/master/docs/multimodal.md) — la alternativa evaluada
- [ONNX Runtime](https://onnxruntime.ai/) · [fastembed](https://github.com/qdrant/fastembed) — plan B
