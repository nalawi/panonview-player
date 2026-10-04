# PanonView Player — Digital Signage Agent

A cross-platform **digital signage / web page player agent** built with
Tauri 2, Rust, and React. It boots straight into a borderless, fullscreen
WebView showing a remote web page, and can be controlled remotely over an
embedded HTTP API on port `8787`.

Designed with device identity and offline-first operation from day one, so it
can later join a central management platform without depending on it.

---

## Features

- **Player mode (default):** boots fullscreen, chrome-less, straight to the
  configured URL. No address bar, no normal UI.
- **Embedded HTTP API (Axum) on `0.0.0.0:8787`** for remote control from a
  REST API, ERP, CMS, dashboard, or scheduler.
- **SQLite persistence** with versioned migrations (`user_version`) — pages,
  schedules, display state, commands, settings, and device identity survive
  upgrades.
- **Time-based + rotation-based scheduling.**
- **Priority overrides** (e.g. emergency dashboards) that temporarily
  supersede the scheduler and auto-resume when they expire.
- **Offline-first:** the scheduler keeps running and the last successful page
  is shown instead of a blank screen.
- **Remembers the last active page:** the current page/URL (and the
  back/forward history) is persisted in SQLite and in local storage, so after
  a stop, crash, or reboot the player boots straight back into the same
  screen.
- **Authentication:** API key / Bearer auth plus CIDR IP allowlisting. An API
  key is generated on first boot so the API is never exposed unauthenticated.
- **Hidden admin UI** (React) reachable at `#/admin`, bypassable so normal
  boots go straight to fullscreen.
- **Single source of truth:** the Rust `DisplayController` mediates every
  display change (HTTP API, scheduler, and admin UI all call it).

---

## Architecture

```
External HTTP ─► Rust Axum API ─► Display Controller ─┬─ SQLite
                                                       ├─ Scheduler
                                                       └─ WebView

React UI ─┬─ Display State
          ├─ Page Management
          └─ Schedule Management
                 │
                 ▼
          Rust Commands ─► SQLite
```

The `DisplayController` is the **single source of truth**. The HTTP API, the
scheduler tick, and the local admin UI all route through it; nothing
manipulates the WebView independently.

---

## Tech stack

| Layer    | Tech                                                        |
| -------- | ----------------------------------------------------------- |
| Frontend | React, TypeScript, Vite, TailwindCSS, Zustand, TanStack Query, React Router |
| Backend  | Tauri 2, Rust, Axum, rusqlite (bundled SQLite), tokio, chrono |
| Display  | Tauri WebView (iframe)                                      |

> The control API uses a real embedded **Axum** server, **not** Tauri's
> `localhost` plugin (which is for serving app assets).

---

## Project layout

```
apps/player/
├── src/                        # React admin UI + player
│   ├── components/             # PlayerView, AdminLayout
│   ├── pages/                  # Dashboard, Pages, Schedules, Settings
│   ├── stores/                 # Zustand display store
│   ├── services/tauri.ts       # Typed IPC wrappers
│   ├── hooks/                  # display event listeners
│   └── types/                  # Shared TS types
└── src-tauri/
    └── src/
        ├── lib.rs              # App setup: DB, controller, HTTP server, scheduler
        ├── main.rs
        ├── commands.rs         # Tauri IPC commands
        ├── models/mod.rs       # Serde models
        ├── database/mod.rs     # SQLite + migrations
        ├── display/
        │   ├── controller.rs   # DisplayController (source of truth)
        │   └── scheduler.rs    # Time + rotation scheduling
        └── api/
            ├── routes.rs       # Axum router
            ├── auth.rs         # API key + CIDR allowlist middleware
            ├── display.rs      # /display endpoints
            ├── pages.rs        # /pages CRUD
            ├── schedules.rs    # /schedules CRUD
            ├── settings.rs     # /settings
            └── system.rs       # /status, /device, /commands, restart, etc.
```

---

## Getting started

### Prerequisites

