//! Hardware power monitoring module
//!
//! Provides abstractions for reading power consumption from various sources:
//! - Linux: RAPL (Intel Running Average Power Limit) via sysfs
//! - Windows: WMI queries and estimation
//! - Fallback: TDP-based estimation

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
mod estimator;

use crate::core::{Error, PowerReading, Result};

/// Power monitor that abstracts over different hardware sources
pub struct PowerMonitor {
    source: Box<dyn PowerSource + Send + Sync>,
}

impl PowerMonitor {
    /// Create a new power monitor, automatically detecting the best source
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            // Try RAPL first (most accurate)
            if let Ok(rapl) = linux::RaplMonitor::new() {
                log::info!("Using RAPL for power monitoring");
                return Ok(Self {
                    source: Box::new(rapl),
                });
            }

            // Try hwmon
            if let Ok(hwmon) = linux::HwmonMonitor::new() {
                log::info!("Using hwmon for power monitoring");
                return Ok(Self {
                    source: Box::new(hwmon),
                });
            }

            // Try battery (for laptops)
            if let Ok(battery) = linux::BatteryMonitor::new() {
                log::info!("Using battery interface for power monitoring");
                return Ok(Self {
                    source: Box::new(battery),
                });
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Try WMI
            if let Ok(wmi) = windows::WmiMonitor::new() {
                log::info!("Using WMI for power monitoring");
                return Ok(Self {
                    source: Box::new(wmi),
                });
            }
        }

        // Fallback to estimation
        log::warn!("No direct power source available, using estimation");
        Err(Error::HardwareNotSupported(
            "No power monitoring hardware detected".to_string(),
        ))
    }

    /// Create a power monitor that uses estimation as fallback
    pub fn estimation_fallback() -> Self {
        Self {
            source: Box::new(estimator::EstimationMonitor::new()),
        }
    }

    /// Get current power consumption in watts
    pub fn get_power_watts(&self) -> Result<f64> {
        self.source.get_power_watts()
    }

    /// Get a full power reading with metadata
    pub fn get_reading(&self) -> Result<PowerReading> {
        self.source.get_reading()
    }

    /// Get the name of the current power source
    pub fn get_source_name(&self) -> &str {
        self.source.name()
    }

    /// Check if readings are estimated (not real measurements)
    pub fn is_estimated(&self) -> bool {
        self.source.is_estimated()
    }
}

/// Trait for power monitoring sources
pub trait PowerSource {
    /// Get current power in watts
    fn get_power_watts(&self) -> Result<f64>;

    /// Get a full reading with metadata
    fn get_reading(&self) -> Result<PowerReading>;

    /// Name of this power source
    fn name(&self) -> &str;

    /// Whether readings are estimated
    fn is_estimated(&self) -> bool;
}
