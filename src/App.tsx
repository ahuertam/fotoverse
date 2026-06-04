import { Route, Routes } from "react-router-dom";
import Welcome from "./pages/Welcome";
import NotFound from "./pages/NotFound";

/**
 * Componente raíz. Sólo define el routing entre páginas.
 * El layout y estado global se gestiona en [lib/tauri.ts] y [hooks/].
 */
function App() {
  return (
    <Routes>
      <Route path="/" element={<Welcome />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}

export default App;
