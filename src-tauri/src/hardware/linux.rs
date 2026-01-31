//! Linux power monitoring and system metrics implementations
//!
//! Supports:
//! - Intel RAPL (Running Average Power Limit) via /sys/class/powercap
//! - AMD hwmon via /sys/class/hwmon
//! - Battery power via /sys/class/power_supply
//! - System metrics: CPU temp/freq, fans, GPU (AMD sysfs), processes

use crate::core::{CpuMetrics, DetailedMetrics, Error, FanMetrics, FanReading, GpuMetrics,
                   MemoryMetrics, PowerReading, ProcessMetrics, Result, SystemMetrics, VoltageReading};
use crate::hardware::PowerSource;
use crate::hardware::nvml_gpu;
use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::ProcessRefreshKind;

// ===== Power Source Implementations (RAPL, hwmon, battery) =====

/// Intel RAPL power monitor
pub struct RaplMonitor {
    energy_path: PathBuf,
    max_energy: u64,
    last_energy: Mutex<u64>,
    last_time: Mutex<Instant>,
    component_paths: HashMap<String, PathBuf>,
}

impl RaplMonitor {
    pub fn new() -> Result<Self> {
        let rapl_base = Path::new("/sys/class/powercap/intel-rapl");

        if !rapl_base.exists() {
            return Err(Error::HardwareNotSupported("RAPL not available".to_string()));
        }

        let package_path = rapl_base.join("intel-rapl:0");
        if !package_path.exists() {
            return Err(Error::HardwareNotSupported("RAPL package not found".to_string()));
        }

        let energy_path = package_path.join("energy_uj");
        if !energy_path.exists() {
            return Err(Error::PermissionDenied(
                "Cannot read RAPL energy (try running with sudo or add CAP_SYS_RAWIO)".to_string(),
            ));
        }

        let max_energy_path = package_path.join("max_energy_range_uj");
        let max_energy: u64 = fs::read_to_string(&max_energy_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(u64::MAX);

        let mut component_paths = HashMap::new();
        for entry in fs::read_dir(&package_path).into_iter().flatten() {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let name_path = path.join("name");
                    let energy_uj_path = path.join("energy_uj");
                    if name_path.exists() && energy_uj_path.exists() {
                        if let Ok(name) = fs::read_to_string(&name_path) {
                            component_paths.insert(name.trim().to_string(), energy_uj_path);
                        }
                    }
                }
            }
        }

        let initial_energy: u64 = fs::read_to_string(&energy_path)
            .map_err(|e| Error::Io(e))?
            .trim()
            .parse()
            .map_err(|_| Error::PowerMonitor("Failed to parse energy value".to_string()))?;

        Ok(Self {
            energy_path,
            max_energy,
            last_energy: Mutex::new(initial_energy),
            last_time: Mutex::new(Instant::now()),
            component_paths,
        })
    }

    fn read_energy(&self) -> Result<u64> {
        fs::read_to_string(&self.energy_path)?
            .trim()
            .parse()
            .map_err(|_| Error::PowerMonitor("Failed to parse energy value".to_string()))
    }

    fn get_power(&self) -> Result<f64> {
        let current_energy = self.read_energy()?;
        let current_time = Instant::now();

        let mut last_energy = self.last_energy.lock().unwrap();
        let mut last_time = self.last_time.lock().unwrap();

        let energy_diff = if current_energy >= *last_energy {
            current_energy - *last_energy
        } else {
            (self.max_energy - *last_energy) + current_energy
        };

        let time_diff = current_time.duration_since(*last_time);
        *last_energy = current_energy;
        *last_time = current_time;

        let power_watts = if time_diff.as_secs_f64() > 0.0 {
            (energy_diff as f64) / time_diff.as_secs_f64() / 1_000_000.0
        } else {
            0.0
        };

        Ok(power_watts)
    }
}

/// AMD/generic hwmon power monitor
pub struct HwmonMonitor {
    power_path: PathBuf,
}

