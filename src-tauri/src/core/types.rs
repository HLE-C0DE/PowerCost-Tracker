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
    /// Average power consumption in watts over the session
    pub avg_power_watts: f64,
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

/// System-wide hardware metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: CpuMetrics,
    pub gpu: Option<GpuMetrics>,
    pub memory: MemoryMetrics,
    pub timestamp: i64,
    /// System fan speeds (only collected when extended metrics enabled)
    #[serde(default)]
    pub fans: Option<FanMetrics>,
    /// Voltage readings (if available from hwmon/sensors)
    #[serde(default)]
    pub voltages: Option<Vec<VoltageReading>>,
}

/// A voltage sensor reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoltageReading {
    pub name: String,
    pub value_volts: f64,
}

/// System fan metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanMetrics {
    pub fans: Vec<FanReading>,
}

/// Individual fan reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanReading {
    pub name: String,
    #[serde(default)]
    pub speed_rpm: Option<u64>,
    #[serde(default)]
    pub speed_percent: Option<u64>,
}

/// CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub name: String,
    pub usage_percent: f64,
    pub per_core_usage: Vec<f64>,
    pub frequency_mhz: Option<u64>,
    pub temperature_celsius: Option<f64>,
    pub core_count: usize,
    pub thread_count: usize,
    /// Per-core clock frequencies in MHz (only collected when extended metrics enabled)
    #[serde(default)]
    pub per_core_frequency_mhz: Option<Vec<u64>>,
    /// Per-core temperatures in Celsius (from hwmon coretemp/k10temp on Linux, or multi-zone on Windows)
    #[serde(default)]
    pub per_core_temperature: Option<Vec<f64>>,
}

/// GPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub name: String,
    pub usage_percent: Option<f64>,
    pub power_watts: Option<f64>,
    pub temperature_celsius: Option<f64>,
    pub vram_used_mb: Option<u64>,
    pub vram_total_mb: Option<u64>,
    pub clock_mhz: Option<u64>,
    pub source: String,
    /// VRAM/memory clock in MHz
    #[serde(default)]
    pub memory_clock_mhz: Option<u64>,
    /// GPU fan speed percentage
    #[serde(default)]
    pub fan_speed_percent: Option<u64>,
}

/// Memory (RAM) metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub usage_percent: f64,
    /// Swap memory used in bytes
    #[serde(default)]
    pub swap_used_bytes: Option<u64>,
    /// Swap memory total in bytes
    #[serde(default)]
    pub swap_total_bytes: Option<u64>,
    /// Swap usage percentage (0-100)
    #[serde(default)]
    pub swap_usage_percent: Option<f64>,
    /// Physical memory speed in MHz (cached, fetched once via WMI)
    #[serde(default)]
    pub memory_speed_mhz: Option<u64>,
}

/// Process metrics for top processes display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub memory_percent: f64,
    #[serde(default)]
    pub gpu_percent: Option<f64>,
    #[serde(default)]
    pub is_pinned: bool,
}

/// Session category for organizing tracking sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCategory {
    pub emoji: String,
    pub name: String,
}

/// Tracking session for baseline/surplus calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub baseline_watts: f64,
    pub total_wh: f64,
    pub surplus_wh: f64,
    pub surplus_cost: f64,
    pub label: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

impl Session {
    pub fn new(baseline_watts: f64, label: Option<String>) -> Self {
        Self {
            id: None,
            start_time: chrono::Utc::now().timestamp(),
            end_time: None,
            baseline_watts,
            total_wh: 0.0,
            surplus_wh: 0.0,
            surplus_cost: 0.0,
            label,
            category: None,
        }
    }
}

/// Baseline detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDetection {
    pub detected_watts: f64,
    pub sample_count: usize,
    pub confidence: f64,
}

/// Critical metrics that need fast updates (power, CPU%, GPU%, cost, timer)
/// Updated at the user's fast refresh rate (e.g., 1s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalMetrics {
    /// Current power consumption in watts
    pub power_watts: f64,
    /// Session average power consumption in watts (smoothed)
    pub avg_power_watts: f64,
    /// CPU usage percentage (0-100)
    pub cpu_usage_percent: f64,
    /// GPU usage percentage (0-100), if available
    pub gpu_usage_percent: Option<f64>,
    /// GPU power in watts (from cache), if available
    pub gpu_power_watts: Option<f64>,
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
    /// Active session data if tracking
    pub active_session: Option<Session>,
    /// Power reading source
    pub source: String,
    /// Whether power reading is estimated
    pub is_estimated: bool,
    /// Timestamp of this reading
    pub timestamp: i64,
}

/// Detailed metrics that can be updated less frequently (processes, temps, VRAM)
/// Updated at the slow refresh rate (e.g., 5s) to avoid blocking GPU commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedMetrics {
    /// Full system metrics including CPU, GPU, and memory
    pub system_metrics: Option<SystemMetrics>,
    /// Top processes by resource usage
    pub top_processes: Vec<ProcessMetrics>,
    /// Timestamp of this reading
    pub timestamp: i64,
    /// Whether extended metrics (per-core freq, fans) were collected this cycle
    #[serde(default)]
    pub extended_collected: bool,
}
