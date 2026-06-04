import { Route, Routes } from "react-router-dom";
import NotFound from "./pages/NotFound";
import ScanProgress from "./pages/ScanProgress";
import Welcome from "./pages/Welcome";

/**
 * Componente raíz. Sólo define el routing entre páginas.
 * El layout y estado global se gestiona en [lib/tauri.ts] y [hooks/].
 */
function App() {
  return (
    <Routes>
      <Route path="/" element={<Welcome />} />
      <Route path="/scan/:id" element={<ScanProgress />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}

export default App;
