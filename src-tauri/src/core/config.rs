//! Configuration management

use crate::core::{Error, Result};
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
        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))
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
    /// Refresh rate in milliseconds
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
    /// Eco mode (reduced refresh rate when minimized)
    #[serde(default)]
    pub eco_mode: bool,
    /// Start minimized to tray
    #[serde(default)]
    pub start_minimized: bool,
    /// Start with system
    #[serde(default)]
    pub start_with_system: bool,
}

fn default_language() -> String { "auto".to_string() }
fn default_theme() -> String { "dark".to_string() }
fn default_refresh_rate() -> u64 { 1000 }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            refresh_rate_ms: default_refresh_rate(),
            eco_mode: false,
            start_minimized: false,
            start_with_system: false,
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
}

fn default_profile() -> String { "default".to_string() }

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            baseline_watts: 0.0,
            baseline_auto: true,
            active_profile: default_profile(),
        }
    }
}

/// Dashboard layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Layout type: "default" or "custom"
    #[serde(default = "default_layout")]
    pub layout: String,
    /// Widget configurations
    #[serde(default = "default_dashboard_widgets")]
    pub widgets: Vec<DashboardWidget>,
}

fn default_layout() -> String { "default".to_string() }

fn default_dashboard_widgets() -> Vec<DashboardWidget> {
    vec![
        DashboardWidget { id: "power".to_string(), visible: true, size: "large".to_string(), position: 0 },
        DashboardWidget { id: "session_energy".to_string(), visible: true, size: "small".to_string(), position: 1 },
        DashboardWidget { id: "session_cost".to_string(), visible: true, size: "small".to_string(), position: 2 },
        DashboardWidget { id: "hourly_estimate".to_string(), visible: true, size: "small".to_string(), position: 3 },
        DashboardWidget { id: "daily_estimate".to_string(), visible: true, size: "small".to_string(), position: 4 },
        DashboardWidget { id: "monthly_estimate".to_string(), visible: true, size: "small".to_string(), position: 5 },
        DashboardWidget { id: "session_duration".to_string(), visible: true, size: "small".to_string(), position: 6 },
        DashboardWidget { id: "cpu".to_string(), visible: true, size: "medium".to_string(), position: 7 },
        DashboardWidget { id: "gpu".to_string(), visible: true, size: "medium".to_string(), position: 8 },
        DashboardWidget { id: "ram".to_string(), visible: true, size: "small".to_string(), position: 9 },
        DashboardWidget { id: "surplus".to_string(), visible: true, size: "medium".to_string(), position: 10 },
    ]
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            layout: default_layout(),
            widgets: default_dashboard_widgets(),
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
    /// Widget size: "small" (1x1), "medium" (2x1), "large" (2x2)
    pub size: String,
    /// Position in the grid (lower = earlier)
    pub position: u32,
}
