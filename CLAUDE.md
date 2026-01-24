# CLAUDE.md - Contexte du projet PowerCost Tracker

## Apercu du projet

PowerCost Tracker est une application desktop cross-platform (Windows/Linux) pour mesurer la consommation electrique du PC en temps reel et calculer le cout selon des tarifs personnalises.

**Stack technique**:
- **Backend**: Rust + Tauri v2
- **Frontend**: Vanilla JS/CSS (pas de framework)
- **Database**: SQLite (rusqlite)
- **Config**: TOML

## Structure du projet

```
PowerCost-Tracker/
├── src-tauri/                  # Backend Rust
│   ├── src/
│   │   ├── main.rs             # Point d'entree Tauri, commandes IPC
│   │   ├── lib.rs              # Export des modules
│   │   ├── core/               # Types, config, erreurs
│   │   │   ├── mod.rs
│   │   │   ├── config.rs       # Gestion config TOML
│   │   │   ├── error.rs        # Types d'erreurs
│   │   │   └── types.rs        # PowerReading, DashboardData, AppState
│   │   ├── hardware/           # Monitoring puissance
│   │   │   ├── mod.rs          # Trait PowerSource, PowerMonitor
│   │   │   ├── linux.rs        # RAPL, hwmon, battery
│   │   │   ├── windows.rs      # WMI (estimation)
│   │   │   └── estimator.rs    # Fallback estimation TDP
│   │   ├── pricing/            # Calcul des couts
│   │   │   └── mod.rs          # PricingEngine (simple, HP/HC, seasonal, tempo)
│   │   ├── db/                 # Persistence SQLite
│   │   │   └── mod.rs          # Database, DailyStats, PowerReadingRecord
│   │   └── i18n/               # Internationalisation
│   │       ├── mod.rs
│   │       ├── en.rs
│   │       └── fr.rs
│   ├── Cargo.toml              # Dependances Rust
│   ├── tauri.conf.json
│   └── build.rs
├── ui/                         # Frontend
│   ├── src/
│   │   ├── main.js
│   │   └── styles/main.css
│   ├── index.html
│   └── package.json
├── config/
│   └── example.config.toml
├── scripts/                    # Scripts de build/setup
├── assets/                     # Icones, images
├── ARCHITECTURE.md             # Documentation technique detaillee
├── TODO.md                     # Suivi des phases
├── README.md / README.fr.md
└── LICENSE
```

## Dependances principales (Cargo.toml)

- `tauri` v2 avec `tray-icon`
- `rusqlite` v0.31 (bundled)
- `serde`, `serde_json`, `toml`
- `chrono` pour les timestamps
- `tokio` pour l'async (monitoring loop)
- `sysinfo` pour detection CPU/load
- `windows` crate (Windows uniquement) pour WMI

## Commandes Tauri exposees au frontend

- `get_power_watts()` - Puissance instantanee
- `get_power_reading()` - Lecture complete avec metadonnees
- `get_energy_wh()` - Energie cumulee session
- `get_current_cost()` - Cout session
- `get_dashboard_data()` - Toutes les donnees dashboard
- `get_config()` / `set_config()` - Configuration
- `translate()` / `get_translations()` - i18n
- `get_history()` / `get_readings()` - Historique

## Sources de monitoring puissance

| OS | Source | Fichier | Precision |
|----|--------|---------|-----------|
| Linux | Intel RAPL | `/sys/class/powercap/intel-rapl` | Haute |
| Linux | hwmon | `/sys/class/hwmon/*/power*_input` | Haute |
| Linux | Battery | `/sys/class/power_supply/BAT*/power_now` | Moyenne |
| Windows | WMI + sysinfo | N/A | Basse (estimation) |
| Fallback | TDP estimation | N/A | Basse |

## Progression des phases

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1 | COMPLETE | Setup & Architecture |
| Phase 2 | COMPLETE | Core Engine |
| Phase 3 | En attente | Interface utilisateur |
| Phase 4 | En attente | Cross-platform |
| Phase 5 | En attente | Bonus features |

## Phase 2: Core Engine (COMPLETE)

### Objectifs
1. **Recuperation consommation** - Priorite Windows (WMI/estimation)
2. **Systeme de calcul de cout** - Deja implemente dans `pricing/mod.rs`
3. **Persistence SQLite** - Schema en place dans `db/mod.rs`
4. **CHECKPOINT**: Demo CLI fonctionnelle

### Etat actuel (Phase 2 Complete)

**Windows (`windows.rs`)** - COMPLETE:
- WmiMonitor avec vrais appels WMI (COM + IWbemLocator)
- Queries Win32_Battery (laptops) et Win32_Processor
- Support GPU NVIDIA via `nvidia-smi --query-gpu=power.draw`
- Support GPU AMD via `rocm-smi` ou `amd-smi`
- Fallback `sysinfo` si WMI echoue
- Combinaison CPU + GPU pour puissance totale

**Linux (`linux.rs`)** - COMPLETE:
- RaplMonitor: Lit `/sys/class/powercap/intel-rapl` avec gestion wraparound
- HwmonMonitor: Lit `/sys/class/hwmon/*/power*_input`
- BatteryMonitor: Lit `/sys/class/power_supply/BAT*`

**Estimator (`estimator.rs`)** - COMPLETE:
- Detection automatique CPU via sysinfo (nom, coeurs)
- Table TDP complete: Intel (desktop/laptop), AMD (desktop/laptop), Apple Silicon
- Formule power = idle + load_factor * (max - idle)
- Prise en compte des coeurs actifs

**Pricing (`pricing/mod.rs`)** - COMPLETE:
- Simple (flat rate) avec tests
- Peak/Offpeak (HP/HC) avec gestion overnight
- Seasonal (ete/hiver)
- Tempo (EDF) avec couleurs de jour

**Database (`db/mod.rs`)** - COMPLETE:
- Schema: `power_readings`, `daily_stats`, `sessions`
- CRUD operations implementees
- Tests unitaires (2 tests passes)

**Demo CLI (`bin/demo.rs`)** - COMPLETE:
- `cargo run --bin powercost-demo`
- Affiche puissance, energie, cout en temps reel
- Persiste dans SQLite

## Commandes de build

```bash
# Dev
cd src-tauri && cargo build

# Dev avec Tauri
cargo tauri dev

# Production
cargo tauri build

# Tests
cd src-tauri && cargo test
```

## Notes importantes

### Ce qu'on evite
- Frameworks JS lourds
- Dependencies inutiles
- App qui consomme plus qu'elle mesure

### Ce qu'on vise
- < 50 MB RAM (cible: 30-40 MB)
- < 1% CPU en idle
- Interface bilingue FR/EN

### Gestion permissions Linux
RAPL necessite `CAP_SYS_RAWIO` ou root. Scripts dans `scripts/` pour setup.

### Windows specifique
Pas d'acces direct aux capteurs de puissance sans outils tiers (LibreHardwareMonitor, HWiNFO). L'estimation basee sur le load CPU est utilisee par defaut.
