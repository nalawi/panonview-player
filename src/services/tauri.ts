import { invoke } from "@tauri-apps/api/core";
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
 * Typed wrappers around the Rust IPC commands. All display mutations flow
 * through these, which in turn call the DisplayController on the Rust side.
 */

// ----- Display -----

export const getDisplayStatus = () =>
  invoke<DisplayStatus>("get_display_status");

export const getInitialUrl = () =>
  invoke<string | null>("get_initial_url");

export const setDisplayUrl = (url: string) =>
  invoke<void>("set_display_url", { url });

export const showDisplayPage = (pageId: number) =>
  invoke<void>("show_display_page", { pageId });

export const refreshDisplay = () => invoke<void>("refresh_display");

export const displayBack = () => invoke<string | null>("display_back");

export const displayForward = () => invoke<string | null>("display_forward");

export const resetDisplay = () => invoke<void>("reset_display");

export const overrideDisplay = (
  url: string,
  duration?: number,
  priority?: number,
) => invoke<void>("override_display", { url, duration, priority });

export const reportPageLoaded = (url: string) =>
  invoke<void>("report_page_loaded", { url });

// ----- Pages -----

export const listPages = () => invoke<Page[]>("list_pages");

export const getPage = (id: number) =>
  invoke<Page | null>("get_page", { id });

export const createPage = (page: NewPage) =>
  invoke<Page>("create_page", { page });

export const updatePage = (id: number, page: UpdatePage) =>
  invoke<Page | null>("update_page", { id, page });

export const deletePage = (id: number) =>
  invoke<boolean>("delete_page", { id });

// ----- Schedules -----

export const listSchedules = () => invoke<Schedule[]>("list_schedules");

export const createSchedule = (schedule: NewSchedule) =>
  invoke<Schedule>("create_schedule", { schedule });

export const updateSchedule = (id: number, schedule: UpdateSchedule) =>
  invoke<Schedule | null>("update_schedule", { id, schedule });

export const deleteSchedule = (id: number) =>
  invoke<boolean>("delete_schedule", { id });

// ----- Settings -----

export const getSettings = () => invoke<SettingsMap>("get_settings");

export const getApiKey = () => invoke<string>("get_api_key");

export const updateSettings = (settings: Record<string, string>) =>
  invoke<void>("update_settings", { settings });

export const regenerateApiKey = () => invoke<string>("regenerate_api_key");

// ----- Device -----

export const getDevice = () => invoke<Device | null>("get_device");

export const updateDevice = (device: UpdateDevice) =>
  invoke<Device | null>("update_device", { device });

// ----- System -----

export const restartHttpServer = () => invoke<void>("restart_http_server");

export const setAdminMode = (enabled: boolean) =>
  invoke<void>("set_admin_mode", { enabled });

export const schedulerTick = () => invoke<void>("scheduler_tick");

export const schedulerStatus = () => invoke<boolean>("scheduler_status");