//! Power estimation fallback
//!
//! When no direct hardware sensors are available, this module provides
//! power estimation based on CPU/GPU load and typical TDP values.
//!
//! The estimator detects CPU specifications via sysinfo and uses
//! realistic TDP values based on the detected processor model.

use crate::core::{PowerReading, Result};
use crate::hardware::PowerSource;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;
use sysinfo::System;

/// CPU category for TDP estimation
#[derive(Debug, Clone, Copy, PartialEq)]
enum CpuCategory {
    // Intel Desktop
    IntelDesktopI3,
    IntelDesktopI5,
    IntelDesktopI7,
    IntelDesktopI9,
    // Intel Laptop
    IntelLaptopU,  // Ultra-low power (15W)
    IntelLaptopP,  // Performance (28W)
    IntelLaptopH,  // High performance (45W)
    IntelLaptopHX, // Extreme (55W+)
    // AMD Desktop
    AmdDesktopRyzen3,
    AmdDesktopRyzen5,
    AmdDesktopRyzen7,
    AmdDesktopRyzen9,
    AmdDesktopThreadripper,
    // AMD Laptop
    AmdLaptopU,  // Ultra-low power (15W)
    AmdLaptopHS, // High performance slim (35W)
    AmdLaptopH,  // High performance (45W)
    AmdLaptopHX, // Extreme (55W+)
    // Apple Silicon
    AppleM1,
    AppleM1Pro,
    AppleM1Max,
    AppleM2,
    AppleM2Pro,
    AppleM2Max,
    AppleM3,
    AppleM3Pro,
    AppleM3Max,
    // Fallbacks
    GenericDesktop,
    GenericLaptop,
}

/// TDP profile containing power characteristics
#[derive(Debug, Clone, Copy)]
struct TdpProfile {
    /// Typical TDP in watts
    tdp: f64,
    /// Idle power as percentage of TDP (typically 10-15%)
    idle_ratio: f64,
    /// Maximum power limit (PL2) as ratio of TDP
    max_power_ratio: f64,
}

impl TdpProfile {
    const fn new(tdp: f64, idle_ratio: f64, max_power_ratio: f64) -> Self {
        Self {
            tdp,
            idle_ratio,
            max_power_ratio,
        }
    }

    /// Get idle power in watts
    fn idle_power(&self) -> f64 {
        self.tdp * self.idle_ratio
    }

    /// Get maximum power in watts (burst/turbo)
    fn max_power(&self) -> f64 {
        self.tdp * self.max_power_ratio
    }
}

