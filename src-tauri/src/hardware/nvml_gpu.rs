//! Shared NVML GPU monitoring module
//!
//! Provides direct access to NVIDIA GPU metrics via the NVML library,
//! replacing slow nvidia-smi CLI subprocess calls.
//! Latency improvement: ~500-1500ms (CLI) → ~1-5ms (NVML).
//!
//! Used by both Windows and Linux backends.

use crate::core::GpuMetrics;
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::enums::device::UsedGpuMemory;
use std::collections::HashMap;

/// Holds the NVML library instance and the primary GPU device index.
pub struct NvmlState {
    nvml: Nvml,
    device_index: u32,
    device_name: String,
}

/// Initialize NVML and grab the first GPU device.
/// Returns None if NVML is not available (no NVIDIA driver, no GPU, etc.)
pub fn init_nvml() -> Option<NvmlState> {
    let nvml = match Nvml::init() {
        Ok(n) => n,
        Err(e) => {
            log::debug!("NVML init failed: {}", e);
            return None;
        }
    };

    let device_count = nvml.device_count().ok()?;
    if device_count == 0 {
        log::debug!("NVML: no devices found");
        return None;
    }

    let device = nvml.device_by_index(0).ok()?;
    let device_name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());

    log::info!("NVML initialized: {} (device 0 of {})", device_name, device_count);

    Some(NvmlState {
        nvml,
        device_index: 0,
        device_name,
    })
}

/// Query full GPU metrics via NVML.
/// Returns None if any critical query fails.
pub fn query_gpu_metrics(state: &NvmlState) -> Option<GpuMetrics> {
    let device = state.nvml.device_by_index(state.device_index).ok()?;

    // Utilization rates (GPU & memory engine usage %)
    let utilization = device.utilization_rates().ok();
    let usage_percent = utilization.as_ref().map(|u| u.gpu as f64);

    // Power usage (milliwatts → watts)
    let power_watts = device.power_usage().ok().map(|mw| mw as f64 / 1000.0);

    // Temperature (GPU die)
    let temperature_celsius = device
        .temperature(TemperatureSensor::Gpu)
        .ok()
        .map(|t| t as f64);

    // VRAM (bytes → MB)
    let mem_info = device.memory_info().ok();
    let vram_used_mb = mem_info.as_ref().map(|m| m.used / (1024 * 1024));
    let vram_total_mb = mem_info.as_ref().map(|m| m.total / (1024 * 1024));

    // Core clock (MHz)
    let clock_mhz = device.clock_info(Clock::Graphics).ok().map(|c| c as u64);

    // Memory clock (MHz)
    let memory_clock_mhz = device.clock_info(Clock::Memory).ok().map(|c| c as u64);

    // Fan speed (percentage) — may fail on laptops without fans
    let fan_speed_percent = device.fan_speed(0).ok().map(|f| f as u64);

    Some(GpuMetrics {
        name: state.device_name.clone(),
        usage_percent,
        power_watts,
        temperature_celsius,
        vram_used_mb,
        vram_total_mb,
        clock_mhz,
        source: "nvml".to_string(),
        memory_clock_mhz,
        fan_speed_percent,
    })
}

/// Query GPU power only (for the fast path).
/// Returns (power_watts, gpu_name).
pub fn query_gpu_power(state: &NvmlState) -> Option<(f64, String)> {
    let device = state.nvml.device_by_index(state.device_index).ok()?;
    let power_mw = device.power_usage().ok()?;
    Some((power_mw as f64 / 1000.0, state.device_name.clone()))
}

/// Query per-process GPU usage via NVML.
/// Returns a map of PID → GPU utilization percentage.
///
/// NVML provides compute and graphics process lists with their SM utilization
/// via running_compute_processes() and running_graphics_processes().
pub fn query_gpu_processes(state: &NvmlState) -> HashMap<u32, f64> {
    let mut result = HashMap::new();

    let device = match state.nvml.device_by_index(state.device_index) {
        Ok(d) => d,
        Err(_) => return result,
    };

    // Helper to extract bytes from UsedGpuMemory enum
    let used_mem_bytes = |mem: &UsedGpuMemory| -> u64 {
        match mem {
            UsedGpuMemory::Used(bytes) => *bytes,
            UsedGpuMemory::Unavailable => 0,
        }
    };

    // Collect compute processes
    if let Ok(procs) = device.running_compute_processes() {
        for proc in procs {
            let mem_bytes = used_mem_bytes(&proc.used_gpu_memory);
            result.insert(proc.pid, if mem_bytes > 0 { 0.1 } else { 0.0 });
        }
    }

    // Collect graphics processes
    if let Ok(procs) = device.running_graphics_processes() {
        for proc in procs {
            let mem_bytes = used_mem_bytes(&proc.used_gpu_memory);
            let entry = result.entry(proc.pid).or_insert(0.0);
            if mem_bytes > 0 && *entry < 0.1 {
                *entry = 0.1;
            }
        }
    }

    // Try to get actual per-process utilization via process_utilization_stats
    // The API takes Option<u64> representing a timestamp in microseconds
    // Use None to get the most recent samples
    if let Ok(samples) = device.process_utilization_stats(None) {
        for sample in samples {
            let sm_percent = sample.sm_util as f64;
            if sm_percent > 0.0 {
                let entry = result.entry(sample.pid).or_insert(0.0);
                *entry = entry.max(sm_percent.clamp(0.0, 100.0));
            }
        }
    }

    result
}
