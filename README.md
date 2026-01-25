# PowerCost Tracker

<p align="center">
  <img width="150" height="150" alt="icon-dark" src="https://github.com/user-attachments/assets/3258ac20-9465-4c8a-8388-2a192af458aa" />
</p>

<h3 align="center">Real-Time PC Power Consumption Monitor & Electricity Cost Calculator</h3>

<p align="center">
  Track your PC's energy usage, calculate electricity costs, and monitor hardware performance.<br>
  Perfect for tracking costs of <b>gaming sessions</b>, <b>local LLM inference</b>, <b>AI training</b>, or <b>crypto mining</b>.
</p>

<p align="center">
  <a href="#installation">Installation</a> •
  <a href="#features">Features</a> •
  <a href="#usage">Usage</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#faq">FAQ</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-1.0.0-blue.svg" alt="Version 1.0.0">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="MIT License">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey.svg" alt="Platform">
  <img src="https://img.shields.io/badge/RAM-<50MB-success.svg" alt="RAM Usage">
  <img src="https://img.shields.io/badge/CPU-<1%25-success.svg" alt="CPU Usage">
</p>

---

## Why PowerCost Tracker?

Running a local LLM like LLaMA or Mistral? Playing games for hours? Training AI models? **Know exactly what it costs.**

PowerCost Tracker monitors your PC's power consumption in real-time and calculates electricity costs based on your actual energy rates. No more guessing - see the real cost of your computing activities.

### Use Cases

- **Local LLM & AI Work** - Track power costs when running local language models, Stable Diffusion, or training neural networks
- **Gaming Sessions** - See how much your gaming sessions really cost in electricity
- **Crypto Mining** - Monitor power efficiency and profitability
- **Work-from-Home** - Calculate actual PC electricity costs for expense reports
- **Energy Optimization** - Identify power-hungry applications and optimize your setup

---

## Features

### Real-Time Power Monitoring

<img width="1400" height="777" alt="image" src="https://github.com/user-attachments/assets/2d756f75-2238-4c64-8178-a2225c4e40da" />

- **Live power reading** in Watts from hardware sensors (Intel RAPL, AMD hwmon) or smart estimation
- **Interactive power graph** with consumption history
- **CPU, GPU, and RAM metrics** - usage, temperature, frequency
- **Top processes by power consumption** - see what's using the most energy
- **Session, daily, weekly, and monthly tracking**


### Lightweight & Native

- **< 50 MB RAM** typical usage
- **< 1% CPU** when idle
- **~5 MB** application size
- **No Electron bloat** - uses native OS webview via Tauri
- **No external dependencies** - single portable executable option

<img width="660" height="374" alt="image" src="https://github.com/user-attachments/assets/5ce8969f-efd0-4102-9087-62954c1d5ab8" />

PowerCost-Tracker vs Windows TaskManager vs NZXT CAM (software I used to track my CPU/GPU usage with dignity) 


### Flexible Pricing Modes

Configure your actual electricity rates:

| Mode | Description |
|------|-------------|
| **Simple** | Single flat rate per kWh |
| **Peak/Off-Peak** | Different rates by time of day (HP/HC) |
| **Seasonal** | Summer vs winter rate differentiation |
| **Tempo** | EDF-style pricing with day colors (Blue/White/Red) |

Supports EUR, USD, GBP, CHF, CAD, and more currencies.

### Surplus Tracking (Session Mode)

Track costs for specific activities:

1. Set your baseline (idle) power consumption
2. Start a tracking session when gaming or running LLMs
3. See only the **surplus cost** - the extra electricity your activity is consuming
4. Perfect for calculating actual costs of compute-intensive tasks

### Hardware Monitoring Dashboard

- **CPU**: Usage %, frequency, temperature
- **GPU**: Usage %, power draw, VRAM, temperature (NVIDIA & AMD)
- **RAM**: Usage percentage and amount
- **Customizable widgets**: Drag-and-drop layout, resize, show/hide

### Floating Widget

A compact overlay that stays visible while you work or game:

- Configurable data display (power, cost, CPU, GPU, temps)
- Multiple sizes and themes
- Adjustable opacity
- Stays on top of other windows


### Bilingual Interface

Full support for **English** and **French** (Francais).

---

## Installation

### Download Pre-Built Binaries

Download the latest release for your platform:

| Platform | Format | Notes |
|----------|--------|-------|
| **Windows 10/11** | `.msi` installer | Recommended |
| **Windows 10/11** | `.exe` portable | No installation required |
| **Linux** | `.AppImage` | Universal, runs on most distros |
| **Linux** | `.deb` | Debian, Ubuntu, Mint |
| **Linux** | `.rpm` | Fedora, RHEL, CentOS |

> **Windows**: WebView2 Runtime is required (included in Windows 11, auto-installed on Windows 10)

### Build from Source

#### Prerequisites

