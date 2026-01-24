//! Common types used across the application

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// A single power reading from the hardware
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerReading {
    /// Total power consumption in watts
    pub power_watts: f64,
    /// Timestamp of the reading (Unix timestamp)
    pub timestamp: i64,
    /// Source of the reading (e.g., "rapl", "wmi", "estimated")
    pub source: String,
    /// Per-component breakdown if available (CPU, GPU, etc.)
    pub components: Option<HashMap<String, f64>>,
    /// Whether this is an estimated value
    pub is_estimated: bool,
}

impl PowerReading {
    pub fn new(power_watts: f64, source: &str, is_estimated: bool) -> Self {
        Self {
            power_watts,
            timestamp: chrono::Utc::now().timestamp(),
            source: source.to_string(),
            components: None,
            is_estimated,
        }
    }

    pub fn with_components(mut self, components: HashMap<String, f64>) -> Self {
        self.components = Some(components);
        self
    }
}

/// Dashboard data returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Current power consumption in watts
    pub power_watts: f64,
    /// Cumulative energy since session start in Wh
    pub cumulative_wh: f64,
    /// Current cost since session start
    pub current_cost: f64,
    /// Estimated hourly cost at current consumption
    pub hourly_cost_estimate: f64,
    /// Estimated daily cost at current consumption
    pub daily_cost_estimate: f64,
    /// Estimated monthly cost at current consumption
    pub monthly_cost_estimate: f64,
    /// Session duration in seconds
    pub session_duration_secs: u64,
    /// Power reading source
    pub source: String,
    /// Whether power reading is estimated
    pub is_estimated: bool,
}

/// Application runtime state (not persisted)
pub struct AppState {
    /// When the current session started
    pub session_start: Instant,
    /// Cumulative energy consumption in Wh for this session
    pub cumulative_wh: f64,
    /// Current cost for this session
    pub current_cost: f64,
    /// Last power reading in watts
    pub last_power_watts: f64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session_start: Instant::now(),
            cumulative_wh: 0.0,
            current_cost: 0.0,
            last_power_watts: 0.0,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