/// Get TDP profile for a CPU category
fn get_tdp_profile(category: CpuCategory) -> TdpProfile {
    match category {
        // Intel Desktop
        CpuCategory::IntelDesktopI3 => TdpProfile::new(65.0, 0.12, 1.2),
        CpuCategory::IntelDesktopI5 => TdpProfile::new(80.0, 0.12, 1.4),
        CpuCategory::IntelDesktopI7 => TdpProfile::new(110.0, 0.10, 1.5),
        CpuCategory::IntelDesktopI9 => TdpProfile::new(150.0, 0.10, 1.7),

        // Intel Laptop
        CpuCategory::IntelLaptopU => TdpProfile::new(15.0, 0.15, 1.8),
        CpuCategory::IntelLaptopP => TdpProfile::new(28.0, 0.14, 1.6),
        CpuCategory::IntelLaptopH => TdpProfile::new(45.0, 0.12, 1.5),
        CpuCategory::IntelLaptopHX => TdpProfile::new(55.0, 0.10, 1.8),

        // AMD Desktop
        CpuCategory::AmdDesktopRyzen3 => TdpProfile::new(65.0, 0.12, 1.2),
        CpuCategory::AmdDesktopRyzen5 => TdpProfile::new(65.0, 0.12, 1.4),
        CpuCategory::AmdDesktopRyzen7 => TdpProfile::new(95.0, 0.11, 1.4),
        CpuCategory::AmdDesktopRyzen9 => TdpProfile::new(125.0, 0.10, 1.6),
        CpuCategory::AmdDesktopThreadripper => TdpProfile::new(280.0, 0.08, 1.3),

        // AMD Laptop
        CpuCategory::AmdLaptopU => TdpProfile::new(15.0, 0.15, 1.8),
        CpuCategory::AmdLaptopHS => TdpProfile::new(35.0, 0.13, 1.5),
        CpuCategory::AmdLaptopH => TdpProfile::new(45.0, 0.12, 1.5),
        CpuCategory::AmdLaptopHX => TdpProfile::new(55.0, 0.10, 1.8),

        // Apple Silicon (very efficient)
        CpuCategory::AppleM1 => TdpProfile::new(15.0, 0.20, 1.5),
        CpuCategory::AppleM1Pro => TdpProfile::new(30.0, 0.18, 1.4),
        CpuCategory::AppleM1Max => TdpProfile::new(40.0, 0.15, 1.5),
        CpuCategory::AppleM2 => TdpProfile::new(15.0, 0.20, 1.5),
        CpuCategory::AppleM2Pro => TdpProfile::new(30.0, 0.18, 1.4),
        CpuCategory::AppleM2Max => TdpProfile::new(40.0, 0.15, 1.5),
        CpuCategory::AppleM3 => TdpProfile::new(15.0, 0.20, 1.5),
        CpuCategory::AppleM3Pro => TdpProfile::new(30.0, 0.18, 1.4),
        CpuCategory::AppleM3Max => TdpProfile::new(40.0, 0.15, 1.5),

        // Generic fallbacks
        CpuCategory::GenericDesktop => TdpProfile::new(85.0, 0.12, 1.4),
        CpuCategory::GenericLaptop => TdpProfile::new(25.0, 0.14, 1.5),
    }
}

/// Detected CPU specifications
#[derive(Debug, Clone)]
struct CpuSpecs {
    /// CPU model name
    name: String,
    /// Number of physical cores
    core_count: usize,
    /// Detected CPU category
    category: CpuCategory,
    /// TDP profile for this CPU
    profile: TdpProfile,
    /// Whether this appears to be a laptop
    is_laptop: bool,
}

impl CpuSpecs {
    /// Detect CPU specifications from sysinfo
    fn detect(sys: &System) -> Self {
        let cpus = sys.cpus();

        // Get CPU name from first CPU (all cores have same name)
        let name = cpus
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        // Count physical cores (sysinfo reports logical CPUs)
        // We estimate physical cores as logical / 2 for most modern CPUs
        // but fallback to logical count if it's small
        let logical_count = cpus.len();
        let core_count = if logical_count > 2 {
            // Assume hyperthreading/SMT for most modern CPUs
            (logical_count + 1) / 2
        } else {
            logical_count
        };

        // Detect category from CPU name
        let (category, is_laptop) = Self::categorize_cpu(&name, core_count);
        let profile = get_tdp_profile(category);

        Self {
            name,
            core_count,
            category,
            profile,
            is_laptop,
        }
    }

