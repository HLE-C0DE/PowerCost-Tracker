//! Configuration management

use crate::core::{Error, Result, SessionCategory};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub pricing: PricingConfig,
    #[serde(default)]
    pub widget: WidgetConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
    #[serde(default)]
    pub dashboard: DashboardConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            pricing: PricingConfig::default(),
            widget: WidgetConfig::default(),
            advanced: AdvancedConfig::default(),
            dashboard: DashboardConfig::default(),
        }
    }
}

impl Config {
    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?;

        let app_config_dir = config_dir.join("powercost-tracker");

        if !app_config_dir.exists() {
            fs::create_dir_all(&app_config_dir)?;
        }

        Ok(app_config_dir.join("config.toml"))
    }

    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;
        let mut config: Config = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        // Merge missing widgets from defaults
        config.merge_missing_widgets();

        Ok(config)
    }

    /// Merge any missing widgets from default config into current config
    fn merge_missing_widgets(&mut self) {
        let default_widgets = default_dashboard_widgets();
        let existing_ids: std::collections::HashSet<_> =
            self.dashboard.widgets.iter().map(|w| w.id.clone()).collect();

        for default_widget in default_widgets {
            if !existing_ids.contains(&default_widget.id) {
                // Assign a new position at the end
                let max_pos = self.dashboard.widgets.iter()
                    .map(|w| w.position)
                    .max()
                    .unwrap_or(0);
                let max_row = self.dashboard.widgets.iter()
                    .map(|w| w.row + w.row_span)
                    .max()
                    .unwrap_or(1);

                let mut new_widget = default_widget;
                new_widget.position = max_pos + 1;
                new_widget.row = max_row;
                new_widget.col = 1;

                self.dashboard.widgets.push(new_widget);
            }
        }
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Language: "auto", "en", "fr"
    #[serde(default = "default_language")]
    pub language: String,
    /// Theme: "dark", "light", "system"
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Refresh rate in milliseconds (for critical/fast metrics)
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
    /// Slow refresh rate in milliseconds (for detailed metrics like processes, temps)
    #[serde(default = "default_slow_refresh_rate")]
    pub slow_refresh_rate_ms: u64,
    /// Eco mode (reduced refresh rate when minimized)
    #[serde(default)]
    pub eco_mode: bool,
    /// Start minimized to tray
    #[serde(default)]
    pub start_minimized: bool,
    /// Start with system
    #[serde(default)]
    pub start_with_system: bool,
    /// Remember window position and size across launches
    #[serde(default = "default_true")]
    pub remember_window_position: bool,
    /// Run as administrator on startup (Windows only)
    #[serde(default)]
    pub run_as_admin: bool,
    /// Saved window X position
    #[serde(default)]
    pub window_x: Option<f64>,
    /// Saved window Y position
    #[serde(default)]
    pub window_y: Option<f64>,
    /// Saved window width
    #[serde(default)]
    pub window_width: Option<f64>,
    /// Saved window height
    #[serde(default)]
    pub window_height: Option<f64>,
}

fn default_language() -> String { "auto".to_string() }
fn default_theme() -> String { "dark".to_string() }
fn default_refresh_rate() -> u64 { 1000 }
fn default_slow_refresh_rate() -> u64 { 5000 }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            refresh_rate_ms: default_refresh_rate(),
            slow_refresh_rate_ms: default_slow_refresh_rate(),
            eco_mode: false,
            start_minimized: false,
            start_with_system: false,
            remember_window_position: true,
            run_as_admin: false,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
        }
    }
}

/// Pricing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Pricing mode: "simple", "peak_offpeak", "seasonal", "tempo"
    #[serde(default = "default_pricing_mode")]
    pub mode: String,
    /// Currency code (EUR, USD, GBP, etc.)
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Currency symbol
    #[serde(default = "default_currency_symbol")]
    pub currency_symbol: String,
    /// Simple mode settings
    #[serde(default)]
    pub simple: SimplePricing,
    /// Peak/off-peak settings
    #[serde(default)]
    pub peak_offpeak: PeakOffpeakPricing,
    /// Seasonal settings
    #[serde(default)]
    pub seasonal: SeasonalPricing,
    /// Tempo (EDF-style) settings
    #[serde(default)]
    pub tempo: TempoPricing,
}

