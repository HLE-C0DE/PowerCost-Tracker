//! Windows power monitoring implementations
//!
//! Uses sysinfo for CPU monitoring and nvidia-smi/rocm-smi for GPU power.
//! WMI is complex and has version-specific API changes, so we avoid it for simplicity.

use crate::core::{CpuMetrics, GpuMetrics, MemoryMetrics, PowerReading, ProcessMetrics, Result, SystemMetrics};
use crate::hardware::PowerSource;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;

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

/// CPU load information
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

/// Cached value with timestamp
struct CachedValue<T> {
    value: T,
    timestamp: std::time::Instant,
}

impl<T: Clone> CachedValue<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            timestamp: std::time::Instant::now(),
        }
    }

    fn get(&self, max_age_ms: u64) -> Option<T> {
        if self.timestamp.elapsed().as_millis() < max_age_ms as u128 {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

/// Windows power monitor using sysinfo + GPU tools
///
/// Combines multiple data sources for power monitoring:
/// - sysinfo for CPU load
/// - nvidia-smi for NVIDIA GPU power
/// - rocm-smi for AMD GPU power
pub struct WmiMonitor {
    /// Detected GPU monitoring source
    gpu_source: GpuSource,
    /// Sysinfo for CPU data
    sys: Mutex<sysinfo::System>,
    /// Cached TDP estimate for CPU (watts)
    cpu_tdp_estimate: f64,
    /// Whether this is a laptop (has battery)
    is_laptop: bool,
    /// Cached GPU power reading (nvidia-smi is slow)
    gpu_cache: Mutex<Option<CachedValue<Option<GpuInfo>>>>,
    /// Cached GPU metrics (full metrics from nvidia-smi)
    gpu_metrics_cache: Mutex<Option<CachedValue<Option<crate::core::GpuMetrics>>>>,
    /// Cached CPU temperature (powershell is slow)
    cpu_temp_cache: Mutex<Option<CachedValue<Option<f64>>>>,
}

impl WmiMonitor {
    /// Create a new power monitor
    pub fn new() -> Result<Self> {
        // Initialize sysinfo
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();

        // Wait a bit and refresh again for accurate readings
        // sysinfo requires two consecutive calls to get accurate CPU usage
        std::thread::sleep(std::time::Duration::from_millis(100));
        sys.refresh_cpu_usage();

        // Detect GPU monitoring source
        let gpu_source = Self::detect_gpu_source();
        log::info!("GPU monitoring source: {:?}", gpu_source);

        // Estimate CPU TDP based on core count
        let cpu_count = sys.cpus().len();
        let cpu_tdp_estimate = Self::estimate_cpu_tdp(cpu_count);

        // Check if this is a laptop
        let is_laptop = Self::check_is_laptop();

        Ok(Self {
            gpu_source,
            sys: Mutex::new(sys),
            cpu_tdp_estimate,
            is_laptop,
            gpu_cache: Mutex::new(None),
            gpu_metrics_cache: Mutex::new(None),
            cpu_temp_cache: Mutex::new(None),
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

    /// Check if running on a laptop (check for battery via PowerShell)
    fn check_is_laptop() -> bool {
        // Use PowerShell to check for battery
        if let Ok(output) = Command::new("powershell")
            .args(["-Command", "(Get-WmiObject Win32_Battery).EstimatedChargeRemaining"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // If we get a number back, there's a battery
                return stdout.trim().parse::<u32>().is_ok();
            }
        }
        false
    }

    /// Get CPU info from sysinfo
    fn get_cpu_info(&self) -> CpuInfo {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();

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
                for line in stdout.lines() {
                    if line.contains("Average") && line.contains("Power") {
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
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(power) = value.get("Average Graphics Package Power (W)") {
                return power.as_f64();
            }
            if let Some(power) = value.get("power") {
                return power.as_f64();
            }
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
        for word in line.split_whitespace() {
            if let Ok(val) = word.trim_end_matches('W').parse::<f64>() {
                return Some(val);
            }
        }
        None
    }

    /// Get GPU power based on detected source (cached for 500ms)
    fn get_gpu_power(&self) -> Option<GpuInfo> {
        // Check cache first (500ms TTL)
        {
            let cache = self.gpu_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(500) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data
        let result = match self.gpu_source {
            GpuSource::Nvidia => self.get_nvidia_gpu_power(),
            GpuSource::Amd => self.get_amd_gpu_power(),
            GpuSource::None => None,
        };

        // Update cache
        {
            let mut cache = self.gpu_cache.lock().unwrap();
            *cache = Some(CachedValue::new(result.clone()));
        }

        result
    }

    /// Calculate CPU power estimate based on load
    fn calculate_cpu_power(&self, cpu_info: &CpuInfo) -> f64 {
        let idle_ratio = 0.15;
        let load_factor = cpu_info.average_load / 100.0;

        let idle_power = self.cpu_tdp_estimate * idle_ratio;
        let active_power = self.cpu_tdp_estimate * (1.0 - idle_ratio);

        idle_power + (load_factor * active_power)
    }

    /// Estimate system base power (motherboard, RAM, storage, etc.)
    fn estimate_base_power(&self) -> f64 {
        if self.is_laptop {
            10.0
        } else {
            30.0
        }
    }

    /// Get total power consumption
    pub fn get_power_watts(&self) -> Result<f64> {
        let mut total_power = 0.0;

        // Get CPU power
        let cpu_info = self.get_cpu_info();
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
        let cpu_info = self.get_cpu_info();
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
            has_real_reading = true;
        }

        // Base system power
        let base_power = self.estimate_base_power();
        components.insert("base".to_string(), base_power);
        total_power += base_power;

        // Determine source name
        let source = match self.gpu_source {
            GpuSource::Nvidia => "sysinfo+nvidia",
            GpuSource::Amd => "sysinfo+amd",
            GpuSource::None => "sysinfo-estimated",
        };

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
        match self.gpu_source {
            GpuSource::Nvidia => "Windows Estimation + NVIDIA",
            GpuSource::Amd => "Windows Estimation + AMD",
            GpuSource::None => "Windows Estimation",
        }
    }

    fn is_estimated(&self) -> bool {
        self.gpu_source == GpuSource::None
    }
}

// ===== System Metrics Implementation =====

impl WmiMonitor {
    /// Get comprehensive system metrics including CPU, GPU, and memory
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        // CPU metrics
        let cpu_usage: f64 = sys.cpus().iter().map(|c| c.cpu_usage() as f64).sum::<f64>() / sys.cpus().len() as f64;
        let per_core_usage: Vec<f64> = sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect();

        let cpu_name = if sys.cpus().is_empty() {
            "Unknown CPU".to_string()
        } else {
            sys.cpus()[0].brand().to_string()
        };

        let cpu_freq = sys.cpus().first().map(|c| c.frequency());

        // Get CPU temperature via WMI (if available)
        let cpu_temp = self.get_cpu_temperature();

        let cpu = CpuMetrics {
            name: cpu_name,
            usage_percent: cpu_usage,
            per_core_usage,
            frequency_mhz: cpu_freq,
            temperature_celsius: cpu_temp,
            core_count: sys.physical_core_count().unwrap_or(0),
            thread_count: sys.cpus().len(),
        };

        // GPU metrics
        let gpu = self.get_gpu_metrics();

        // Memory metrics
        let memory = MemoryMetrics {
            used_bytes: sys.used_memory(),
            total_bytes: sys.total_memory(),
            usage_percent: (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0,
        };

        Ok(SystemMetrics {
            cpu,
            gpu,
            memory,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Get top N processes by CPU usage with optional pinned processes
    pub fn get_top_processes(&self, limit: usize) -> Result<Vec<ProcessMetrics>> {
        self.get_top_processes_with_pinned(limit, &[])
    }

    /// Get top N processes by CPU usage, including pinned processes
    pub fn get_top_processes_with_pinned(&self, limit: usize, pinned_names: &[String]) -> Result<Vec<ProcessMetrics>> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_processes();

        let total_memory = sys.total_memory();

        let mut processes: Vec<ProcessMetrics> = sys
            .processes()
            .iter()
            .map(|(pid, process)| {
                let memory_bytes = process.memory();
                let name = process.name().to_string();
                let is_pinned = pinned_names.iter().any(|p| p.eq_ignore_ascii_case(&name));
                ProcessMetrics {
                    pid: pid.as_u32(),
                    name,
                    cpu_percent: process.cpu_usage() as f64,
                    memory_bytes,
                    memory_percent: (memory_bytes as f64 / total_memory as f64) * 100.0,
                    gpu_percent: None, // GPU per-process not available via sysinfo
                    is_pinned,
                }
            })
            .collect();

        // Separate pinned and non-pinned
        let (mut pinned, mut others): (Vec<_>, Vec<_>) = processes.into_iter()
            .partition(|p| p.is_pinned);

        // Sort both by CPU usage descending
        pinned.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
        others.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N from others, then prepend pinned
        let remaining_slots = limit.saturating_sub(pinned.len());
        others.truncate(remaining_slots);

        // Combine: pinned first, then top others
        pinned.extend(others);

        Ok(pinned)
    }

    /// Get all processes (for advanced/discovery mode)
    pub fn get_all_processes(&self) -> Result<Vec<ProcessMetrics>> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_processes();

        let total_memory = sys.total_memory();

        let mut processes: Vec<ProcessMetrics> = sys
            .processes()
            .iter()
            .filter(|(_, process)| process.cpu_usage() > 0.0 || process.memory() > 0)
            .map(|(pid, process)| {
                let memory_bytes = process.memory();
                ProcessMetrics {
                    pid: pid.as_u32(),
                    name: process.name().to_string(),
                    cpu_percent: process.cpu_usage() as f64,
                    memory_bytes,
                    memory_percent: (memory_bytes as f64 / total_memory as f64) * 100.0,
                    gpu_percent: None,
                    is_pinned: false,
                }
            })
            .collect();

        // Sort by CPU usage descending
        processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));

        Ok(processes)
    }

    /// Get CPU temperature via WMI (cached for 3 seconds - powershell is slow)
    fn get_cpu_temperature(&self) -> Option<f64> {
        // Check cache first (3000ms TTL - temperature doesn't change rapidly)
        {
            let cache = self.cpu_temp_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(3000) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data
        let result = self.fetch_cpu_temperature();

        // Update cache
        {
            let mut cache = self.cpu_temp_cache.lock().unwrap();
            *cache = Some(CachedValue::new(result));
        }

        result
    }

    /// Actually fetch CPU temperature (slow - calls powershell)
    fn fetch_cpu_temperature(&self) -> Option<f64> {
        // Try WMI query for thermal zone temperature
        // Temperature is in tenths of Kelvin, convert: (value / 10) - 273.15
        if let Ok(output) = Command::new("powershell")
            .args([
                "-Command",
                "Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace root/wmi 2>$null | Select-Object -First 1 -ExpandProperty CurrentTemperature"
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(temp_decikelvin) = stdout.trim().parse::<f64>() {
                    let temp_celsius = (temp_decikelvin / 10.0) - 273.15;
                    if temp_celsius > 0.0 && temp_celsius < 150.0 {
                        return Some(temp_celsius);
                    }
                }
            }
        }

        // Try Open Hardware Monitor WMI if available
        if let Ok(output) = Command::new("powershell")
            .args([
                "-Command",
                "Get-WmiObject Sensor -Namespace root/OpenHardwareMonitor 2>$null | Where-Object { $_.SensorType -eq 'Temperature' -and $_.Name -like '*CPU*' } | Select-Object -First 1 -ExpandProperty Value"
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(temp) = stdout.trim().parse::<f64>() {
                    if temp > 0.0 && temp < 150.0 {
                        return Some(temp);
                    }
                }
            }
        }

        None
    }

    /// Get GPU metrics including usage, power, temperature, and VRAM (cached for 500ms)
    fn get_gpu_metrics(&self) -> Option<GpuMetrics> {
        // Check cache first (500ms TTL)
        {
            let cache = self.gpu_metrics_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(500) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data
        let result = match self.gpu_source {
            GpuSource::Nvidia => self.get_nvidia_gpu_metrics(),
            GpuSource::Amd => self.get_amd_gpu_metrics(),
            GpuSource::None => None,
        };

        // Update cache
        {
            let mut cache = self.gpu_metrics_cache.lock().unwrap();
            *cache = Some(CachedValue::new(result.clone()));
        }

        result
    }

    /// Get NVIDIA GPU metrics via nvidia-smi
    fn get_nvidia_gpu_metrics(&self) -> Option<GpuMetrics> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,utilization.gpu,power.draw,temperature.gpu,memory.used,memory.total,clocks.gr",
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

        if parts.len() >= 7 {
            Some(GpuMetrics {
                name: parts[0].to_string(),
                usage_percent: parts[1].parse().ok(),
                power_watts: parts[2].parse().ok(),
                temperature_celsius: parts[3].parse().ok(),
                vram_used_mb: parts[4].parse().ok(),
                vram_total_mb: parts[5].parse().ok(),
                clock_mhz: parts[6].parse().ok(),
                source: "nvidia-smi".to_string(),
            })
        } else {
            None
        }
    }

    /// Get AMD GPU metrics via rocm-smi or amd-smi
    fn get_amd_gpu_metrics(&self) -> Option<GpuMetrics> {
        // Try rocm-smi first
        if let Ok(output) = Command::new("rocm-smi")
            .args(["--showuse", "--showpower", "--showtemp", "--showmemuse", "--json"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(metrics) = self.parse_rocm_smi_metrics(&stdout) {
                    return Some(metrics);
                }
            }
        }

        // Try amd-smi as fallback
        if let Ok(output) = Command::new("amd-smi")
            .args(["metric", "--json"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(metrics) = self.parse_amd_smi_metrics(&stdout) {
                    return Some(metrics);
                }
            }
        }

        // Fallback: just get power info
        if let Some(gpu_info) = self.get_amd_gpu_power() {
            return Some(GpuMetrics {
                name: gpu_info.name,
                usage_percent: None,
                power_watts: Some(gpu_info.power_watts),
                temperature_celsius: None,
                vram_used_mb: None,
                vram_total_mb: None,
                clock_mhz: None,
                source: "rocm-smi".to_string(),
            });
        }

        None
    }

    /// Parse rocm-smi JSON output to extract GPU metrics
    fn parse_rocm_smi_metrics(&self, json_str: &str) -> Option<GpuMetrics> {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            // rocm-smi JSON structure varies, try common paths
            let card = value.get("card0").or_else(|| value.as_object()?.values().next())?;

            Some(GpuMetrics {
                name: "AMD GPU".to_string(),
                usage_percent: card.get("GPU use (%)").and_then(|v| v.as_f64())
                    .or_else(|| card.get("GPU Usage").and_then(|v| v.as_f64())),
                power_watts: card.get("Average Graphics Package Power (W)").and_then(|v| v.as_f64())
                    .or_else(|| card.get("power").and_then(|v| v.as_f64())),
                temperature_celsius: card.get("Temperature (Sensor edge) (C)").and_then(|v| v.as_f64())
                    .or_else(|| card.get("temperature").and_then(|v| v.as_f64())),
                vram_used_mb: card.get("VRAM Total Used Memory (B)").and_then(|v| v.as_u64()).map(|v| v / 1_000_000),
                vram_total_mb: card.get("VRAM Total Memory (B)").and_then(|v| v.as_u64()).map(|v| v / 1_000_000),
                clock_mhz: card.get("sclk clock speed (MHz)").and_then(|v| v.as_u64()),
                source: "rocm-smi".to_string(),
            })
        } else {
            None
        }
    }

    /// Parse amd-smi JSON output to extract GPU metrics
    fn parse_amd_smi_metrics(&self, json_str: &str) -> Option<GpuMetrics> {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(arr) = value.as_array() {
                if let Some(first) = arr.first() {
                    return Some(GpuMetrics {
                        name: first.get("asic").and_then(|a| a.get("name")).and_then(|n| n.as_str()).unwrap_or("AMD GPU").to_string(),
                        usage_percent: first.get("usage").and_then(|u| u.get("gfx_activity")).and_then(|v| v.as_f64()),
                        power_watts: first.get("power").and_then(|p| p.get("socket_power")).and_then(|v| v.as_f64()),
                        temperature_celsius: first.get("temperature").and_then(|t| t.get("edge")).and_then(|v| v.as_f64()),
                        vram_used_mb: first.get("vram").and_then(|v| v.get("used")).and_then(|v| v.as_u64()),
                        vram_total_mb: first.get("vram").and_then(|v| v.get("total")).and_then(|v| v.as_u64()),
                        clock_mhz: first.get("clock").and_then(|c| c.get("gfx")).and_then(|v| v.as_u64()),
                        source: "amd-smi".to_string(),
                    });
                }
            }
        }
        None
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
