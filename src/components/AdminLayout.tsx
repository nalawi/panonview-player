import { NavLink, useNavigate } from "react-router-dom";
import { useState } from "react";
import { setAdminMode, isTauri } from "../services/api";
import { useDisplayStore } from "../stores/display";
import type { Device } from "../types";
import PanonViewLogo from "./PanonViewLogo";

interface Props {
  children: React.ReactNode;
  device: Device | null;
  online: boolean;
  /** True when rendered in a normal browser (no in-app player to return to). */
  webMode?: boolean;
  /** Clears the session and returns to the sign-in screen (web console). */
  onLogout?: () => void;
}

const NAV = [
  { to: "/admin", label: "Dashboard", end: true },
  { to: "/admin/pages", label: "Pages", end: false },
  { to: "/admin/schedules", label: "Schedules", end: false },
  { to: "/admin/settings", label: "Settings", end: false },
];

export function AdminLayout({
  children,
  device,
  online,
  webMode,
  onLogout,
}: Props) {
  const navigate = useNavigate();
  const setAdminModeLocal = useDisplayStore((s) => s.setAdminMode);
  const [busy, setBusy] = useState(false);
  const canReturnToPlayer = isTauri() && !webMode;

  const backToPlayer = async () => {
    setBusy(true);
    try {
      await setAdminMode(false);
      setAdminModeLocal(false);
      navigate("/");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full w-full flex-col bg-player-bg text-slate-100">
      <header className="flex items-center justify-between border-b border-slate-800 bg-panel px-5 py-3">
        <div className="flex items-center gap-3">
         
          {/* <h1 className="text-lg font-semibold">PanonView Player</h1> */}
          <PanonViewLogo className="flex h-14 w-60" />
          <span className="text-xs text-slate-500">
            {device?.name ?? "unconfigured"}
          </span>
          {webMode && (
            <span className="rounded bg-slate-700 px-2 py-0.5 text-[10px] uppercase tracking-wide text-slate-300">
              web console
            </span>
          )}
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 text-sm">
            <span
              className={`h-2.5 w-2.5 rounded-full ${
                online ? "bg-emerald-400" : "bg-red-500"
              }`}
            />
            <span className="text-slate-300">
              {device?.id ? device.id.slice(0, 8) : "—"}
            </span>
          </div>
          {canReturnToPlayer && (
            <button
              onClick={backToPlayer}
              disabled={busy}
              className="rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-white hover:brightness-110 disabled:opacity-50"
            >
              Back to Player
            </button>
          )}
          {onLogout && (
            <button
              onClick={onLogout}
              title="Sign out of the web console"
              className="rounded-md border border-slate-700 px-3 py-1.5 text-sm text-slate-300 hover:bg-slate-800"
            >
              Log out
            </button>
          )}
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <nav className="w-52 shrink-0 border-r border-slate-800 bg-panel p-3">
          <ul className="space-y-1">
            {NAV.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  end={item.end}
                  className={({ isActive }) =>
                    `block rounded-md px-3 py-2 text-sm transition-colors ${
                      isActive
                        ? "bg-accent/20 text-accent"
                        : "text-slate-300 hover:bg-slate-800"
                    }`
                  }
                >
                  {item.label}
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>

        <main className="flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  );
}