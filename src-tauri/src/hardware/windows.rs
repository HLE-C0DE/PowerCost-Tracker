//! Windows power monitoring implementations
//!
//! Uses sysinfo for CPU monitoring and nvidia-smi/rocm-smi for GPU power.
//! WMI is complex and has version-specific API changes, so we avoid it for simplicity.

use crate::core::{CpuMetrics, DetailedMetrics, FanMetrics, FanReading, GpuMetrics, MemoryMetrics, PowerReading, ProcessMetrics, Result, SystemMetrics};
use crate::hardware::PowerSource;
use crate::hardware::nvml_gpu;
use std::any::Any;
use std::collections::HashMap;
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::ProcessRefreshKind;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// PDH query handle for reading thermal zone temperature counters.
/// Lazily initialized on first use and reused across calls.
#[cfg(target_os = "windows")]
struct PdhThermalQuery {
    query: isize,
    counter: isize,
}

#[cfg(target_os = "windows")]
impl Drop for PdhThermalQuery {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Performance::PdhCloseQuery(self.query);
        }
    }
}

// SAFETY: PDH handles are thread-safe when access is serialized via Mutex
#[cfg(target_os = "windows")]
unsafe impl Send for PdhThermalQuery {}
#[cfg(target_os = "windows")]
unsafe impl Sync for PdhThermalQuery {}

/// Windows flag to hide console window when spawning processes
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Default timeout for GPU commands (nvidia-smi, rocm-smi, etc.)
const GPU_COMMAND_TIMEOUT_MS: u64 = 1500;

