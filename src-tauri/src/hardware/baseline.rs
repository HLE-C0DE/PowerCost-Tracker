//! Baseline power detection for surplus tracking
//!
//! Detects idle power consumption to calculate power usage above baseline.

use crate::core::BaselineDetection;
use std::collections::VecDeque;

/// Sample window size for baseline detection
const DEFAULT_SAMPLE_WINDOW: usize = 60; // 60 samples at 1s = 1 minute
const BASELINE_PERCENTILE: f64 = 0.05; // 5th percentile for baseline

/// Baseline detector for power consumption
///
/// Uses a sliding window of power readings to detect the baseline (idle) power
/// consumption. The baseline is calculated as the 5th percentile of readings
/// to filter out occasional low spikes while capturing true idle power.
pub struct BaselineDetector {
    /// Power readings window for detection
    samples: VecDeque<f64>,
    /// Maximum samples to keep
    max_samples: usize,
    /// Manually set baseline (overrides auto-detection)
    manual_baseline: Option<f64>,
    /// Last detected baseline
    last_detected: Option<f64>,
}

impl BaselineDetector {
    /// Create a new baseline detector with default settings
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(DEFAULT_SAMPLE_WINDOW),
            max_samples: DEFAULT_SAMPLE_WINDOW,
            manual_baseline: None,
            last_detected: None,
        }
    }

    /// Create a baseline detector with custom sample window size
    pub fn with_window_size(size: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(size),
            max_samples: size,
            manual_baseline: None,
            last_detected: None,
        }
    }

    /// Add a power reading sample
    pub fn add_sample(&mut self, power_watts: f64) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(power_watts);
    }

    /// Set a manual baseline (disables auto-detection)
    pub fn set_manual_baseline(&mut self, watts: f64) {
        self.manual_baseline = Some(watts);
    }

    /// Clear manual baseline (re-enables auto-detection)
    pub fn clear_manual_baseline(&mut self) {
        self.manual_baseline = None;
    }

    /// Get the current baseline power
    ///
    /// Returns manual baseline if set, otherwise auto-detected baseline.
    pub fn get_baseline(&self) -> Option<f64> {
        self.manual_baseline.or(self.last_detected)
    }

    /// Check if using manual baseline
    pub fn is_manual(&self) -> bool {
        self.manual_baseline.is_some()
    }

    /// Get the number of samples collected
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Detect baseline from current samples
    ///
    /// Uses the 5th percentile of readings as the baseline.
    /// Returns None if not enough samples are available.
    pub fn detect_baseline(&mut self) -> Option<BaselineDetection> {
        if self.samples.len() < 10 {
            // Need at least 10 samples for meaningful detection
            return None;
        }

        // Sort samples to find percentile
        let mut sorted: Vec<f64> = self.samples.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate 5th percentile index
        let index = ((sorted.len() as f64) * BASELINE_PERCENTILE).floor() as usize;
        let baseline = sorted.get(index).copied().unwrap_or(sorted[0]);

        // Calculate confidence based on sample count and variance
        let sample_ratio = self.samples.len() as f64 / self.max_samples as f64;
        let variance = self.calculate_variance(&sorted);
        let variance_factor = 1.0 / (1.0 + variance / 100.0); // Normalize variance impact
        let confidence = (sample_ratio * variance_factor).clamp(0.0, 1.0);

        self.last_detected = Some(baseline);

        Some(BaselineDetection {
            detected_watts: baseline,
            sample_count: self.samples.len(),
            confidence,
        })
    }

    /// Calculate variance of samples
    fn calculate_variance(&self, sorted: &[f64]) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }

        let mean: f64 = sorted.iter().sum::<f64>() / sorted.len() as f64;
        let variance: f64 = sorted.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / sorted.len() as f64;

        variance.sqrt() // Return standard deviation
    }

    /// Calculate surplus power above baseline
    ///
    /// Returns (surplus_watts, is_above_baseline)
    pub fn calculate_surplus(&self, current_watts: f64) -> (f64, bool) {
        match self.get_baseline() {
            Some(baseline) => {
                let surplus = current_watts - baseline;
                (surplus.max(0.0), surplus > 0.0)
            }
            None => (0.0, false),
        }
    }

    /// Calculate surplus energy over time
    ///
    /// Given power readings and duration, calculate total surplus Wh.
    pub fn calculate_surplus_wh(&self, power_watts: f64, duration_hours: f64) -> f64 {
        let (surplus, _) = self.calculate_surplus(power_watts);
        surplus * duration_hours
    }

    /// Clear all samples and reset detection
    pub fn reset(&mut self) {
        self.samples.clear();
        self.last_detected = None;
    }
}

impl Default for BaselineDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_detection() {
        let mut detector = BaselineDetector::with_window_size(20);

        // Add idle samples (40-50W)
        for i in 0..10 {
            detector.add_sample(45.0 + (i as f64 % 5.0));
        }

        // Add some higher samples (100-150W)
        for i in 0..10 {
            detector.add_sample(100.0 + (i as f64 * 5.0));
        }

        let detection = detector.detect_baseline().unwrap();

        // Baseline should be close to the lower values
        assert!(detection.detected_watts < 60.0);
        assert!(detection.confidence > 0.0);
    }

    #[test]
    fn test_manual_baseline() {
        let mut detector = BaselineDetector::new();

        // Set manual baseline
        detector.set_manual_baseline(50.0);
        assert!(detector.is_manual());
        assert_eq!(detector.get_baseline(), Some(50.0));

        // Calculate surplus
        let (surplus, above) = detector.calculate_surplus(120.0);
        assert_eq!(surplus, 70.0);
        assert!(above);

        // Clear manual baseline
        detector.clear_manual_baseline();
        assert!(!detector.is_manual());
    }

    #[test]
    fn test_surplus_calculation() {
        let mut detector = BaselineDetector::new();
        detector.set_manual_baseline(50.0);

        // Current power at 150W for 1 hour
        let surplus_wh = detector.calculate_surplus_wh(150.0, 1.0);
        assert_eq!(surplus_wh, 100.0); // 150 - 50 = 100Wh

        // Below baseline should give 0
        let surplus_wh = detector.calculate_surplus_wh(40.0, 1.0);
        assert_eq!(surplus_wh, 0.0);
    }
}
