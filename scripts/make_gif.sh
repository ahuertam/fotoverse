#!/usr/bin/env bash
# Genera un GIF demo de fotoverse a partir de la app corriendo.
#
# Requisitos:
#   - macOS (screencapture nativo)
#   - ffmpeg (`brew install ffmpeg`)
#   - pnpm (para arrancar vite con mocks)
#   - playwright (npm) o similar para automatizar clicks
#
# Uso:
#   bash scripts/make_gif.sh
#
# Output:
#   docs/assets/demo.gif (~1-2 MB, 5-10 segundos, ~600x400)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/docs/assets"
OUT_FILE="$OUT_DIR/demo.gif"
TMP_DIR="$(mktemp -d)"
trap "rm -rf $TMP_DIR" EXIT

mkdir -p "$OUT_DIR"

# 1. Servir dist/ + mock-tauri.html en localhost:5173.
echo "▶ Sirviendo mock en :5173…"
cd "$ROOT"

# Copiamos el mock a dist/ como index.html (para que sea
# servible por vite).
cp scripts/demo/mock-tauri.html dist/index.html

# Servimos dist/ en :5173 con vite preview.
( pnpm exec vite preview --port 5173 --strictPort ) &
SERVER_PID=$!
sleep 3

# 2. Capturar screenshots con Playwright headless.
echo "▶ Capturando screenshots…"
PNAME="$(mktemp -d)"
cd "$PNAME"
pnpm init >/dev/null 2>&1 || true
pnpm add -D playwright >/dev/null 2>&1
npx playwright install chromium >/dev/null 2>&1

cat > capture.mjs <<'EOF'
import { chromium } from "playwright";
const FRAMES = [
  { url: "http://localhost:5173/", out: "01-welcome.png", wait: 1500, actions: [] },
  {
    url: "http://localhost:5173/scan/42",
    out: "02-scan-done.png",
    wait: 800,
    actions: [],
  },
  {
    url: "http://localhost:5173/review/42/mode1",
    out: "03-mode1-list.png",
    wait: 800,
    actions: [],
  },
  {
    url: "http://localhost:5173/review/42/mode1",
    out: "04-mode1-expanded.png",
    wait: 600,
    actions: [
      async (page) => {
        await page.locator(".group-summary").first().click();
      },
    ],
  },
  {
    url: "http://localhost:5173/review/42/mode2",
    out: "05-mode2.png",
    wait: 800,
    actions: [],
  },
  {
    url: "http://localhost:5173/review/42/mode4",
    out: "06-mode4.png",
    wait: 800,
    actions: [],
  },
  {
    url: "http://localhost:5173/review/42/final",
    out: "07-final.png",
    wait: 800,
    actions: [],
  },
  {
    url: "http://localhost:5173/settings",
    out: "08-settings.png",
    wait: 800,
    actions: [],
  },
];

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 960, height: 640 },
  deviceScaleFactor: 1,
});
const page = await ctx.newPage();

for (const f of FRAMES) {
  await page.goto(f.url);
  await page.waitForTimeout(f.wait);
  for (const action of f.actions) await action(page);
  await page.screenshot({ path: `${process.env.TMP}/${f.out}`, fullPage: false });
  console.log("  ✓", f.out);
}
await browser.close();
EOF

# Playwright no necesita el server si capturamos goto()
# contra :5173; ya está corriendo.
TMP="$TMP_DIR" node capture.mjs

cd "$ROOT"

# 3. Unir PNGs en GIF con ffmpeg.
echo "▶ Uniendo PNGs en $OUT_FILE…"
if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ERROR: ffmpeg no está instalado. brew install ffmpeg" >&2
  exit 1
fi

ffmpeg -y \
  -framerate 1 -pattern_type glob -i "$TMP_DIR/*.png" \
  -vf "fps=2,scale=960:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5" \
  -loop 0 \
  "$OUT_FILE"

# Cleanup
kill $SERVER_PID 2>/dev/null || true
rm -f dist/index.html

echo ""
echo "✓ GIF demo: $OUT_FILE"
ls -lh "$OUT_FILE"
