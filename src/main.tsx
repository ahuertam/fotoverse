import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./styles/global.css";

// TEMP: StrictMode desactivado para diagnosticar el race del scan.
//      Lo reactivaremos en Fase 9 (polish).
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <BrowserRouter>
    <App />
  </BrowserRouter>,
);