- **Rust** 1.70 or later ([install](https://rustup.rs/))
- **Node.js** 18 or later ([install](https://nodejs.org/))
- **Platform dependencies**:
  - **Linux**: `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev`
  - **Windows**: WebView2 Runtime

#### Build Commands

```bash
# Clone the repository
git clone https://github.com/HLE-C0DE/PowerCost-Tracker.git
cd PowerCost-Tracker

# Install frontend dependencies
cd ui && npm install && cd ..

# Development mode (with hot reload)
cargo tauri dev

# Production build (creates installers in src-tauri/target/release/bundle/)
cargo tauri build
```

---

## Usage

### Quick Start

1. **Launch** PowerCost Tracker
2. **View** your current power consumption on the dashboard
3. **Configure** your electricity rate in Settings (optional but recommended)
4. **Track** your costs over time

### Power Sources

The application auto-detects the best available power source:

| Platform | Source | Accuracy | Notes |
|----------|--------|----------|-------|
| Linux | Intel RAPL | High | Requires permissions (see below) |
| Linux | AMD hwmon | High | Native support |
| Linux | Battery | Medium | For laptops |
| Windows | WMI + CPU estimation | Medium | Combined with GPU power |
| Windows | NVIDIA GPU | High | Via nvidia-smi |
| Windows | AMD GPU | High | Via rocm-smi / amd-smi |
| All | TDP estimation | Low | Fallback when no sensors |

### Linux Permissions (RAPL)

Intel RAPL requires read access to `/sys/class/powercap/`. Choose one:

**Option A: udev rule (Recommended)**
```bash
echo 'SUBSYSTEM=="powercap", ACTION=="add", RUN+="/bin/chmod -R a+r /sys/class/powercap/"' | \
  sudo tee /etc/udev/rules.d/99-powercap.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

**Option B: Capability**
```bash
sudo setcap cap_sys_rawio+ep /path/to/powercost-tracker
```

---

## Configuration

Configuration file location:
- **Linux**: `~/.config/powercost-tracker/config.toml`
- **Windows**: `%APPDATA%/PowerCost-Tracker/config.toml`

### Example Configuration

```toml
[general]
language = "auto"        # "auto", "en", "fr"
theme = "dark"           # "dark", "light", "system"
refresh_rate_ms = 1000   # Update interval in ms
start_minimized = false
start_with_system = false

[pricing]
mode = "simple"          # "simple", "peak_offpeak", "seasonal", "tempo"
currency = "EUR"
currency_symbol = "€"

[pricing.simple]
rate_per_kwh = 0.2276    # Your rate in currency/kWh

[pricing.peak_offpeak]
peak_rate = 0.27
offpeak_rate = 0.20
offpeak_start = "22:00"
offpeak_end = "06:00"

[widget]
enabled = true
position = "bottom_right"
opacity = 0.9

[baseline]
power_watts = 45.0       # Your PC's idle power for surplus tracking
```

See `config/example.config.toml` for all options including seasonal and tempo pricing.

---

## FAQ

### Why does it show "Estimated" on Windows?

Windows doesn't provide direct access to power sensors like Linux RAPL. The app uses CPU load estimation combined with actual GPU power (if NVIDIA/AMD GPU detected). This is normal and still provides useful relative measurements.

### How accurate is the power reading?

- **Linux with RAPL**: Very accurate (actual hardware measurement)
- **Windows with GPU**: Good (GPU power is real, CPU is estimated)
- **Pure estimation**: Approximate (based on TDP and load)

For precise measurements, use a hardware power meter (like Kill-A-Watt) to calibrate.

### How do I track my gaming session costs?

1. Go to **Settings** > **Baseline & Surplus**
2. Click **Detect Baseline** when your PC is idle
3. Start your game
4. Click **Start Session** on the dashboard
5. Play your game
6. Click **End Session** to see the surplus cost

### What pricing mode should I use?

- **Simple**: Fixed rate per kWh (most common)
- **Peak/Off-Peak**: If your provider charges more during peak hours
- **Seasonal**: If rates differ between summer and winter
- **Tempo**: For French EDF Tempo subscribers with colored day pricing

### Does it work with laptops?

Yes! On Linux, it can read battery discharge rate for power measurement. On Windows, estimation mode works on all hardware.

### How much system resources does it use?

- **RAM**: 30-50 MB typical
- **CPU**: < 1% when idle, brief spikes during reads
- **Disk**: SQLite database grows slowly with history

---

## Technical Details

### Architecture

- **Backend**: Rust + Tauri v2 (native performance, small footprint)
- **Frontend**: Vanilla JavaScript + CSS (no heavy frameworks)
- **Database**: SQLite for historical data
- **Configuration**: TOML format

See [ARCHITECTURE.md](ARCHITECTURE.md) for in-depth technical documentation.

### Data Storage

| Table | Purpose |
|-------|---------|
| `power_readings` | Time-series power data |
| `daily_stats` | Aggregated daily statistics |
| `sessions` | Surplus tracking session history |

---

## Contributing

Contributions are welcome!

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

```bash
# Run in development mode with hot reload
cargo tauri dev

# Run tests
cd src-tauri && cargo test

# Run CLI demo (no UI)
cd src-tauri && cargo run --bin powercost-demo
```

---

## Roadmap

### v1.0.0 (Current Release)

- Real-time power monitoring (RAPL, WMI, GPU)
- 4 flexible pricing modes
- Hardware monitoring (CPU, GPU, RAM, temperatures)
- Surplus/session tracking for activity cost calculation
- Customizable dashboard with drag-and-drop widgets
- Floating overlay widget
- Bilingual UI (EN/FR)
- Dark/Light themes
- System tray integration

### Future Plans

- CSV/JSON data export
- Power consumption alerts and notifications
- Multiple hardware profiles
- Component-level power breakdown
- macOS support

---

## License

MIT License - see [LICENSE](LICENSE) for details.

Free to use, modify, and distribute.

---

## Acknowledgments

- [Tauri](https://tauri.app/) - Lightweight native app framework
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite for Rust
- Intel RAPL and AMD hwmon documentation
- The open-source community

---

<p align="center">
  <b>Know your PC's power cost. Track your energy. Save money.</b>
</p>

<p align="center">
  <a href="https://github.com/HLE-C0DE/PowerCost-Tracker/issues">Report Bug</a> •
  <a href="https://github.com/HLE-C0DE/PowerCost-Tracker/issues">Request Feature</a>
</p>
