/**
 * Shared application types mirroring the Rust models and IPC payloads.
 */

export type DisplayMode = "manual" | "scheduler" | "override" | "idle";

export interface Page {
  id: number;
  name: string;
  url: string;
  duration: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface NewPage {
  name: string;
  url: string;
  duration?: number;
  enabled?: boolean;
}

export interface UpdatePage {
  name?: string;
  url?: string;
  duration?: number;
  enabled?: boolean;
}

export type ScheduleType = "TIME" | "ROTATION";

export interface Schedule {
  id: number;
  page_id: number;
  schedule_type: ScheduleType;
  start_date: string | null;
  end_date: string | null;
  start_time: string | null;
  end_time: string | null;
  days: string | null;
  priority: number;
  enabled: boolean;
  sequence: number | null;
}

export interface NewSchedule {
  page_id: number;
  schedule_type: ScheduleType;
  start_date?: string | null;
  end_date?: string | null;
  start_time?: string | null;
  end_time?: string | null;
  days?: string[] | null;
  priority?: number;
  enabled?: boolean;
  sequence?: number | null;
}

export interface UpdateSchedule extends Partial<NewSchedule> {}

export interface DisplayStatus {
  current_url: string | null;
  current_page_id: number | null;
  mode: DisplayMode;
  started_at: string | null;
  expires_at: string | null;
  scheduler_running: boolean;
  can_back: boolean;
  can_forward: boolean;
  last_successful_url: string | null;
  last_successful_at: string | null;
}

export interface NavigatePayload {
  url: string;
  mode: DisplayMode;
  page_id: number | null;
  reload_token: number;
}

export interface Device {
  id: string;
  name: string;
  hostname: string | null;
  platform: string | null;
  version: string | null;
  location: string | null;
  group_name: string | null;
  last_seen: string | null;
  created_at: string;
}

export interface UpdateDevice {
  name?: string;
  location?: string;
  group_name?: string;
}

/** Settings map returned by Rust: values may be strings or a masked object. */
export type SettingsValue = string | { masked: boolean; value: string };
export type SettingsMap = Record<string, SettingsValue>;

export const DAYS = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"] as const;
export type Day = (typeof DAYS)[number];