fn default_pricing_mode() -> String { "simple".to_string() }
fn default_currency() -> String { "EUR".to_string() }
fn default_currency_symbol() -> String { "\u{20AC}".to_string() } // Euro sign

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            mode: default_pricing_mode(),
            currency: default_currency(),
            currency_symbol: default_currency_symbol(),
            simple: SimplePricing::default(),
            peak_offpeak: PeakOffpeakPricing::default(),
            seasonal: SeasonalPricing::default(),
            tempo: TempoPricing::default(),
        }
    }
}

/// Simple flat rate pricing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplePricing {
    /// Rate per kWh
    #[serde(default = "default_rate")]
    pub rate_per_kwh: f64,
}

fn default_rate() -> f64 { 0.2276 }

impl Default for SimplePricing {
    fn default() -> Self {
        Self {
            rate_per_kwh: default_rate(),
        }
    }
}

/// Peak/off-peak pricing (HP/HC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakOffpeakPricing {
    /// Peak rate per kWh
    #[serde(default = "default_peak_rate")]
    pub peak_rate: f64,
    /// Off-peak rate per kWh
    #[serde(default = "default_offpeak_rate")]
    pub offpeak_rate: f64,
    /// Off-peak start time (HH:MM)
    #[serde(default = "default_offpeak_start")]
    pub offpeak_start: String,
    /// Off-peak end time (HH:MM)
    #[serde(default = "default_offpeak_end")]
    pub offpeak_end: String,
}

fn default_peak_rate() -> f64 { 0.27 }
fn default_offpeak_rate() -> f64 { 0.20 }
fn default_offpeak_start() -> String { "22:00".to_string() }
fn default_offpeak_end() -> String { "06:00".to_string() }

impl Default for PeakOffpeakPricing {
    fn default() -> Self {
        Self {
            peak_rate: default_peak_rate(),
            offpeak_rate: default_offpeak_rate(),
            offpeak_start: default_offpeak_start(),
            offpeak_end: default_offpeak_end(),
        }
    }
}

/// Seasonal pricing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPricing {
    /// Summer rate per kWh
    #[serde(default = "default_summer_rate")]
    pub summer_rate: f64,
    /// Winter rate per kWh
    #[serde(default = "default_winter_rate")]
    pub winter_rate: f64,
    /// Months considered as winter (1-12)
    #[serde(default = "default_winter_months")]
    pub winter_months: Vec<u32>,
}

fn default_summer_rate() -> f64 { 0.20 }
fn default_winter_rate() -> f64 { 0.25 }
fn default_winter_months() -> Vec<u32> { vec![11, 12, 1, 2, 3] }

impl Default for SeasonalPricing {
    fn default() -> Self {
        Self {
            summer_rate: default_summer_rate(),
            winter_rate: default_winter_rate(),
            winter_months: default_winter_months(),
        }
    }
}

/// Tempo pricing (EDF-style with day colors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoPricing {
    /// Blue day peak rate
    #[serde(default = "default_blue_peak")]
    pub blue_peak: f64,
    /// Blue day off-peak rate
    #[serde(default = "default_blue_offpeak")]
    pub blue_offpeak: f64,
    /// White day peak rate
    #[serde(default = "default_white_peak")]
    pub white_peak: f64,
    /// White day off-peak rate
    #[serde(default = "default_white_offpeak")]
    pub white_offpeak: f64,
    /// Red day peak rate
    #[serde(default = "default_red_peak")]
    pub red_peak: f64,
    /// Red day off-peak rate
    #[serde(default = "default_red_offpeak")]
    pub red_offpeak: f64,
}

fn default_blue_peak() -> f64 { 0.16 }
fn default_blue_offpeak() -> f64 { 0.13 }
fn default_white_peak() -> f64 { 0.19 }
fn default_white_offpeak() -> f64 { 0.15 }
fn default_red_peak() -> f64 { 0.76 }
fn default_red_offpeak() -> f64 { 0.16 }

impl Default for TempoPricing {
    fn default() -> Self {
        Self {
            blue_peak: default_blue_peak(),
            blue_offpeak: default_blue_offpeak(),
            white_peak: default_white_peak(),
            white_offpeak: default_white_offpeak(),
            red_peak: default_red_peak(),
            red_offpeak: default_red_offpeak(),
        }
    }
}

/// Widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    /// Widget enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Show cost (true) or consumption only (false)
    #[serde(default = "default_true")]
    pub show_cost: bool,
    /// Position: "top_left", "top_right", "bottom_left", "bottom_right", or "custom"
    #[serde(default = "default_position")]
    pub position: String,
    /// Widget opacity (0.0 - 1.0)
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    /// Items to display in widget: "power", "cost", "cpu", "gpu", "ram", "temp"
    #[serde(default = "default_display_items")]
    pub display_items: Vec<String>,
    /// Widget size: "compact", "normal", "large"
    #[serde(default = "default_widget_size")]
    pub size: String,
    /// Widget theme: "default", "minimal", "detailed"
    #[serde(default = "default_widget_theme")]
    pub theme: String,
}

fn default_true() -> bool { true }
fn default_position() -> String { "bottom_right".to_string() }
fn default_opacity() -> f64 { 0.9 }
fn default_display_items() -> Vec<String> { vec!["power".to_string(), "cost".to_string()] }
fn default_widget_size() -> String { "normal".to_string() }
fn default_widget_theme() -> String { "default".to_string() }

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_cost: true,
            position: default_position(),
            opacity: default_opacity(),
            display_items: default_display_items(),
            size: default_widget_size(),
            theme: default_widget_theme(),
        }
    }
}

/// Advanced settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Baseline power in watts (0 = auto-detect)
    #[serde(default)]
    pub baseline_watts: f64,
    /// Auto-detect baseline
    #[serde(default = "default_true")]
    pub baseline_auto: bool,
    /// Active hardware profile
    #[serde(default = "default_profile")]
    pub active_profile: String,
    /// Pinned process names for tracking
    #[serde(default)]
    pub pinned_processes: Vec<String>,
    /// Number of processes to show in widget (default 10)
    #[serde(default = "default_process_limit")]
    pub process_list_limit: usize,
    /// CPU/GPU load threshold (%) to collect extended metrics (per-core freq, fans)
    #[serde(default = "default_extended_threshold")]
    pub extended_metrics_threshold: f64,
    /// Session categories for organizing tracking sessions
    #[serde(default = "default_session_categories")]
    pub session_categories: Vec<SessionCategory>,
}

fn default_profile() -> String { "default".to_string() }
fn default_process_limit() -> usize { 10 }
fn default_extended_threshold() -> f64 { 15.0 }
fn default_session_categories() -> Vec<SessionCategory> {
    vec![
        SessionCategory { emoji: "\u{1F3AE}".to_string(), name: "Gaming".to_string() },
        SessionCategory { emoji: "\u{1F4BB}".to_string(), name: "Work".to_string() },
        SessionCategory { emoji: "\u{1F916}".to_string(), name: "AI".to_string() },
        SessionCategory { emoji: "\u{1F310}".to_string(), name: "Browsing".to_string() },
    ]
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            baseline_watts: 0.0,
            baseline_auto: true,
            active_profile: default_profile(),
            pinned_processes: Vec::new(),
            process_list_limit: default_process_limit(),
            extended_metrics_threshold: default_extended_threshold(),
            session_categories: default_session_categories(),
        }
    }
}

/// A named layout profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutProfile {
    pub name: String,
    pub widgets: Vec<DashboardWidget>,
    #[serde(default = "default_global_display")]
    pub global_display: String,
}

/// Dashboard layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Layout type: "default" or "custom"
    #[serde(default = "default_layout")]
    pub layout: String,
    /// Global display mode: "normal", "minimize", "hard"
    /// - "normal": Full display with labels and details
    /// - "minimize": Compact display with reduced labels
    /// - "hard": Data-only display, no labels
    #[serde(default = "default_global_display")]
    pub global_display: String,
    /// Widget configurations
    #[serde(default = "default_dashboard_widgets")]
    pub widgets: Vec<DashboardWidget>,
    /// Active profile name (empty = custom/no profile)
    #[serde(default)]
    pub active_profile: String,
    /// Saved layout profiles
    #[serde(default)]
    pub profiles: Vec<LayoutProfile>,
}

fn default_layout() -> String { "default".to_string() }
fn default_global_display() -> String { "normal".to_string() }

