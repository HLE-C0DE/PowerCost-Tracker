//! Linux power monitoring implementations
//!
//! Supports:
//! - Intel RAPL (Running Average Power Limit) via /sys/class/powercap
//! - AMD hwmon via /sys/class/hwmon
//! - Battery power via /sys/class/power_supply

use crate::core::{Error, PowerReading, Result};
use crate::hardware::PowerSource;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

/// Intel RAPL power monitor
pub struct RaplMonitor {
    /// Path to the RAPL energy file
    energy_path: PathBuf,
    /// Max energy value before wraparound
    max_energy: u64,
    /// Last energy reading
    last_energy: Mutex<u64>,
    /// Time of last reading
    last_time: Mutex<Instant>,
    /// Component paths for breakdown
    component_paths: HashMap<String, PathBuf>,
}

impl RaplMonitor {
    pub fn new() -> Result<Self> {
        let rapl_base = Path::new("/sys/class/powercap/intel-rapl");

        if !rapl_base.exists() {
            return Err(Error::HardwareNotSupported(
                "RAPL not available".to_string(),
            ));
        }

        // Find the main package (intel-rapl:0)
        let package_path = rapl_base.join("intel-rapl:0");
        if !package_path.exists() {
            return Err(Error::HardwareNotSupported(
                "RAPL package not found".to_string(),
            ));
        }

        let energy_path = package_path.join("energy_uj");
        if !energy_path.exists() {
            return Err(Error::PermissionDenied(
                "Cannot read RAPL energy (try running with sudo or add CAP_SYS_RAWIO)".to_string(),
            ));
        }

        // Read max energy for wraparound handling
        let max_energy_path = package_path.join("max_energy_range_uj");
        let max_energy: u64 = fs::read_to_string(&max_energy_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(u64::MAX);

        // Find component paths (cores, uncore, dram if available)
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

        // Read initial energy value
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
}

impl PowerSource for RaplMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        let current_energy = self.read_energy()?;
        let current_time = Instant::now();

        let mut last_energy = self.last_energy.lock().unwrap();
        let mut last_time = self.last_time.lock().unwrap();

        // Calculate energy difference (handle wraparound)
        let energy_diff = if current_energy >= *last_energy {
            current_energy - *last_energy
        } else {
            // Wraparound occurred
            (self.max_energy - *last_energy) + current_energy
        };

        let time_diff = current_time.duration_since(*last_time);

        // Update last values
        *last_energy = current_energy;
        *last_time = current_time;

        // Calculate power in watts (energy is in microjoules)
        let power_watts = if time_diff.as_secs_f64() > 0.0 {
            (energy_diff as f64) / time_diff.as_secs_f64() / 1_000_000.0
        } else {
            0.0
        };

        Ok(power_watts)
    }

    fn get_reading(&self) -> Result<PowerReading> {
        let power = self.get_power_watts()?;
        Ok(PowerReading::new(power, "rapl", false))
    }

    fn name(&self) -> &str {
        "Intel RAPL"
    }

    fn is_estimated(&self) -> bool {
        false
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

        // Search for a device with power readings
        for entry in fs::read_dir(hwmon_base)? {
            let entry = entry?;
            let path = entry.path();

            // Check for power input files (power1_input, power2_input, etc.)
            for i in 1..=10 {
                let power_path = path.join(format!("power{}_input", i));
                if power_path.exists() {
                    log::info!("Found hwmon power sensor at {:?}", power_path);
                    return Ok(Self { power_path });
                }
            }
        }

        Err(Error::HardwareNotSupported(
            "No hwmon power sensor found".to_string(),
        ))
    }
}

impl PowerSource for HwmonMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        // Power is typically in microwatts
        let power_uw: f64 = fs::read_to_string(&self.power_path)?
            .trim()
            .parse()
            .map_err(|_| Error::PowerMonitor("Failed to parse power value".to_string()))?;

        Ok(power_uw / 1_000_000.0)
    }

    fn get_reading(&self) -> Result<PowerReading> {
        let power = self.get_power_watts()?;
        Ok(PowerReading::new(power, "hwmon", false))
    }

    fn name(&self) -> &str {
        "Linux hwmon"
    }

    fn is_estimated(&self) -> bool {
        false
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
            return Err(Error::HardwareNotSupported(
                "power_supply not available".to_string(),
            ));
        }

        // Look for battery with power_now reading
        for entry in fs::read_dir(power_supply)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Check if this is a battery (BAT0, BAT1, etc.)
            if name_str.starts_with("BAT") {
                let power_path = path.join("power_now");
                if power_path.exists() {
                    return Ok(Self { power_path });
                }
            }
        }

        Err(Error::HardwareNotSupported(
            "No battery power sensor found".to_string(),
        ))
    }
}

impl PowerSource for BatteryMonitor {
    fn get_power_watts(&self) -> Result<f64> {
        // Power is in microwatts
        let power_uw: f64 = fs::read_to_string(&self.power_path)?
            .trim()
            .parse()
            .map_err(|_| Error::PowerMonitor("Failed to parse battery power".to_string()))?;

        Ok(power_uw / 1_000_000.0)
    }

    fn get_reading(&self) -> Result<PowerReading> {
        let power = self.get_power_watts()?;
        Ok(PowerReading::new(power, "battery", false))
    }

    fn name(&self) -> &str {
        "Battery sensor"
    }

    fn is_estimated(&self) -> bool {
        false
    }
}