impl HwmonMonitor {
    pub fn new() -> Result<Self> {
        let hwmon_base = Path::new("/sys/class/hwmon");

        if !hwmon_base.exists() {
            return Err(Error::HardwareNotSupported("hwmon not available".to_string()));
        }

        for entry in fs::read_dir(hwmon_base)? {
            let entry = entry?;
            let path = entry.path();
            for i in 1..=10 {
                let power_path = path.join(format!("power{}_input", i));
                if power_path.exists() {
                    log::info!("Found hwmon power sensor at {:?}", power_path);
                    return Ok(Self { power_path });
                }
            }
        }

        Err(Error::HardwareNotSupported("No hwmon power sensor found".to_string()))
    }

    fn get_power(&self) -> Result<f64> {
        let power_uw: f64 = fs::read_to_string(&self.power_path)?
            .trim()
            .parse()
            .map_err(|_| Error::PowerMonitor("Failed to parse power value".to_string()))?;
        Ok(power_uw / 1_000_000.0)
    }
}

/// Battery-based power monitor (for laptops)
pub struct BatteryMonitor {
    power_path: PathBuf,
}

impl BatteryMonitor {
    pub fn new() -> Result<Self> {
        let power_supply = Path::new("/sys/class/power_supply");

        if !power_supply.exists() {
            return Err(Error::HardwareNotSupported("power_supply not available".to_string()));
        }

        for entry in fs::read_dir(power_supply)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with("BAT") {
                let power_path = path.join("power_now");
                if power_path.exists() {
                    return Ok(Self { power_path });
                }
            }
        }

        Err(Error::HardwareNotSupported("No battery power sensor found".to_string()))
    }

    fn get_power(&self) -> Result<f64> {
        let power_uw: f64 = fs::read_to_string(&self.power_path)?
            .trim()
            .parse()
            .map_err(|_| Error::PowerMonitor("Failed to parse battery power".to_string()))?;
        Ok(power_uw / 1_000_000.0)
    }
}

/// Which underlying power source is used
enum InnerPowerSource {
    Rapl(RaplMonitor),
    Hwmon(HwmonMonitor),
    Battery(BatteryMonitor),
}

impl InnerPowerSource {
    fn get_power_watts(&self) -> Result<f64> {
        match self {
            InnerPowerSource::Rapl(m) => m.get_power(),
            InnerPowerSource::Hwmon(m) => m.get_power(),
            InnerPowerSource::Battery(m) => m.get_power(),
        }
    }

    fn name(&self) -> &str {
        match self {
            InnerPowerSource::Rapl(_) => "Intel RAPL",
            InnerPowerSource::Hwmon(_) => "Linux hwmon",
            InnerPowerSource::Battery(_) => "Battery sensor",
        }
    }

    fn source_tag(&self) -> &str {
        match self {
            InnerPowerSource::Rapl(_) => "rapl",
            InnerPowerSource::Hwmon(_) => "hwmon",
            InnerPowerSource::Battery(_) => "battery",
        }
    }
}

// ===== Hwmon Discovery =====

/// Maps hwmon chip names to their sysfs paths
struct HwmonDiscovery {
    /// chip name → hwmon path (e.g. "coretemp" → "/sys/class/hwmon/hwmon3")
    chips: HashMap<String, PathBuf>,
}

impl HwmonDiscovery {
    fn scan() -> Self {
        let mut chips = HashMap::new();
        let hwmon_base = Path::new("/sys/class/hwmon");

        if let Ok(entries) = fs::read_dir(hwmon_base) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name_path = path.join("name");
                if let Ok(name) = fs::read_to_string(&name_path) {
                    let name = name.trim().to_string();
                    chips.insert(name, path);
                }
            }
        }

        log::info!("hwmon discovery: found chips {:?}", chips.keys().collect::<Vec<_>>());
        HwmonDiscovery { chips }
    }

    /// Get path for a chip by name (e.g. "coretemp", "k10temp", "amdgpu")
    fn get_chip_path(&self, name: &str) -> Option<&Path> {
        self.chips.get(name).map(|p| p.as_path())
    }
}

// ===== Linux System Monitor =====

