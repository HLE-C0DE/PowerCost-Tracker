# PowerCost Tracker - Architecture Document

## Overview

PowerCost Tracker is a lightweight, cross-platform desktop application for monitoring PC power consumption and calculating electricity costs in real-time.

---

## Tech Stack Decision

### Framework: Tauri v2 (Rust + WebView)

**Why Tauri over alternatives:**

| Criteria | Tauri | Electron | Qt | GTK |
|----------|-------|----------|-----|-----|
| Binary Size | ~3-5 MB | 150+ MB | ~30 MB | ~20 MB |
| RAM Usage (idle) | 20-30 MB | 100-150 MB | 40-60 MB | 30-50 MB |
| CPU Usage (idle) | <0.5% | 1-3% | <1% | <1% |
| Cross-platform | Excellent | Excellent | Good | Moderate |
| System API Access | Native (Rust) | Limited | Good | Good |
| Development Speed | Fast | Fast | Moderate | Slow |
| Modern UI | Yes (Web tech) | Yes | Moderate | Limited |

**Decision**: Tauri is the clear winner for our requirements:
- **Ultra-lightweight**: Uses native OS webview (WebView2/WebKitGTK) instead of bundling Chromium
- **Rust backend**: Direct access to system APIs, memory safety, excellent performance
- **Small footprint**: Meets our <50MB RAM target easily
- **Modern stack**: Web frontend allows clean, responsive UI without heavy frameworks

### Backend: Rust

**Justification:**
- Zero-cost abstractions for maximum performance
- Direct access to system APIs (RAPL, sysfs, WMI)
- Memory safety without garbage collection pauses
- Excellent cross-compilation support
- Rich ecosystem for system programming

### Frontend: Vanilla JS + CSS (No Framework)

**Justification:**
- Zero JavaScript framework overhead
- Minimal bundle size (<50KB total)
- Fast initial load
- Easy i18n implementation
- No virtual DOM overhead

For reactive updates, we use a minimal reactive store pattern (~100 lines) instead of React/Vue/Solid.

### Database: SQLite

**Justification:**
- Zero configuration, embedded database
- Single file storage
- Excellent Rust support via `rusqlite`
- Efficient for time-series data (consumption history)
- ACID compliant

### Configuration: TOML

**Justification:**
- Human-readable and easy to edit manually
- Native Rust support via `toml` crate
- Perfect for configuration files
- Comments supported (unlike JSON)

---

## Power Monitoring Strategy

### Linux: RAPL (Running Average Power Limit)

**Primary Source**: `/sys/class/powercap/intel-rapl/`

```
/sys/class/powercap/
└── intel-rapl/
    └── intel-rapl:0/           # Package (CPU + integrated GPU)
        ├── energy_uj           # Energy counter in microjoules
        ├── max_energy_range_uj
        ├── name                # "package-0"
        └── intel-rapl:0:0/     # Core domain
            └── energy_uj
```

**How it works:**
1. Read `energy_uj` at time T1
2. Wait interval (e.g., 1 second)
3. Read `energy_uj` at time T2
4. Power (W) = (energy_T2 - energy_T1) / (T2 - T1) / 1,000,000

**Fallback sources:**
- AMD: `/sys/class/hwmon/hwmon*/power*_input` (k10temp driver)
- Battery: `/sys/class/power_supply/BAT*/power_now`
- ACPI: `/sys/class/power_supply/AC/power_now`

**Limitations:**
- RAPL requires root OR specific capabilities (`CAP_SYS_RAWIO`)
- Workaround: Create a setuid helper or use polkit
- Only measures CPU/iGPU, not peripherals

### Windows: LibreHardwareMonitor Library

**Primary approach**: Use `windows` crate to query WMI or integrate with LibreHardwareMonitor via:

1. **WMI Queries** (no external dependency):
   ```
   Win32_Processor -> CurrentVoltage, LoadPercentage
   Win32_Battery -> EstimatedChargeRemaining, Voltage
   ```

