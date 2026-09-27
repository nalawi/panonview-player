import type {
  Device,
  DisplayStatus,
  NewPage,
  NewSchedule,
  Page,
  Schedule,
  SettingsMap,
  UpdateDevice,
  UpdatePage,
  UpdateSchedule,
} from "../types";
import * as ipc from "./tauri";
import * as http from "./http";

/**
 * Unified API facade.
 *
 * The admin console runs in two places:
 *   - Inside the Tauri WebView  → uses IPC commands (fast, no auth needed).
 *   - In a normal web browser   → uses the embedded REST API at /api/v1.
 *
 * All pages call this module, so they work unchanged in both modes.
 */

/** True when running inside the Tauri WebView. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** True when running as a plain browser page (served at /ui). */
export function isWeb(): boolean {
  return !isTauri();
}

// ----- Display -----

export const getDisplayStatus = (): Promise<DisplayStatus> =>
  isTauri() ? ipc.getDisplayStatus() : http.httpGetStatus();

export const setDisplayUrl = (url: string): Promise<unknown> =>
  isTauri() ? ipc.setDisplayUrl(url) : http.httpSetUrl(url);

export const showDisplayPage = (pageId: number): Promise<unknown> =>
  isTauri() ? ipc.showDisplayPage(pageId) : http.httpShowPage(pageId);

export const refreshDisplay = (): Promise<unknown> =>
  isTauri() ? ipc.refreshDisplay() : http.httpRefresh();

export const displayBack = (): Promise<unknown> =>
  isTauri() ? ipc.displayBack() : http.httpBack();

export const displayForward = (): Promise<unknown> =>
  isTauri() ? ipc.displayForward() : http.httpForward();

/** Clear the display so the player shows the default standby screen. */
export const resetDisplay = (): Promise<unknown> =>
  isTauri() ? ipc.resetDisplay() : http.httpReset();

export const overrideDisplay = (
  url: string,
  duration?: number,
  priority?: number,
): Promise<unknown> =>
  isTauri()
    ? ipc.overrideDisplay(url, duration, priority)
    : http.httpOverride(url, duration, priority);

// ----- Pages -----

export const listPages = (): Promise<Page[]> =>
  isTauri() ? ipc.listPages() : http.httpListPages();

export const createPage = (page: NewPage): Promise<Page> =>
  isTauri() ? ipc.createPage(page) : http.httpCreatePage(page);

export const updatePage = (id: number, page: UpdatePage): Promise<Page | null> =>
  isTauri() ? ipc.updatePage(id, page) : http.httpUpdatePage(id, page);

export const deletePage = (id: number): Promise<unknown> =>
  isTauri() ? ipc.deletePage(id) : http.httpDeletePage(id);

// ----- Schedules -----

export const listSchedules = (): Promise<Schedule[]> =>
  isTauri() ? ipc.listSchedules() : http.httpListSchedules();

export const createSchedule = (schedule: NewSchedule): Promise<Schedule> =>
  isTauri() ? ipc.createSchedule(schedule) : http.httpCreateSchedule(schedule);

export const updateSchedule = (
  id: number,
  schedule: UpdateSchedule,
): Promise<Schedule | null> =>
  isTauri()
    ? ipc.updateSchedule(id, schedule)
    : http.httpUpdateSchedule(id, schedule);

export const deleteSchedule = (id: number): Promise<unknown> =>
  isTauri() ? ipc.deleteSchedule(id) : http.httpDeleteSchedule(id);

// ----- Settings -----

export const getSettings = (): Promise<SettingsMap> =>
  isTauri() ? ipc.getSettings() : http.httpGetSettings();

export const updateSettings = (
  settings: Record<string, string>,
): Promise<unknown> =>
  isTauri() ? ipc.updateSettings(settings) : http.httpUpdateSettings(settings);

export const getApiKey = (): Promise<string> =>
  isTauri() ? ipc.getApiKey() : Promise.resolve(http.getStoredKey());

export const regenerateApiKey = (): Promise<string> =>
  isTauri() ? ipc.regenerateApiKey() : http.httpRegenerateKey();

export const restartHttpServer = (): Promise<unknown> =>
  isTauri() ? ipc.restartHttpServer() : Promise.resolve(null);

/** Toggle the desktop window between player (fullscreen) and admin mode. */
export const setAdminMode = (enabled: boolean): Promise<unknown> =>
  isTauri() ? ipc.setAdminMode(enabled) : Promise.resolve(null);

export const reloadDisplay = (): Promise<unknown> =>
  isTauri() ? ipc.refreshDisplay() : http.httpReload();

// ----- Device -----

export const getDevice = (): Promise<Device | null> =>
  isTauri() ? ipc.getDevice() : http.httpGetDevice();

export const updateDevice = (device: UpdateDevice): Promise<Device | null> =>
  isTauri() ? ipc.updateDevice(device) : http.httpUpdateDevice(device);

// ----- Auth (browser only) -----

export { httpVerifyKey, setStoredKey, clearStoredKey, getStoredKey } from "./http";