- Node.js 18+ and npm
- Rust (stable) and Cargo
- Tauri 2 system dependencies ([see Tauri docs](https://v2.tauri.app/start/prerequisites/))

### Install & run (development)

```bash
cd apps/player
npm install
npm run tauri dev
```

The app boots fullscreen showing the seeded "Welcome" page. The embedded HTTP
server listens on `0.0.0.0:8787`.

### Open the admin UI

There are **two ways** to reach the administration console, and both talk to
the same `DisplayController`:

**1. In-app (desktop window)** — hover the top-right corner of the player and
click **admin** (or navigate to `#/admin`). Uses Tauri IPC; no login needed.

**2. From any browser on the LAN — the Web Console** — open:

```
http://<device-ip>:8787/
```

This redirects to `http://<device-ip>:8787/ui/`, a full administration console
served by the embedded HTTP server. It is the recommended way to manage a
headless/unattended player.

- **From any browser, including the same machine:** a login screen asks for
  the API key. Find it in the desktop app under **Settings → API Key** (copy
  it), or read it locally from the database (see below). The key is verified
  against the server on every request and is never handed out by the API.

To read the key from the local database:

```bash
sqlite3 "<app-data-dir>/com.baina.webplayer/web_player.db" \
  "SELECT value FROM settings WHERE key='api_key';"
```

> The web console is served only when a built UI bundle is present. In
> development run `npm run build` first (or use `npm run tauri dev`, which
> builds the frontend automatically). In production the bundle is packaged as
> a resource.

### Build a release

```bash
cd apps/player
npm run tauri build
```

---

## Embedded HTTP API

Base URL: `http://<device-ip>:8787`

Authentication (required for non-loopback clients):

```http
X-API-Key: <key>
```
or
```http
Authorization: Bearer <key>
```

The API key is generated on first boot and shown in **Settings → API Key**.

### Display

```http
GET  /api/v1/display
POST /api/v1/display                 { "url": "https://example.com" }
POST /api/v1/display/override        { "url": "...", "duration": 300, "priority": 100 }
POST /api/v1/display/refresh
POST /api/v1/display/back
POST /api/v1/display/forward
```

### Pages

```http
GET    /api/v1/pages
GET    /api/v1/pages/:id
POST   /api/v1/pages
PUT    /api/v1/pages/:id
DELETE /api/v1/pages/:id
```

### Schedules

```http
GET    /api/v1/schedules
GET    /api/v1/schedules/:id
POST   /api/v1/schedules
PUT    /api/v1/schedules/:id
DELETE /api/v1/schedules/:id
```

Time-based schedule example:

```json
{
  "page_id": 3,
  "schedule_type": "TIME",
  "days": ["MON", "TUE", "WED", "THU", "FRI"],
  "start_time": "08:00",
  "end_time": "12:00",
  "priority": 10
}
```

Rotation schedule example:

```json
{
  "page_id": 4,
  "schedule_type": "ROTATION",
  "sequence": 1
}
```

### Settings / Device / System

```http
GET  /api/v1/settings
PUT  /api/v1/settings            { "settings": { "http_port": "8787" } }
GET  /api/v1/device
PUT  /api/v1/device              { "name": "TV-001", "location": "Lobby", "group_name": "Lobby" }
GET  /api/v1/status
GET  /api/v1/commands
POST /api/v1/restart
POST /api/v1/shutdown
POST /api/v1/reload
GET  /health                     (unauthenticated liveness probe)
```

`GET /api/v1/status` returns:

```json
{
  "success": true,
  "device_id": "…",
  "device_name": "…",
  "hostname": "DISPLAY-001",
  "platform": "macos",
  "version": "1.0.0",
  "current_url": "https://dashboard.company.com",
  "mode": "scheduler",
  "online": true
}
```

---

## Scheduling precedence

```
1. Active override (highest priority, until it expires)
2. Active TIME schedule (highest `priority` wins)
3. ROTATION schedules (cycled by each page's duration)
4. Last successful page (offline fallback)
5. First enabled page
```

---

## Security

- API key / Bearer authentication (auto-generated on first boot) — enforced
  for **all** HTTP clients, loopback included.
- CIDR-based IP allowlisting (`192.168.1.0/24, 10.10.0.0/16`).
- `Allow Remote Control` toggle (applies to non-loopback clients).
- Secrets are masked in API/UI responses; the key is never exposed via the
  public `/api/v1/auth` probe.

Configure under **Settings → HTTP Server**.

---

## Testing

Backend unit tests cover scheduling precedence, overrides, rotation, and the
auth allowlist logic:

```bash
cd apps/player/src-tauri
cargo test
```

Frontend type-check and build:

```bash
cd apps/player
npx tsc --noEmit
npm run build
```

---

## Platform notes

The Rust core, scheduler, controller, and HTTP API are shared across all
platforms. Only startup/window behavior differs:

- **Windows / Linux / macOS:** native fullscreen, always-on-top, borderless.
- **Android / Android TV:** the same codebase builds; treat Android TV as a
  device-tested target for WebView behavior, immersive fullscreen, D-pad focus,
  autostart, and kiosk mode.

To build for Android, add the Tauri Android target and run
`npm run tauri android build`.

---

## License

This project is licensed under the MIT License.

```
MIT License

Copyright (c) 2026 baina

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
