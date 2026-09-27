import { create } from "zustand";
import type { DisplayMode, NavigatePayload } from "../types";

interface DisplayStore {
  /** Current URL shown in the player view. */
  url: string | null;
  mode: DisplayMode;
  pageId: number | null;
  /** Increments on every forced reload (used to re-key iframes). */
  reloadToken: number;
  /** True when the player has detected a load failure. */
  offline: boolean;
  /** Whether the admin UI is active (vs pure player mode). */
  adminMode: boolean;

  applyNavigate: (payload: NavigatePayload) => void;
  setUrl: (url: string | null) => void;
  setOffline: (offline: boolean) => void;
  setAdminMode: (admin: boolean) => void;
}

export const useDisplayStore = create<DisplayStore>((set) => ({
  url: null,
  mode: "scheduler",
  pageId: null,
  reloadToken: 0,
  offline: false,
  adminMode: false,

  applyNavigate: (payload) =>
    set({
      url: payload.url,
      mode: payload.mode,
      pageId: payload.page_id,
      reloadToken: payload.reload_token,
      offline: false,
    }),
  setUrl: (url) => set({ url }),
  setOffline: (offline) => set({ offline }),
  setAdminMode: (adminMode) => set({ adminMode }),
}));