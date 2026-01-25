//! Core module - Application state, configuration, and common types

mod config;
mod error;
mod types;

pub use config::{Config, GeneralConfig, PricingConfig, WidgetConfig, AdvancedConfig, SimplePricing, PeakOffpeakPricing, SeasonalPricing, TempoPricing, DashboardConfig, DashboardWidget};
pub use error::{Error, Result};
pub use types::{PowerReading, DashboardData, AppState, SystemMetrics, CpuMetrics, GpuMetrics, MemoryMetrics, ProcessMetrics, Session, BaselineDetection};
