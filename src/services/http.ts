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

/**
 * REST client for the embedded HTTP API.
 *
 * Used when the admin console runs in a normal browser (served at `/ui`), as
 * opposed to inside the Tauri WebView (which uses IPC). The API talks to the
 * very same DisplayController the in-app UI does.
 */

const KEY_STORAGE = "webplayer.apiKey";

export function getStoredKey(): string {
  try {
    return localStorage.getItem(KEY_STORAGE) ?? "";
  } catch {
    return "";
  }
}

export function setStoredKey(key: string): void {
  try {
    localStorage.setItem(KEY_STORAGE, key);
  } catch {
    /* ignore */
  }
}

export function clearStoredKey(): void {
  try {
    localStorage.removeItem(KEY_STORAGE);
  } catch {
    /* ignore */
  }
}

/** API calls are same-origin (the UI is served by the player itself). */
function baseUrl(): string {
  return "";
}

export class HttpError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = {};
  const key = getStoredKey();
  if (key) {
    headers["X-API-Key"] = key;
  }
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
  }

  const res = await fetch(`${baseUrl()}${path}`, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  let json: unknown = null;
  const text = await res.text();
  if (text) {
    try {
      json = JSON.parse(text);
    } catch {
      json = null;
    }
  }

  if (!res.ok) {
    const msg =
      (json as { error?: string } | null)?.error ??
      `HTTP ${res.status} ${res.statusText}`;
    throw new HttpError(res.status, msg);
  }

  // Unwrap the { success, ...data } envelope.
  if (json && typeof json === "object" && "success" in json) {
    const { success: _success, ...rest } = json as Record<string, unknown>;
    return rest as T;
  }
  return json as T;
}

// ----- Display -----

export const httpGetStatus = () =>
  request<unknown>("GET", "/api/v1/status").then(normalizeStatus);

export const httpSetUrl = (url: string) =>
  request<unknown>("POST", "/api/v1/display", { url });

export const httpRefresh = () =>
  request<unknown>("POST", "/api/v1/display/refresh");

export const httpBack = () => request<unknown>("POST", "/api/v1/display/back");

export const httpForward = () =>
  request<unknown>("POST", "/api/v1/display/forward");

export const httpOverride = (
  url: string,
  duration?: number,
  priority?: number,
) =>
  request<unknown>("POST", "/api/v1/display/override", {
    url,
    duration,
    priority,
  });

export const httpShowPage = (pageId: number) =>
  request<unknown>("POST", "/api/v1/display/page", { page_id: pageId });

export const httpReset = () =>
  request<unknown>("POST", "/api/v1/display/reset");

// ----- Pages -----

export const httpListPages = () =>
  request<{ pages: Page[] }>("GET", "/api/v1/pages").then((r) => r.pages ?? []);

export const httpCreatePage = (page: NewPage) =>
  request<Page>("POST", "/api/v1/pages", page);

export const httpUpdatePage = (id: number, page: UpdatePage) =>
  request<Page>("PUT", `/api/v1/pages/${id}`, page);

export const httpDeletePage = (id: number) =>
  request<unknown>("DELETE", `/api/v1/pages/${id}`);

// ----- Schedules -----

export const httpListSchedules = () =>
  request<{ schedules: Schedule[] }>("GET", "/api/v1/schedules").then(
    (r) => r.schedules ?? [],
  );

export const httpCreateSchedule = (schedule: NewSchedule) =>
  request<Schedule>("POST", "/api/v1/schedules", schedule);

export const httpUpdateSchedule = (id: number, schedule: UpdateSchedule) =>
  request<Schedule>("PUT", `/api/v1/schedules/${id}`, schedule);

export const httpDeleteSchedule = (id: number) =>
  request<unknown>("DELETE", `/api/v1/schedules/${id}`);

// ----- Settings -----

export const httpGetSettings = () =>
  request<SettingsMap>("GET", "/api/v1/settings");

export const httpUpdateSettings = (settings: Record<string, string>) =>
  request<SettingsMap>("PUT", "/api/v1/settings", { settings }).then(
    () => undefined,
  );

export const httpRegenerateKey = () =>
  request<{ api_key: string }>("POST", "/api/v1/settings/regenerate-key").then(
    (r) => r.api_key,
  );

// ----- Device -----

export const httpGetDevice = () =>
  request<Device>("GET", "/api/v1/device").catch(() => null);

export const httpUpdateDevice = (device: UpdateDevice) =>
  request<Device>("PUT", "/api/v1/device", device);

// ----- System -----

export const httpReload = () => request<unknown>("POST", "/api/v1/reload");
export const httpRestart = () => request<unknown>("POST", "/api/v1/restart");

/**
 * The `/status` endpoint has a slightly different shape than the in-app
 * DisplayStatus; normalise it so the UI is identical in both modes.
 */
function normalizeStatus(raw: unknown): DisplayStatus {
  const r = (raw ?? {}) as Record<string, unknown>;
  return {
    current_url: (r.current_url as string | null) ?? null,
    current_page_id: (r.current_page_id as number | null) ?? null,
    mode: (r.mode as DisplayStatus["mode"]) ?? "scheduler",
    started_at:
      (r.started_at as string | null) ??
      (r.uptime_started_at as string | null) ??
      null,
    expires_at: (r.expires_at as string | null) ?? null,
    scheduler_running: Boolean(r.scheduler_running),
    can_back: Boolean(r.can_back),
    can_forward: Boolean(r.can_forward),
    last_successful_url: (r.last_successful_url as string | null) ?? null,
    last_successful_at: (r.last_successful_at as string | null) ?? null,
  };
}

/** Verify an API key by hitting an authenticated endpoint. */
export async function httpVerifyKey(key: string): Promise<boolean> {
  const res = await fetch("/api/v1/status", {
    headers: { "X-API-Key": key },
  });
  return res.ok;
}

/**
 * Ask the player whether API-key auth is required (`auth_mode`). The probe
 * never reveals the key itself — the user must paste it from the desktop
 * app's Settings → API Key. Returns true when credentials are required.
 */
export async function httpAuthRequired(): Promise<boolean> {
  const res = await fetch("/api/v1/auth");
  if (!res.ok) return true;
  const json = (await res.json()) as { auth_required?: boolean };
  return json.auth_required !== false;
}
