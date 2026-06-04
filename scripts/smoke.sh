#!/usr/bin/env bash
# ==============================================================================
# fotoverse — smoke test del pipeline de escaneo
# ==============================================================================
# Crea una fixture temporal con archivos sintéticos y ejecuta el test de
# integración que verifica el walk sobre ella.
#
# Uso:    ./scripts/smoke.sh
# Salida: exit 0 si todo pasa, exit != 0 si falla.
# ==============================================================================

set -euo pipefail

# Resolver el directorio del repo a partir de este script.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_DIR"

echo "▶ Smoke test fotoverse"
echo "  Repo: $REPO_DIR"

# Asegurar que cargo está en PATH.
if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  else
    echo "✗ cargo no encontrado. Instala rustup: https://rustup.rs/" >&2
    exit 1
  fi
fi

echo "▶ Compilando tests de integración (puede tardar la primera vez)…"
. "$HOME/.cargo/env"
(cd src-tauri && cargo test --test walk_test --release -- --nocapture)

echo "✓ Smoke test pasado."
