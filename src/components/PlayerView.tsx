import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useDisplayStore } from "../stores/display";
import { getInitialUrl, reportPageLoaded } from "../services/tauri";
import type { DisplayStatus } from "../types";

/**
 * The fullscreen player. Renders the current URL inside a borderless iframe
 * that fills the window. Navigation is driven entirely by the Rust
 * DisplayController via the `display://navigate` event — this component never
 * decides what to show on its own, it only renders and reports load results.
 */
export function PlayerView() {
  const url = useDisplayStore((s) => s.url);
  const reloadToken = useDisplayStore((s) => s.reloadToken);
  const mode = useDisplayStore((s) => s.mode);
  const applyNavigate = useDisplayStore((s) => s.applyNavigate);
  const setOffline = useDisplayStore((s) => s.setOffline);
  const setAdminMode = useDisplayStore((s) => s.setAdminMode);

  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);
  const [showHint, setShowHint] = useState(true);
  const timeoutRef = useRef<number | null>(null);

  // Live clock for the "waiting for content" standby screen.
  const [clock, setClock] = useState(() => new Date());
  useEffect(() => {
    const t = window.setInterval(() => setClock(new Date()), 1000);
    return () => window.clearInterval(t);
  }, []);

  // Resolve the initial URL on first mount.
  useEffect(() => {
    getInitialUrl().then((u) => {
      if (u) {
        useDisplayStore.getState().setUrl(u);
      }
    });
  }, []);

  // Navigate events from Rust.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ url: string; mode: string; page_id: number | null; reload_token: number }>(
      "display://navigate",
      (e) => {
        applyNavigate(e.payload as never);
        setFailed(false);
      },
    ).then((fn) => {
      unlisten = fn;
    });

    // Status events can flip admin mode awareness.
    let unlistenStatus: (() => void) | undefined;
    listen<DisplayStatus>("display://state", () => {
      /* reserved for future use */
    }).then((fn) => {
      unlistenStatus = fn;
    });

    return () => {
      unlisten?.();
      unlistenStatus?.();
    };
  }, [applyNavigate]);

  // When the URL changes, start a load timeout. If nothing loads in time we
  // treat it as a failure and keep showing the last page (offline-first).
  useEffect(() => {
    if (!url) return;
    setLoading(true);
    setFailed(false);
    if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    timeoutRef.current = window.setTimeout(() => {
      setLoading(false);
      setFailed(true);
      setOffline(true);
    }, 20000);
    return () => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, [url, reloadToken, setOffline]);

  // Hide the small loading hint after a few seconds.
  useEffect(() => {
    const t = window.setTimeout(() => setShowHint(false), 4000);
    return () => window.clearTimeout(t);
  }, [reloadToken]);

  const iframeKey = useMemo(() => `${url}#${reloadToken}`, [url, reloadToken]);

  const handleLoad = () => {
    setLoading(false);
    setFailed(false);
    setOffline(false);
    if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    if (url) {
      reportPageLoaded(url).catch(() => {
        /* non-fatal */
      });
    }
  };

  const enterAdmin = () => setAdminMode(true);


  return (
    <div className="relative h-full w-full overflow-hidden bg-player-bg">
      {url ? (
        <iframe
          key={iframeKey}
          src={url}
          title="Player"
          className="h-full w-full border-0"
          onLoad={handleLoad}
          allow="autoplay; fullscreen; geolocation; camera; microphone"
          sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals allow-presentation"
          referrerPolicy="no-referrer"
        />
      ) : (
        // Standby screen shown until a page/URL is configured: a calm,
        // full-screen default view instead of a bare error message.
        <div
          className="flex h-full w-full items-center justify-center"
          style={{
            background:
              "radial-gradient(120% 120% at 50% 0%, #16213e 0%, #0b0f19 55%, #070a12 100%)",
          }}
        >
          <div className="flex flex-col items-center px-6 text-center">
            <div
              className="flex h-20 w-20 items-center justify-center rounded-2xl"
              style={{
                background: "linear-gradient(135deg, #3b82f6, #6366f1)",
                boxShadow: "0 18px 40px -12px rgba(59, 130, 246, 0.55)",
              }}
            >
              <svg
                width="40"
                height="40"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#ffffff"
                strokeWidth="1.6"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <rect x="2" y="4" width="20" height="14" rx="2" />
                <path d="M8 21h8" />
                <path d="M12 18v3" />
                <path d="M10 9.5l4 2.5-4 2.5z" fill="#ffffff" stroke="none" />
              </svg>
            </div>

            <div className="mt-6 text-2xl font-semibold text-slate-100">
              Ready to play
            </div>
            <div className="mt-2 max-w-md text-sm leading-relaxed text-slate-400">
              No page is configured yet. Add one in the admin UI or send a URL
              to the player API — your content will appear here fullscreen.
            </div>

            <div className="mt-6 rounded-md border border-slate-700 bg-panel px-3 py-2 font-mono text-xs text-slate-400">
              {"POST http://<device>:8787/api/v1/display"}
            </div>

            <div className="mt-10 text-5xl font-semibold text-slate-200">
              {clock.toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </div>
            <div className="mt-1 text-xs uppercase tracking-wide text-slate-500">
              {clock.toLocaleDateString([], {
                weekday: "long",
                month: "long",
                day: "numeric",
              })}
            </div>
          </div>
        </div>   
      )}

      {/* Loading hint (subtle, auto-hides). */}
      {loading && showHint && (
        <div className="pointer-events-none absolute bottom-4 left-4 rounded bg-black/60 px-3 py-1.5 text-xs text-slate-200">
          Loading {url}…
        </div>
      )}

      {/* Offline fallback banner. */}
      {failed && (
        <div className="pointer-events-none absolute inset-x-0 top-0 flex justify-center">
          <div className="mt-3 rounded-full bg-amber-500/90 px-4 py-2 text-sm font-medium text-black shadow-lg">
            Network unavailable — showing last available page
          </div>
        </div>
      )}

      {/* Mode badge + hidden admin entry (small dot in the corner). */}
      <div className="absolute right-3 top-3 flex items-center gap-2 opacity-0 transition-opacity hover:opacity-100">
        <span className="rounded-full bg-black/50 px-2 py-1 text-[10px] uppercase tracking-wide text-slate-300">
          {mode}
        </span>
        <button
          onClick={enterAdmin}
          className="rounded-full bg-black/50 px-2 py-1 text-[10px] text-slate-300 hover:bg-black/70"
          title="Open admin UI"
        >
          admin
        </button>
      </div>
    </div>
  );
}