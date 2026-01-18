import { Routes, Route } from "react-router-dom";
import { AppShell } from "@/app/layout/AppShell";
import Dashboard from "@/pages/Dashboard";
import Listeners from "@/pages/Listeners";
import RoutesPage from "@/pages/Routes";
import Upstreams from "@/pages/Upstreams";
import Tls from "@/pages/Tls";
import Versions from "@/pages/Versions";
import Nodes from "@/pages/Nodes";
import Audit from "@/pages/Audit";

export default function App() {
  return (
    <AppShell>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/listeners" element={<Listeners />} />
        <Route path="/routes" element={<RoutesPage />} />
        <Route path="/upstreams" element={<Upstreams />} />
        <Route path="/tls" element={<Tls />} />
        <Route path="/versions" element={<Versions />} />
        <Route path="/nodes" element={<Nodes />} />
        <Route path="/audit" element={<Audit />} />
      </Routes>
    </AppShell>
  );
}
