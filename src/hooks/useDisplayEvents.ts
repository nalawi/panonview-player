import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DisplayStatus, NavigatePayload } from "../types";
import { useDisplayStore } from "../stores/display";

/**
 * Subscribe to Rust-emitted navigation events and apply them to the store.
 * This is the single path by which the WebView learns about display changes.
 */
export function useDisplayEvents() {
  const applyNavigate = useDisplayStore((s) => s.applyNavigate);

  useEffect(() => {
    let unlistenNav: (() => void) | undefined;

    listen<NavigatePayload>("display://navigate", (event) => {
      applyNavigate(event.payload);
    }).then((fn) => {
      unlistenNav = fn;
    });

    return () => {
      unlistenNav?.();
    };
  }, [applyNavigate]);
}

/**
 * Subscribe to display status events (emitted after API/UI changes).
 */
export function useStatusEvents(onStatus: (status: DisplayStatus) => void) {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<DisplayStatus>("display://state", (event) => {
      onStatus(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}