/// Comprehensive Linux system monitor wrapping a power source and adding system metrics
pub struct LinuxSystemMonitor {
    inner_power: InnerPowerSource,
    sys: Mutex<sysinfo::System>,
    hwmon: HwmonDiscovery,
    /// NVML state for NVIDIA GPU (if available)
    nvml_state: Option<nvml_gpu::NvmlState>,
}

impl LinuxSystemMonitor {
    /// Create a new LinuxSystemMonitor wrapping the given power source
    fn new_with_source(source: InnerPowerSource) -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());
        std::thread::sleep(std::time::Duration::from_millis(200));
        sys.refresh_cpu_usage();
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());

        let hwmon = HwmonDiscovery::scan();
        let nvml_state = nvml_gpu::init_nvml();

        if nvml_state.is_some() {
            log::info!("NVML initialized for Linux GPU monitoring");
        }

        LinuxSystemMonitor {
            inner_power: source,
            sys: Mutex::new(sys),
            hwmon,
            nvml_state,
        }
    }

    /// Try to create with RAPL
    pub fn try_rapl() -> Result<Self> {
        let rapl = RaplMonitor::new()?;
        Ok(Self::new_with_source(InnerPowerSource::Rapl(rapl)))
    }

    /// Try to create with hwmon
    pub fn try_hwmon() -> Result<Self> {
        let hwmon = HwmonMonitor::new()?;
        Ok(Self::new_with_source(InnerPowerSource::Hwmon(hwmon)))
    }

    /// Try to create with battery
    pub fn try_battery() -> Result<Self> {
        let battery = BatteryMonitor::new()?;
        Ok(Self::new_with_source(InnerPowerSource::Battery(battery)))
    }

    // ----- CPU Temperature -----

    /// Read CPU temperature from hwmon (coretemp for Intel, k10temp for AMD)
    fn get_cpu_temperature(&self) -> (Option<f64>, Option<Vec<f64>>) {
        // Try coretemp (Intel)
        if let Some(path) = self.hwmon.get_chip_path("coretemp") {
            return self.read_coretemp_temps(path);
        }
        // Try k10temp (AMD)
        if let Some(path) = self.hwmon.get_chip_path("k10temp") {
            return self.read_k10temp(path);
        }
        // Try zenpower (AMD alternative)
        if let Some(path) = self.hwmon.get_chip_path("zenpower") {
            return self.read_k10temp(path); // Same interface
        }
        (None, None)
    }

    /// Read Intel coretemp: temp1_input is package, temp2+ are per-core
    fn read_coretemp_temps(&self, path: &Path) -> (Option<f64>, Option<Vec<f64>>) {
        let mut package_temp = None;
        let mut core_temps = Vec::new();

        // temp1 is usually the package, temp2+ are cores
        for i in 1..=128 {
            let temp_path = path.join(format!("temp{}_input", i));
            if let Ok(content) = fs::read_to_string(&temp_path) {
                if let Ok(millideg) = content.trim().parse::<f64>() {
                    let celsius = millideg / 1000.0;
                    if celsius > 0.0 && celsius < 150.0 {
                        if i == 1 {
                            package_temp = Some(celsius);
                        } else {
                            core_temps.push(celsius);
                        }
                    }
                }
            } else {
                break; // No more temp inputs
            }
        }

        let per_core = if core_temps.is_empty() { None } else { Some(core_temps) };
        // If no package temp, use average of core temps
        if package_temp.is_none() {
            if let Some(ref cores) = per_core {
                package_temp = Some(cores.iter().sum::<f64>() / cores.len() as f64);
            }
        }

        (package_temp, per_core)
    }

    /// Read AMD k10temp/zenpower: temp1_input is Tctl (or Tdie)
    fn read_k10temp(&self, path: &Path) -> (Option<f64>, Option<Vec<f64>>) {
        let mut temps = Vec::new();
        let mut main_temp = None;

        for i in 1..=10 {
            let temp_path = path.join(format!("temp{}_input", i));
            if let Ok(content) = fs::read_to_string(&temp_path) {
                if let Ok(millideg) = content.trim().parse::<f64>() {
                    let celsius = millideg / 1000.0;
                    if celsius > 0.0 && celsius < 150.0 {
                        if i == 1 {
                            main_temp = Some(celsius);
                        }
                        temps.push(celsius);
                    }
                }
            }
        }

        let per_core = if temps.len() > 1 { Some(temps) } else { None };
        (main_temp, per_core)
    }

    // ----- CPU Frequency -----

    /// Read per-core frequency from cpufreq sysfs
    fn get_per_core_frequency(&self) -> Option<Vec<u64>> {
        let mut freqs = Vec::new();
        let cpu_base = Path::new("/sys/devices/system/cpu");

        for i in 0..512 {
            let freq_path = cpu_base
                .join(format!("cpu{}", i))
                .join("cpufreq/scaling_cur_freq");
            if let Ok(content) = fs::read_to_string(&freq_path) {
                if let Ok(khz) = content.trim().parse::<u64>() {
                    freqs.push(khz / 1000); // kHz → MHz
                }
            } else if i > 0 {
                break; // No more CPUs
            }
        }

        if freqs.is_empty() { None } else { Some(freqs) }
    }

    // ----- Fans -----

    /// Read fan speeds from all hwmon chips
    fn get_fan_speeds(&self) -> Option<FanMetrics> {
        let mut fans = Vec::new();

        for (chip_name, chip_path) in &self.hwmon.chips {
            for i in 1..=10 {
                let fan_path = chip_path.join(format!("fan{}_input", i));
                if let Ok(content) = fs::read_to_string(&fan_path) {
                    if let Ok(rpm) = content.trim().parse::<u64>() {
                        // Try to get the label
                        let label = fs::read_to_string(chip_path.join(format!("fan{}_label", i)))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| format!("{} fan{}", chip_name, i));

                        fans.push(FanReading {
                            name: label,
                            speed_rpm: Some(rpm),
                            speed_percent: None,
                        });
                    }
                }
            }
        }

        if fans.is_empty() { None } else { Some(FanMetrics { fans }) }
    }

    // ----- Voltages -----

    /// Read voltage sensors from hwmon
    fn get_voltages(&self) -> Option<Vec<VoltageReading>> {
        let mut voltages = Vec::new();

        for (chip_name, chip_path) in &self.hwmon.chips {
            for i in 0..=15 {
                let in_path = chip_path.join(format!("in{}_input", i));
                if let Ok(content) = fs::read_to_string(&in_path) {
                    if let Ok(millivolts) = content.trim().parse::<f64>() {
                        let label = fs::read_to_string(chip_path.join(format!("in{}_label", i)))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| format!("{} in{}", chip_name, i));

                        voltages.push(VoltageReading {
                            name: label,
                            value_volts: millivolts / 1000.0,
                        });
                    }
                }
            }
        }

        if voltages.is_empty() { None } else { Some(voltages) }
    }

    // ----- GPU (AMD sysfs) -----

    /// Try to read AMD GPU metrics from DRM sysfs
    fn get_amd_gpu_sysfs(&self) -> Option<GpuMetrics> {
        // Find /sys/class/drm/card*/device with amdgpu driver
        let drm_base = Path::new("/sys/class/drm");
        if !drm_base.exists() {
            return None;
        }

        for entry in fs::read_dir(drm_base).ok()?.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Only look at cardN entries (not cardN-DP-1 etc.)
            if !name_str.starts_with("card") || name_str.contains('-') {
                continue;
            }

            let device_path = entry.path().join("device");
            if !device_path.exists() {
                continue;
            }

            // Check if this is an amdgpu device
            let gpu_busy = device_path.join("gpu_busy_percent");
            if !gpu_busy.exists() {
                continue;
            }

            let usage_percent = fs::read_to_string(&gpu_busy)
                .ok()
                .and_then(|s| s.trim().parse::<f64>().ok());

            // Find the hwmon subdir for this GPU
            let hwmon_path = self.find_gpu_hwmon(&device_path);

            let temperature_celsius = hwmon_path.as_ref().and_then(|p| {
                fs::read_to_string(p.join("temp1_input"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|md| md / 1000.0)
            });

            let power_watts = hwmon_path.as_ref().and_then(|p| {
                fs::read_to_string(p.join("power1_average"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|uw| uw / 1_000_000.0)
            });

            let fan_speed_rpm = hwmon_path.as_ref().and_then(|p| {
                fs::read_to_string(p.join("fan1_input"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
            });

            // VRAM
            let vram_total_mb = fs::read_to_string(device_path.join("mem_info_vram_total"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|b| b / (1024 * 1024));

            let vram_used_mb = fs::read_to_string(device_path.join("mem_info_vram_used"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|b| b / (1024 * 1024));

            // Clock from pp_dpm_sclk (current marked with *)
            let clock_mhz = self.parse_dpm_clock(&device_path.join("pp_dpm_sclk"));
            let memory_clock_mhz = self.parse_dpm_clock(&device_path.join("pp_dpm_mclk"));

            // GPU name from device marketing name or fallback
            let gpu_name = fs::read_to_string(device_path.join("product_name"))
                .or_else(|_| fs::read_to_string(device_path.join("device")))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "AMD GPU".to_string());

            return Some(GpuMetrics {
                name: gpu_name,
                usage_percent,
                power_watts,
                temperature_celsius,
                vram_used_mb,
                vram_total_mb,
                clock_mhz,
                source: "amdgpu-sysfs".to_string(),
                memory_clock_mhz,
                fan_speed_percent: fan_speed_rpm.map(|_| 0), // RPM only, not %
            });
        }

        None
    }

    /// Find the hwmon subdirectory under a DRM device
    fn find_gpu_hwmon(&self, device_path: &Path) -> Option<PathBuf> {
        let hwmon_dir = device_path.join("hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.join("temp1_input").exists() || path.join("power1_average").exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Parse pp_dpm_sclk/pp_dpm_mclk to find current clock (line with *)
    fn parse_dpm_clock(&self, path: &Path) -> Option<u64> {
        let content = fs::read_to_string(path).ok()?;
        for line in content.lines() {
            if line.contains('*') {
                // Format: "1: 1800Mhz *"
                for word in line.split_whitespace() {
                    if let Some(mhz_str) = word.strip_suffix("Mhz").or_else(|| word.strip_suffix("MHz")) {
                        if let Ok(mhz) = mhz_str.parse::<u64>() {
                            return Some(mhz);
                        }
                    }
                }
            }
        }
        None
    }

    // ----- GPU (combined: NVML → AMD sysfs) -----

    fn get_gpu_metrics(&self) -> Option<GpuMetrics> {
        // Try NVML first (NVIDIA)
        if let Some(ref nvml) = self.nvml_state {
            if let Some(metrics) = nvml_gpu::query_gpu_metrics(nvml) {
                return Some(metrics);
            }
        }
        // Try AMD sysfs
        self.get_amd_gpu_sysfs()
    }

    fn get_gpu_power(&self) -> Option<f64> {
        if let Some(ref nvml) = self.nvml_state {
            if let Some((power, _)) = nvml_gpu::query_gpu_power(nvml) {
                return Some(power);
            }
        }
        // AMD sysfs power
        self.get_amd_gpu_sysfs().and_then(|g| g.power_watts)
    }

    // ----- System Metrics (full) -----

    fn get_system_metrics_impl(&self, extended: bool) -> Result<SystemMetrics> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let cpu_usage: f64 = sys.cpus().iter().map(|c| c.cpu_usage() as f64).sum::<f64>()
            / sys.cpus().len().max(1) as f64;
        let per_core_usage: Vec<f64> = sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect();
        let cpu_name = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown CPU".to_string());
        let cpu_freq = sys.cpus().first().map(|c| c.frequency());
        let physical_core_count = sys.physical_core_count().unwrap_or(0);
        let thread_count = sys.cpus().len();

        let used_memory = sys.used_memory();
        let total_memory = sys.total_memory();
        let used_swap = sys.used_swap();
        let total_swap = sys.total_swap();
        drop(sys);

        // CPU temperature
        let (cpu_temp, per_core_temp) = self.get_cpu_temperature();

        // Per-core frequency from cpufreq
        let per_core_frequency_mhz = self.get_per_core_frequency();

        let cpu = CpuMetrics {
            name: cpu_name,
            usage_percent: cpu_usage,
            per_core_usage,
            frequency_mhz: cpu_freq,
            temperature_celsius: cpu_temp,
            core_count: physical_core_count,
            thread_count,
            per_core_frequency_mhz,
            per_core_temperature: per_core_temp,
        };

        // GPU
        let gpu = self.get_gpu_metrics();

        // Fans (always try on Linux since reading sysfs is cheap)
        let fans = if extended { self.get_fan_speeds() } else { None };

        // Voltages
        let voltages = if extended { self.get_voltages() } else { None };

        // Memory
        let (swap_used, swap_total, swap_percent) = if total_swap > 0 {
            (Some(used_swap), Some(total_swap), Some((used_swap as f64 / total_swap as f64) * 100.0))
        } else {
            (None, None, None)
        };

        let memory = MemoryMetrics {
            used_bytes: used_memory,
            total_bytes: total_memory,
            usage_percent: (used_memory as f64 / total_memory as f64) * 100.0,
            swap_used_bytes: swap_used,
            swap_total_bytes: swap_total,
            swap_usage_percent: swap_percent,
            memory_speed_mhz: None, // Not easily available on Linux without dmidecode
            memory_type: None,
        };

        Ok(SystemMetrics {
            cpu,
            gpu,
            memory,
            timestamp: chrono::Utc::now().timestamp(),
            fans,
            voltages,
        })
    }

    // ----- Processes -----

    fn get_top_processes_impl(&self, limit: usize, pinned_names: &[String]) -> Result<Vec<ProcessMetrics>> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());
        sys.refresh_memory();

        let total_memory = sys.total_memory();

        let process_data: Vec<_> = sys.processes().iter()
            .map(|(pid, process)| {
                (pid.as_u32(), process.name().to_string(), process.cpu_usage() as f64, process.memory())
            })
            .collect();
        drop(sys);

        // GPU process usage from NVML
        let gpu_usage: HashMap<u32, f64> = self.nvml_state.as_ref()
            .map(nvml_gpu::query_gpu_processes)
            .unwrap_or_default();

        // Aggregate by name
        let mut aggregated: HashMap<String, ProcessMetrics> = HashMap::new();
        for (pid, name, cpu_percent, memory_bytes) in process_data {
            let is_pinned = pinned_names.iter().any(|p| p.eq_ignore_ascii_case(&name));
            let gpu_percent = gpu_usage.get(&pid).copied();

            let entry = aggregated.entry(name.clone()).or_insert(ProcessMetrics {
                pid,
                name,
                cpu_percent: 0.0,
                memory_bytes: 0,
                memory_percent: 0.0,
                gpu_percent: None,
                is_pinned: false,
            });
            entry.cpu_percent += cpu_percent;
            entry.memory_bytes += memory_bytes;
            entry.memory_percent += (memory_bytes as f64 / total_memory as f64) * 100.0;
            if let Some(gpu) = gpu_percent {
                entry.gpu_percent = Some(entry.gpu_percent.unwrap_or(0.0) + gpu);
            }
            if is_pinned {
                entry.is_pinned = true;
            }
        }

        // Clamp
        let processes: Vec<ProcessMetrics> = aggregated.into_values()
            .map(|mut p| {
                p.cpu_percent = p.cpu_percent.min(100.0 * 128.0); // Linux reports per-core, can exceed 100%
                p.memory_percent = p.memory_percent.min(100.0);
                if let Some(gpu) = p.gpu_percent {
                    p.gpu_percent = Some(gpu.min(100.0));
                }
                p
            })
            .collect();

        // Separate pinned and non-pinned
        let (mut pinned, mut others): (Vec<_>, Vec<_>) = processes.into_iter()
            .partition(|p| p.is_pinned);

        let usage_score = |p: &ProcessMetrics| -> f64 {
            let cpu = p.cpu_percent;
            let gpu = p.gpu_percent.unwrap_or(0.0);
            let mem = p.memory_percent;
            cpu * 0.4 + gpu * 0.4 + mem * 0.2
        };

        pinned.sort_by(|a, b| usage_score(b).partial_cmp(&usage_score(a)).unwrap_or(std::cmp::Ordering::Equal));
        others.sort_by(|a, b| usage_score(b).partial_cmp(&usage_score(a)).unwrap_or(std::cmp::Ordering::Equal));

        let remaining_slots = limit.saturating_sub(pinned.len());
        others.truncate(remaining_slots);
        pinned.extend(others);

        Ok(pinned)
    }

    pub fn get_all_processes_impl(&self) -> Result<Vec<ProcessMetrics>> {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());
        sys.refresh_memory();

        let total_memory = sys.total_memory();
        let process_data: Vec<_> = sys.processes().iter()
            .filter(|(_, process)| process.cpu_usage() > 0.0 || process.memory() > 0)
            .map(|(pid, process)| {
                (pid.as_u32(), process.name().to_string(), process.cpu_usage() as f64, process.memory())
            })
            .collect();
        drop(sys);

        let gpu_usage: HashMap<u32, f64> = self.nvml_state.as_ref()
            .map(nvml_gpu::query_gpu_processes)
            .unwrap_or_default();

        let mut aggregated: HashMap<String, ProcessMetrics> = HashMap::new();
        for (pid, name, cpu_percent, memory_bytes) in process_data {
            let gpu_percent = gpu_usage.get(&pid).copied();
            let entry = aggregated.entry(name.clone()).or_insert(ProcessMetrics {
                pid,
                name,
                cpu_percent: 0.0,
                memory_bytes: 0,
                memory_percent: 0.0,
                gpu_percent: None,
                is_pinned: false,
            });
            entry.cpu_percent += cpu_percent;
            entry.memory_bytes += memory_bytes;
            entry.memory_percent += (memory_bytes as f64 / total_memory as f64) * 100.0;
            if let Some(gpu) = gpu_percent {
                entry.gpu_percent = Some(entry.gpu_percent.unwrap_or(0.0) + gpu);
            }
        }

        let mut processes: Vec<ProcessMetrics> = aggregated.into_values().collect();
        processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));

        Ok(processes)
    }
}

