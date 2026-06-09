import { Route, Routes } from "react-router-dom";
import Done from "./pages/Done";
import FinalReview from "./pages/FinalReview";
import Mode1Review from "./pages/Mode1Review";
import Mode2Review from "./pages/Mode2Review";
import Mode3Review from "./pages/Mode3Review";
import ModeVideoAndHeavy from "./pages/ModeVideoAndHeavy";
import NotFound from "./pages/NotFound";
import ScanProgress from "./pages/ScanProgress";
import Settings from "./pages/Settings";
import Welcome from "./pages/Welcome";

/**
 * Componente raíz. Sólo define el routing entre páginas.
 * El layout y estado global se gestiona en [lib/tauri.ts] y [hooks/].
 */
function App() {
  return (
    <Routes>
      <Route path="/" element={<Welcome />} />
      <Route path="/settings" element={<Settings />} />
      <Route path="/scan/:id" element={<ScanProgress />} />
      <Route path="/review/:id/mode1" element={<Mode1Review />} />
      <Route path="/review/:id/mode2" element={<Mode2Review />} />
      <Route path="/review/:id/mode3" element={<Mode3Review />} />
      <Route path="/review/:id/mode4" element={<ModeVideoAndHeavy />} />
      <Route path="/review/:id/final" element={<FinalReview />} />
      <Route path="/done/:id" element={<Done />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}

export default App;
