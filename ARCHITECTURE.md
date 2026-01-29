# PowerCost Tracker - Architecture Document

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


