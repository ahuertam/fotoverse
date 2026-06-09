import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./styles/global.css";

// Nota: StrictMode está desactivado. Si lo activamos, hay que
// revisar los useEffect que usan cleanup con cancellable
// closures (Modo 3, useScanProgress) para evitar dobles
// suscripciones durante el double-mount de dev.
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <BrowserRouter>
    <App />
  </BrowserRouter>,
);
