import { Component, type ReactNode } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./styles/global.css";

// TEMP: StrictMode desactivado para diagnosticar el race del scan.
//      Lo reactivaremos en Fase 9 (polish).

// TEMP: espejo de console en pantalla — colapsable, no tapa la UI.
const consoleHeader = document.createElement("div");
consoleHeader.id = "console-mirror-header";
consoleHeader.style.cssText =
  "position:fixed;bottom:0;left:0;right:0;height:22px;line-height:22px;" +
  "background:#000;color:#0f0;font:11px monospace;padding:0 8px;z-index:99999;" +
  "border-top:1px solid #333;cursor:pointer;user-select:none;";
consoleHeader.textContent = "▶ consola (click para abrir)";
const consoleBox = document.createElement("div");
consoleBox.id = "console-mirror";
consoleBox.style.cssText =
  "position:fixed;bottom:22px;left:0;right:0;max-height:33vh;overflow:auto;" +
  "display:none;background:#000;color:#0f0;font:11px monospace;padding:4px 8px;" +
  "z-index:99999;border-top:1px solid #333;white-space:pre-wrap;";
let consoleOpen = false;
consoleHeader.addEventListener("click", () => {
  consoleOpen = !consoleOpen;
  consoleBox.style.display = consoleOpen ? "block" : "none";
  consoleHeader.textContent = consoleOpen
    ? "▼ consola (click para cerrar)"
    : "▶ consola (click para abrir)";
});
document.body.appendChild(consoleHeader);
document.body.appendChild(consoleBox);
function mirror(level: string, args: unknown[]) {
  try {
    const line = document.createElement("div");
    line.textContent =
      `[${level}] ` +
      args.map((a) => (typeof a === "string" ? a : JSON.stringify(a))).join(" ");
    consoleBox.appendChild(line);
    while (consoleBox.children.length > 80) consoleBox.removeChild(consoleBox.firstChild!);
    consoleBox.scrollTop = consoleBox.scrollHeight;
  } catch {
    // ignore
  }
}
const origLog = console.log.bind(console);
const origErr = console.error.bind(console);
const origWarn = console.warn.bind(console);
console.log = (...a) => { origLog(...a); mirror("log", a); };
console.error = (...a) => { origErr(...a); mirror("err", a); };
console.warn = (...a) => { origWarn(...a); mirror("warn", a); };
origLog("[boot] console mirror activo (click en la barra negra para abrir)");

// TEMP: error boundary global para mostrar el error en pantalla en
//       lugar de sólo en consola (que el usuario no ve).
class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  componentDidCatch(error: Error, info: unknown) {
    origErr("[ErrorBoundary] error:", error, info);
  }
  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 24, fontFamily: "monospace" }}>
          <h1>⚠️ Error de la UI</h1>
          <h2>Mensaje:</h2>
          <pre
            style={{
              background: "#1a1c22",
              color: "#ea4335",
              padding: 12,
              borderRadius: 8,
              overflow: "auto",
              whiteSpace: "pre-wrap",
            }}
          >
            {this.state.error.message}
          </pre>
          <h2>Stack:</h2>
          <pre
            style={{
              background: "#1a1c22",
              color: "#9aa0a6",
              padding: 12,
              borderRadius: 8,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              fontSize: 11,
            }}
          >
            {String(this.state.error.stack || "(no stack)")}
          </pre>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <BrowserRouter>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </BrowserRouter>,
);
