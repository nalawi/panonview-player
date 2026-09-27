import { useEffect, useState } from "react";
import { Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { PlayerView } from "./components/PlayerView";
import { AdminLayout } from "./components/AdminLayout";
import { AuthGate } from "./components/AuthGate";
import { DashboardPage } from "./pages/DashboardPage";
import { PagesPage } from "./pages/PagesPage";
import { SchedulesPage } from "./pages/SchedulesPage";
import { SettingsPage } from "./pages/SettingsPage";
import { useDisplayStore } from "./stores/display";
import { useDisplayEvents } from "./hooks/useDisplayEvents";
import { getDevice } from "./services/api";
import { isTauri, getStoredKey, clearStoredKey, httpVerifyKey } from "./services/api";
import { httpBootstrapAuth } from "./services/http";
import type { Device } from "./types";

/**
 * Root application.
 *
 * Two deployment shapes share this code:
 *
 *  - Tauri desktop: boots into the fullscreen Player at `/`; the admin console
 *    is reachable at `/admin` (hidden until opened). Uses IPC.
 *
 *  - Web browser (served at `/ui`): the admin console is the whole app. The
 *    user authenticates with the device API key, then all pages talk to the
 *    embedded REST API. There is no player frame in the browser.
 */
function App() {
  const web = !isTauri();

  if (web) {
    return <WebAdminApp />;
  }
  return <DesktopApp />;
}

/** Desktop (Tauri) routing: player by default, admin at /admin. */
function DesktopApp() {
  useDisplayEvents();
  const location = useLocation();
  const adminMode = useDisplayStore((s) => s.adminMode);
  const setAdminMode = useDisplayStore((s) => s.setAdminMode);

  const isAdminRoute = location.pathname.startsWith("/admin");
  if (isAdminRoute && !adminMode) {
    setAdminMode(true);
  }

  const { data: device } = useQuery<Device | null>({
    queryKey: ["device"],
    queryFn: getDevice,
    enabled: isAdminRoute || adminMode,
  });

  const wrap = (node: React.ReactNode) => (
    <AdminLayout device={device ?? null} online={true}>
      {node}
    </AdminLayout>
  );

  return (
    <Routes>
      <Route path="/" element={<PlayerView />} />
      <Route path="/admin" element={wrap(<DashboardPage />)} />
      <Route path="/admin/pages" element={wrap(<PagesPage />)} />
      <Route path="/admin/schedules" element={wrap(<SchedulesPage />)} />
      <Route path="/admin/settings" element={wrap(<SettingsPage />)} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

/** Browser routing: authenticate, then show the admin console. */
function WebAdminApp() {
  const [authed, setAuthed] = useState(false);
  const [checking, setChecking] = useState(true);
  const navigate = useNavigate();
  const location = useLocation();

  // Sign out of the web console: drop the stored API key and show the
  // sign-in screen again.
  const logout = () => {
    clearStoredKey();
    setAuthed(false);
  };

  // Auto-authenticate: first try a stored key, otherwise ask the player
  // whether this (loopback) client may be trusted with the key automatically,
  // so the local machine does not have to type it in.
  useEffect(() => {
    (async () => {
      const stored = getStoredKey();
      if (stored && (await httpVerifyKey(stored))) {
        setAuthed(true);
        setChecking(false);
        return;
      }
      const bootstrapped = await httpBootstrapAuth();
      if (bootstrapped && (await httpVerifyKey(bootstrapped))) {
        setAuthed(true);
      }
      setChecking(false);
    })();
  }, []);

  // In the browser, the console lives at the root.
  useEffect(() => {
    if (authed && location.pathname === "/") {
      navigate("/admin", { replace: true });
    }
  }, [authed, location.pathname, navigate]);

  if (checking) {
    return (
      <div className="flex h-full w-full items-center justify-center bg-player-bg text-slate-400">
        Connecting…
      </div>
    );
  }

  if (!authed) {
    return <AuthGate onAuthed={() => setAuthed(true)} />;
  }

  return (
    <Routes>
      <Route
        path="/admin"
        element={
          <WebAdminShell onLogout={logout}>
            <DashboardPage />
          </WebAdminShell>
        }
      />
      <Route
        path="/admin/pages"
        element={
          <WebAdminShell onLogout={logout}>
            <PagesPage />
          </WebAdminShell>
        }
      />
      <Route
        path="/admin/schedules"
        element={
          <WebAdminShell onLogout={logout}>
            <SchedulesPage />
          </WebAdminShell>
        }
      />
      <Route
        path="/admin/settings"
        element={
          <WebAdminShell onLogout={logout}>
            <SettingsPage />
          </WebAdminShell>
        }
      />
      <Route path="*" element={<Navigate to="/admin" replace />} />
    </Routes>
  );
}

/** Shared shell that fetches device info for the browser console. */
function WebAdminShell({
  children,
  onLogout,
}: {
  children: React.ReactNode;
  onLogout?: () => void;
}) {
  const { data: device } = useQuery<Device | null>({
    queryKey: ["device"],
    queryFn: getDevice,
    refetchInterval: 15000,
  });
  return (
    <AdminLayout device={device ?? null} online={true} webMode onLogout={onLogout}>
      {children}
    </AdminLayout>
  );
}

export default App;