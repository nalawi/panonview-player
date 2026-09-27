use crate::database::{now, Database};
use crate::display::scheduler;
use crate::models::*;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

/// Navigation payload emitted to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NavigatePayload {
    pub url: String,
    pub mode: String,
    pub page_id: Option<i64>,
    /// Incremented every time the player must force a reload.
    pub reload_token: i64,
}

/// Full status payload emitted to the admin UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DisplayStatus {
    pub current_url: Option<String>,
    pub current_page_id: Option<i64>,
    pub mode: String,
    pub started_at: Option<String>,
    pub expires_at: Option<String>,
    pub scheduler_running: bool,
    pub can_back: bool,
    pub can_forward: bool,
    pub last_successful_url: Option<String>,
    pub last_successful_at: Option<String>,
}

struct History {
    entries: Vec<String>,
    index: i64,
}

impl History {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: -1,
        }
    }

    fn push(&mut self, url: &str) {
        // Truncate forward history when a new navigation happens.
        if self.index >= 0 && (self.index as usize) < self.entries.len() - 1 {
            self.entries.truncate(self.index as usize + 1);
        }
        if self.entries.last().map(|u| u == url).unwrap_or(false) {
            return;
        }
        self.entries.push(url.to_string());
        self.index = self.entries.len() as i64 - 1;
    }

    fn back(&mut self) -> Option<String> {
        if self.index > 0 {
            self.index -= 1;
            self.entries.get(self.index as usize).cloned()
        } else {
            None
        }
    }

    fn forward(&mut self) -> Option<String> {
        if self.can_forward() {
            self.index += 1;
            self.entries.get(self.index as usize).cloned()
        } else {
            None
        }
    }

    fn can_back(&self) -> bool {
        self.index > 0
    }

    fn can_forward(&self) -> bool {
        // `index` is -1 when the history is empty; guard before casting to
        // usize to avoid an overflow (and an index past the end).
        self.index >= 0 && (self.index as usize + 1) < self.entries.len()
    }
}

/// The DisplayController is the single source of truth for what is shown on
/// screen. The HTTP API, the scheduler, and the local admin UI all call into
/// this controller — nothing manipulates the WebView directly.
#[derive(Clone)]
pub struct DisplayController {
    db: Database,
    app: Arc<Mutex<Option<AppHandle>>>,
    history: Arc<Mutex<History>>,
    scheduler_running: Arc<AtomicBool>,
    reload_token: Arc<AtomicI64>,
}

impl DisplayController {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            app: Arc::new(Mutex::new(None)),
            history: Arc::new(Mutex::new(History::new())),
            scheduler_running: Arc::new(AtomicBool::new(false)),
            reload_token: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Attach the Tauri app handle once it becomes available at setup time.
    pub fn attach_app(&self, app: AppHandle) {
        if let Ok(mut guard) = self.app.lock() {
            *guard = Some(app);
        }
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    fn emit_navigate(&self, url: &str, mode: &str, page_id: Option<i64>) {
        let token = self.reload_token.fetch_add(1, Ordering::SeqCst) + 1;
        let payload = NavigatePayload {
            url: url.to_string(),
            mode: mode.to_string(),
            page_id,
            reload_token: token,
        };
        if let Ok(guard) = self.app.lock() {
            if let Some(app) = guard.as_ref() {
                let _ = app.emit("display://navigate", payload);
                let _ = app.emit("display://change", ());
            }
        }
    }

    /// Emit the current state to the admin UI so it can live-update.
    pub fn emit_status(&self) {
        if let Ok(guard) = self.app.lock() {
            if let Some(app) = guard.as_ref() {
                if let Ok(status) = self.status() {
                    let _ = app.emit("display://state", status);
                }
            }
        }
    }

    /// Persist the new display state and notify the WebView.
    fn apply(&self, url: &str, mode: &str, page_id: Option<i64>, expires_at: Option<String>) {
        let state = DisplayState {
            id: 1,
            current_page_id: page_id,
            current_url: Some(url.to_string()),
            mode: mode.to_string(),
            started_at: Some(now()),
            expires_at,
        };
        let _ = self.db.set_display_state(&state);
        {
            if let Ok(mut h) = self.history.lock() {
                h.push(url);
            }
        }
        self.emit_navigate(url, mode, page_id);
    }

    // ----- Public operations (single source of truth) -----

    /// Manually set the displayed URL. Switches to manual mode.
    pub fn set_url(&self, url: &str) -> Result<(), String> {
        self.db.log_command("set_url", Some(url)).ok();
        self.apply(url, MODE_MANUAL, None, None);
        Ok(())
    }

    /// Show a stored page by id.
    pub fn show_page(&self, page_id: i64) -> Result<(), String> {
        let page = self
            .db
            .get_page(page_id)?
            .ok_or_else(|| format!("page {} not found", page_id))?;
        self.db.log_command("show_page", Some(&page_id.to_string())).ok();
        self.apply(&page.url, MODE_MANUAL, Some(page.id), None);
        Ok(())
    }

    /// Force a reload of the current page.
    pub fn refresh(&self) -> Result<(), String> {
        let state = self.db.get_display_state()?;
        if let Some(url) = state.current_url {
            self.db.log_command("refresh", None).ok();
            self.emit_navigate(&url, &state.mode, state.current_page_id);
            Ok(())
        } else {
            Err("no current URL".to_string())
        }
    }

    /// Navigate back in the controller's internal history.
    pub fn back(&self) -> Result<Option<String>, String> {
        let url = {
            let mut h = self.history.lock().map_err(|e| e.to_string())?;
            h.back()
        };
        if let Some(ref current) = url {
            self.db.log_command("back", None).ok();
            let state = self.db.get_display_state().unwrap_or_default();
            // Do not rewrite history when moving through it.
            let new_state = DisplayState {
                current_url: Some(current.clone()),
                started_at: Some(now()),
                ..state
            };
            let _ = self.db.set_display_state(&new_state);
            self.emit_navigate(current, &new_state.mode, new_state.current_page_id);
        }
        Ok(url)
    }

    /// Navigate forward in the controller's internal history.
    pub fn forward(&self) -> Result<Option<String>, String> {
        let url = {
            let mut h = self.history.lock().map_err(|e| e.to_string())?;
            h.forward()
        };
        if let Some(ref current) = url {
            self.db.log_command("forward", None).ok();
            let state = self.db.get_display_state().unwrap_or_default();
            let new_state = DisplayState {
                current_url: Some(current.clone()),
                started_at: Some(now()),
                ..state
            };
            let _ = self.db.set_display_state(&new_state);
            self.emit_navigate(current, &new_state.mode, new_state.current_page_id);
        }
        Ok(url)
    }

    /// Apply a temporary, priority override. After `duration` seconds the
    /// scheduler resumes automatically.
    pub fn override_page(&self, url: &str, duration: i64, _priority: i64) -> Result<(), String> {
        let expires = chrono::Utc::now() + chrono::Duration::seconds(duration.max(1));
        self.db
            .log_command("override", Some(&format!("{}|{}", url, duration)))
            .ok();
        self.apply(url, MODE_OVERRIDE, None, Some(expires.to_rfc3339()));
        Ok(())
    }

    /// Clear the display so the player shows the default standby screen.
    ///
    /// The state is persisted with mode `idle` and no URL; the scheduler
    /// treats that as "no content" and stops applying fallbacks until a
    /// manual URL, an override, or an active schedule provides content again.
    pub fn reset(&self) -> Result<(), String> {
        self.db.log_command("reset", None).ok();
        let state = DisplayState {
            id: 1,
            current_page_id: None,
            current_url: None,
            mode: MODE_IDLE.to_string(),
            started_at: Some(now()),
            expires_at: None,
        };
        let _ = self.db.set_display_state(&state);
        // Empty URL tells the frontend to drop the iframe and show the
        // standby screen.
        self.emit_navigate("", MODE_IDLE, None);
        self.emit_status();
        Ok(())
    }

    /// Start the scheduler loop (idempotent).
    pub fn start_schedule(&self) {
        if self
            .scheduler_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let this = self.clone();
            tauri::async_runtime::spawn(async move {
                this.scheduler_loop().await;
            });
        }
    }

