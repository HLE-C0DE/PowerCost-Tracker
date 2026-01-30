//! Core module - Application state, configuration, and common types

mod config;
mod error;
mod types;

// SimplePricing is used by bin/demo.rs
#[allow(unused_imports)]
pub use config::{Config, PricingConfig, DashboardConfig, SimplePricing};
pub use error::{Error, Result};
#[allow(unused_imports)]
pub use types::{PowerReading, DashboardData, AppState, SystemMetrics, CpuMetrics, GpuMetrics, MemoryMetrics, ProcessMetrics, Session, BaselineDetection, CriticalMetrics, DetailedMetrics, FanMetrics, FanReading, VoltageReading};
