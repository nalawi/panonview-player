import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
  displayBack,
  displayForward,
  getDisplayStatus,
  overrideDisplay,
  refreshDisplay,
  resetDisplay,
  setDisplayUrl,
} from "../services/api";
import type { DisplayStatus } from "../types";

function StatCard({
  label,
  value,
  accent,
}: {
  label: string;
  value: string;
  accent?: string;
}) {
  return (
    <div className="rounded-lg border border-slate-800 bg-panel p-4">
      <div className="text-xs uppercase tracking-wide text-slate-500">
        {label}
      </div>
      <div className={`mt-1 truncate text-lg font-semibold ${accent ?? ""}`}>
        {value}
      </div>
    </div>
  );
}

export function DashboardPage() {
  const { data: status } = useQuery<DisplayStatus>({
    queryKey: ["display-status"],
    queryFn: getDisplayStatus,
    refetchInterval: 3000,
  });

  const [url, setUrl] = useState("");
  const [overrideUrl, setOverrideUrl] = useState("");
  const [overrideSecs, setOverrideSecs] = useState(300);
  const [msg, setMsg] = useState<string | null>(null);

  const notify = (m: string) => {
    setMsg(m);
    window.setTimeout(() => setMsg(null), 2500);
  };

  const doSetUrl = async () => {
    if (!url.trim()) return;
    await setDisplayUrl(url.trim());
    setUrl("");
    notify("Display updated");
  };

  const doOverride = async () => {
    if (!overrideUrl.trim()) return;
    await overrideDisplay(overrideUrl.trim(), overrideSecs, 100);
    setOverrideUrl("");
    notify(`Override active for ${overrideSecs}s`);
  };

  const doReset = async () => {
    await resetDisplay();
    notify("Display reset to default page");
  };

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-4 gap-4">
        <StatCard label="Mode" value={status?.mode ?? "—"} accent="text-accent" />
        <StatCard
          label="Scheduler"
          value={status?.scheduler_running ? "running" : "stopped"}
          accent={status?.scheduler_running ? "text-emerald-400" : "text-red-400"}
        />
        <StatCard
          label="Page ID"
          value={
            status?.current_page_id != null ? String(status.current_page_id) : "—"
          }
        />
        <StatCard
          label="Override Expires"
          value={
            status?.expires_at
              ? new Date(status.expires_at).toLocaleTimeString()
              : "—"
          }
        />
      </div>

      <div className="rounded-lg border border-slate-800 bg-panel p-4">
        <div className="mb-2 text-sm font-medium text-slate-300">
          Current Display
        </div>
        <div className="flex items-center gap-3">
          <input
            readOnly
            value={status?.current_url ?? ""}
            className="flex-1 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm text-slate-200"
          />
          <button
            onClick={() => refreshDisplay()}
            className="rounded-md border border-slate-700 px-3 py-2 text-sm hover:bg-slate-800"
          >
            Refresh
          </button>
          <button
            onClick={doReset}
            title="Clear the display and show the default standby page"
            className="rounded-md border border-slate-700 px-3 py-2 text-sm text-amber-400 hover:bg-slate-800"
          >
            Reset
          </button>
          <button
            onClick={() => displayBack()}
            disabled={!status?.can_back}
            className="rounded-md border border-slate-700 px-3 py-2 text-sm hover:bg-slate-800 disabled:opacity-40"
          >
            Back
          </button>
          <button
            onClick={() => displayForward()}
            disabled={!status?.can_forward}
            className="rounded-md border border-slate-700 px-3 py-2 text-sm hover:bg-slate-800 disabled:opacity-40"
          >
            Forward
          </button>
        </div>
        {status?.last_successful_url && (
          <div className="mt-2 text-xs text-slate-500">
            Last successful: {status.last_successful_url}{" "}
            {status.last_successful_at
              ? `at ${new Date(status.last_successful_at).toLocaleString()}`
              : ""}
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg border border-slate-800 bg-panel p-4">
          <div className="mb-2 text-sm font-medium text-slate-300">
            Set URL (manual mode)
          </div>
          <div className="flex gap-2">
            <input
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://example.com/dashboard"
              className="flex-1 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
            <button
              onClick={doSetUrl}
              className="rounded-md bg-accent px-3 py-2 text-sm font-medium text-white hover:brightness-110"
            >
              Go
            </button>
          </div>
        </div>

        <div className="rounded-lg border border-slate-800 bg-panel p-4">
          <div className="mb-2 text-sm font-medium text-slate-300">
            Emergency Override
          </div>
          <div className="flex gap-2">
            <input
              value={overrideUrl}
              onChange={(e) => setOverrideUrl(e.target.value)}
              placeholder="https://company.com/emergency"
              className="flex-1 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
            <input
              type="number"
              value={overrideSecs}
              min={1}
              onChange={(e) => setOverrideSecs(Number(e.target.value))}
              className="w-24 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
              title="Duration (seconds)"
            />
            <button
              onClick={doOverride}
              className="rounded-md bg-amber-500 px-3 py-2 text-sm font-medium text-black hover:brightness-110"
            >
              Override
            </button>
          </div>
        </div>
      </div>

      {msg && (
        <div className="rounded-md bg-emerald-500/15 px-3 py-2 text-sm text-emerald-300">
          {msg}
        </div>
      )}
    </div>
  );
}