    /// Stop the scheduler loop.
    pub fn stop_schedule(&self) {
        self.scheduler_running.store(false, Ordering::SeqCst);
    }

    pub fn is_scheduler_running(&self) -> bool {
        self.scheduler_running.load(Ordering::SeqCst)
    }

    /// The core scheduler loop. Computes the target and applies it only when it
    /// differs from the current state.
    async fn scheduler_loop(&self) {
        // Apply the initial target immediately.
        self.tick();

        while self.scheduler_running.load(Ordering::SeqCst) {
            let sleep_secs = scheduler::seconds_until_next_transition(&self.db).clamp(1, 60);
            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs as u64)).await;
            if !self.scheduler_running.load(Ordering::SeqCst) {
                break;
            }
            self.tick();
        }
    }

    /// One scheduler evaluation.
    pub fn tick(&self) {
        let state = self.db.get_display_state().unwrap_or_default();
        let target = match scheduler::compute_target(&self.db) {
            Some(t) => t,
            None => return,
        };

        let same_url = state.current_url.as_deref() == Some(target.url.as_str());
        let same_mode = state.mode == target.mode;

        // If an override just expired, the mode changes and we re-apply.
        if same_url && same_mode {
            // Still ensure expiry gets cleared when an override ends.
            if state.mode == MODE_OVERRIDE && target.mode != MODE_OVERRIDE {
                // handled below
            } else {
                return;
            }
        }

        let _ = self.db.set_display_state(&DisplayState {
            id: 1,
            current_page_id: target.page_id,
            current_url: Some(target.url.clone()),
            mode: target.mode.clone(),
            started_at: Some(now()),
            expires_at: target.expires_at.clone(),
        });
        {
            if let Ok(mut h) = self.history.lock() {
                h.push(&target.url);
            }
        }
        self.emit_navigate(&target.url, &target.mode, target.page_id);
    }

    /// Called by the frontend once a page has loaded successfully, to update
    /// the offline-first "last successful page" record.
    pub fn report_loaded(&self, url: &str) {
        let _ = self.db.record_successful_page(url);
        self.emit_status();
    }

    /// Build the current status snapshot.
    pub fn status(&self) -> Result<DisplayStatus, String> {
        let state = self.db.get_display_state()?;
        let (last_url, last_at) = self.db.last_successful();
        let (can_back, can_forward) = {
            let h = self.history.lock().map_err(|e| e.to_string())?;
            (h.can_back(), h.can_forward())
        };
        Ok(DisplayStatus {
            current_url: state.current_url,
            current_page_id: state.current_page_id,
            mode: state.mode,
            started_at: state.started_at,
            expires_at: state.expires_at,
            scheduler_running: self.is_scheduler_running(),
            can_back,
            can_forward,
            last_successful_url: last_url,
            last_successful_at: last_at,
        })
    }

    /// Resolve the URL the player should show at startup.
    pub fn initial_url(&self) -> Option<String> {
        let state = self.db.get_display_state().ok()?;
        if let Some(url) = state.current_url {
            if !url.is_empty() {
                return Some(url);
            }
        }
        let target = scheduler::compute_target(&self.db)?;
        Some(target.url)
    }
}