fn default_dashboard_widgets() -> Vec<DashboardWidget> {
    vec![
        // Row 1-3: CPU, GPU, RAM radials + Processes list (all 3x3, fills 12 cols)
        DashboardWidget { id: "cpu".to_string(), visible: true, size: "small".to_string(), position: 7, col: 1, row: 1, col_span: 3, row_span: 3, display_mode: "radial".to_string(), show_wh: true },
        DashboardWidget { id: "gpu".to_string(), visible: true, size: "small".to_string(), position: 8, col: 4, row: 1, col_span: 3, row_span: 3, display_mode: "radial".to_string(), show_wh: true },
        DashboardWidget { id: "ram".to_string(), visible: true, size: "small".to_string(), position: 9, col: 7, row: 1, col_span: 3, row_span: 3, display_mode: "radial".to_string(), show_wh: true },
        DashboardWidget { id: "processes".to_string(), visible: true, size: "large".to_string(), position: 11, col: 10, row: 1, col_span: 3, row_span: 3, display_mode: "text".to_string(), show_wh: true },
        // Row 4-6: Power, Surplus, Session, Estimates (fills 12 cols)
        DashboardWidget { id: "power".to_string(), visible: true, size: "large".to_string(), position: 0, col: 1, row: 4, col_span: 4, row_span: 3, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "surplus".to_string(), visible: true, size: "small".to_string(), position: 10, col: 5, row: 4, col_span: 2, row_span: 2, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "session_controls".to_string(), visible: true, size: "small".to_string(), position: 12, col: 5, row: 6, col_span: 2, row_span: 2, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "session_cost".to_string(), visible: true, size: "small".to_string(), position: 2, col: 7, row: 4, col_span: 3, row_span: 1, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "session_energy".to_string(), visible: true, size: "small".to_string(), position: 1, col: 7, row: 5, col_span: 3, row_span: 1, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "monthly_estimate".to_string(), visible: true, size: "small".to_string(), position: 5, col: 7, row: 6, col_span: 3, row_span: 1, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "session_duration".to_string(), visible: true, size: "small".to_string(), position: 6, col: 10, row: 4, col_span: 3, row_span: 1, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "daily_estimate".to_string(), visible: true, size: "small".to_string(), position: 4, col: 10, row: 5, col_span: 3, row_span: 1, display_mode: "text".to_string(), show_wh: true },
        DashboardWidget { id: "hourly_estimate".to_string(), visible: true, size: "small".to_string(), position: 3, col: 10, row: 6, col_span: 3, row_span: 1, display_mode: "text".to_string(), show_wh: true },
    ]
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            layout: default_layout(),
            global_display: default_global_display(),
            widgets: default_dashboard_widgets(),
            active_profile: String::new(),
            profiles: Vec::new(),
        }
    }
}

/// Individual dashboard widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    /// Widget identifier
    pub id: String,
    /// Whether widget is visible
    pub visible: bool,
    /// Widget size: "small" (1x1), "medium" (2x1), "large" (2x2) - legacy field for backwards compat
    #[serde(default = "default_widget_size_small")]
    pub size: String,
    /// Position in the grid (lower = earlier) - legacy field for backwards compat
    #[serde(default)]
    pub position: u32,
    /// Grid column position (1-based, 1-6)
    #[serde(default = "default_col")]
    pub col: u32,
    /// Grid row position (1-based)
    #[serde(default = "default_row")]
    pub row: u32,
    /// Column span (1-3 for width)
    #[serde(default = "default_col_span")]
    pub col_span: u32,
    /// Row span (1-2 for height)
    #[serde(default = "default_row_span")]
    pub row_span: u32,
    /// Display mode: "text", "bar", "radial", "chart"
    #[serde(default = "default_display_mode")]
    pub display_mode: String,
    /// Whether to show Wh line in estimation widgets (default true)
    #[serde(default = "default_true")]
    pub show_wh: bool,
}

fn default_widget_size_small() -> String { "small".to_string() }
fn default_col() -> u32 { 1 }
fn default_row() -> u32 { 1 }
fn default_col_span() -> u32 { 2 }
fn default_row_span() -> u32 { 1 }
fn default_display_mode() -> String { "text".to_string() }
