//! Windows power monitoring implementations
//!
//! Uses WMI (Windows Management Instrumentation) to query power data,
//! with support for NVIDIA and AMD GPUs via their respective CLI tools.

use crate::core::{Error, PowerReading, Result};
use crate::hardware::PowerSource;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;

use windows::core::{BSTR, VARIANT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Wmi::{
    IEnumWbemClassObject, IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator,
    WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE,
};

/// Available GPU monitoring sources
#[derive(Debug, Clone, Copy, PartialEq)]
enum GpuSource {
    /// NVIDIA GPU via nvidia-smi
    Nvidia,
    /// AMD GPU via rocm-smi
    Amd,
    /// No GPU monitoring available
    None,
}

/// WMI connection wrapper for safe resource management
struct WmiConnection {
    #[allow(dead_code)]
    locator: IWbemLocator,
    services: IWbemServices,
}

impl WmiConnection {
    /// Initialize COM and connect to WMI
    fn new() -> std::result::Result<Self, windows::core::Error> {
        unsafe {
            // Initialize COM library
            CoInitializeEx(None, COINIT_MULTITHREADED)?;

            // Create WMI locator
            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;

            // Connect to local WMI namespace
            let server = BSTR::from("ROOT\\CIMV2");
            let services = locator.ConnectServer(&server, None, None, None, 0, None, None)?;

            // Set security levels for WMI calls
            CoSetProxyBlanket(
                &services,
                RPC_C_AUTHN_LEVEL_DEFAULT,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
                None,
            )?;

            Ok(Self { locator, services })
        }
    }

    /// Execute a WQL query and return results
    fn query(&self, wql: &str) -> std::result::Result<Vec<IWbemClassObject>, windows::core::Error> {
        unsafe {
            let query_lang = BSTR::from("WQL");
            let query = BSTR::from(wql);

            let enumerator: IEnumWbemClassObject = self.services.ExecQuery(
                &query_lang,
                &query,
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            )?;

            let mut results = Vec::new();
            loop {
                let mut objects: [Option<IWbemClassObject>; 1] = [None];
                let mut returned: u32 = 0;

                let hr = enumerator.Next(WBEM_INFINITE, &mut objects, &mut returned);

                if hr.is_err() || returned == 0 {
                    break;
                }

                if let Some(obj) = objects[0].take() {
                    results.push(obj);
                }
            }

            Ok(results)
        }
    }

    /// Get a property value as i32 from a WMI object
    fn get_property_i32(
        obj: &IWbemClassObject,
        name: &str,
    ) -> std::result::Result<Option<i32>, windows::core::Error> {
        unsafe {
            let prop_name = BSTR::from(name);
            let mut value = VARIANT::default();

            if obj.Get(&prop_name, 0, &mut value, None, None).is_ok() {
                // Extract the value from VARIANT
                if let Ok(val) = Self::variant_to_i32(&value) {
                    return Ok(Some(val));
                }
            }
            Ok(None)
        }
    }

    /// Get a property value as u16 from a WMI object
    fn get_property_u16(
        obj: &IWbemClassObject,
        name: &str,
    ) -> std::result::Result<Option<u16>, windows::core::Error> {
        unsafe {
            let prop_name = BSTR::from(name);
            let mut value = VARIANT::default();

            if obj.Get(&prop_name, 0, &mut value, None, None).is_ok() {
                if let Ok(val) = Self::variant_to_i32(&value) {
                    return Ok(Some(val as u16));
                }
            }
            Ok(None)
        }
    }

    /// Convert VARIANT to i32
    fn variant_to_i32(variant: &VARIANT) -> std::result::Result<i32, ()> {
        unsafe {
            // Access the Anonymous union to get the actual value
            let vt = variant.Anonymous.Anonymous.vt;

            // VT_I4 = 3, VT_I2 = 2, VT_UI2 = 18, VT_UI4 = 19
            match vt.0 {
                3 => Ok(variant.Anonymous.Anonymous.Anonymous.lVal), // VT_I4
                2 => Ok(variant.Anonymous.Anonymous.Anonymous.iVal as i32), // VT_I2
                18 => Ok(variant.Anonymous.Anonymous.Anonymous.uiVal as i32), // VT_UI2
                19 => Ok(variant.Anonymous.Anonymous.Anonymous.ulVal as i32), // VT_UI4
                _ => Err(()),
            }
        }
    }
}

impl Drop for WmiConnection {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

/// Battery information from WMI
#[derive(Debug, Clone)]
struct BatteryInfo {
    /// Estimated charge remaining (percentage)
    pub charge_remaining: u16,
    /// Battery voltage in millivolts (if available)
    pub voltage_mv: Option<u32>,
    /// Design voltage in millivolts (if available)
    pub design_voltage_mv: Option<u32>,
    /// Full charge capacity in mWh (if available)
    pub full_charge_capacity_mwh: Option<u32>,
    /// Indicates if battery is discharging
    pub is_discharging: bool,
}

/// CPU load information from WMI
#[derive(Debug, Clone)]
struct CpuInfo {
    /// Load percentage per processor (0-100)
    pub load_percentages: Vec<u16>,
    /// Average load across all processors
    pub average_load: f64,
}

/// GPU power information
#[derive(Debug, Clone)]
struct GpuInfo {
    /// Power draw in watts
    pub power_watts: f64,
    /// GPU name/model
    pub name: String,
    /// Source of the reading
    pub source: GpuSource,
}

/// WMI-based power monitor for Windows
///
/// Combines multiple data sources for comprehensive power monitoring:
/// - WMI queries for CPU load and battery status
/// - nvidia-smi for NVIDIA GPU power
/// - rocm-smi for AMD GPU power
/// - sysinfo as fallback
pub struct WmiMonitor {
    /// WMI connection (optional - may fail to initialize)
    wmi: Option<WmiConnection>,
    /// Detected GPU monitoring source
    gpu_source: GpuSource,
    /// Sysinfo for fallback and additional data
    sys: Mutex<sysinfo::System>,
    /// Cached TDP estimate for CPU (watts)
    cpu_tdp_estimate: f64,
    /// Whether this is a laptop (has battery)
    is_laptop: bool,
}

impl WmiMonitor {
    /// Create a new WMI-based power monitor
    ///
    /// Initializes WMI connection and detects available power sources:
    /// - Attempts WMI connection for CPU and battery data
    /// - Checks for nvidia-smi availability
    /// - Checks for rocm-smi availability
    /// - Falls back to sysinfo-based estimation if WMI fails
    pub fn new() -> Result<Self> {
        // Initialize sysinfo
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_all();

        // Try to initialize WMI
        let wmi = match WmiConnection::new() {
            Ok(conn) => {
                log::info!("WMI connection established successfully");
                Some(conn)
            }
            Err(e) => {
                log::warn!("Failed to initialize WMI: {:?}, using fallback", e);
                None
            }
        };

        // Detect GPU monitoring source
        let gpu_source = Self::detect_gpu_source();
        log::info!("GPU monitoring source: {:?}", gpu_source);

        // Estimate CPU TDP based on core count
        let cpu_count = sys.cpus().len();
        let cpu_tdp_estimate = Self::estimate_cpu_tdp(cpu_count);

        // Check if this is a laptop (has battery)
        let is_laptop = Self::check_is_laptop(&wmi);

        Ok(Self {
            wmi,
            gpu_source,
            sys: Mutex::new(sys),
            cpu_tdp_estimate,
            is_laptop,
        })
    }

    /// Detect available GPU monitoring tool
    fn detect_gpu_source() -> GpuSource {
        // Check for NVIDIA GPU (nvidia-smi)
        if let Ok(output) = Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                log::info!("NVIDIA GPU detected via nvidia-smi");
                return GpuSource::Nvidia;
            }
        }

        // Check for AMD GPU (rocm-smi)
        if let Ok(output) = Command::new("rocm-smi").arg("--showpower").output() {
            if output.status.success() {
                log::info!("AMD GPU detected via rocm-smi");
                return GpuSource::Amd;
            }
        }

        // Also try amd-smi (newer AMD tool)
        if let Ok(output) = Command::new("amd-smi").arg("metric").arg("-p").output() {
            if output.status.success() {
                log::info!("AMD GPU detected via amd-smi");
                return GpuSource::Amd;
            }
        }

        GpuSource::None
    }

    /// Estimate CPU TDP based on core count
    fn estimate_cpu_tdp(core_count: usize) -> f64 {
        // Rough TDP estimates based on typical desktop/laptop CPUs
        // These are ballpark figures for modern processors
        match core_count {
            0..=2 => 35.0,    // Low-power dual-core
            3..=4 => 65.0,    // Quad-core desktop
            5..=6 => 95.0,    // 6-core
            7..=8 => 105.0,   // 8-core
            9..=12 => 125.0,  // High-end desktop
            13..=16 => 150.0, // HEDT
            _ => 200.0,       // Workstation/server
        }
    }

    /// Check if running on a laptop
    fn check_is_laptop(wmi: &Option<WmiConnection>) -> bool {
        if let Some(conn) = wmi {
            if let Ok(batteries) = conn.query("SELECT * FROM Win32_Battery") {
                return !batteries.is_empty();
            }
        }
        false
    }

    /// Query battery information via WMI
    fn get_battery_info(&self) -> Option<BatteryInfo> {
        let wmi = self.wmi.as_ref()?;

        let results = wmi
            .query(
                "SELECT EstimatedChargeRemaining, DesignVoltage, BatteryStatus FROM Win32_Battery",
            )
            .ok()?;

        let obj = results.first()?;

        let charge_remaining = WmiConnection::get_property_u16(obj, "EstimatedChargeRemaining")
            .ok()
            .flatten()
            .unwrap_or(0);

        let design_voltage = WmiConnection::get_property_i32(obj, "DesignVoltage")
            .ok()
            .flatten()
            .map(|v| v as u32);

        let battery_status = WmiConnection::get_property_u16(obj, "BatteryStatus")
            .ok()
            .flatten()
            .unwrap_or(0);

        // BatteryStatus: 1=Discharging, 2=AC connected, etc.
        let is_discharging = battery_status == 1;

        Some(BatteryInfo {
            charge_remaining,
            voltage_mv: design_voltage,
            design_voltage_mv: design_voltage,
            full_charge_capacity_mwh: None, // Not directly available via Win32_Battery
            is_discharging,
        })
    }

    /// Query CPU load via WMI
    fn get_cpu_info_wmi(&self) -> Option<CpuInfo> {
        let wmi = self.wmi.as_ref()?;

        let results = wmi
            .query("SELECT LoadPercentage FROM Win32_Processor")
            .ok()?;

        if results.is_empty() {
            return None;
        }

        let mut load_percentages = Vec::new();
        for obj in &results {
            if let Some(load) = WmiConnection::get_property_u16(obj, "LoadPercentage")
                .ok()
                .flatten()
            {
                load_percentages.push(load);
            }
        }

        if load_percentages.is_empty() {
            return None;
        }

        let average_load =
            load_percentages.iter().map(|&l| l as f64).sum::<f64>() / load_percentages.len() as f64;

        Some(CpuInfo {
            load_percentages,
            average_load,
        })
    }

    /// Get CPU info from sysinfo (fallback)
    fn get_cpu_info_sysinfo(&self) -> CpuInfo {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_all();

        let load_percentages: Vec<u16> = sys
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage() as u16)
            .collect();

        let average_load = if load_percentages.is_empty() {
            0.0
        } else {
            load_percentages.iter().map(|&l| l as f64).sum::<f64>() / load_percentages.len() as f64
        };

        CpuInfo {
            load_percentages,
            average_load,
        }
    }

    /// Get GPU power via nvidia-smi
    fn get_nvidia_gpu_power(&self) -> Option<GpuInfo> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=power.draw,name",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

        if parts.len() >= 2 {
            let power = parts[0].parse::<f64>().ok()?;
            let name = parts[1].to_string();

            return Some(GpuInfo {
                power_watts: power,
                name,
                source: GpuSource::Nvidia,
            });
        }

        None
    }

    /// Get GPU power via rocm-smi (AMD)
    fn get_amd_gpu_power(&self) -> Option<GpuInfo> {
        // Try rocm-smi first
        if let Ok(output) = Command::new("rocm-smi")
            .args(["--showpower", "--json"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse JSON output for power value
                // Format varies, but typically includes "Average Graphics Package Power"
                if let Some(power) = Self::parse_rocm_smi_power(&stdout) {
                    return Some(GpuInfo {
                        power_watts: power,
                        name: "AMD GPU".to_string(),
                        source: GpuSource::Amd,
                    });
                }
            }
        }

        // Try amd-smi as fallback
        if let Ok(output) = Command::new("amd-smi")
            .args(["metric", "-p", "--json"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(power) = Self::parse_amd_smi_power(&stdout) {
                    return Some(GpuInfo {
                        power_watts: power,
                        name: "AMD GPU".to_string(),
                        source: GpuSource::Amd,
                    });
                }
            }
        }

        // Try simple text output
        if let Ok(output) = Command::new("rocm-smi").args(["--showpower"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Look for power value in text output
                for line in stdout.lines() {
                    if line.contains("Average") && line.contains("Power") {
                        // Extract number from line like "Average Graphics Package Power (W): 45.0"
                        if let Some(power) = Self::extract_power_from_line(line) {
                            return Some(GpuInfo {
                                power_watts: power,
                                name: "AMD GPU".to_string(),
                                source: GpuSource::Amd,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Parse power from rocm-smi JSON output
    fn parse_rocm_smi_power(json_str: &str) -> Option<f64> {
        // Simple parsing - look for power-related keys
        // rocm-smi JSON format varies by version
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            // Try to find power value in various locations
            if let Some(power) = value.get("Average Graphics Package Power (W)") {
                return power.as_f64();
            }
            if let Some(power) = value.get("power") {
                return power.as_f64();
            }
            // Look for nested structure
            if let Some(card) = value.get("card0") {
                if let Some(power) = card.get("Average Graphics Package Power (W)") {
                    return power.as_f64();
                }
            }
        }
        None
    }

    /// Parse power from amd-smi JSON output
    fn parse_amd_smi_power(json_str: &str) -> Option<f64> {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            // amd-smi format
            if let Some(arr) = value.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(power) = first.get("power") {
                        if let Some(socket_power) = power.get("socket_power") {
                            return socket_power.as_f64();
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract power value from a text line
    fn extract_power_from_line(line: &str) -> Option<f64> {
        // Look for a floating point number in the line
        for word in line.split_whitespace() {
            if let Ok(val) = word.trim_end_matches('W').parse::<f64>() {
                return Some(val);
            }
        }
        None
    }

    /// Get GPU power based on detected source
    fn get_gpu_power(&self) -> Option<GpuInfo> {
        match self.gpu_source {
            GpuSource::Nvidia => self.get_nvidia_gpu_power(),
            GpuSource::Amd => self.get_amd_gpu_power(),
            GpuSource::None => None,
        }
    }

    /// Calculate CPU power estimate based on load
    fn calculate_cpu_power(&self, cpu_info: &CpuInfo) -> f64 {
        // Power scales roughly with load, but not linearly
        // Idle power is typically 10-20% of TDP
        // Formula: idle_power + (load_factor * active_power)
        let idle_ratio = 0.15;
        let load_factor = cpu_info.average_load / 100.0;

        let idle_power = self.cpu_tdp_estimate * idle_ratio;
        let active_power = self.cpu_tdp_estimate * (1.0 - idle_ratio);

        idle_power + (load_factor * active_power)
    }

    /// Estimate system base power (motherboard, RAM, storage, etc.)
    fn estimate_base_power(&self) -> f64 {
        if self.is_laptop {
            // Laptops have more efficient components
            10.0
        } else {
            // Desktop base power
            30.0
        }
    }

    /// Get total power consumption
    pub fn get_power_watts(&self) -> Result<f64> {
        let mut total_power = 0.0;

        // Get CPU power
        let cpu_info = self
            .get_cpu_info_wmi()
            .unwrap_or_else(|| self.get_cpu_info_sysinfo());
        let cpu_power = self.calculate_cpu_power(&cpu_info);
        total_power += cpu_power;

        // Add GPU power if available
        if let Some(gpu_info) = self.get_gpu_power() {
            total_power += gpu_info.power_watts;
        }

        // Add base system power
        total_power += self.estimate_base_power();

        Ok(total_power)
    }

    /// Get detailed power reading with component breakdown
    pub fn get_reading(&self) -> Result<PowerReading> {
        let mut components = HashMap::new();
        let mut total_power = 0.0;
        let mut has_real_reading = false;

        // Get CPU power
        let cpu_info = self
            .get_cpu_info_wmi()
            .unwrap_or_else(|| self.get_cpu_info_sysinfo());
        let cpu_power = self.calculate_cpu_power(&cpu_info);
        components.insert("cpu".to_string(), cpu_power);
        total_power += cpu_power;

        // Get GPU power if available
        if let Some(gpu_info) = self.get_gpu_power() {
            components.insert("gpu".to_string(), gpu_info.power_watts);
            components.insert(
                format!("gpu_{}", gpu_info.name.to_lowercase().replace(' ', "_")),
                gpu_info.power_watts,
            );
            total_power += gpu_info.power_watts;
            has_real_reading = true; // GPU power is a real measurement
        }

        // Add battery info if available
        if let Some(battery) = self.get_battery_info() {
            components.insert(
                "battery_percent".to_string(),
                battery.charge_remaining as f64,
            );
            if battery.is_discharging {
                components.insert("battery_discharging".to_string(), 1.0);
            }
        }

        // Base system power
        let base_power = self.estimate_base_power();
        components.insert("base".to_string(), base_power);
        total_power += base_power;

        // Determine source name
        let source = if self.wmi.is_some() {
            match self.gpu_source {
                GpuSource::Nvidia => "wmi+nvidia",
                GpuSource::Amd => "wmi+amd",
                GpuSource::None => "wmi",
            }
        } else {
            match self.gpu_source {
                GpuSource::Nvidia => "sysinfo+nvidia",
                GpuSource::Amd => "sysinfo+amd",
                GpuSource::None => "sysinfo-estimated",
            }
        };

        // Reading is estimated if no GPU (only real measurement we can get)
        let is_estimated = !has_real_reading;

        Ok(PowerReading::new(total_power, source, is_estimated).with_components(components))
    }
}

impl PowerSource for WmiMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        self.get_power_watts()
    }

    fn get_reading(&self) -> Result<PowerReading> {
        self.get_reading()
    }

    fn name(&self) -> &str {
        if self.wmi.is_some() {
            match self.gpu_source {
                GpuSource::Nvidia => "Windows WMI + NVIDIA",
                GpuSource::Amd => "Windows WMI + AMD",
                GpuSource::None => "Windows WMI",
            }
        } else {
            match self.gpu_source {
                GpuSource::Nvidia => "Windows Estimation + NVIDIA",
                GpuSource::Amd => "Windows Estimation + AMD",
                GpuSource::None => "Windows Estimation",
            }
        }
    }

    fn is_estimated(&self) -> bool {
        // We have real GPU readings if nvidia-smi or rocm-smi is available
        // CPU/base power is always estimated on Windows without specialized tools
        self.gpu_source == GpuSource::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_tdp_estimation() {
        assert_eq!(WmiMonitor::estimate_cpu_tdp(2), 35.0);
        assert_eq!(WmiMonitor::estimate_cpu_tdp(4), 65.0);
        assert_eq!(WmiMonitor::estimate_cpu_tdp(8), 105.0);
        assert_eq!(WmiMonitor::estimate_cpu_tdp(16), 150.0);
        assert_eq!(WmiMonitor::estimate_cpu_tdp(32), 200.0);
    }

    #[test]
    fn test_power_line_extraction() {
        assert_eq!(
            WmiMonitor::extract_power_from_line("Average Graphics Package Power (W): 45.5"),
            Some(45.5)
        );
        assert_eq!(
            WmiMonitor::extract_power_from_line("Power: 100W"),
            Some(100.0)
        );
    }

    #[test]
    fn test_gpu_source_display() {
        assert_eq!(format!("{:?}", GpuSource::Nvidia), "Nvidia");
        assert_eq!(format!("{:?}", GpuSource::Amd), "Amd");
        assert_eq!(format!("{:?}", GpuSource::None), "None");
    }
}
