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
pub mod baseline;

pub use baseline::BaselineDetector;

use crate::core::{DetailedMetrics, Error, PowerReading, ProcessMetrics, Result, SystemMetrics};
use std::any::Any;

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

    /// Get power reading using fast path (CPU-only + cached GPU, no blocking commands)
    /// Returns (power_watts, cpu_usage_percent, cached_gpu_usage_percent, cached_gpu_power_watts)
    pub fn get_power_watts_fast(&self) -> Result<(f64, f64, Option<f64>, Option<f64>)> {
        self.source.get_power_watts_fast()
    }

    /// Collect detailed metrics (processes, temps, VRAM) - may block for GPU commands
    pub fn collect_detailed_metrics(&self, limit: usize, pinned: &[String]) -> Result<DetailedMetrics> {
        self.source.collect_detailed_metrics(limit, pinned)
    }

    /// Get system metrics (CPU, GPU, RAM) - uses stored source for cache sharing
    #[cfg(target_os = "windows")]
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        // Downcast to WmiMonitor to access system metrics using the stored instance
        if let Some(wmi) = self.source.as_any().downcast_ref::<windows::WmiMonitor>() {
            wmi.get_system_metrics()
        } else {
            Err(Error::HardwareNotSupported("System metrics not available for this source".to_string()))
        }
    }

    /// Get system metrics (Linux stub)
    #[cfg(target_os = "linux")]
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        // TODO: Implement Linux system metrics
        Err(Error::HardwareNotSupported("System metrics not yet implemented for Linux".to_string()))
    }

    /// Get top processes by CPU usage - uses stored source for cache sharing
    #[cfg(target_os = "windows")]
    pub fn get_top_processes(&self, limit: usize) -> Result<Vec<ProcessMetrics>> {
        if let Some(wmi) = self.source.as_any().downcast_ref::<windows::WmiMonitor>() {
            wmi.get_top_processes(limit)
        } else {
            Err(Error::HardwareNotSupported("Process metrics not available for this source".to_string()))
        }
    }

    /// Get top processes (Linux stub)
    #[cfg(target_os = "linux")]
    pub fn get_top_processes(&self, _limit: usize) -> Result<Vec<ProcessMetrics>> {
        // TODO: Implement Linux process metrics
        Err(Error::HardwareNotSupported("Process metrics not yet implemented for Linux".to_string()))
    }

    /// Get top processes with pinned processes prioritized - uses stored source
    #[cfg(target_os = "windows")]
    pub fn get_top_processes_with_pinned(&self, limit: usize, pinned: &[String]) -> Result<Vec<ProcessMetrics>> {
        if let Some(wmi) = self.source.as_any().downcast_ref::<windows::WmiMonitor>() {
            wmi.get_top_processes_with_pinned(limit, pinned)
        } else {
            Err(Error::HardwareNotSupported("Process metrics not available for this source".to_string()))
        }
    }

    /// Get top processes with pinned (Linux stub)
    #[cfg(target_os = "linux")]
    pub fn get_top_processes_with_pinned(&self, _limit: usize, _pinned: &[String]) -> Result<Vec<ProcessMetrics>> {
        Err(Error::HardwareNotSupported("Process metrics not yet implemented for Linux".to_string()))
    }

    /// Get all processes (for discovery mode) - uses stored source
    #[cfg(target_os = "windows")]
    pub fn get_all_processes(&self) -> Result<Vec<ProcessMetrics>> {
        if let Some(wmi) = self.source.as_any().downcast_ref::<windows::WmiMonitor>() {
            wmi.get_all_processes()
        } else {
            Err(Error::HardwareNotSupported("Process metrics not available for this source".to_string()))
        }
    }

    /// Get all processes (Linux stub)
    #[cfg(target_os = "linux")]
    pub fn get_all_processes(&self) -> Result<Vec<ProcessMetrics>> {
        Err(Error::HardwareNotSupported("Process metrics not yet implemented for Linux".to_string()))
    }
}

/// Trait for power monitoring sources
pub trait PowerSource: Send + Sync {
    /// Get current power in watts
    fn get_power_watts(&self) -> Result<f64>;

    /// Get power reading using fast path (CPU-only + cached GPU, no blocking commands)
    /// Returns (power_watts, cpu_usage_percent, cached_gpu_usage_percent, cached_gpu_power_watts)
    fn get_power_watts_fast(&self) -> Result<(f64, f64, Option<f64>, Option<f64>)> {
        // Default implementation falls back to normal method
        let power = self.get_power_watts()?;
        Ok((power, 0.0, None, None))
    }

    /// Collect detailed metrics (processes, temps, VRAM) - may block for GPU commands
    fn collect_detailed_metrics(&self, _limit: usize, _pinned: &[String]) -> Result<DetailedMetrics> {
        Err(Error::HardwareNotSupported("Detailed metrics not implemented".to_string()))
    }

    /// Get a full reading with metadata
    fn get_reading(&self) -> Result<PowerReading>;

    /// Name of this power source
    fn name(&self) -> &str;

    /// Whether readings are estimated
    fn is_estimated(&self) -> bool;

    /// Downcast support for type-specific operations
    fn as_any(&self) -> &dyn Any;
}
