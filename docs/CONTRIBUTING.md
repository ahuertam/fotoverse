# Guía para contribuir

> Cómo trabajar en el código de fotoverse: setup, convenciones, flujo de PR
> y reglas de seguridad. Si vas a tocar **código de producto** (no docs),
> lee esto antes de abrir tu primer PR.

## Tabla de contenidos

- [Setup del entorno](#setup-del-entorno)
- [Estructura del repo](#estructura-del-repo)
- [Convenciones de código](#convenciones-de-código)
- [Cómo correr los tests](#cómo-correr-los-tests)
- [Añadir una dependencia Rust](#añadir-una-dependencia-rust)
- [Añadir un comando Tauri](#añadir-un-comando-tauri)
- [Flujo de Pull Request](#flujo-de-pull-request)
- [Seguridad](#seguridad)

---

## Setup del entorno

### 1. Prerrequisitos

| Herramienta | Versión mínima | Notas                                                                 |
|-------------|----------------|-----------------------------------------------------------------------|
| Rust        | 1.75           | Instalar con [rustup](https://rustup.rs/).                            |
| Node.js     | 20             | LTS actual.                                                           |
| pnpm        | 9              | `npm install -g pnpm` o vía Corepack (`corepack enable pnpm`).        |
| Tauri CLI   | 2.x            | Incluido en `package.json`; se invoca con `pnpm tauri …`.             |
| Xcode CLT   | —              | macOS: `xcode-select --install`.                                      |
| MSVC Build Tools | —         | Windows: instalar "Desktop development with C++" desde Visual Studio Installer. |

### 2. Clonar y arrancar

```bash
git clone https://github.com/ahuertam/fotoverse.git
cd fotoverse
pnpm install
pnpm tauri dev
```

La **primera compilación** tarda varios minutos (Rust compila todas las
dependencias). Las siguientes son incrementales.

### 3. Troubleshooting común

| Síntoma                                                                  | Solución                                                                                                          |
|--------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------|
| macOS: `error: failed to run custom build command for … openssl-sys`     | `brew install openssl@3` y exportar las vars que sugiere el error, o `rustup target add x86_64-apple-darwin`.     |
| macOS Apple Silicon: `Bad CPU type` al ejecutar el binario               | `softwareupdate --install-rosetta`.                                                                               |
| Windows: `error: linker 'link.exe' not found`                            | Instalar MSVC Build Tools + Windows SDK.                                                                          |
| Windows: `error: Microsoft Visual C++ 14.0 or greater is required`       | Igual: MSVC Build Tools.                                                                                          |
| Linux: `webkit2gtk-4.0 not found`                                        | `sudo apt install libwebkit2gtk-4.0-dev build-essential` (no soportado oficialmente, pero funciona para dev).       |
| `pnpm tauri dev` se cuelga en "Compiling tauri…"                         | Normal en la primera ejecución. Paciencia. Ctrl+C y reintentar; usa la caché incremental.                         |
| Thumbnails no aparecen                                                   | Verificar que `app_cache_dir()` tiene permisos de escritura; revisar logs con `RUST_LOG=fotoverse=debug pnpm tauri dev`. |

---

## Estructura del repo

Resumen 1-línea por carpeta. Para el árbol completo, ver
[`ARCHITECTURE.md`](ARCHITECTURE.md#layout-del-repositorio).

| Carpeta / archivo         | Contenido                                                                       |
|---------------------------|---------------------------------------------------------------------------------|
| `src-tauri/src/`          | Backend Rust: scanner, hasher, base de datos, comandos Tauri, thumbnails.       |
| `src-tauri/src/db/`       | Migraciones y esquema SQLite. **Una única fuente de verdad del esquema.**       |
| `src-tauri/src/walk/`     | Recorrido del filesystem con `jwalk`.                                           |
| `src-tauri/src/hash/`     | pHash de imágenes (`img_hash`) y keyframes de vídeos (`ffmpeg-sidecar`).        |
| `src-tauri/src/detect/`   | Algoritmo de los tres modos (exacto, perceptual, manual).                       |
| `src-tauri/src/review/`   | `trash::delete_all` y reporte de fallos.                                        |
| `src-tauri/src/thumbs/`   | Generación y cache de thumbnails en `app_cache_dir()`.                          |
| `src/`                    | Frontend React + TypeScript. Páginas, componentes, hooks, lib/tauri.ts (contrato). |
| `src/pages/`              | Una pantalla por archivo (Welcome, ScanProgress, Mode1/2/3Review, FinalReview, Done). |
| `src/lib/tauri.ts`        | Wrappers tipados de `invoke()` y `listen()`. **No invocar `invoke` directamente** fuera de aquí. |
| `src/i18n/es.json`        | Todas las cadenas de la UI. Sin strings hardcoded en componentes.               |
| `docs/`                   | Documentación del proyecto (este archivo, ARCHITECTURE, ROADMAP).               |
| `scripts/`                | Scripts auxiliares (smoke test, fixtures).                                      |
| `.github/workflows/`      | CI y release pipelines.                                                         |

---

## Convenciones de código

### Rust

- **Formato**: `cargo fmt` antes de commit (configuración por defecto).
- **Lints**: `cargo clippy -- -D warnings` debe pasar. CI lo exige.
- **Errores**:
  - Tipos de error de librería con `thiserror` (un enum por módulo si
    tiene sentido).
  - En el `main` y comandos Tauri, propagar con `anyhow::Result<…>` o
    convertir a `AppError`.
  - **Nunca `unwrap()` en producción**. Permitido en tests y fixtures.
- **Logging**: usar `tracing` (no `println!`). Niveles:
  - `error!` para fallos irrecuperables que el usuario verá.
  - `warn!` para casos recuperables (permiso denegado, ffmpeg falló).
  - `info!` para eventos de ciclo de vida (escaneo iniciado/completado).
  - `debug!` / `trace!` para info de diagnóstico, off por defecto.
- **Concurrencia**: preferir `rayon` para CPU-bound, `tokio` para I/O. No
  mezclar sin necesidad.
- **Cancelación**: si tu código participa en el pipeline, debe chequear
  `CancellationToken::is_cancelled()` periódicamente.
- **Documentación pública**: `///` doc comments en toda función pública
  expuesta como comando Tauri. La doc se publica en `cargo doc`.

### TypeScript

- **`tsc --strict` siempre verde**. CI lo exige.
- **Sin `any`** salvo en adaptadores de Tauri donde el tipo viene del
  backend (en esos casos, declara la interfaz en `lib/types.ts`).
- **Componentes**: funcionales con hooks. Sin clases.
- **Props**: siempre `interface`, no `type` (consistencia).
- **Estado global**: `zustand`. Estado local con `useState` /
  `useReducer`.
- **Estilos**: CSS vanilla con variables CSS. Sin frameworks CSS.
- **Estructura de carpetas por dominio** (`pages/`, `components/`,
  `hooks/`, `lib/`), no por tipo de archivo.
- **i18n**: todas las cadenas de la UI en `src/i18n/es.json`. Sin texto
  hardcoded en JSX (excepto nombres propios de marcas, etc.).

### Commits

- **Conventional Commits**:
  - `feat: …` — nueva funcionalidad visible al usuario.
  - `fix: …` — corrección de bug.
  - `docs: …` — sólo documentación.
  - `chore: …` — mantenimiento (deps, CI, build).
  - `refactor: …` — cambio interno sin cambio de comportamiento.
  - `test: …` — añadir o arreglar tests.
  - `perf: …` — mejora de rendimiento medible.
- **Idioma**: español o inglés, pero **consistente dentro del mismo
  PR**. Si tocas un módulo en inglés, mantén el inglés.
- **Scope** (opcional, recomendado): `feat(scanner): …`,
  `fix(mode2): …`, `docs(arch): …`.

### Mensajes de PR

- Título: resumen imperativo, ≤ 72 chars.
- Cuerpo: **qué** cambia, **por qué** (enlace a issue/ADR), **cómo
  probarlo** (pasos manuales + comandos), capturas si la UI cambia.
- Marcar como Draft si todavía no está listo para review.

---

## Cómo correr los tests

### Rust

```bash
cd src-tauri
cargo test                          # unit + integration
cargo test --release                 # más realista pero más lento
cargo clippy -- -D warnings          # lints
```

Hay un test de integración en `src-tauri/tests/pipeline.rs` que crea
ficheros sintéticos y comprueba los grupos resultantes. Antes de añadir
una dependencia pesada, escribe al menos un test que la use.

### TypeScript

```bash
pnpm test                            # vitest
pnpm tsc                             # type-check
pnpm lint                            # eslint
```

### Smoke test end-to-end

```bash
./scripts/smoke.sh
```

Crea una fixture de ~1000 archivos en `tmp/fotoverse-fixture/`, lanza la
app con esa ruta, corre el pipeline headless y aserta los grupos
esperados. Se ejecuta en CI en macOS y Windows.

---

## Añadir una dependencia Rust

1. **Comprueba la licencia**. Tiene que ser compatible con la del proyecto
   (**MIT + Commons Clause v1.0**, ver [`LICENSE`](../LICENSE)).
   Licencias copyleft (GPL, AGPL) **no** son aceptables. La Commons Clause
   **no** es un problema para dependencias: aplica sólo a este proyecto
   como producto; puedes usar crates/libs de cualquier licencia
   compatible con MIT.
2. **Comprueba la salud del crate**: último release reciente, issues
   abiertas razonables, mantenimiento activo.
3. **Añade con versión exacta o caret**:
   ```bash
   cargo add <crate>
   ```
4. **Justifica en el mensaje del commit** por qué este crate y no una
   alternativa (puede ser una frase).
5. **Si es una dependencia de seguridad (paperera, FS, etc.)**,
   actualiza el ADR correspondiente en `docs/ARCHITECTURE.md`.
6. **Si introduce una API pública nueva** (comando Tauri, struct de
   configuración), actualiza el contrato TS en el mismo PR.

---

## Añadir un comando Tauri

Los comandos Tauri son el **contrato** entre Rust y TypeScript. Cualquier
cambio debe tocar ambos lados en el mismo PR para que no diverja.

### Lado Rust

1. Declara el comando en `src-tauri/src/commands.rs`:

   ```rust
   #[tauri::command]
   pub async fn mi_comando(
       state: tauri::State<'_, AppState>,
       arg: MiArg,
   ) -> Result<MiRetorno, AppError> {
       // ...
   }
   ```

2. Regístralo en `src-tauri/src/lib.rs` dentro del
   `tauri::Builder::default().invoke_handler(…)`.

3. Si necesita un nuevo tipo de error, añádelo a `AppError` (este
   archivo) con `thiserror`.

### Lado TypeScript

1. Declara el tipo en `src/lib/types.ts` (espejo de Rust):

   ```ts
   export interface MiArg { /* ... */ }
   export interface MiRetorno { /* ... */ }
   ```

2. Añade el wrapper en `src/lib/tauri.ts`:

   ```ts
   import { invoke } from '@tauri-apps/api/core';
   export const miComando = (arg: MiArg): Promise<MiRetorno> =>
     invoke('mi_comando', { arg });
   ```

3. Usa el wrapper en la página, **nunca** llames a `invoke` directamente
   desde un componente.

### Si emite eventos

Define el payload en `lib/types.ts` y crea un wrapper en
`hooks/useScanProgress.ts` (o equivalente) que use `listen<T>(…)`.

---

## Flujo de Pull Request

1. **Crea una rama con prefijo**:
   - `feat/…` — nueva funcionalidad.
   - `fix/…` — corrección de bug.
   - `docs/…` — sólo docs.
   - `refactor/…` — refactor sin cambio de comportamiento.
   - `chore/…` — mantenimiento.

2. **Commits pequeños y enfocados**. Si puedes dividir tu cambio en 2–3
   commits lógicos, hazlo. Squash antes de merge si quedó alguno de
   "wip".

3. **Auto-revisión antes de pedir review**: lee tu propio diff como si
   fueras el revisor. ¿Falta un test? ¿Un doc comment? ¿Un `tracing`?

4. **Asegúrate de que CI pasa** localmente antes de pushear:
   ```bash
   cd src-tauri && cargo test && cargo clippy -- -D warnings
   pnpm tsc && pnpm lint && pnpm test
   ```

5. **Abre el PR como Draft** si todavía no está listo. Marca "Ready for
   review" cuando:
   - [ ] CI pasa.
   - [ ] Has añadido tests para el código nuevo.
   - [ ] Has actualizado la doc (JSDoc/rustdoc + `ARCHITECTURE.md` si
         aplica).
   - [ ] Has añadido entrada en `CHANGELOG.md` si es visible al usuario.

6. **Rebase antes de merge**. Sin commits de merge en la historia.
   Preferiblemente squash a un único commit con mensaje convencional.

7. **Al menos una revisión aprobatoria** de un mantenedor antes de
   mergear.

---

## Seguridad

Si descubres una vulnerabilidad, **no abras un issue público**. Escribe a
`ahuertam@gmail.com` (o el email de seguridad que figure en
`SECURITY.md` cuando exista) con:

- Descripción del problema.
- Pasos para reproducir.
- Impacto potencial.
- Sugerencia de fix (opcional).

Coordinaremos un parche y una divulgación responsable.

### Reglas de seguridad que aplican a todo PR

- **Nunca** llames a `std::fs::remove_file` ni a `std::fs::remove_dir` en
  código de producto. Sólo `trash::delete_all` (o variantes del crate
  `trash`). Verificable con:
  ```bash
  ! grep -rn 'remove_file\|remove_dir' src-tauri/src/ | grep -v test
  ```
  (Esto corre en CI; el PR no mergeará si falla.)
- **Nunca** desactives el scope FS de Tauri. La app sólo puede leer de
  las carpetas que el usuario haya elegido explícitamente.
- **Nunca** añadas un `tauri-plugin-http` o similar que habilite
  peticiones de red. La app es 100% offline por diseño.
- **Normaliza paths** con `normpath` antes de almacenarlos o usarlos
  como clave de cache. Evita Unicode confusables y ataques de path
  traversal.
- **Valida entradas del frontend** en el backend. El frontend es UI, no
  perímetro de seguridad.
- **No logees paths completos de ficheros del usuario** a `info!`. Usa
  `debug!` o muestra sólo el basename en logs visibles al usuario final.