    /// Categorize CPU based on its name string
    fn categorize_cpu(name: &str, core_count: usize) -> (CpuCategory, bool) {
        let name_lower = name.to_lowercase();

        // Apple Silicon detection
        if name_lower.contains("apple")
            || name_lower.contains("m1")
            || name_lower.contains("m2")
            || name_lower.contains("m3")
        {
            if name_lower.contains("m3") {
                if name_lower.contains("max") {
                    return (CpuCategory::AppleM3Max, true);
                } else if name_lower.contains("pro") {
                    return (CpuCategory::AppleM3Pro, true);
                }
                return (CpuCategory::AppleM3, true);
            } else if name_lower.contains("m2") {
                if name_lower.contains("max") {
                    return (CpuCategory::AppleM2Max, true);
                } else if name_lower.contains("pro") {
                    return (CpuCategory::AppleM2Pro, true);
                }
                return (CpuCategory::AppleM2, true);
            } else if name_lower.contains("m1") {
                if name_lower.contains("max") {
                    return (CpuCategory::AppleM1Max, true);
                } else if name_lower.contains("pro") {
                    return (CpuCategory::AppleM1Pro, true);
                }
                return (CpuCategory::AppleM1, true);
            }
        }

        // Intel detection
        if name_lower.contains("intel") {
            let is_laptop = name_lower.contains("u ")
                || name_lower.contains("-u")
                || name_lower.ends_with("u")
                || name_lower.contains("p ")
                || name_lower.contains("-p")
                || name_lower.ends_with("p")
                || name_lower.contains("h ")
                || name_lower.contains("-h")
                || name_lower.ends_with("h")
                || name_lower.contains("hx")
                || name_lower.ends_with("hx")
                || name_lower.contains("mobile");

            if is_laptop {
                if name_lower.contains("hx") || name_lower.ends_with("hx") {
                    return (CpuCategory::IntelLaptopHX, true);
                } else if name_lower.contains("h ") || name_lower.contains("-h") || name_lower.ends_with("h") {
                    return (CpuCategory::IntelLaptopH, true);
                } else if name_lower.contains("p ") || name_lower.contains("-p") || name_lower.ends_with("p") {
                    return (CpuCategory::IntelLaptopP, true);
                } else {
                    return (CpuCategory::IntelLaptopU, true);
                }
            }

            // Desktop Intel
            if name_lower.contains("i9") {
                return (CpuCategory::IntelDesktopI9, false);
            } else if name_lower.contains("i7") {
                return (CpuCategory::IntelDesktopI7, false);
            } else if name_lower.contains("i5") {
                return (CpuCategory::IntelDesktopI5, false);
            } else if name_lower.contains("i3") {
                return (CpuCategory::IntelDesktopI3, false);
            }

            // Unknown Intel - guess based on core count
            if core_count >= 8 {
                return (CpuCategory::IntelDesktopI7, false);
            } else if core_count >= 4 {
                return (CpuCategory::IntelDesktopI5, false);
            }
            return (CpuCategory::IntelDesktopI3, false);
        }

        // AMD detection
        if name_lower.contains("amd")
            || name_lower.contains("ryzen")
            || name_lower.contains("threadripper")
        {
            // Threadripper
            if name_lower.contains("threadripper") {
                return (CpuCategory::AmdDesktopThreadripper, false);
            }

            let is_laptop = name_lower.contains("u ")
                || name_lower.contains("-u")
                || name_lower.ends_with("u")
                || name_lower.contains("hs")
                || name_lower.contains("h ")
                || name_lower.contains("-h")
                || name_lower.ends_with("h")
                || name_lower.contains("hx")
                || name_lower.ends_with("hx")
                || name_lower.contains("mobile");

            if is_laptop {
                if name_lower.contains("hx") || name_lower.ends_with("hx") {
                    return (CpuCategory::AmdLaptopHX, true);
                } else if name_lower.contains("hs") || name_lower.ends_with("hs") {
                    return (CpuCategory::AmdLaptopHS, true);
                } else if name_lower.contains("h ") || name_lower.contains("-h") || name_lower.ends_with("h") {
                    return (CpuCategory::AmdLaptopH, true);
                } else {
                    return (CpuCategory::AmdLaptopU, true);
                }
            }

            // Desktop Ryzen
            if name_lower.contains("ryzen 9") || name_lower.contains("ryzen9") {
                return (CpuCategory::AmdDesktopRyzen9, false);
            } else if name_lower.contains("ryzen 7") || name_lower.contains("ryzen7") {
                return (CpuCategory::AmdDesktopRyzen7, false);
            } else if name_lower.contains("ryzen 5") || name_lower.contains("ryzen5") {
                return (CpuCategory::AmdDesktopRyzen5, false);
            } else if name_lower.contains("ryzen 3") || name_lower.contains("ryzen3") {
                return (CpuCategory::AmdDesktopRyzen3, false);
            }

            // Unknown AMD Ryzen - guess based on core count
            if core_count >= 12 {
                return (CpuCategory::AmdDesktopRyzen9, false);
            } else if core_count >= 8 {
                return (CpuCategory::AmdDesktopRyzen7, false);
            } else if core_count >= 6 {
                return (CpuCategory::AmdDesktopRyzen5, false);
            }
            return (CpuCategory::AmdDesktopRyzen3, false);
        }

        // Generic fallback based on core count
        // Laptops typically have fewer cores
        let is_laptop_guess = core_count <= 4;
        if is_laptop_guess {
            (CpuCategory::GenericLaptop, true)
        } else {
            (CpuCategory::GenericDesktop, false)
        }
    }
}

