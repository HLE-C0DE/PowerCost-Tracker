# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PowerCost Tracker is a lightweight cross-platform desktop application (Windows/Linux) for monitoring PC power consumption in real-time and calculating electricity costs based on customizable pricing plans.

**Tech Stack**: Rust + Tauri v2 backend, Vanilla JS/CSS frontend, SQLite database, TOML configuration

## Build Commands

```bash
# Development (Tauri + hot reload)
cargo tauri dev

# Build Rust backend only
cd src-tauri && cargo build

# Production build (creates installers)
cargo tauri build

# Run tests
cd src-tauri && cargo test

# Run CLI demo (no UI)
cd src-tauri && cargo run --bin powercost-demo

# Install frontend dependencies (required once)
cd ui && npm install
```

## Architecture

### Backend Modules (`src-tauri/src/`)

- **`main.rs`**: Tauri entry point, defines `TauriState` (shared app state), IPC commands, and the background `monitoring_loop` that periodically reads power and emits events
- **`core/`**: Configuration (`Config` struct, TOML load/save), error types, shared types (`PowerReading`, `DashboardData`, `AppState`, `SystemMetrics`, `Session`)
- **`hardware/`**: Power monitoring abstraction
  - `PowerSource` trait for platform-specific implementations
  - `linux.rs`: RAPL (`/sys/class/powercap`), hwmon, battery sources
  - `windows.rs`: WMI queries + GPU support (nvidia-smi, rocm-smi), system metrics (CPU/GPU/RAM), top processes
  - `estimator.rs`: TDP-based fallback with CPU detection
  - `baseline.rs`: Baseline power detection for surplus tracking
- **`pricing/`**: Cost calculation engine supporting 4 modes: simple (flat rate), peak/offpeak (HP/HC), seasonal, and tempo (EDF-style)
- **`db/`**: SQLite persistence with tables `power_readings`, `daily_stats`, `sessions`
- **`i18n/`**: Bilingual support (FR/EN) with translation strings in `en.rs`/`fr.rs`

### Frontend (`ui/`)

- **`index.html`**: Main app with Dashboard, History, Settings views
- **`src/main.js`**: Vanilla JS with reactive store pattern, communicates with backend via Tauri IPC (`invoke()`)
- **`src/styles/main.css`**: Dark/light theme support

### Data Flow

1. `monitoring_loop` (async background task) reads power via `PowerMonitor` at configurable intervals
2. Updates `AppState` with cumulative energy and cost
3. Emits `power-update` event to frontend
4. Stores readings in SQLite every 10 cycles

### Tauri Commands (IPC API)

| Command | Returns | Purpose |
|---------|---------|---------|
| `get_dashboard_data()` | `DashboardData` | All dashboard metrics in one call |
| `get_power_watts()` | `f64` | Instantaneous power |
| `get_config()` / `set_config()` | `Config` | Read/write TOML config |
| `get_translations()` | `HashMap` | All i18n strings |
| `get_history()` / `get_readings()` | Stats/Records | Historical data |
| `toggle_widget()` | `bool` | Show/hide floating widget |
| `get_system_metrics()` | `SystemMetrics` | CPU, GPU, RAM metrics |
| `get_top_processes()` | `Vec<ProcessMetrics>` | Top N processes by CPU |
| `start_tracking_session()` | `i64` | Start a surplus tracking session |
| `end_tracking_session()` | `Session` | End session with stats |
| `get_session_stats()` | `Session` | Current session data |
| `detect_baseline()` | `BaselineDetection` | Auto-detect idle power |
| `get_dashboard_config()` / `save_dashboard_config()` | `DashboardConfig` | Widget layout config |

## Power Monitoring Sources

| Platform | Source | Location | Accuracy |
|----------|--------|----------|----------|
| Linux | Intel RAPL | `/sys/class/powercap/intel-rapl` | High |
| Linux | hwmon | `/sys/class/hwmon/*/power*_input` | High |
| Linux | Battery | `/sys/class/power_supply/BAT*/power_now` | Medium |
| Windows | WMI + sysinfo | COM/WMI APIs | Medium (hybrid: real GPU + estimated CPU) |
| Windows | NVIDIA GPU | `nvidia-smi` | High |
| Windows | AMD GPU | `rocm-smi` / `amd-smi` | High |
| Fallback | TDP estimation | CPU detection via sysinfo | Low |

## Configuration

Files stored at:
- Linux: `~/.config/powercost-tracker/config.toml`
- Windows: `%APPDATA%/PowerCost-Tracker/config.toml`

## Development Notes

### Performance Targets
- < 50 MB RAM (target: 30-40 MB)
- < 1% CPU idle
- No heavy JS frameworks

### Linux Permissions
RAPL requires elevated access. Solutions:
- `sudo setcap cap_sys_rawio+ep /path/to/binary`
- udev rule for `/sys/class/powercap/`

### Windows Limitations
No direct CPU power sensor API; uses WMI-based CPU estimation combined with real GPU readings (NVML/rocm-smi) when available. With a supported GPU, readings are hybrid (real GPU + estimated CPU) rather than pure estimation.

## Project Phase Status

| Phase | Status |
|-------|--------|
| Phase 1: Setup & Architecture | Complete |
| Phase 2: Core Engine | Complete |
| Phase 3: User Interface | Complete |
| Phase 4: Cross-platform packaging | Pending |
| Phase 5: Bonus features | Complete |

## New Features (Major Improvements)

### Enhanced Hardware Monitoring
- Real-time CPU metrics (usage, frequency, temperature via WMI)
- GPU metrics (usage, power, temp, VRAM) for NVIDIA and AMD
- RAM usage monitoring
- Top 10 processes by CPU usage

### Baseline/Surplus Tracking
- Auto-detect idle power baseline (5th percentile method)
- Manual baseline setting
- Session-based surplus calculation
- Surplus energy and cost tracking

### Customizable Dashboard
- Drag-and-drop widget reordering
- Widget visibility toggles
- Resizable widgets (small/medium/large)
- Persistent layout configuration

### Customizable Floating Widget
- Configurable display items (power, cost, CPU, GPU, RAM, temp)
- Multiple size options (compact/normal/large)
- Theme selection (default/minimal/detailed)
- Opacity control
