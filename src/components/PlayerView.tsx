import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useDisplayStore } from "../stores/display";
import { getDisplayStatus, getInitialUrl, reportPageLoaded } from "../services/tauri";
import type { DisplayMode, DisplayStatus } from "../types";
import PlayerIdleScreen from './PlayerIdleScreen';

/** Snapshot of the last active page, kept locally so the player can show it
 * instantly on the very first frame after a restart — before the Rust side
 * has even answered. The Rust display state remains authoritative; this is
 * only the boot-up seed. */
interface LastSession {
  url: string;
  mode: DisplayMode;
  pageId: number | null;
}

const SESSION_KEY = "panonview:last_session";

function readSession(): LastSession | null {
  try {
    const raw = window.localStorage.getItem(SESSION_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<LastSession>;
    if (typeof parsed.url === "string" && parsed.url) {
      return {
        url: parsed.url,
        mode: (parsed.mode ?? "manual") as DisplayMode,
        pageId: typeof parsed.pageId === "number" ? parsed.pageId : null,
      };
    }
  } catch {
    /* corrupt or unavailable storage — nothing to restore */
  }
  return null;
}

function writeSession(session: LastSession) {
  try {
    window.localStorage.setItem(SESSION_KEY, JSON.stringify(session));
  } catch {
    /* storage may be unavailable — restoring still works via Rust */
  }
}

function clearSession() {
  try {
    window.localStorage.removeItem(SESSION_KEY);
  } catch {
    /* ignore */
  }
}

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
  const pageId = useDisplayStore((s) => s.pageId);
  const applyNavigate = useDisplayStore((s) => s.applyNavigate);
  const setOffline = useDisplayStore((s) => s.setOffline);
  const setAdminMode = useDisplayStore((s) => s.setAdminMode);

  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);
  const [showHint, setShowHint] = useState(true);
  /** True once Rust has answered with the authoritative startup URL. */
  const [resolved, setResolved] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  // Live clock for the "waiting for content" standby screen.
  // const [clock, setClock] = useState(() => new Date());
  // useEffect(() => {
  //   const t = window.setInterval(() => setClock(new Date()), 1000);
  //   return () => window.clearInterval(t);
  // }, []);

  // Resolve the initial URL on first mount. The locally remembered session is
  // applied synchronously (so the last active page shows up immediately after
  // a restart), then reconciled with the Rust display state, which is the
  // single source of truth.
  useEffect(() => {
    const session = readSession();
    if (session) {
      useDisplayStore.setState({
        url: session.url,
        mode: session.mode,
        pageId: session.pageId,
      });
    }

    let cancelled = false;
    (async () => {
      try {
        const [url, status] = await Promise.all([getInitialUrl(), getDisplayStatus()]);
        if (cancelled) return;
        if (url) {
          // Continue showing the last active page/URL.
          useDisplayStore.getState().setUrl(url);
          useDisplayStore.setState({
            mode: status?.mode ?? "manual",
            pageId: status?.current_page_id ?? null,
          });
        } else {
          // Rust says there is nothing to show (idle/standby) — forget the
          // remembered session so it does not come back on the next boot.
          clearSession();
          useDisplayStore.getState().setUrl(null);
        }
      } catch {
        // IPC unavailable: keep the remembered page on screen (offline-first).
      } finally {
        if (!cancelled) setResolved(true);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Record the active page/URL on every navigation so a stop or crash can be
  // recovered from on the next launch.
  useEffect(() => {
    if (url) {
      writeSession({ url, mode, pageId });
    } else if (resolved) {
      // Only clear once Rust has confirmed there is no active page.
      clearSession();
    }
  }, [url, mode, pageId, resolved]);

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
        <PlayerIdleScreen />
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