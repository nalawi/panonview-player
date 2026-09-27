use crate::models::*;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Thread-safe database handle shared across the app.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (or create) the SQLite database at the given path and run migrations.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| e.to_string())?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        db.ensure_seed()?;
        Ok(db)
    }

    /// In-memory database (used by the test suite).
    #[cfg(test)]
    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| e.to_string())?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        db.ensure_seed()?;
        Ok(db)
    }

    fn with_conn<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, String>,
    {
        let guard = self.conn.lock().map_err(|e| e.to_string())?;
        f(&guard)
    }

    /// Apply schema migrations. Uses `user_version` pragma for versioning so
    /// schema changes survive application upgrades.
    fn run_migrations(&self) -> Result<(), String> {
        self.with_conn(|conn| {
            let version: i64 = conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;

            if version < 1 {
                conn.execute_batch(MIGRATION_V1)
                    .map_err(|e| e.to_string())?;
                conn.pragma_update(None, "user_version", 1)
                    .map_err(|e| e.to_string())?;
            }

            Ok(())
        })
    }

    /// Seed default settings and device identity, and ensure a display_state row.
    fn ensure_seed(&self) -> Result<(), String> {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "display".to_string());

        self.with_conn(|conn| {
            // Ensure the singleton display_state row exists.
            conn.execute(
                "INSERT OR IGNORE INTO display_state (id, mode) VALUES (1, 'manual')",
                [],
            )
            .map_err(|e| e.to_string())?;

            // Seed default settings.
            let defaults: [(&str, &str); 9] = [
                ("http_enabled", "true"),
                ("http_port", "8787"),
                ("http_bind", "0.0.0.0"),
                ("auth_mode", "apikey"),
                ("api_key", ""),
                ("allow_remote", "true"),
                ("allowed_ips", ""),
                ("startup_mode", "player"),
                ("scheduler_tick_seconds", "5"),
            ];
            for (k, v) in defaults {
                conn.execute(
                    "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                    params![k, v],
                )
                .map_err(|e| e.to_string())?;
            }

            // Seed a device identity if none exists.
            let device_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM devices", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            if device_count == 0 {
                let id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO devices (id, name, hostname, platform, version, location, group_name, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        id,
                        hostname,
                        hostname,
                        std::env::consts::OS,
                        env!("CARGO_PKG_VERSION"),
                        "",
                        "",
                        now(),
                    ],
                )
                .map_err(|e| e.to_string())?;

                // Also record the identity in settings for easy lookup.
                conn.execute(
                    "INSERT OR IGNORE INTO settings (key, value) VALUES ('device_id', ?1)",
                    params![id],
                )
                .map_err(|e| e.to_string())?;

                // If no page exists, seed a friendly default page so the player
                // always has something to show.
                let page_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM pages", [], |r| r.get(0))
                    .map_err(|e| e.to_string())?;
                if page_count == 0 {
                    conn.execute(
                        "INSERT INTO pages (name, url, duration, enabled, created_at, updated_at)
                         VALUES (?1, ?2, ?3, 1, ?4, ?4)",
                        params![
                            "Welcome",
                            "https://tauri.app/",
                            60,
                            now(),
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }

            // Generate an API key on first boot so the control API is never
            // exposed unauthenticated by default. Users can change it later.
            let api_key = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'api_key'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_default();
            if api_key.trim().is_empty() {
                let key = uuid::Uuid::new_v4().simple().to_string();
                conn.execute(
                    "INSERT INTO settings (key, value) VALUES ('api_key', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key],
                )
                .map_err(|e| e.to_string())?;
            }

            Ok(())
        })
    }

    // ----- Settings -----

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT value FROM settings WHERE key = ?1")
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![key]).map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(Some(row.get::<_, String>(0).map_err(|e| e.to_string())?))
            } else {
                Ok(None)
            }
        })
    }

    pub fn get_setting_or(&self, key: &str, default: &str) -> String {
        self.get_setting(key)
            .ok()
            .flatten()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn all_settings(&self) -> Result<Vec<(String, String)>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT key, value FROM settings ORDER BY key")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
    }

    // ----- Pages -----

    pub fn list_pages(&self) -> Result<Vec<Page>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, url, duration, enabled, created_at, updated_at
                     FROM pages ORDER BY id",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], row_to_page)
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
    }

    pub fn get_page(&self, id: i64) -> Result<Option<Page>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, url, duration, enabled, created_at, updated_at
                     FROM pages WHERE id = ?1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(Some(row_to_page(row).map_err(|e| e.to_string())?))
            } else {
                Ok(None)
            }
        })
    }

    pub fn create_page(&self, p: &NewPage) -> Result<Page, String> {
        self.with_conn(|conn| {
            let ts = now();
            conn.execute(
                "INSERT INTO pages (name, url, duration, enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![p.name, p.url, p.duration, p.enabled as i64, ts],
            )
            .map_err(|e| e.to_string())?;
            let id = conn.last_insert_rowid();
            get_page_conn(conn, id)?
                .ok_or_else(|| "failed to read back created page".to_string())
        })
    }

    pub fn update_page(&self, id: i64, u: &UpdatePage) -> Result<Option<Page>, String> {
        self.with_conn(|conn| {
            let existing = match get_page_conn(conn, id)? {
                Some(p) => p,
                None => return Ok(None),
            };
            let name = u.name.clone().unwrap_or(existing.name);
            let url = u.url.clone().unwrap_or(existing.url);
            let duration = u.duration.unwrap_or(existing.duration);
            let enabled = u.enabled.unwrap_or(existing.enabled);
            conn.execute(
                "UPDATE pages SET name = ?1, url = ?2, duration = ?3, enabled = ?4, updated_at = ?5
                 WHERE id = ?6",
                params![name, url, duration, enabled as i64, now(), id],
            )
            .map_err(|e| e.to_string())?;
            Ok(get_page_conn(conn, id)?)
        })
    }

    pub fn delete_page(&self, id: i64) -> Result<bool, String> {
        self.with_conn(|conn| {
            let n = conn
                .execute("DELETE FROM pages WHERE id = ?1", params![id])
                .map_err(|e| e.to_string())?;
            Ok(n > 0)
        })
    }

    // ----- Schedules -----

    pub fn list_schedules(&self) -> Result<Vec<Schedule>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, page_id, schedule_type, start_date, end_date, start_time,
                            end_time, days, priority, enabled, sequence
                     FROM schedules ORDER BY priority DESC, id",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], row_to_schedule)
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
    }

    pub fn get_schedule(&self, id: i64) -> Result<Option<Schedule>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, page_id, schedule_type, start_date, end_date, start_time,
                            end_time, days, priority, enabled, sequence
                     FROM schedules WHERE id = ?1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(Some(row_to_schedule(row).map_err(|e| e.to_string())?))
            } else {
                Ok(None)
            }
        })
    }

    pub fn create_schedule(&self, s: &NewSchedule) -> Result<Schedule, String> {
        let days = s.days.as_ref().map(|d| d.join(","));
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO schedules (page_id, schedule_type, start_date, end_date,
                    start_time, end_time, days, priority, enabled, sequence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    s.page_id,
                    s.schedule_type,
                    s.start_date,
                    s.end_date,
                    s.start_time,
                    s.end_time,
                    days,
                    s.priority.unwrap_or(0),
                    s.enabled as i64,
                    s.sequence,
                ],
            )
            .map_err(|e| e.to_string())?;
            let id = conn.last_insert_rowid();
            get_schedule_conn(conn, id)?
                .ok_or_else(|| "failed to read back created schedule".to_string())
        })
    }

    pub fn update_schedule(
        &self,
        id: i64,
        u: &UpdateSchedule,
    ) -> Result<Option<Schedule>, String> {
        self.with_conn(|conn| {
            let existing = match get_schedule_conn(conn, id)? {
                Some(s) => s,
                None => return Ok(None),
            };
            let page_id = u.page_id.unwrap_or(existing.page_id);
            let schedule_type = u
                .schedule_type
                .clone()
                .unwrap_or(existing.schedule_type);
            let start_date = u.start_date.clone().or(existing.start_date);
            let end_date = u.end_date.clone().or(existing.end_date);
            let start_time = u.start_time.clone().or(existing.start_time);
            let end_time = u.end_time.clone().or(existing.end_time);
            let days = u
                .days
                .as_ref()
                .map(|d| d.join(","))
                .or(existing.days);
            let priority = u.priority.unwrap_or(existing.priority);
            let enabled = u.enabled.unwrap_or(existing.enabled);
            let sequence = u.sequence.or(existing.sequence);
            conn.execute(
                "UPDATE schedules SET page_id = ?1, schedule_type = ?2, start_date = ?3,
                    end_date = ?4, start_time = ?5, end_time = ?6, days = ?7, priority = ?8,
                    enabled = ?9, sequence = ?10 WHERE id = ?11",
                params![
                    page_id,
                    schedule_type,
                    start_date,
                    end_date,
                    start_time,
                    end_time,
                    days,
                    priority,
                    enabled as i64,
                    sequence,
                    id
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(get_schedule_conn(conn, id)?)
        })
    }

    pub fn delete_schedule(&self, id: i64) -> Result<bool, String> {
        self.with_conn(|conn| {
            let n = conn
                .execute("DELETE FROM schedules WHERE id = ?1", params![id])
                .map_err(|e| e.to_string())?;
            Ok(n > 0)
        })
    }

    // ----- Display state -----

    pub fn get_display_state(&self) -> Result<DisplayState, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, current_page_id, current_url, mode, started_at, expires_at
                     FROM display_state WHERE id = 1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(DisplayState {
                    id: row.get(0).map_err(|e| e.to_string())?,
                    current_page_id: row.get(1).map_err(|e| e.to_string())?,
                    current_url: row.get(2).map_err(|e| e.to_string())?,
                    mode: row.get(3).map_err(|e| e.to_string())?,
                    started_at: row.get(4).map_err(|e| e.to_string())?,
                    expires_at: row.get(5).map_err(|e| e.to_string())?,
                })
            } else {
                Ok(DisplayState::default())
            }
        })
    }

    pub fn set_display_state(&self, state: &DisplayState) -> Result<(), String> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO display_state (id, current_page_id, current_url, mode, started_at, expires_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    current_page_id = excluded.current_page_id,
                    current_url = excluded.current_url,
                    mode = excluded.mode,
                    started_at = excluded.started_at,
                    expires_at = excluded.expires_at",
                params![
                    state.current_page_id,
                    state.current_url,
                    state.mode,
                    state.started_at,
                    state.expires_at
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    // ----- Offline fallback bookkeeping -----

    pub fn record_successful_page(&self, url: &str) -> Result<(), String> {
        self.set_setting("last_successful_url", url)?;
        self.set_setting("last_successful_at", &now())?;
        Ok(())
    }

    pub fn last_successful(&self) -> (Option<String>, Option<String>) {
        (
            self.get_setting("last_successful_url").ok().flatten(),
            self.get_setting("last_successful_at").ok().flatten(),
        )
    }

    // ----- Device -----

    pub fn get_device(&self) -> Result<Option<Device>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, hostname, platform, version, location, group_name,
                            last_seen, created_at
                     FROM devices ORDER BY created_at LIMIT 1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(Some(Device {
                    id: row.get(0).map_err(|e| e.to_string())?,
                    name: row.get(1).map_err(|e| e.to_string())?,
                    hostname: row.get(2).map_err(|e| e.to_string())?,
                    platform: row.get(3).map_err(|e| e.to_string())?,
                    version: row.get(4).map_err(|e| e.to_string())?,
                    location: row.get(5).map_err(|e| e.to_string())?,
                    group_name: row.get(6).map_err(|e| e.to_string())?,
                    last_seen: row.get(7).map_err(|e| e.to_string())?,
                    created_at: row.get(8).map_err(|e| e.to_string())?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn update_device(&self, u: &UpdateDevice) -> Result<Option<Device>, String> {
        let id = self.get_setting_or("device_id", "");
        if id.is_empty() {
            return Ok(None);
        }
        self.with_conn(|conn| {
            let existing = get_device_by_id_conn(conn, &id)?;
            let existing = match existing {
                Some(d) => d,
                None => return Ok(None),
            };
            let name = u.name.clone().unwrap_or(existing.name);
            let location = u.location.clone().unwrap_or(existing.location.unwrap_or_default());
            let group_name = u
                .group_name
                .clone()
                .unwrap_or(existing.group_name.unwrap_or_default());
            conn.execute(
                "UPDATE devices SET name = ?1, location = ?2, group_name = ?3 WHERE id = ?4",
                params![name, location, group_name, id],
            )
            .map_err(|e| e.to_string())?;
            get_device_by_id_conn(conn, &id)
        })
    }

    pub fn touch_device(&self) -> Result<(), String> {
        let id = self.get_setting_or("device_id", "");
        if id.is_empty() {
            return Ok(());
        }
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE devices SET last_seen = ?1 WHERE id = ?2",
                params![now(), id],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    // ----- Commands audit -----

    pub fn log_command(&self, command: &str, payload: Option<&str>) -> Result<(), String> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO commands (command, payload, status, created_at)
                 VALUES (?1, ?2, 'pending', ?3)",
                params![command, payload, now()],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    /// Return the most recent command records (newest first).
    pub fn list_commands(&self, limit: i64) -> Result<Vec<CommandRecord>, String> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, command, payload, status, created_at, executed_at
                     FROM commands ORDER BY id DESC LIMIT ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![limit], |row| {
                    Ok(CommandRecord {
                        id: row.get(0)?,
                        command: row.get(1)?,
                        payload: row.get(2)?,
                        status: row.get(3)?,
                        created_at: row.get(4)?,
                        executed_at: row.get(5)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
    }
}

// ----- Helper functions -----

fn get_page_conn(conn: &Connection, id: i64) -> Result<Option<Page>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, url, duration, enabled, created_at, updated_at
             FROM pages WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(row_to_page(row).map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

fn row_to_page(row: &rusqlite::Row) -> rusqlite::Result<Page> {
    Ok(Page {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        duration: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn get_schedule_conn(conn: &Connection, id: i64) -> Result<Option<Schedule>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, page_id, schedule_type, start_date, end_date, start_time,
                    end_time, days, priority, enabled, sequence
             FROM schedules WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(row_to_schedule(row).map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

fn row_to_schedule(row: &rusqlite::Row) -> rusqlite::Result<Schedule> {
    Ok(Schedule {
        id: row.get(0)?,
        page_id: row.get(1)?,
        schedule_type: row.get(2)?,
        start_date: row.get(3)?,
        end_date: row.get(4)?,
        start_time: row.get(5)?,
        end_time: row.get(6)?,
        days: row.get(7)?,
        priority: row.get(8)?,
        enabled: row.get::<_, i64>(9)? != 0,
        sequence: row.get(10)?,
    })
}

fn get_device_by_id_conn(conn: &Connection, id: &str) -> Result<Option<Device>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, hostname, platform, version, location, group_name,
                    last_seen, created_at
             FROM devices WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(Device {
            id: row.get(0).map_err(|e| e.to_string())?,
            name: row.get(1).map_err(|e| e.to_string())?,
            hostname: row.get(2).map_err(|e| e.to_string())?,
            platform: row.get(3).map_err(|e| e.to_string())?,
            version: row.get(4).map_err(|e| e.to_string())?,
            location: row.get(5).map_err(|e| e.to_string())?,
            group_name: row.get(6).map_err(|e| e.to_string())?,
            last_seen: row.get(7).map_err(|e| e.to_string())?,
            created_at: row.get(8).map_err(|e| e.to_string())?,
        }))
    } else {
        Ok(None)
    }
}

/// Current UTC timestamp as an RFC3339 string.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Migration 1 — initial schema (see design doc §7).
const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    hostname TEXT,
    platform TEXT,
    version TEXT,
    location TEXT,
    group_name TEXT,
    last_seen DATETIME,
    created_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS pages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    duration INTEGER DEFAULT 60,
    enabled INTEGER DEFAULT 1,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS schedules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_id INTEGER NOT NULL,
    schedule_type TEXT NOT NULL,
    start_date TEXT,
    end_date TEXT,
    start_time TEXT,
    end_time TEXT,
    days TEXT,
    priority INTEGER DEFAULT 0,
    enabled INTEGER DEFAULT 1,
    sequence INTEGER,
    FOREIGN KEY(page_id) REFERENCES pages(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS display_state (
    id INTEGER PRIMARY KEY,
    current_page_id INTEGER,
    current_url TEXT,
    mode TEXT,
    started_at DATETIME,
    expires_at DATETIME
);

CREATE TABLE IF NOT EXISTS commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command TEXT NOT NULL,
    payload TEXT,
    status TEXT,
    created_at DATETIME,
    executed_at DATETIME
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);

CREATE INDEX IF NOT EXISTS idx_schedules_enabled ON schedules(enabled);
CREATE INDEX IF NOT EXISTS idx_pages_enabled ON pages(enabled);
"#;