2. **Open Hardware Monitor shared memory** (if installed):
   - Read from `AIDA64` or `HWiNFO` shared memory interface

3. **Estimation mode**:
   - Query CPU/GPU specs and load percentage
   - Apply TDP-based estimation formulas

**Windows-specific challenges:**
- No direct power reading API (unlike Linux RAPL)
- Requires admin rights for accurate readings
- Often relies on 3rd party software

---

## Application Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Tauri Application                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 Frontend (WebView)                   │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐            │   │
│  │  │Dashboard │ │ Settings │ │  History │            │   │
│  │  └──────────┘ └──────────┘ └──────────┘            │   │
│  │  ┌──────────────────────────────────────┐          │   │
│  │  │         Reactive Store (i18n)        │          │   │
│  │  └──────────────────────────────────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │ IPC (Tauri Commands)            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Rust Backend                        │   │
│  │  ┌────────────┐  ┌─────────────┐  ┌────────────┐   │   │
│  │  │  Hardware  │  │   Pricing   │  │   Storage  │   │   │
│  │  │  Monitor   │  │   Engine    │  │   (SQLite) │   │   │
│  │  └────────────┘  └─────────────┘  └────────────┘   │   │
│  │  ┌────────────────────────────────────────────┐    │   │
│  │  │          System Tray / Widget              │    │   │
│  │  └────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │  RAPL    │   │   WMI    │   │  Config  │
        │ (Linux)  │   │(Windows) │   │  (TOML)  │
        └──────────┘   └──────────┘   └──────────┘
```

### Module Responsibilities

#### `core/` - Application Core
- `app.rs` - Main application state and lifecycle
- `config.rs` - Configuration management (TOML)
- `error.rs` - Error types and handling

#### `hardware/` - Power Monitoring
- `mod.rs` - Trait definition for power sources
- `linux.rs` - RAPL/sysfs implementation
- `windows.rs` - WMI/estimation implementation
- `estimator.rs` - TDP-based power estimation fallback

#### `pricing/` - Cost Calculation
- `mod.rs` - Pricing engine interface
- `simple.rs` - Single rate pricing
- `peak_offpeak.rs` - HP/HC (peak/off-peak hours)
- `seasonal.rs` - Summer/winter rates
- `tempo.rs` - Combined seasonal + HP/HC (EDF Tempo-like)

#### `i18n/` - Internationalization
- `mod.rs` - Translation system
- `en.rs` - English strings
- `fr.rs` - French strings

#### `db/` - Data Persistence
- `mod.rs` - Database interface
- `schema.rs` - SQLite schema
- `queries.rs` - Data access layer

---

## Data Models

### Configuration (TOML)

```toml
[general]
language = "auto"  # "auto", "en", "fr"
theme = "dark"     # "dark", "light", "system"
refresh_rate_ms = 1000
eco_mode = false
start_minimized = true
start_with_system = true

[pricing]
mode = "simple"  # "simple", "peak_offpeak", "seasonal", "tempo"
currency = "EUR"
currency_symbol = "€"

[pricing.simple]
rate_per_kwh = 0.2276

[pricing.peak_offpeak]
peak_rate = 0.27
offpeak_rate = 0.20
# Time ranges in 24h format
offpeak_start = "22:00"
offpeak_end = "06:00"

[pricing.seasonal]
summer_rate = 0.20
winter_rate = 0.25
# Months considered winter (November to March)
winter_months = [11, 12, 1, 2, 3]

[pricing.tempo]
# Blue days
blue_peak = 0.16
blue_offpeak = 0.13
# White days
white_peak = 0.19
white_offpeak = 0.15
# Red days
red_peak = 0.76
red_offpeak = 0.16

[widget]
enabled = true
show_cost = true  # false = show consumption only
position = "bottom_right"  # "top_left", "top_right", "bottom_left", "bottom_right"
opacity = 0.9

