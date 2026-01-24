# PowerCost Tracker

<p align="center">
  <img src="assets/logo.svg" alt="PowerCost Tracker Logo" width="120">
</p>

<p align="center">
  <strong>A lightweight desktop application for monitoring PC power consumption and calculating electricity costs in real-time.</strong>
</p>

<p align="center">
  <a href="README.fr.md">Francais</a> | English
</p>

---

## Features

### Real-time Monitoring
- **Instant power reading** (Watts) from hardware sensors
- **Live power graph** showing consumption history
- **Cumulative energy** tracking since session start
- **Multiple time period views** (session, day, week, month)

### Flexible Pricing Configuration
- **Simple mode**: Single flat rate per kWh
- **Peak/Off-peak mode**: Different rates by time of day (HP/HC)
- **Seasonal mode**: Summer/winter rate differentiation
- **Tempo mode**: EDF-style pricing with day colors (blue/white/red)
- **Multi-currency support**: EUR, USD, GBP, CHF, and more

### Cost Estimation
- Real-time cost calculation
- Hourly, daily, and monthly projections
- Works without pricing config (consumption-only mode)

### Minimal Footprint
- **< 50 MB RAM** usage
- **< 1% CPU** when idle
- **~5 MB** application size
- No Electron - uses native OS webview

---

## Installation

### Pre-built Binaries

Download the latest release for your platform:

| Platform | Format |
|----------|--------|
| Windows | `.msi` installer or `.exe` portable |
| Linux | `.deb`, `.rpm`, or `.AppImage` |

### Building from Source

#### Prerequisites

- **Rust** 1.70 or later
- **Node.js** 18 or later
- **System dependencies**:
  - Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`
  - Windows: WebView2 Runtime (included in Windows 11)

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/HLE-C0DE/PowerCost-Tracker.git
cd PowerCost-Tracker

# Install frontend dependencies
cd ui && npm install && cd ..

# Build the application
cargo tauri build
```

The built application will be in `src-tauri/target/release/`.

---

## Usage

### Power Monitoring

The application automatically detects available power monitoring sources:

| Platform | Source | Accuracy |
|----------|--------|----------|
| Linux | Intel RAPL | High (actual measurement) |
| Linux | AMD hwmon | High (actual measurement) |
| Linux | Battery sensor | Medium (for laptops) |
| Windows | WMI + estimation | Low (based on CPU load) |

If no hardware sensor is available, the app falls back to estimation mode based on CPU load.

### Configuring Pricing

1. Open **Settings** from the sidebar
2. Select your **Pricing Mode**:
   - **Simple**: Enter your rate per kWh
   - **Peak/Off-peak**: Set peak rate, off-peak rate, and time windows
3. Choose your **Currency**
4. Click **Save**

### Reading the Dashboard

- **Current Power**: Instantaneous power draw in Watts
- **Session Energy**: Total energy consumed since app launch
- **Session Cost**: Running cost for the current session
- **Estimates**: Projected costs at current consumption rate

---

## Configuration

Configuration is stored in:
- **Linux**: `~/.config/powercost-tracker/config.toml`
- **Windows**: `%APPDATA%/PowerCost-Tracker/config.toml`

### Example Configuration

```toml
[general]
language = "auto"        # "auto", "en", "fr"
theme = "dark"           # "dark", "light", "system"
refresh_rate_ms = 1000   # Update interval (1000-60000)
eco_mode = false         # Reduce refresh when minimized
start_minimized = false
start_with_system = false

[pricing]
mode = "simple"          # "simple", "peak_offpeak", "seasonal", "tempo"
currency = "EUR"
currency_symbol = "\u20AC"

[pricing.simple]
rate_per_kwh = 0.2276

[pricing.peak_offpeak]
peak_rate = 0.27
offpeak_rate = 0.20
offpeak_start = "22:00"
offpeak_end = "06:00"

[widget]
enabled = true
show_cost = true
position = "bottom_right"
opacity = 0.9
```

---

## Linux Permissions

To read Intel RAPL data on Linux, the application needs access to `/sys/class/powercap/`. Options:

### Option 1: Run with elevated privileges (not recommended)
```bash
sudo powercost-tracker
```

### Option 2: Add udev rule (recommended)
```bash
# Create udev rule
echo 'SUBSYSTEM=="powercap", ACTION=="add", RUN+="/bin/chmod -R a+r /sys/class/powercap/"' | \
  sudo tee /etc/udev/rules.d/99-powercap.rules

# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Option 3: Grant capability
```bash
sudo setcap cap_sys_rawio+ep /path/to/powercost-tracker
```

---

## Troubleshooting

### "Estimated" badge showing

This means no hardware power sensor was detected. Possible causes:
- **Linux**: RAPL not available or permission denied
- **Windows**: Normal behavior (direct sensors not available)
- **Virtual machine**: Power sensors not exposed

### Values seem incorrect

Power estimation is based on CPU load and typical TDP values. For accurate readings:
- Use hardware with RAPL support (Intel/AMD)
- On Linux, ensure proper permissions (see above)
- Consider using external power meters for validation

---

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed technical documentation.

### Tech Stack

- **Backend**: Rust + Tauri v2
- **Frontend**: Vanilla JS + CSS (no framework)
- **Database**: SQLite (for history)
- **Config**: TOML

---

## Roadmap

### v0.1 (Current)
- [x] Real-time power monitoring
- [x] Multiple pricing modes
- [x] Bilingual UI (EN/FR)
- [x] Dark/light themes

### v0.2 (Planned)
- [ ] System tray widget
- [ ] Session surplus tracking
- [ ] Export to CSV

### v0.3 (Planned)
- [ ] Hardware component breakdown
- [ ] Multiple profiles
- [ ] Notification alerts

---

## Contributing

Contributions are welcome! Please read the contributing guidelines before submitting PRs.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- [Tauri](https://tauri.app/) - For the amazing framework
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings for Rust
- Intel RAPL documentation for power monitoring insights
