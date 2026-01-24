//! Pricing engine for electricity cost calculation
//!
//! Supports multiple pricing modes:
//! - Simple: flat rate per kWh
//! - Peak/Off-peak: different rates by time of day (HP/HC)
//! - Seasonal: different rates by season (summer/winter)
//! - Tempo: EDF-style with day colors (blue/white/red) and peak/off-peak

use crate::core::PricingConfig;
use chrono::{Local, Timelike, Datelike};

/// Pricing engine that calculates electricity costs
pub struct PricingEngine {
    config: PricingConfig,
}

impl PricingEngine {
    /// Create a new pricing engine with the given configuration
    pub fn new(config: &PricingConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Update the pricing configuration
    pub fn update_config(&mut self, config: &PricingConfig) {
        self.config = config.clone();
    }

    /// Get the current rate per kWh based on the pricing mode and current time
    pub fn get_current_rate(&self) -> f64 {
        match self.config.mode.as_str() {
            "simple" => self.config.simple.rate_per_kwh,
            "peak_offpeak" => self.get_peak_offpeak_rate(),
            "seasonal" => self.get_seasonal_rate(),
            "tempo" => self.get_tempo_rate(),
            _ => self.config.simple.rate_per_kwh, // Default to simple
        }
    }

    /// Calculate cost for a given energy consumption in kWh
    pub fn calculate_cost(&self, kwh: f64) -> f64 {
        kwh * self.get_current_rate()
    }

    /// Calculate estimated hourly cost at current power consumption
    pub fn calculate_hourly_cost(&self, watts: f64) -> f64 {
        let kwh_per_hour = watts / 1000.0;
        self.calculate_cost(kwh_per_hour)
    }

    /// Calculate estimated daily cost at current power consumption
    pub fn calculate_daily_cost(&self, watts: f64) -> f64 {
        self.calculate_hourly_cost(watts) * 24.0
    }

    /// Calculate estimated monthly cost at current power consumption
    pub fn calculate_monthly_cost(&self, watts: f64) -> f64 {
        self.calculate_daily_cost(watts) * 30.0
    }

    /// Get the currency symbol
    pub fn get_currency_symbol(&self) -> &str {
        &self.config.currency_symbol
    }

    /// Check if pricing is configured (not just using defaults)
    pub fn is_configured(&self) -> bool {
        // Check if user has set a rate different from 0
        self.config.simple.rate_per_kwh > 0.0
    }

    // Private methods for each pricing mode

    fn get_peak_offpeak_rate(&self) -> f64 {
        if self.is_offpeak_time() {
            self.config.peak_offpeak.offpeak_rate
        } else {
            self.config.peak_offpeak.peak_rate
        }
    }

    fn is_offpeak_time(&self) -> bool {
        let now = Local::now();
        let current_hour = now.hour();
        let current_minute = now.minute();
        let current_time = current_hour * 60 + current_minute;

        // Parse offpeak start and end times
        let offpeak_start = self.parse_time(&self.config.peak_offpeak.offpeak_start);
        let offpeak_end = self.parse_time(&self.config.peak_offpeak.offpeak_end);

        // Handle overnight offpeak periods (e.g., 22:00 to 06:00)
        if offpeak_start > offpeak_end {
            // Overnight period
            current_time >= offpeak_start || current_time < offpeak_end
        } else {
            // Same-day period
            current_time >= offpeak_start && current_time < offpeak_end
        }
    }

    fn parse_time(&self, time_str: &str) -> u32 {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() == 2 {
            let hours: u32 = parts[0].parse().unwrap_or(0);
            let minutes: u32 = parts[1].parse().unwrap_or(0);
            hours * 60 + minutes
        } else {
            0
        }
    }

    fn get_seasonal_rate(&self) -> f64 {
        let now = Local::now();
        let current_month = now.month();

        if self.config.seasonal.winter_months.contains(&current_month) {
            self.config.seasonal.winter_rate
        } else {
            self.config.seasonal.summer_rate
        }
    }

    fn get_tempo_rate(&self) -> f64 {
        // Tempo uses day colors (blue, white, red) combined with peak/offpeak
        // For simplicity, we'll use a simple heuristic:
        // - Winter weekdays during peak months: red days
        // - Transition periods: white days
        // - Summer and weekends: blue days
        //
        // Note: Real Tempo implementation would require fetching day colors from EDF API

        let now = Local::now();
        let month = now.month();
        let weekday = now.weekday();

        // Determine day color (simplified)
        let is_winter = [12, 1, 2].contains(&month);
        let is_weekday = matches!(
            weekday,
            chrono::Weekday::Mon
                | chrono::Weekday::Tue
                | chrono::Weekday::Wed
                | chrono::Weekday::Thu
                | chrono::Weekday::Fri
        );

        let day_color = if is_winter && is_weekday {
            // Cold winter weekdays: higher chance of red/white
            if month == 1 || month == 2 {
                "white" // Could be red on very cold days
            } else {
                "white"
            }
        } else if is_weekday && [3, 4, 10, 11].contains(&month) {
            "white"
        } else {
            "blue"
        };

        let is_offpeak = self.is_offpeak_time();

        match (day_color, is_offpeak) {
            ("blue", true) => self.config.tempo.blue_offpeak,
            ("blue", false) => self.config.tempo.blue_peak,
            ("white", true) => self.config.tempo.white_offpeak,
            ("white", false) => self.config.tempo.white_peak,
            ("red", true) => self.config.tempo.red_offpeak,
            ("red", false) => self.config.tempo.red_peak,
            _ => self.config.tempo.blue_peak, // Default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{SimplePricing, PeakOffpeakPricing, SeasonalPricing, TempoPricing};

    fn default_pricing_config() -> PricingConfig {
        PricingConfig {
            mode: "simple".to_string(),
            currency: "EUR".to_string(),
            currency_symbol: "\u{20AC}".to_string(),
            simple: SimplePricing { rate_per_kwh: 0.20 },
            peak_offpeak: PeakOffpeakPricing::default(),
            seasonal: SeasonalPricing::default(),
            tempo: TempoPricing::default(),
        }
    }

    #[test]
    fn test_simple_pricing() {
        let config = default_pricing_config();
        let engine = PricingEngine::new(&config);

        assert_eq!(engine.get_current_rate(), 0.20);
        assert_eq!(engine.calculate_cost(1.0), 0.20);
        assert_eq!(engine.calculate_cost(10.0), 2.0);
    }

    #[test]
    fn test_hourly_cost() {
        let config = default_pricing_config();
        let engine = PricingEngine::new(&config);

        // 100W = 0.1 kWh per hour
        let hourly = engine.calculate_hourly_cost(100.0);
        assert!((hourly - 0.02).abs() < 0.001);
    }
}
