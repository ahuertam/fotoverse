import { Route, Routes } from "react-router-dom";
import Mode1Review from "./pages/Mode1Review";
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
      <Route path="/review/:id/mode1" element={<Mode1Review />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}

export default App;
