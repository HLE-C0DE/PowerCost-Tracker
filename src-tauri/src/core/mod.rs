//! Core module - Application state, configuration, and common types

mod config;
mod error;
mod types;

pub use config::{Config, GeneralConfig, PricingConfig, WidgetConfig, AdvancedConfig};
pub use error::{Error, Result};
pub use types::{PowerReading, DashboardData, AppState};
