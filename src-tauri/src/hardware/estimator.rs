//! Power estimation fallback
//!
//! When no direct hardware sensors are available, this module provides
//! power estimation based on CPU/GPU load and typical TDP values.

use crate::core::{PowerReading, Result};
use crate::hardware::PowerSource;
use sysinfo::System;
use std::sync::Mutex;

/// TDP-based power estimator
pub struct EstimationMonitor {
    sys: Mutex<System>,
    /// Estimated idle power (configurable)
    idle_power: f64,
    /// Estimated maximum additional power at full load
    max_load_power: f64,
}

impl EstimationMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();

        // Default power estimates (can be adjusted based on hardware detection)
        let idle_power = 30.0; // Watts at idle
        let max_load_power = 120.0; // Additional watts at full load

        Self {
            sys: Mutex::new(sys),
            idle_power,
            max_load_power,
        }
    }

    /// Create with custom power values
    pub fn with_power_values(idle_power: f64, max_load_power: f64) -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();

        Self {
            sys: Mutex::new(sys),
            idle_power,
            max_load_power,
        }
    }

    fn calculate_estimated_power(&self) -> f64 {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_all();

        // Calculate average CPU load
        let cpus = sys.cpus();
        if cpus.is_empty() {
            return self.idle_power;
        }

        let avg_load: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;

        // Linear interpolation between idle and max power based on load
        let load_factor = (avg_load as f64) / 100.0;

        self.idle_power + (load_factor * self.max_load_power)
    }
}

impl Default for EstimationMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerSource for EstimationMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        Ok(self.calculate_estimated_power())
    }

    fn get_reading(&self) -> Result<PowerReading> {
        let power = self.calculate_estimated_power();
        Ok(PowerReading::new(power, "estimated", true))
    }

    fn name(&self) -> &str {
        "Estimation (no hardware sensor)"
    }

    fn is_estimated(&self) -> bool {
        true
    }
}
