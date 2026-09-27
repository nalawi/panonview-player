import { useState } from "react";
import { setStoredKey, httpVerifyKey } from "../services/http";

interface Props {
  onAuthed: () => void;
}

/**
 * API-key gate shown when the admin console runs in a normal browser.
 *
 * The embedded HTTP API is authenticated, so the browser UI must present a
 * key. The key is stored in localStorage and sent as `X-API-Key` on every
 * request. Inside the Tauri WebView this gate is bypassed entirely (IPC is
 * trusted), so this component only renders for browser users.
 */
export function AuthGate({ onAuthed }: Props) {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const ok = await httpVerifyKey(key.trim());
      if (!ok) {
        setError("Invalid API key. Find it in the app: Settings → API Key.");
        return;
      }
      setStoredKey(key.trim());
      onAuthed();
    } catch {
      setError("Could not reach the player API. Is the HTTP server running?");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full w-full items-center justify-center bg-player-bg p-6">
      <form
        onSubmit={submit}
        className="w-full max-w-md rounded-xl border border-slate-800 bg-panel p-6 shadow-2xl"
      >
        <div className="mb-1 flex items-center gap-2">
          <div className="h-2.5 w-2.5 rounded-full bg-accent" />
          <h1 className="text-lg font-semibold text-slate-100">
            PanonView Player Admin
          </h1>
        </div>
        <p className="mb-5 text-sm text-slate-400">
          Enter the device API key to manage this player.
        </p>

        <label className="mb-1 block text-xs uppercase tracking-wide text-slate-500">
          API Key
        </label>
        <input
          type="password"
          value={key}
          autoFocus
          onChange={(e) => setKey(e.target.value)}
          placeholder="Paste the API key"
          className="w-full rounded-md border border-slate-700 bg-panel-2 px-3 py-2 font-mono text-sm text-slate-100 outline-none focus:border-accent"
        />

        {error && <div className="mt-3 text-sm text-red-400">{error}</div>}

        <button
          type="submit"
          disabled={busy || !key.trim()}
          className="mt-5 w-full rounded-md bg-accent px-4 py-2 text-sm font-medium text-white hover:brightness-110 disabled:opacity-50"
        >
          {busy ? "Checking…" : "Sign in"}
        </button>

        <p className="mt-4 text-xs leading-relaxed text-slate-500">
          The key is generated on first boot and shown in the desktop app under
          <span className="text-slate-400"> Settings → API Key</span>. On this
          machine you can also read it from the player's local database.
        </p>
      </form>
    </div>
  );
}