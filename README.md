# fotoverse

> Encuentra duplicados en tu biblioteca de fotos y vídeos.

[![Estado](https://img.shields.io/badge/estado-en%20desarrollo-yellow)](docs/ROADMAP.md)
[![Plataformas](https://img.shields.io/badge/plataformas-macOS%20%7C%20Windows-blue)](#)

## ¿Qué es?

**fotoverse** es una aplicación de escritorio que recorre una carpeta o un disco
entero en busca de **fotos y vídeos duplicados**, y te ofrece una interfaz
visual para decidir qué copias borrar y desde qué ruta — siempre enviando los
ficheros a la papelera, nunca con borrado permanente.

Está pensada para bibliotecas grandes (decenas de miles de archivos, varios TB)
acumuladas a lo largo de años, con muchos duplicados "camuflados": el mismo
`IMG_1234.jpg` en tres carpetas, fotos recomprimidas con tamaños distintos,
vídeos en diferentes calidades pero con el mismo contenido, etc.

## Características

- 🔍 **Tres modos de búsqueda complementarios**:
  1. **Exacto** — mismo nombre de archivo, extensión y fecha de creación.
  2. **Por propiedades** — similitud perceptual de imágenes (pHash) y
     comparación de keyframes en vídeos.
  3. **Visual** — comparativa lado a lado (izquierda vs derecha) para que tú
     decidas caso por caso.
- 🖼️ **Revisión visual** con thumbnails y metadatos (ruta, tamaño, fecha,
  dimensiones).
- ✅ **Lista de confirmación final** con checks por archivo para que puedas
  desmarcar cualquier fichero en el último momento.
- 🗑️ **Borrado seguro**: los archivos se mueven a la Papelera de macOS o a la
  Papelera de reciclaje de Windows. Nunca se borran permanentemente.
- ⚡ **Backend en Rust**: aprovecha todos los núcleos de la CPU para procesar
  bibliotecas grandes en minutos, no en horas.
- 💾 **Persistencia de sesión**: el escaneo y tus decisiones se guardan en una
  base de datos local; puedes cerrar la app y continuar después.

## Formatos soportados

| Tipo    | Formatos                                       |
|---------|------------------------------------------------|
| Imágenes | JPG, PNG, WebP, GIF, BMP, TIFF                |
| Vídeos   | MP4, MOV, MKV, AVI, WebM                     |

> **Nota**: HEIC/HEIF (formato nativo de iPhone) y RAW de cámara (CR2, NEF,
> ARW, DNG, RAF) **no están soportados** en esta versión. Si los necesitas,
> consulta el roadmap o ábrelo como propuesta.

## Estado del proyecto

**En desarrollo activo.** El plan completo por fases y los criterios de
"hecho" están en [`docs/ROADMAP.md`](docs/ROADMAP.md). El diseño técnico
detallado está en [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Capturas / GIFs

> _Pendiente: cuando la UI esté implementada se insertarán capturas en esta
> sección._

## Requisitos para desarrollo

- **Rust** (toolchain estable, ≥ 1.75). Instálalo con [rustup](https://rustup.rs/).
- **Node.js** ≥ 20 y **pnpm** ≥ 9.
- **Tauri CLI**: `cargo install tauri-cli --version "^2.0"` (o usar el
  prefijo `pnpm tauri`, ya integrado).
- **macOS**: Xcode Command Line Tools (`xcode-select --install`).
- **Windows**: Microsoft Visual C++ Build Tools y WebView2 (preinstalado en
  Windows 10/11 modernos). Consulta [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)
  para troubleshooting.

## Desarrollo local

```bash
git clone https://github.com/ahuertam/fotoverse.git
cd fotoverse
pnpm install
pnpm tauri dev
```

La primera compilación puede tardar varios minutos (descarga y compila
dependencias Rust). Las siguientes son incrementales y mucho más rápidas.

## Build de producción

```bash
pnpm tauri build
```

Esto produce:

- **macOS**: `.dmg` y `.app` universales (Apple Silicon + Intel), firmados y
  notarizados si los secrets están configurados.
- **Windows**: instalador NSIS (`.exe`) y/o MSI (`.msi`), firmados con Azure
  Trusted Signing si los secrets están configurados.

Los binarios se generan en `src-tauri/target/release/bundle/`.

## Licencia

**MIT + Commons Clause v1.0** — ver [`LICENSE`](LICENSE) para el texto
completo. En resumen: puedes usar, copiar, modificar, fusionar, publicar,
distribuir y sublicenciar el código libremente; la única restricción es que
**no puedes vender el software o trabajos derivados** sin un acuerdo
comercial separado con el titular del copyright. Para uso comercial,
contacta con el titular.

## Documentación

- 📐 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — diseño técnico, esquema
  de la base de datos, decisiones arquitectónicas (ADRs).
- 🛠️ [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) — cómo contribuir:
  setup, convenciones de código, flujo de PR.
- 🗺️ [`docs/ROADMAP.md`](docs/ROADMAP.md) — fases del proyecto y criterios
  de "hecho".

## Estado v0.1.0

Esta es la **primera release jugable end-to-end** (Fases 0–9 completas).
Funcionalidades incluidas:

| # | Modo / Función | Estado |
|---|---|---|
| 1 | Escaneo recursivo de imágenes (JPG, PNG, WebP, GIF, BMP, TIFF) y vídeos (MP4, MOV, MKV, AVI, WebM) | ✅ |
| 2 | Thumbnails cacheados (SHA-256 del path + mtime) | ✅ |
| 3 | **Modo 1** — Duplicados exactos (filename + ext + ctime + size) | ✅ |
| 4 | **Modo 2** — pHash perceptual (DCT 32×32, Hamming ≤ 10) | ✅ |
| 5 | **Modo 3** — Side-by-side visual con atajos de teclado | ✅ |
| 6 | **Modo 4** (bonus v0.2) — Vídeos duplicados + top-N más pesados con ahorro estimado al 70% | ✅ |
| 7 | **Final Review** — checks por archivo, confirmación tipada > 1000 files / 5 GB | ✅ |
| 8 | **Papelera** — `trash::delete_all` (Finder / Recycle Bin, nunca borrado permanente) | ✅ |
| 9 | **Settings** — `tauri-plugin-store` con `lastFolder` y `theme` | ✅ |
| 10 | **integrity_check** al arranque (recovery con reset) | ✅ |
| 11 | **VACUUM** al borrar sesión | ✅ |
| 12 | **Splash** con imagen placeholder | ✅ |

**Diferido para v0.2** (no incluido en esta release):
- Modo vídeos perceptual (Fase 6: `ffmpeg-sidecar`)
- Toggle "incluir archivos ocultos" en walk (backend preparado, falta cablear)
- Tema claro (sólo dark por ahora)
- Code signing + auto-update (requiere Apple Developer ID y/o Azure account)
- Capturas de pantalla reales (esta release sólo tiene la splash placeholder)

## Cómo probar

```bash
git clone https://github.com/ahuertam/fotoverse
cd fotoverse
pnpm install
pnpm tauri dev
```

Para una sesión de prueba rápida:

```bash
# Generar fixture (27 imágenes: 20 únicas + hard links para Modo 1)
cargo test --test fixture -- --nocapture

# Lanzar la app y elegir ~/Desktop/proyects/trial/
# → Deberías ver 3 grupos en Modo 1 (foto_1 ×3, foto_2 ×2, foto_3 ×2)
# → 4 grupos en Modo 2 (4 imágenes con gradientes similares)
# → 346 pares en Modo 3
# → Final Review con 4 ficheros marcados como delete (si decides)
# → Papelera real en Finder
```

## Troubleshooting

**La ventana Tauri no abre / "Port 1420 already in use"**: mata el proceso
anterior y rearranca:
```bash
pkill -f "tauri dev\|target/debug/fotoverse"
pnpm tauri dev
```

**Error "no such column member_count"**: la DB es de una versión anterior.
Bórrala y rearranca:
```bash
rm "$HOME/Library/Application Support/com.ahuertam.fotoverse/fotoverse.db"*  # macOS
# O en Windows: %APPDATA%\com.ahuertam.fotoverse\fotoverse.db*
```

**Pantalla en negro tras elegir carpeta**: refresca con Cmd+R dentro de la
ventana Tauri. Si persiste, abre devtools (Cmd+Option+I) y revisa la
pestaña Console.