/// TDP-based power estimator with CPU detection
pub struct EstimationMonitor {
    sys: Mutex<System>,
    /// Detected CPU specifications
    cpu_specs: CpuSpecs,
    /// Override idle power (if set via with_power_values)
    idle_power_override: Option<f64>,
    /// Override max power (if set via with_power_values)
    max_power_override: Option<f64>,
}

impl EstimationMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();

        // Wait a bit and refresh again for accurate readings
        std::thread::sleep(std::time::Duration::from_millis(100));
        sys.refresh_cpu_usage();

        let cpu_specs = CpuSpecs::detect(&sys);

        log::info!(
            "CPU detected: {} ({} cores, {:?}, TDP: {:.0}W)",
            cpu_specs.name,
            cpu_specs.core_count,
            cpu_specs.category,
            cpu_specs.profile.tdp
        );

        Self {
            sys: Mutex::new(sys),
            cpu_specs,
            idle_power_override: None,
            max_power_override: None,
        }
    }

    /// Get the effective idle power
    fn get_idle_power(&self) -> f64 {
        self.idle_power_override
            .unwrap_or_else(|| self.cpu_specs.profile.idle_power())
    }

    /// Get the effective max power
    fn get_max_power(&self) -> f64 {
        self.max_power_override
            .unwrap_or_else(|| self.cpu_specs.profile.max_power())
    }

    /// Calculate estimated power consumption
    ///
    /// Formula: power = idle_power + (load_factor * (max_power - idle_power))
    ///
    /// We also factor in the number of active cores:
    /// - If only some cores are loaded, power consumption is lower
    /// - We use a weighted average that considers per-core load distribution
    fn calculate_estimated_power(&self) -> f64 {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();

        let cpus = sys.cpus();
        if cpus.is_empty() {
            return self.get_idle_power();
        }

        // Calculate per-core loads
        let loads: Vec<f64> = cpus
            .iter()
            .map(|cpu| cpu.cpu_usage() as f64 / 100.0)
            .collect();
        let total_cores = loads.len();

        // Average load across all cores
        let avg_load: f64 = loads.iter().sum::<f64>() / total_cores as f64;

        // Calculate "active core factor"
        // This estimates how many cores are actually doing work
        // A core is considered "active" if it has > 5% load
        let active_threshold = 0.05;
        let active_cores = loads
            .iter()
            .filter(|&&load| load > active_threshold)
            .count();
        let active_ratio = active_cores as f64 / total_cores as f64;

        // Weighted load factor that accounts for:
        // 1. Average load (main factor)
        // 2. Active core ratio (secondary factor - more active cores = more power)
        //
        // Power doesn't scale perfectly linearly with load due to:
        // - Base power for active cores
        // - Frequency scaling at low loads
        // - Efficiency curves
        //
        // We use: load_factor = avg_load * (0.7 + 0.3 * active_ratio)
        // This means:
        // - At 100% load on all cores: factor = 1.0 * (0.7 + 0.3 * 1.0) = 1.0
        // - At 100% load on half cores: factor = 0.5 * (0.7 + 0.3 * 0.5) = 0.425
        // - At 50% load on all cores: factor = 0.5 * (0.7 + 0.3 * 1.0) = 0.5
        let load_factor = avg_load * (0.7 + 0.3 * active_ratio);

        // Clamp load factor to valid range
        let load_factor = load_factor.clamp(0.0, 1.0);

        let idle_power = self.get_idle_power();
        let max_power = self.get_max_power();

        // Final power calculation
        // power = idle_power + (load_factor * (max_power - idle_power))
        let power = idle_power + (load_factor * (max_power - idle_power));

        // Ensure we return at least idle power and at most max power
        power.clamp(idle_power, max_power)
    }

    /// Get per-component power breakdown estimation
    fn get_component_breakdown(&self) -> HashMap<String, f64> {
        let total_power = self.calculate_estimated_power();
        let mut components = HashMap::new();

        // Estimate component breakdown (rough estimates)
        // CPU typically uses 60-80% of total system power
        let cpu_ratio = if self.cpu_specs.is_laptop { 0.70 } else { 0.65 };
        let cpu_power = total_power * cpu_ratio;

        // Remaining power distributed to other components
        let other_power = total_power - cpu_power;

        components.insert("cpu".to_string(), cpu_power);
        components.insert("other".to_string(), other_power);

        components
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
        let components = self.get_component_breakdown();

        Ok(PowerReading::new(power, "estimated", true).with_components(components))
    }

    fn name(&self) -> &str {
        "TDP Estimation (auto-detected)"
    }

    fn is_estimated(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tdp_profiles() {
        // Verify TDP profiles have reasonable values
        let profile = get_tdp_profile(CpuCategory::IntelDesktopI7);
        assert!(profile.tdp > 0.0);
        assert!(profile.idle_ratio > 0.0 && profile.idle_ratio < 1.0);
        assert!(profile.max_power_ratio >= 1.0);
        assert!(profile.idle_power() < profile.tdp);
        assert!(profile.max_power() >= profile.tdp);
    }

    #[test]
    fn test_cpu_categorization() {
        // Intel laptop
        let (cat, is_laptop) = CpuSpecs::categorize_cpu("Intel Core i7-1165G7 @ 2.80GHz", 4);
        assert!(matches!(
            cat,
            CpuCategory::IntelDesktopI7 | CpuCategory::IntelLaptopU | CpuCategory::IntelLaptopP
        ));

        // Intel desktop
        let (cat, is_laptop) = CpuSpecs::categorize_cpu("Intel Core i9-13900K", 24);
        assert_eq!(cat, CpuCategory::IntelDesktopI9);
        assert!(!is_laptop);

        // AMD Ryzen
        let (cat, is_laptop) = CpuSpecs::categorize_cpu("AMD Ryzen 7 5800X", 8);
        assert_eq!(cat, CpuCategory::AmdDesktopRyzen7);
        assert!(!is_laptop);

        // AMD laptop
        let (cat, is_laptop) = CpuSpecs::categorize_cpu("AMD Ryzen 7 6800H", 8);
        assert_eq!(cat, CpuCategory::AmdLaptopH);
        assert!(is_laptop);
    }

    #[test]
    fn test_estimation_monitor_creation() {
        let monitor = EstimationMonitor::new();
        assert!(monitor.get_detected_tdp() > 0.0);
        assert!(!monitor.get_cpu_name().is_empty());
        assert!(monitor.get_core_count() > 0);
    }

    #[test]
    fn test_power_calculation() {
        let monitor = EstimationMonitor::new();
        let power = monitor.calculate_estimated_power();

        // Power should be between idle and max
        let idle = monitor.get_idle_power();
        let max = monitor.get_max_power();

        assert!(power >= idle, "Power {} should be >= idle {}", power, idle);
        assert!(power <= max, "Power {} should be <= max {}", power, max);
    }

    #[test]
    fn test_custom_power_values() {
        let monitor = EstimationMonitor::with_power_values(10.0, 100.0);
        assert_eq!(monitor.get_idle_power(), 10.0);
        assert_eq!(monitor.get_max_power(), 110.0);
    }

    #[test]
    fn test_power_source_trait() {
        let monitor = EstimationMonitor::new();

        // Test trait methods
        assert!(monitor.is_estimated());
        assert!(!monitor.name().is_empty());

        let reading = monitor.get_reading().unwrap();
        assert!(reading.power_watts > 0.0);
        assert!(reading.is_estimated);
        assert!(reading.components.is_some());
    }
}