/// Run a command with a timeout. Returns None if timeout exceeded or command fails.
/// On Windows, hides the console window to prevent flashing.
fn run_command_with_timeout(program: &str, args: &[&str], timeout_ms: u64) -> Option<Output> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd.spawn().ok()?;

    let timeout = Duration::from_millis(timeout_ms);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished
                let stdout = child.stdout.take().map(|mut s| {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut s, &mut buf).ok();
                    buf
                }).unwrap_or_default();

                let stderr = child.stderr.take().map(|mut s| {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut s, &mut buf).ok();
                    buf
                }).unwrap_or_default();

                return Some(Output { status, stdout, stderr });
            }
            Ok(None) => {
                // Still running
                if start.elapsed() > timeout {
                    // Timeout - kill the process
                    let _ = child.kill();
                    let _ = child.wait(); // Reap the zombie
                    log::warn!("{} command timed out after {}ms", program, timeout_ms);
                    return None;
                }
                // Sleep briefly before checking again
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

/// Available GPU monitoring sources
#[derive(Debug, Clone, Copy, PartialEq)]
enum GpuSource {
    /// NVIDIA GPU via NVML library (fast, direct API)
    NvmlNvidia,
    /// NVIDIA GPU via nvidia-smi CLI (fallback)
    Nvidia,
    /// AMD GPU via rocm-smi
    Amd,
    /// No GPU monitoring available
    None,
}

/// CPU load information
#[derive(Debug, Clone)]
struct CpuInfo {
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
    /// NVML state for direct NVIDIA GPU access (if available)
    nvml_state: Option<nvml_gpu::NvmlState>,
    /// Sysinfo for CPU data
    sys: Mutex<sysinfo::System>,
    /// Cached TDP estimate for CPU (watts)
    cpu_tdp_estimate: f64,
    /// Whether this is a laptop (has battery)
    is_laptop: bool,
    /// Cached GPU power reading (used for CLI fallback; NVML is fast enough to skip cache)
    gpu_cache: Mutex<Option<CachedValue<Option<GpuInfo>>>>,
    /// Cached GPU metrics (full metrics)
    gpu_metrics_cache: Mutex<Option<CachedValue<Option<crate::core::GpuMetrics>>>>,
    /// Cached CPU temperature (powershell is slow)
    cpu_temp_cache: Mutex<Option<CachedValue<Option<f64>>>>,
    /// Cached per-process GPU usage (PID -> GPU% usage)
    gpu_process_cache: Mutex<Option<CachedValue<HashMap<u32, f64>>>>,
    /// Cached system fan speeds (WMI is slow, cache for 5s)
    fan_cache: Mutex<Option<CachedValue<Option<FanMetrics>>>>,
    /// Cached memory info: (speed_mhz, type_string) - permanent cache, RAM never changes at runtime
    memory_info_cache: Mutex<Option<(Option<u64>, Option<String>)>>,
    /// PDH query handle for thermal zone temperature (lazily initialized, reused)
    #[cfg(target_os = "windows")]
    pdh_thermal_query: Mutex<Option<PdhThermalQuery>>,
}

impl WmiMonitor {
    /// Create a new power monitor
    pub fn new() -> Result<Self> {
        // Initialize sysinfo
        let mut sys = sysinfo::System::new();

        // First refresh: establish baselines for CPU and processes
        sys.refresh_cpu_usage();
        sys.refresh_processes_specifics(
            ProcessRefreshKind::new().with_cpu().with_memory()
        );

        // Wait for baseline (200ms for process data)
        // sysinfo requires two consecutive calls to get accurate CPU/process usage
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Second refresh: now values will be accurate
        sys.refresh_cpu_usage();
        sys.refresh_processes_specifics(
            ProcessRefreshKind::new().with_cpu().with_memory()
        );

        // Try NVML first for NVIDIA GPU (fast, direct API)
        let nvml_state = nvml_gpu::init_nvml();
        let gpu_source = if nvml_state.is_some() {
            log::info!("Using NVML for NVIDIA GPU monitoring (direct API)");
            GpuSource::NvmlNvidia
        } else {
            // Fallback to CLI-based detection
            let source = Self::detect_gpu_source();
            log::info!("GPU monitoring source: {:?}", source);
            source
        };

        // Estimate CPU TDP based on core count
        let cpu_count = sys.cpus().len();
        let cpu_tdp_estimate = Self::estimate_cpu_tdp(cpu_count);

        // Check if this is a laptop
        let is_laptop = Self::check_is_laptop();

        Ok(Self {
            gpu_source,
            nvml_state,
            sys: Mutex::new(sys),
            cpu_tdp_estimate,
            is_laptop,
            gpu_cache: Mutex::new(None),
            gpu_metrics_cache: Mutex::new(None),
            cpu_temp_cache: Mutex::new(None),
            gpu_process_cache: Mutex::new(None),
            fan_cache: Mutex::new(None),
            memory_info_cache: Mutex::new(None),
            #[cfg(target_os = "windows")]
            pdh_thermal_query: Mutex::new(None),
        })
    }

    /// Detect available GPU monitoring tool
    fn detect_gpu_source() -> GpuSource {
        // Helper to create a command with hidden console window on Windows
        fn create_hidden_command(program: &str) -> Command {
            let mut cmd = Command::new(program);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        }

        // Check for NVIDIA GPU (nvidia-smi)
        if let Ok(output) = create_hidden_command("nvidia-smi")
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
        if let Ok(output) = create_hidden_command("rocm-smi").arg("--showpower").output() {
            if output.status.success() {
                log::info!("AMD GPU detected via rocm-smi");
                return GpuSource::Amd;
            }
        }

        // Also try amd-smi (newer AMD tool)
        if let Ok(output) = create_hidden_command("amd-smi").arg("metric").arg("-p").output() {
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
        // Use PowerShell to check for battery (with hidden console window)
        let mut cmd = Command::new("powershell");
        cmd.args(["-Command", "(Get-WmiObject Win32_Battery).EstimatedChargeRemaining"]);

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
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

        let cpus = sys.cpus();
        let average_load = if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|cpu| cpu.cpu_usage() as f64).sum::<f64>() / cpus.len() as f64
        };

        CpuInfo { average_load }
    }

    /// Get GPU power via nvidia-smi (with timeout)
    fn get_nvidia_gpu_power(&self) -> Option<GpuInfo> {
        let output = run_command_with_timeout(
            "nvidia-smi",
            &["--query-gpu=power.draw,name", "--format=csv,noheader,nounits"],
            GPU_COMMAND_TIMEOUT_MS,
        )?;

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
            });
        }

        None
    }

    /// Get GPU power via rocm-smi (AMD) - with timeout
    fn get_amd_gpu_power(&self) -> Option<GpuInfo> {
        // Try rocm-smi first
        if let Some(output) = run_command_with_timeout("rocm-smi", &["--showpower", "--json"], GPU_COMMAND_TIMEOUT_MS) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(power) = Self::parse_rocm_smi_power(&stdout) {
                    return Some(GpuInfo {
                        power_watts: power,
                        name: "AMD GPU".to_string(),
                    });
                }
            }
        }

        // Try amd-smi as fallback
        if let Some(output) = run_command_with_timeout("amd-smi", &["metric", "-p", "--json"], GPU_COMMAND_TIMEOUT_MS) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(power) = Self::parse_amd_smi_power(&stdout) {
                    return Some(GpuInfo {
                        power_watts: power,
                        name: "AMD GPU".to_string(),
                    });
                }
            }
        }

        // Try simple text output
        if let Some(output) = run_command_with_timeout("rocm-smi", &["--showpower"], GPU_COMMAND_TIMEOUT_MS) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("Average") && line.contains("Power") {
                        if let Some(power) = Self::extract_power_from_line(line) {
                            return Some(GpuInfo {
                                power_watts: power,
                                name: "AMD GPU".to_string(),
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

    /// Get GPU power based on detected source.
    /// NVML path skips the cache (fast enough at ~1-5ms).
    /// CLI fallback is cached for 2000ms to reduce command overhead.
    fn get_gpu_power(&self) -> Option<GpuInfo> {
        // NVML fast path — no cache needed
        if self.gpu_source == GpuSource::NvmlNvidia {
            if let Some(ref nvml) = self.nvml_state {
                if let Some((power, name)) = nvml_gpu::query_gpu_power(nvml) {
                    return Some(GpuInfo { power_watts: power, name });
                }
            }
            // NVML query failed, fall through to CLI
        }

        // Check cache first (2000ms TTL - GPU commands are slow)
        {
            let cache = self.gpu_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(2000) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data via CLI
        let result = match self.gpu_source {
            GpuSource::NvmlNvidia | GpuSource::Nvidia => self.get_nvidia_gpu_power(),
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

    /// Fast path for power reading - uses CPU power + cached GPU data (accepts 10s stale)
    /// Returns (power_watts, cpu_usage_percent, cached_gpu_usage_percent, cached_gpu_power_watts)
    /// This method NEVER blocks on GPU commands - it only uses cached values
    pub fn get_power_watts_fast_impl(&self) -> Result<(f64, f64, Option<f64>, Option<f64>)> {
        let mut total_power = 0.0;

        // Get CPU usage and power (fast - uses sysinfo which is non-blocking)
        let cpu_info = self.get_cpu_info();
        let cpu_power = self.calculate_cpu_power(&cpu_info);
        total_power += cpu_power;

        // Get cached GPU data with extended staleness tolerance (10s for fast path)
        let (gpu_usage, gpu_power_watts) = self.get_cached_gpu_data_for_fast_path();

        // Add GPU power if we have cached data
        if let Some(power) = gpu_power_watts {
            total_power += power;
        }

        // Add base system power
        total_power += self.estimate_base_power();

        Ok((total_power, cpu_info.average_load, gpu_usage, gpu_power_watts))
    }

    /// Get cached GPU data with extended staleness tolerance for fast path (10s)
    /// This NEVER triggers a GPU command - it only reads from cache
    fn get_cached_gpu_data_for_fast_path(&self) -> (Option<f64>, Option<f64>) {
        // Extended staleness tolerance for fast path: 10 seconds
        const FAST_PATH_CACHE_TTL_MS: u64 = 10000;

        // Check GPU metrics cache for usage
        let gpu_usage = {
            let cache = self.gpu_metrics_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(metrics) = cached.get(FAST_PATH_CACHE_TTL_MS) {
                    metrics.and_then(|m| m.usage_percent)
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Check GPU power cache
        let gpu_power = {
            let cache = self.gpu_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(info) = cached.get(FAST_PATH_CACHE_TTL_MS) {
                    info.map(|i| i.power_watts)
                } else {
                    None
                }
            } else {
                None
            }
        };

        (gpu_usage, gpu_power)
    }

    /// Collect all detailed metrics in one blocking call
    /// This consolidates all slow operations: GPU commands, temps, processes
    /// Should be called from a background task, not the main monitoring loop
    /// When `extended` is true, also collects per-core frequencies and fan speeds
    pub fn collect_detailed_metrics_impl(&self, limit: usize, pinned: &[String], extended: bool) -> Result<DetailedMetrics> {
        // Get full system metrics (this will refresh GPU cache via nvidia-smi)
        let system_metrics = self.get_system_metrics_impl(extended).ok();

        // Get top processes (uses sysinfo which is relatively fast)
        let top_processes = self.get_top_processes_with_pinned(limit, pinned).unwrap_or_default();

        Ok(DetailedMetrics {
            system_metrics,
            top_processes,
            timestamp: chrono::Utc::now().timestamp(),
            extended_collected: extended,
        })
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
            GpuSource::NvmlNvidia => "sysinfo+nvml",
            GpuSource::Nvidia => "sysinfo+nvidia",
            GpuSource::Amd => "sysinfo+amd",
            GpuSource::None => "sysinfo",
        };

        let is_estimated = !has_real_reading;

        Ok(PowerReading::new(total_power, source, is_estimated).with_components(components))
    }
}

impl PowerSource for WmiMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        self.get_power_watts()
    }

    fn get_power_watts_fast(&self) -> Result<(f64, f64, Option<f64>, Option<f64>)> {
        self.get_power_watts_fast_impl()
    }

    fn collect_detailed_metrics(&self, limit: usize, pinned: &[String], extended: bool) -> Result<DetailedMetrics> {
        self.collect_detailed_metrics_impl(limit, pinned, extended)
    }

    fn get_reading(&self) -> Result<PowerReading> {
        self.get_reading()
    }

    fn name(&self) -> &str {
        match self.gpu_source {
            GpuSource::NvmlNvidia => "Windows Monitor + NVIDIA (NVML)",
            GpuSource::Nvidia => "Windows Monitor + NVIDIA",
            GpuSource::Amd => "Windows Monitor + AMD",
            GpuSource::None => "Windows Monitor (estimated)",
        }
    }

    fn is_estimated(&self) -> bool {
        self.gpu_source == GpuSource::None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ===== Native Per-Core Frequency via CallNtPowerInformation =====

/// Layout matching the Win32 PROCESSOR_POWER_INFORMATION struct returned by
/// `CallNtPowerInformation(ProcessorInformation, ...)`.
/// See: https://learn.microsoft.com/en-us/windows/win32/power/processor-power-information-str
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ProcessorPowerInformation {
    number: u32,
    max_mhz: u32,
    current_mhz: u32,
    mhz_limit: u32,
    max_idle_state: u32,
    current_idle_state: u32,
}

impl WmiMonitor {
    /// Read per-core CPU frequencies using the native Windows
    /// `CallNtPowerInformation(ProcessorInformation)` API.
    ///
    /// This returns the actual current P-state frequency for each logical
    /// processor, which is more accurate than sysinfo (which often reports the
    /// base/nominal frequency on Windows).
    ///
    /// Returns `None` if the call fails for any reason (non-zero NTSTATUS,
    /// buffer mismatch, etc.).
    fn get_per_core_frequency_native(&self) -> Option<Vec<u64>> {
        use windows_sys::Win32::System::Power::CallNtPowerInformation;

        // ProcessorInformation = 11
        const PROCESSOR_INFORMATION: i32 = 11;

        let num_cpus = {
            let sys = self.sys.lock().unwrap();
            sys.cpus().len()
        };

        if num_cpus == 0 {
            return None;
        }

        let struct_size = std::mem::size_of::<ProcessorPowerInformation>();
        let buffer_size = struct_size * num_cpus;
        let mut buffer: Vec<u8> = vec![0u8; buffer_size];

        // SAFETY: We allocate a correctly-sized buffer and pass its length.
        // CallNtPowerInformation writes `num_cpus` PROCESSOR_POWER_INFORMATION
        // structs into the output buffer. The input buffer is unused for this
        // information class (null, 0).
        let status = unsafe {
            CallNtPowerInformation(
                PROCESSOR_INFORMATION,
                std::ptr::null(),      // no input buffer
                0,                     // input buffer size = 0
                buffer.as_mut_ptr() as *mut _,
                buffer_size as u32,
            )
        };

        // NTSTATUS 0 == STATUS_SUCCESS
        if status != 0 {
            log::debug!(
                "CallNtPowerInformation(ProcessorInformation) failed with NTSTATUS 0x{:08X}",
                status as u32
            );
            return None;
        }

        // Reinterpret the buffer as a slice of ProcessorPowerInformation
        let infos: &[ProcessorPowerInformation] = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const ProcessorPowerInformation,
                num_cpus,
            )
        };

        let freqs: Vec<u64> = infos.iter().map(|info| info.current_mhz as u64).collect();

        // Sanity check: if every core reports 0 MHz, treat as failure
        if freqs.iter().all(|&f| f == 0) {
            log::debug!("CallNtPowerInformation returned all-zero frequencies, ignoring");
            return None;
        }

        Some(freqs)
    }
}

// ===== System Metrics Implementation =====

impl WmiMonitor {
    /// Get comprehensive system metrics including CPU, GPU, and memory
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        self.get_system_metrics_impl(false)
    }

    /// Get system metrics with optional extended collection (per-core freq, fans)
    fn get_system_metrics_impl(&self, extended: bool) -> Result<SystemMetrics> {
        let mut sys = self.sys.lock().unwrap();
        // NOTE: Do NOT refresh CPU here - it interferes with critical loop baseline.
        // CPU values are already refreshed by get_cpu_info() in the critical loop.
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

        // Per-core frequencies: collect sysinfo values while lock is held,
        // but call get_per_core_frequency_native() AFTER dropping the lock
        // (it also needs self.sys.lock(), so calling it here would deadlock).
        let sysinfo_freqs: Vec<u64> = sys.cpus().iter().map(|c| c.frequency()).collect();

        // Release sys lock before slow operations
        let used_memory = sys.used_memory();
        let total_memory = sys.total_memory();
        let used_swap = sys.used_swap();
        let total_swap = sys.total_swap();
        let physical_core_count = sys.physical_core_count().unwrap_or(0);
        let thread_count = sys.cpus().len();
        drop(sys);

        // Safe now: sys lock is released, get_per_core_frequency_native can acquire it
        let per_core_frequency_mhz = self.get_per_core_frequency_native()
            .or(Some(sysinfo_freqs));

        // Get CPU temperature via WMI (if available)
        let cpu_temp = self.get_cpu_temperature();

        let cpu = CpuMetrics {
            name: cpu_name,
            usage_percent: cpu_usage,
            per_core_usage,
            frequency_mhz: cpu_freq,
            temperature_celsius: cpu_temp,
            core_count: physical_core_count,
            thread_count,
            per_core_frequency_mhz,
            per_core_temperature: None, // Per-core temps not available on Windows without OHM/LHM
        };

        // GPU metrics (fan speed and mem clock come free from nvidia-smi query)
        let gpu = self.get_gpu_metrics();

        // System fan speeds - only when extended (WMI call is slow)
        let fans = if extended {
            self.get_system_fans()
        } else {
            None
        };

        // Memory metrics (including swap and speed)
        let (swap_used, swap_total, swap_percent) = if total_swap > 0 {
            (Some(used_swap), Some(total_swap), Some((used_swap as f64 / total_swap as f64) * 100.0))
        } else {
            (None, None, None)
        };
        let (mem_speed, mem_type) = self.get_memory_info();
        let memory = MemoryMetrics {
            used_bytes: used_memory,
            total_bytes: total_memory,
            usage_percent: (used_memory as f64 / total_memory as f64) * 100.0,
            swap_used_bytes: swap_used,
            swap_total_bytes: swap_total,
            swap_usage_percent: swap_percent,
            memory_speed_mhz: mem_speed,
            memory_type: mem_type,
            power_watts: None, // DRAM power not available on Windows (no RAPL access)
        };

        Ok(SystemMetrics {
            cpu,
            gpu,
            memory,
            timestamp: chrono::Utc::now().timestamp(),
            fans,
            voltages: None, // Not available on Windows without LibreHardwareMonitor
        })
    }

    /// Get top N processes by CPU usage with optional pinned processes
    pub fn get_top_processes(&self, limit: usize) -> Result<Vec<ProcessMetrics>> {
        self.get_top_processes_with_pinned(limit, &[])
    }

    /// Get top N processes by CPU usage, including pinned processes
    pub fn get_top_processes_with_pinned(&self, limit: usize, pinned_names: &[String]) -> Result<Vec<ProcessMetrics>> {
        let mut sys = self.sys.lock().unwrap();
        // Must use refresh_processes_specifics with cpu AND memory flags
        // CPU flag is required for per-process CPU usage calculation
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());
        sys.refresh_memory();

        let total_memory = sys.total_memory();

        // Get GPU usage per process (cached)
        // Release the sys lock before calling get_gpu_process_usage to avoid deadlock
        let process_data: Vec<_> = sys
            .processes()
            .iter()
            .map(|(pid, process)| {
                (pid.as_u32(), process.name().to_string(), process.cpu_usage() as f64, process.memory())
            })
            .collect();
        drop(sys);

        let gpu_usage = self.get_gpu_process_usage();

        // First pass: build individual process metrics
        let raw_processes: Vec<ProcessMetrics> = process_data
            .into_iter()
            .map(|(pid, name, cpu_percent, memory_bytes)| {
                let is_pinned = pinned_names.iter().any(|p| p.eq_ignore_ascii_case(&name));
                let gpu_percent = gpu_usage.get(&pid).copied();
                ProcessMetrics {
                    pid,
                    name,
                    cpu_percent,
                    memory_bytes,
                    memory_percent: (memory_bytes as f64 / total_memory as f64) * 100.0,
                    gpu_percent,
                    is_pinned,
                }
            })
            .collect();

        // Second pass: aggregate processes by name to avoid duplicates
        let mut aggregated: HashMap<String, ProcessMetrics> = HashMap::new();
        for proc in raw_processes {
            let entry = aggregated.entry(proc.name.clone()).or_insert(ProcessMetrics {
                pid: proc.pid, // Keep first PID encountered
                name: proc.name.clone(),
                cpu_percent: 0.0,
                memory_bytes: 0,
                memory_percent: 0.0,
                gpu_percent: None,
                is_pinned: proc.is_pinned,
            });
            entry.cpu_percent += proc.cpu_percent;
            entry.memory_bytes += proc.memory_bytes;
            entry.memory_percent += proc.memory_percent;
            // For GPU, sum up all GPU usage from same-named processes
            if let Some(gpu) = proc.gpu_percent {
                entry.gpu_percent = Some(entry.gpu_percent.unwrap_or(0.0) + gpu);
            }
            // If any instance is pinned, mark aggregated as pinned
            if proc.is_pinned {
                entry.is_pinned = true;
            }
        }

        // Clamp aggregated percentages to 100% max
        let processes: Vec<ProcessMetrics> = aggregated.into_values()
            .map(|mut p| {
                p.cpu_percent = p.cpu_percent.min(100.0);
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

        // Helper to calculate combined usage score (40% CPU + 40% GPU + 20% Memory)
        let usage_score = |p: &ProcessMetrics| -> f64 {
            let cpu = p.cpu_percent;
            let gpu = p.gpu_percent.unwrap_or(0.0);
            let mem = p.memory_percent;
            cpu * 0.4 + gpu * 0.4 + mem * 0.2
        };

        // Sort both by global usage score descending
        pinned.sort_by(|a, b| usage_score(b).partial_cmp(&usage_score(a)).unwrap_or(std::cmp::Ordering::Equal));
        others.sort_by(|a, b| usage_score(b).partial_cmp(&usage_score(a)).unwrap_or(std::cmp::Ordering::Equal));

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
        // Must use refresh_processes_specifics with cpu AND memory flags
        // CPU flag is required for per-process CPU usage calculation
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());
        sys.refresh_memory();

        let total_memory = sys.total_memory();

        // Get process data and release the lock before calling get_gpu_process_usage
        let process_data: Vec<_> = sys
            .processes()
            .iter()
            .filter(|(_, process)| process.cpu_usage() > 0.0 || process.memory() > 0)
            .map(|(pid, process)| {
                (pid.as_u32(), process.name().to_string(), process.cpu_usage() as f64, process.memory())
            })
            .collect();
        drop(sys);

        // Get GPU usage per process (cached)
        let gpu_usage = self.get_gpu_process_usage();

        // First pass: build individual process metrics
        let raw_processes: Vec<ProcessMetrics> = process_data
            .into_iter()
            .map(|(pid, name, cpu_percent, memory_bytes)| {
                let gpu_percent = gpu_usage.get(&pid).copied();
                ProcessMetrics {
                    pid,
                    name,
                    cpu_percent,
                    memory_bytes,
                    memory_percent: (memory_bytes as f64 / total_memory as f64) * 100.0,
                    gpu_percent,
                    is_pinned: false,
                }
            })
            .collect();

        // Second pass: aggregate processes by name to avoid duplicates
        let mut aggregated: HashMap<String, ProcessMetrics> = HashMap::new();
        for proc in raw_processes {
            let entry = aggregated.entry(proc.name.clone()).or_insert(ProcessMetrics {
                pid: proc.pid,
                name: proc.name.clone(),
                cpu_percent: 0.0,
                memory_bytes: 0,
                memory_percent: 0.0,
                gpu_percent: None,
                is_pinned: false,
            });
            entry.cpu_percent += proc.cpu_percent;
            entry.memory_bytes += proc.memory_bytes;
            entry.memory_percent += proc.memory_percent;
            if let Some(gpu) = proc.gpu_percent {
                entry.gpu_percent = Some(entry.gpu_percent.unwrap_or(0.0) + gpu);
            }
        }

        // Clamp aggregated percentages to 100% max
        let mut processes: Vec<ProcessMetrics> = aggregated.into_values()
            .map(|mut p| {
                p.cpu_percent = p.cpu_percent.min(100.0);
                p.memory_percent = p.memory_percent.min(100.0);
                if let Some(gpu) = p.gpu_percent {
                    p.gpu_percent = Some(gpu.min(100.0));
                }
                p
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

    /// Actually fetch CPU temperature using a cascade of sources:
    /// PDH Thermal Zone -> OHM WMI -> LHM WMI -> MSAcpi WMI (fallback)
    fn fetch_cpu_temperature(&self) -> Option<f64> {
        // 1. Try PDH Thermal Zone Information (fast, no PowerShell, no admin needed)
        #[cfg(target_os = "windows")]
        if let Some(temp) = self.fetch_cpu_temperature_pdh() {
            log::debug!("CPU temperature from PDH: {:.1}°C", temp);
            return Some(temp);
        }

        // 2. Try Open Hardware Monitor WMI if available
        if let Some(temp) = self.fetch_cpu_temperature_ohm() {
            log::debug!("CPU temperature from OHM: {:.1}°C", temp);
            return Some(temp);
        }

        // 3. Try LibreHardwareMonitor WMI if available
        if let Some(temp) = self.fetch_cpu_temperature_lhm() {
            log::debug!("CPU temperature from LHM: {:.1}°C", temp);
            return Some(temp);
        }

        // 4. Fallback: MSAcpi_ThermalZoneTemperature (requires admin, often unreliable)
        if let Some(temp) = self.fetch_cpu_temperature_msacpi() {
            log::debug!("CPU temperature from MSAcpi: {:.1}°C", temp);
            return Some(temp);
        }

        None
    }

    /// Fetch CPU temperature via PDH (Performance Data Helper) API.
    /// Reads `\Thermal Zone Information(*)\Temperature` which returns Kelvin.
    /// The PDH query handle is lazily initialized and cached for reuse.
    #[cfg(target_os = "windows")]
    fn fetch_cpu_temperature_pdh(&self) -> Option<f64> {
        use windows_sys::Win32::System::Performance::{
            PdhOpenQueryW, PdhAddEnglishCounterW, PdhCollectQueryData,
            PdhGetFormattedCounterValue, PdhCloseQuery,
            PDH_FMT_DOUBLE, PDH_FMT_COUNTERVALUE,
        };

        // Ensure the PDH query is initialized (lazy init)
        let mut pdh_lock = self.pdh_thermal_query.lock().unwrap();

        if pdh_lock.is_none() {
            // Initialize PDH query and add the thermal zone counter
            let mut query: isize = 0;
            let status = unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) };
            if status != 0 {
                log::debug!("PDH: PdhOpenQueryW failed with status 0x{:08X}", status);
                return None;
            }

            // Counter path: \Thermal Zone Information(*)\Temperature
            // Using wide string (UTF-16) for the W-suffix API
            let counter_path: Vec<u16> = "\\Thermal Zone Information(*)\\Temperature\0"
                .encode_utf16()
                .collect();

            let mut counter: isize = 0;
            let status = unsafe {
                PdhAddEnglishCounterW(query, counter_path.as_ptr(), 0, &mut counter)
            };
            if status != 0 {
                log::debug!("PDH: PdhAddEnglishCounterW failed with status 0x{:08X}", status);
                unsafe { PdhCloseQuery(query); }
                return None;
            }

            // First collect to establish baseline (PDH needs at least one collect before reading)
            let status = unsafe { PdhCollectQueryData(query) };
            if status != 0 {
                log::debug!("PDH: initial PdhCollectQueryData failed with status 0x{:08X}", status);
                unsafe { PdhCloseQuery(query); }
                return None;
            }

            *pdh_lock = Some(PdhThermalQuery { query, counter });
            log::info!("PDH thermal zone query initialized successfully");
        }

        let pdh = pdh_lock.as_ref()?;

        // Collect fresh data
        let status = unsafe { PdhCollectQueryData(pdh.query) };
        if status != 0 {
            log::debug!("PDH: PdhCollectQueryData failed with status 0x{:08X}", status);
            return None;
        }

        // Read the formatted counter value
        let mut counter_type: u32 = 0;
        let mut value: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
        let status = unsafe {
            PdhGetFormattedCounterValue(pdh.counter, PDH_FMT_DOUBLE, &mut counter_type, &mut value)
        };
        if status != 0 {
            log::debug!("PDH: PdhGetFormattedCounterValue failed with status 0x{:08X}", status);
            return None;
        }

        // PDH returns temperature in Kelvin, convert to Celsius
        let temp_kelvin = unsafe { value.Anonymous.doubleValue };
        let temp_celsius = temp_kelvin - 273.15;

        if temp_celsius > 0.0 && temp_celsius < 150.0 {
            Some(temp_celsius)
        } else {
            log::debug!("PDH: temperature out of range: {:.1}K = {:.1}°C", temp_kelvin, temp_celsius);
            None
        }
    }

    /// Fetch CPU temperature via Open Hardware Monitor WMI namespace (with timeout)
    fn fetch_cpu_temperature_ohm(&self) -> Option<f64> {
        let output = run_command_with_timeout(
            "powershell",
            &["-Command", "Get-WmiObject Sensor -Namespace root/OpenHardwareMonitor 2>$null | Where-Object { $_.SensorType -eq 'Temperature' -and $_.Name -like '*CPU*' } | Select-Object -First 1 -ExpandProperty Value"],
            GPU_COMMAND_TIMEOUT_MS,
        )?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(temp) = stdout.trim().parse::<f64>() {
                if temp > 0.0 && temp < 150.0 {
                    return Some(temp);
                }
            }
        }

        None
    }

    /// Fetch CPU temperature via LibreHardwareMonitor WMI namespace (with timeout)
    fn fetch_cpu_temperature_lhm(&self) -> Option<f64> {
        let output = run_command_with_timeout(
            "powershell",
            &["-Command", "Get-WmiObject Sensor -Namespace root/LibreHardwareMonitor 2>$null | Where-Object { $_.SensorType -eq 'Temperature' -and $_.Name -like '*CPU*' } | Select-Object -First 1 -ExpandProperty Value"],
            GPU_COMMAND_TIMEOUT_MS,
        )?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(temp) = stdout.trim().parse::<f64>() {
                if temp > 0.0 && temp < 150.0 {
                    return Some(temp);
                }
            }
        }

        None
    }

    /// Fetch CPU temperature via MSAcpi_ThermalZoneTemperature WMI (fallback, requires admin)
    fn fetch_cpu_temperature_msacpi(&self) -> Option<f64> {
        let output = run_command_with_timeout(
            "powershell",
            &["-Command", "Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace root/wmi 2>$null | Select-Object -First 1 -ExpandProperty CurrentTemperature"],
            GPU_COMMAND_TIMEOUT_MS,
        )?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(temp_decikelvin) = stdout.trim().parse::<f64>() {
                // MSAcpi returns tenths of Kelvin
                let temp_celsius = (temp_decikelvin / 10.0) - 273.15;
                if temp_celsius > 0.0 && temp_celsius < 150.0 {
                    return Some(temp_celsius);
                }
            }
        }

        None
    }

    /// Get GPU metrics including usage, power, temperature, and VRAM.
    /// NVML: cached for 500ms (fast API). CLI: cached for 2000ms (slow subprocess).
    fn get_gpu_metrics(&self) -> Option<GpuMetrics> {
        let cache_ttl = if self.gpu_source == GpuSource::NvmlNvidia { 500 } else { 2000 };

        // Check cache first
        {
            let cache = self.gpu_metrics_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(cache_ttl) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data
        let result = match self.gpu_source {
            GpuSource::NvmlNvidia => {
                // Try NVML first
                self.nvml_state.as_ref()
                    .and_then(nvml_gpu::query_gpu_metrics)
                    .or_else(|| self.get_nvidia_gpu_metrics()) // CLI fallback
            }
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

    /// Get NVIDIA GPU metrics via nvidia-smi (with timeout)
    /// Queries clocks.mem and fan.speed in the same call (zero extra process spawns)
    fn get_nvidia_gpu_metrics(&self) -> Option<GpuMetrics> {
        let output = run_command_with_timeout(
            "nvidia-smi",
            &["--query-gpu=name,utilization.gpu,power.draw,temperature.gpu,memory.used,memory.total,clocks.gr,clocks.mem,fan.speed", "--format=csv,noheader,nounits"],
            GPU_COMMAND_TIMEOUT_MS,
        )?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

        if parts.len() >= 7 {
            // Parse optional extended fields (clocks.mem at index 7, fan.speed at index 8)
            // nvidia-smi returns "[N/A]" on laptops without fans, which parse().ok() handles as None
            let memory_clock_mhz = parts.get(7).and_then(|s| s.parse::<u64>().ok());
            let fan_speed_percent = parts.get(8).and_then(|s| s.parse::<u64>().ok());

            Some(GpuMetrics {
                name: parts[0].to_string(),
                usage_percent: parts[1].parse().ok(),
                power_watts: parts[2].parse().ok(),
                temperature_celsius: parts[3].parse().ok(),
                vram_used_mb: parts[4].parse().ok(),
                vram_total_mb: parts[5].parse().ok(),
                clock_mhz: parts[6].parse().ok(),
                source: "nvidia-smi".to_string(),
                memory_clock_mhz,
                fan_speed_percent,
            })
        } else {
            None
        }
    }

    /// Get AMD GPU metrics via rocm-smi or amd-smi (with timeout)
    fn get_amd_gpu_metrics(&self) -> Option<GpuMetrics> {
        // Try rocm-smi first
        if let Some(output) = run_command_with_timeout(
            "rocm-smi",
            &["--showuse", "--showpower", "--showtemp", "--showmemuse", "--json"],
            GPU_COMMAND_TIMEOUT_MS,
        ) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(metrics) = self.parse_rocm_smi_metrics(&stdout) {
                    return Some(metrics);
                }
            }
        }

        // Try amd-smi as fallback
        if let Some(output) = run_command_with_timeout("amd-smi", &["metric", "--json"], GPU_COMMAND_TIMEOUT_MS) {
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
                memory_clock_mhz: None,
                fan_speed_percent: None,
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
                memory_clock_mhz: card.get("mclk clock speed (MHz)").and_then(|v| v.as_u64()),
                fan_speed_percent: card.get("Fan speed (%)").and_then(|v| v.as_u64())
                    .or_else(|| card.get("Fan Speed (%)").and_then(|v| v.as_u64())),
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
                        memory_clock_mhz: first.get("clock").and_then(|c| c.get("mem")).and_then(|v| v.as_u64()),
                        fan_speed_percent: first.get("fan").and_then(|f| f.get("speed")).and_then(|v| v.as_u64()),
                    });
                }
            }
        }
        None
    }

    /// Get memory speed in MHz (permanently cached - RAM speed never changes at runtime)
    fn get_memory_info(&self) -> (Option<u64>, Option<String>) {
        // Check permanent cache first
        {
            let cache = self.memory_info_cache.lock().unwrap();
            if let Some(ref value) = *cache {
                return value.clone();
            }
        }

        // Cache miss - fetch via WMI (one-time cost)
        let result = self.fetch_memory_info();

        // Store permanently
        {
            let mut cache = self.memory_info_cache.lock().unwrap();
            *cache = Some(result.clone());
        }

        result
    }

    /// Fetch memory speed and type from WMI Win32_PhysicalMemory (slow - calls PowerShell, one-time only)
    fn fetch_memory_info(&self) -> (Option<u64>, Option<String>) {
        let output = run_command_with_timeout(
            "powershell",
            &["-Command", "Get-WmiObject Win32_PhysicalMemory | Select-Object -First 1 Speed, SMBIOSMemoryType | ForEach-Object { \"$($_.Speed)|$($_.SMBIOSMemoryType)\" }"],
            GPU_COMMAND_TIMEOUT_MS,
        );

        if let Some(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.trim().split('|').collect();

                let speed = parts.first()
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|&s| s > 0);

                let mem_type = parts.get(1)
                    .and_then(|s| s.parse::<u32>().ok())
                    .and_then(|code| match code {
                        20 => Some("DDR"),
                        21 => Some("DDR2"),
                        24 => Some("DDR3"),
                        26 => Some("DDR4"),
                        34 => Some("DDR5"),
                        _ => None,
                    })
                    .map(String::from);

                return (speed, mem_type);
            }
        }

        (None, None)
    }

    /// Get system fan speeds via WMI (cached for 5 seconds - WMI/PowerShell is slow)
    fn get_system_fans(&self) -> Option<FanMetrics> {
        // Check cache first (5000ms TTL - fans change slowly)
        {
            let cache = self.fan_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(5000) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data
        let result = self.fetch_system_fans();

        // Update cache
        {
            let mut cache = self.fan_cache.lock().unwrap();
            *cache = Some(CachedValue::new(result.clone()));
        }

        result
    }

    /// Fetch system fan speeds via WMI Win32_Fan (slow - calls PowerShell)
    fn fetch_system_fans(&self) -> Option<FanMetrics> {
        let output = run_command_with_timeout(
            "powershell",
            &["-Command", "Get-WmiObject Win32_Fan 2>$null | Select-Object Name,DesiredSpeed,ActiveCooling | ConvertTo-Json -Compress"],
            2000,
        )?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut fans = Vec::new();

        // WMI may return a single object or an array
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let items: Vec<&serde_json::Value> = if let Some(arr) = value.as_array() {
                arr.iter().collect()
            } else {
                vec![&value]
            };

            for item in items {
                let name = item.get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Fan")
                    .to_string();
                let speed_rpm = item.get("DesiredSpeed")
                    .and_then(|v| v.as_u64());

                fans.push(FanReading {
                    name,
                    speed_rpm,
                    speed_percent: None,
                });
            }
        }

        if fans.is_empty() {
            None
        } else {
            Some(FanMetrics { fans })
        }
    }

    /// Get per-process GPU usage.
    /// NVML: cached for 500ms (fast). CLI: cached for 2000ms (slow subprocess).
    fn get_gpu_process_usage(&self) -> HashMap<u32, f64> {
        let cache_ttl = if self.gpu_source == GpuSource::NvmlNvidia { 500 } else { 2000 };

        // Check cache first
        {
            let cache = self.gpu_process_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if let Some(value) = cached.get(cache_ttl) {
                    return value;
                }
            }
        }

        // Cache miss - fetch fresh data based on GPU source
        let result = match self.gpu_source {
            GpuSource::NvmlNvidia => {
                // Try NVML first, fall back to CLI pmon
                self.nvml_state.as_ref()
                    .map(nvml_gpu::query_gpu_processes)
                    .unwrap_or_else(|| self.fetch_nvidia_gpu_processes())
            }
            GpuSource::Nvidia => self.fetch_nvidia_gpu_processes(),
            GpuSource::Amd => self.fetch_amd_gpu_processes(),
            GpuSource::None => HashMap::new(),
        };

        // Update cache
        {
            let mut cache = self.gpu_process_cache.lock().unwrap();
            *cache = Some(CachedValue::new(result.clone()));
        }

        result
    }

    /// Fetch per-process GPU usage from nvidia-smi pmon
    ///
    /// Parses output like:
    /// ```text
    /// # gpu        pid  type    sm   mem   enc   dec   jpg   ofa  command
    ///     0       1234    C    45    12     0     0     -     -  game.exe
    /// ```
    fn fetch_nvidia_gpu_processes(&self) -> HashMap<u32, f64> {
        let mut result = HashMap::new();

        // Use nvidia-smi pmon for per-process GPU utilization (with timeout)
        // -c 1 means capture one sample
        let output = match run_command_with_timeout("nvidia-smi", &["pmon", "-c", "1", "-s", "u"], GPU_COMMAND_TIMEOUT_MS) {
            Some(o) => o,
            None => return result,
        };

        if !output.status.success() {
            return result;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            // Skip header lines (start with #) and empty lines
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse columns: gpu, pid, type, sm, mem, enc, dec, jpg, ofa, command
            // We want pid (column 1) and sm (column 3) for GPU compute utilization
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                // pid is at index 1, sm (GPU utilization) is at index 3
                if let (Ok(pid), Ok(sm)) = (parts[1].parse::<u32>(), parts[3].parse::<f64>()) {
                    // Clamp GPU usage to 0-100 range (nvidia-smi can report invalid values)
                    let sm_clamped = sm.clamp(0.0, 100.0);
                    // If we already have this PID, take the max (multi-GPU scenarios)
                    let entry = result.entry(pid).or_insert(0.0);
                    *entry = entry.max(sm_clamped);
                }
            }
        }

        result
    }

    /// Fetch per-process GPU usage from AMD tools (with timeout)
    ///
    /// AMD provides limited per-process GPU data. We try amd-smi process command
    /// which may show active processes, but exact utilization % is often unavailable.
    fn fetch_amd_gpu_processes(&self) -> HashMap<u32, f64> {
        let mut result = HashMap::new();

        // Try amd-smi process --json
        if let Some(output) = run_command_with_timeout("amd-smi", &["process", "--json"], GPU_COMMAND_TIMEOUT_MS) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    // amd-smi process output structure varies, try to extract PIDs
                    if let Some(arr) = value.as_array() {
                        for item in arr {
                            if let Some(pid) = item.get("pid").and_then(|v| v.as_u64()) {
                                // AMD doesn't always provide per-process GPU%,
                                // try to get it or use a marker value indicating "active"
                                let gpu_usage = item.get("gpu_memory_usage")
                                    .and_then(|v| v.as_f64())
                                    .or_else(|| item.get("usage").and_then(|v| v.as_f64()));

                                if let Some(usage) = gpu_usage {
                                    result.insert(pid as u32, usage);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: try rocm-smi --showpidgpus
        if result.is_empty() {
            if let Some(output) = run_command_with_timeout("rocm-smi", &["--showpidgpus"], GPU_COMMAND_TIMEOUT_MS) {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Parse text output - format varies by rocm version
                    for line in stdout.lines() {
                        // Look for lines containing PID information
                        if line.contains("PID") || line.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                            // Try to extract PID from the line
                            for word in line.split_whitespace() {
                                if let Ok(pid) = word.parse::<u32>() {
                                    // Mark as active (we don't have exact %)
                                    // Use a small positive value to indicate GPU activity
                                    result.entry(pid).or_insert(0.1);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        result
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
