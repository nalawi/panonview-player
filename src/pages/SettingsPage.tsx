import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import {
  getApiKey,
  getDevice,
  getSettings,
  regenerateApiKey,
  restartHttpServer,
  setStoredKey,
  updateDevice,
  updateSettings,
} from "../services/api";
import type { Device, SettingsMap } from "../types";

function str(v: SettingsMap[string] | undefined): string {
  if (v == null) return "";
  if (typeof v === "string") return v;
  return v.value;
}

export function SettingsPage() {
  const qc = useQueryClient();
  const { data: settings } = useQuery<SettingsMap>({
    queryKey: ["settings"],
    queryFn: getSettings,
  });
  const { data: device } = useQuery<Device | null>({
    queryKey: ["device"],
    queryFn: getDevice,
  });
  const { data: apiKey } = useQuery<string>({
    queryKey: ["api-key"],
    queryFn: getApiKey,
  });

  const [form, setForm] = useState<Record<string, string>>({});
  const [dev, setDev] = useState({ name: "", location: "", group_name: "" });
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    if (settings) {
      setForm({
        http_enabled: str(settings.http_enabled) || "true",
        http_port: str(settings.http_port) || "8787",
        http_bind: str(settings.http_bind) || "0.0.0.0",
        auth_mode: str(settings.auth_mode) || "apikey",
        allow_remote: str(settings.allow_remote) || "true",
        allowed_ips: str(settings.allowed_ips) || "",
        startup_mode: str(settings.startup_mode) || "player",
      });
    }
  }, [settings]);

  useEffect(() => {
    if (device) {
      setDev({
        name: device.name ?? "",
        location: device.location ?? "",
        group_name: device.group_name ?? "",
      });
    }
  }, [device]);

  const notify = (m: string) => {
    setMsg(m);
    window.setTimeout(() => setMsg(null), 2500);
  };

  const saveSettings = useMutation({
    mutationFn: () => updateSettings(form),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["settings"] });
      notify("Settings saved");
    },
  });

  const saveDevice = useMutation({
    mutationFn: () => updateDevice(dev),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["device"] });
      notify("Device identity saved");
    },
  });

  const regen = useMutation({
    mutationFn: () => regenerateApiKey(),
    onSuccess: (newKey) => {
      // The API never hands the key back out (it is only readable via IPC on
      // desktop or from the local database), so this response is the sole
      // chance to capture it: persist it for future requests and seed the
      // query cache so the input shows the new key immediately. Re-fetching
      // ["api-key"] alone would just re-read the stale localStorage value.
      setStoredKey(newKey);
      qc.setQueryData(["api-key"], newKey);
      notify("API key regenerated");
    },
    onError: (e) => notify(`Regenerate failed: ${String(e)}`),
  });

  const restartServer = useMutation({
    mutationFn: () => restartHttpServer(),
    onSuccess: () => notify("HTTP server restarted"),
    onError: (e) => notify(`Restart failed: ${String(e)}`),
  });

  const copyKey = async () => {
    if (!apiKey) return;
    try {
      await navigator.clipboard.writeText(apiKey);
      notify("API key copied");
    } catch {
      notify("Copy failed");
    }
  };

  const authHint =
    "Send as X-API-Key or Authorization: Bearer KEY. Every HTTP client, including this machine's browser, must present the key.";

  return (
    <div className="space-y-6">
      {msg && (
        <div className="rounded-md bg-emerald-500/15 px-3 py-2 text-sm text-emerald-300">
          {msg}
        </div>
      )}

      <section className="rounded-lg border border-slate-800 bg-panel p-4">
        <h2 className="mb-3 text-sm font-semibold text-slate-200">
          Device Identity
        </h2>
        <div className="grid grid-cols-3 gap-3">
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Device ID
            <input
              readOnly
              value={device?.id ?? ""}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm text-slate-300"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Name
            <input
              value={dev.name}
              onChange={(e) => setDev({ ...dev, name: e.target.value })}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Hostname / Platform
            <input
              readOnly
              value={`${device?.hostname ?? ""} (${device?.platform ?? ""})`}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm text-slate-300"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Location
            <input
              value={dev.location}
              onChange={(e) => setDev({ ...dev, location: e.target.value })}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Group
            <input
              value={dev.group_name}
              onChange={(e) => setDev({ ...dev, group_name: e.target.value })}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
          </label>
          <div className="flex items-end">
            <button
              onClick={() => saveDevice.mutate()}
              className="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white hover:brightness-110"
            >
              Save Identity
            </button>
          </div>
        </div>
      </section>

      <section className="rounded-lg border border-slate-800 bg-panel p-4">
        <h2 className="mb-3 text-sm font-semibold text-slate-200">
          HTTP Server
        </h2>
        <div className="grid grid-cols-3 gap-3">
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Enabled
            <select
              value={form.http_enabled}
              onChange={(e) =>
                setForm({ ...form, http_enabled: e.target.value })
              }
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            >
              <option value="true">Enabled</option>
              <option value="false">Disabled</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Port
            <input
              value={form.http_port}
              onChange={(e) => setForm({ ...form, http_port: e.target.value })}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Bind Address
            <input
              value={form.http_bind}
              onChange={(e) => setForm({ ...form, http_bind: e.target.value })}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Authentication
            <select
              value={form.auth_mode}
              onChange={(e) => setForm({ ...form, auth_mode: e.target.value })}
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            >
              <option value="apikey">API Key</option>
              <option value="none">None (trusted LAN only)</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Allow Remote Control
            <select
              value={form.allow_remote}
              onChange={(e) =>
                setForm({ ...form, allow_remote: e.target.value })
              }
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            >
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs text-slate-400">
            Startup Mode
            <select
              value={form.startup_mode}
              onChange={(e) =>
                setForm({ ...form, startup_mode: e.target.value })
              }
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            >
              <option value="player">Player (fullscreen)</option>
              <option value="admin">Admin (windowed)</option>
            </select>
          </label>
          <label className="col-span-2 flex flex-col gap-1 text-xs text-slate-400">
            Allowed IPs (comma-separated CIDR, empty = allow all)
            <input
              value={form.allowed_ips}
              onChange={(e) => setForm({ ...form, allowed_ips: e.target.value })}
              placeholder="192.168.1.0/24, 10.10.0.0/16"
              className="rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            />
          </label>
          <div className="flex items-end gap-2">
            <button
              onClick={() => saveSettings.mutate()}
              className="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white hover:brightness-110"
            >
              Save
            </button>
            <button
              onClick={() => restartServer.mutate()}
              className="rounded-md border border-slate-700 px-4 py-2 text-sm hover:bg-slate-800"
            >
              Restart Server
            </button>
          </div>
        </div>
      </section>

      <section className="rounded-lg border border-slate-800 bg-panel p-4">
        <h2 className="mb-3 text-sm font-semibold text-slate-200">API Key</h2>
        <div className="flex items-center gap-3">
          <input
            readOnly
            value={apiKey ?? ""}
            className="flex-1 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 font-mono text-sm text-slate-200"
          />
          <button
            onClick={copyKey}
            className="rounded-md border border-slate-700 px-3 py-2 text-sm hover:bg-slate-800"
          >
            Copy
          </button>
          <button
            onClick={() => {
              if (confirm("Regenerate API key? Existing clients will break.")) {
                regen.mutate();
              }
            }}
            className="rounded-md bg-amber-500 px-3 py-2 text-sm font-medium text-black hover:brightness-110"
          >
            Regenerate
          </button>
        </div>
        <p className="mt-2 text-xs text-slate-500">{authHint}</p>
      </section>

      <section className="rounded-lg border border-slate-800 bg-panel p-4">
        <h2 className="mb-3 text-sm font-semibold text-slate-200">
          API Reference
        </h2>
        <div className="grid grid-cols-2 gap-x-8 gap-y-1 font-mono text-xs text-slate-400">
          <div>GET /api/v1/status</div>
          <div>GET /api/v1/display</div>
          <div>POST /api/v1/display</div>
          <div>POST /api/v1/display/override</div>
          <div>POST /api/v1/display/refresh</div>
          <div>POST /api/v1/display/back</div>
          <div>POST /api/v1/display/forward</div>
          <div>GET/POST /api/v1/pages</div>
          <div>GET/PUT/DELETE /api/v1/pages/:id</div>
          <div>GET/POST /api/v1/schedules</div>
          <div>GET/PUT/DELETE /api/v1/schedules/:id</div>
          <div>GET/PUT /api/v1/settings</div>
          <div>GET/PUT /api/v1/device</div>
          <div>POST /api/v1/restart</div>
          <div>POST /api/v1/shutdown</div>
          <div>POST /api/v1/reload</div>
        </div>
      </section>
    </div>
  );
}