// ===== PowerSource trait implementation =====

impl PowerSource for LinuxSystemMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        self.inner_power.get_power_watts()
    }

    fn get_power_watts_fast(&self) -> Result<(f64, f64, Option<f64>, Option<f64>)> {
        let power = self.inner_power.get_power_watts()?;
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        let cpu_usage = sys.cpus().iter().map(|c| c.cpu_usage() as f64).sum::<f64>()
            / sys.cpus().len().max(1) as f64;
        drop(sys);

        let gpu_power = self.get_gpu_power();
        let gpu_usage = self.nvml_state.as_ref()
            .and_then(|nvml| nvml_gpu::query_gpu_metrics(nvml))
            .and_then(|m| m.usage_percent);

        Ok((power, cpu_usage, gpu_usage, gpu_power))
    }

    fn collect_detailed_metrics(&self, limit: usize, pinned: &[String], extended: bool) -> Result<DetailedMetrics> {
        let system_metrics = self.get_system_metrics_impl(extended).ok();
        let top_processes = self.get_top_processes_impl(limit, pinned).unwrap_or_default();

        Ok(DetailedMetrics {
            system_metrics,
            top_processes,
            timestamp: chrono::Utc::now().timestamp(),
            extended_collected: extended,
        })
    }

    fn get_reading(&self) -> Result<PowerReading> {
        let power = self.inner_power.get_power_watts()?;
        Ok(PowerReading::new(power, self.inner_power.source_tag(), false))
    }

    fn name(&self) -> &str {
        self.inner_power.name()
    }

    fn is_estimated(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