[advanced]
# Baseline for surplus tracking (Bonus 1)
baseline_watts = 0  # 0 = auto-detect
baseline_auto = true
# Hardware profiles (Bonus 2)
active_profile = "default"
```

### Database Schema

```sql
-- Power readings history
CREATE TABLE power_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,  -- Unix timestamp
    power_watts REAL NOT NULL,
    source TEXT NOT NULL,        -- "rapl", "wmi", "estimated"
    components TEXT              -- JSON: {"cpu": 45.2, "gpu": 120.0}
);

-- Daily aggregates (for fast historical queries)
CREATE TABLE daily_stats (
    date TEXT PRIMARY KEY,       -- YYYY-MM-DD
    total_wh REAL NOT NULL,
    total_cost REAL,
    avg_watts REAL,
    max_watts REAL,
    pricing_mode TEXT
);

-- Sessions (for surplus tracking)
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    baseline_watts REAL,
    total_wh REAL,
    surplus_wh REAL,
    surplus_cost REAL,
    label TEXT
);

-- Indexes for performance
CREATE INDEX idx_readings_timestamp ON power_readings(timestamp);
CREATE INDEX idx_daily_date ON daily_stats(date);
```

---

## Performance Considerations

### Memory Budget: <50 MB

| Component | Estimated RAM |
|-----------|---------------|
| Tauri core | 15 MB |
| WebView (idle) | 20 MB |
| SQLite | 2 MB |
| Rust heap | 5 MB |
| **Total** | **~42 MB** |

### CPU Budget: <1% idle

- Use event-driven architecture (no polling when window hidden)
- Configurable refresh rate (1s-60s)
- Eco mode: 30s refresh, minimal processing
- Background tray mode: Only updates on demand

### Optimization Strategies

1. **Lazy loading**: Load historical data only when viewing history tab
2. **Data compression**: Store only daily aggregates after 30 days
3. **Efficient queries**: Use prepared statements, proper indexes
4. **Minimal redraws**: Update only changed DOM elements

---

## Security Considerations

### Linux
- RAPL requires elevated permissions
- Solution: Use `pkexec` for initial setup or provide udev rules
- User data stored in `~/.config/powercost-tracker/`

### Windows
- WMI queries may require admin for some sensors
- Installer provides UAC elevation if needed
- User data stored in `%APPDATA%/PowerCost-Tracker/`

### General
- No network access required (fully offline)
- No telemetry or data collection
- All data stored locally

---

## Build & Distribution

### Build Commands

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build

# Cross-compilation (from Linux to Windows)
cargo tauri build --target x86_64-pc-windows-gnu
```

### Distribution Formats

| Platform | Format |
|----------|--------|
| Windows | `.msi` installer, `.exe` portable |
| Linux | `.deb`, `.AppImage`, `.rpm` |

### Estimated Bundle Sizes

| Platform | Size |
|----------|------|
| Windows (installer) | ~4 MB |
| Linux (AppImage) | ~5 MB |

---

## Future Considerations

### Bonus 1: Session Surplus Tracking
- Idle detection via input monitoring (mouse/keyboard)
- Automatic baseline calculation from idle periods
- Session labeling (e.g., "3D Render", "LLM Training")

### Bonus 2: Hardware Component Breakdown
- Parse RAPL subdomains for CPU/GPU split
- NVIDIA GPU: `nvidia-smi` power reading
- AMD GPU: ROCm SMI or hwmon

### Potential Enhancements
- Export to CSV/JSON
- System notifications for cost thresholds
- Multi-monitor widget support
- Power profile recommendations

---

## References

- [Tauri Documentation](https://tauri.app/v2/guides/)
- [Intel RAPL Documentation](https://www.kernel.org/doc/html/latest/power/powercap/powercap.html)
- [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor)
- [Rusqlite](https://github.com/rusqlite/rusqlite)
