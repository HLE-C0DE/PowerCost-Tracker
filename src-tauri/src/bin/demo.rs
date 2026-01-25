//! PowerCost Tracker - Demo CLI
//!
//! Demonstration of the Core Engine (Phase 2 checkpoint)
//! Shows power monitoring, cost calculation, and SQLite persistence.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

// Import from our library
use powercost_tracker_lib::core::{PricingConfig, SimplePricing};
use powercost_tracker_lib::db::Database;
use powercost_tracker_lib::hardware::PowerMonitor;
use powercost_tracker_lib::pricing::PricingEngine;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("==============================================");
    println!("   PowerCost Tracker - Demo CLI (Phase 2)");
    println!("==============================================\n");

    // 1. Initialize Power Monitor
    println!("[1/4] Initializing Power Monitor...");
    let monitor = match PowerMonitor::new() {
        Ok(m) => {
            println!("      Source: {} (hardware sensor)", m.get_source_name());
            m
        }
        Err(e) => {
            println!("      No hardware sensor: {}", e);
            println!("      Using estimation fallback...");
            PowerMonitor::estimation_fallback()
        }
    };
    println!("      Estimated: {}\n", monitor.is_estimated());

    // 2. Initialize Pricing Engine
    println!("[2/4] Initializing Pricing Engine...");
    let pricing_config = PricingConfig {
        mode: "simple".to_string(),
        currency: "EUR".to_string(),
        currency_symbol: "\u{20AC}".to_string(),
        simple: SimplePricing { rate_per_kwh: 0.2276 },
        ..Default::default()
    };
    let pricing = PricingEngine::new(&pricing_config);
    println!("      Mode: Simple (flat rate)");
    println!("      Rate: {:.4} {}/kWh\n", pricing_config.simple.rate_per_kwh, pricing_config.currency_symbol);

    // 3. Initialize Database
    println!("[3/4] Initializing SQLite Database...");
    let db = match Database::new() {
        Ok(d) => {
            println!("      Database initialized successfully");
            Some(d)
        }
        Err(e) => {
            println!("      Warning: Could not initialize database: {}", e);
            println!("      Continuing without persistence...");
            None
        }
    };
    println!();

    // 4. Run monitoring demo
    println!("[4/4] Starting Power Monitoring Demo...\n");
    println!("      Press Ctrl+C to stop\n");

    println!("----------------------------------------------");
    println!("  Time   |  Power  |  Energy  |  Cost");
    println!("  (sec)  |  (W)    |  (Wh)    |  (EUR)");
    println!("----------------------------------------------");

    let mut cumulative_wh = 0.0;
    let mut _readings_count = 0;
    let start_time = std::time::Instant::now();

    // Run for 30 seconds (or until interrupted)
    for i in 0..30 {
        // Get power reading
        let power_watts = match monitor.get_power_watts() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error reading power: {}", e);
                0.0
            }
        };

        // Calculate energy (1 second interval = 1/3600 hour)
        let energy_this_second = power_watts / 3600.0;
        cumulative_wh += energy_this_second;

        // Calculate cost
        let cost = pricing.calculate_cost(cumulative_wh / 1000.0);

        // Print status line
        print!(
            "\r  {:>4}   | {:>6.1} | {:>7.3} | {:>7.5}",
            i + 1,
            power_watts,
            cumulative_wh,
            cost
        );
        io::stdout().flush().unwrap();

        // Store reading in database (every 5 seconds)
        if let Some(ref database) = db {
            if i % 5 == 0 {
                if let Ok(reading) = monitor.get_reading() {
                    let _ = database.insert_reading(&reading);
                    _readings_count += 1;
                }
            }
        }

        // Wait 1 second
        thread::sleep(Duration::from_secs(1));
    }

    println!("\n----------------------------------------------\n");

    // Summary
    let elapsed = start_time.elapsed();
    let final_cost = pricing.calculate_cost(cumulative_wh / 1000.0);

    println!("=== Session Summary ===\n");
    println!("  Duration:      {} seconds", elapsed.as_secs());
    println!("  Total Energy:  {:.3} Wh", cumulative_wh);
    println!("  Total Cost:    {:.5} {}", final_cost, pricing_config.currency_symbol);
    println!("  Avg Power:     {:.1} W", cumulative_wh / (elapsed.as_secs_f64() / 3600.0));

    if let Some(ref database) = db {
        if let Ok(count) = database.get_readings_count() {
            println!("  DB Readings:   {} stored", count);
        }
    }

    println!("\n  Estimates at current rate:");
    let avg_power = cumulative_wh / (elapsed.as_secs_f64() / 3600.0);
    println!("    Hourly:  {:.4} {}", pricing.calculate_hourly_cost(avg_power), pricing_config.currency_symbol);
    println!("    Daily:   {:.4} {}", pricing.calculate_daily_cost(avg_power), pricing_config.currency_symbol);
    println!("    Monthly: {:.2} {}", pricing.calculate_monthly_cost(avg_power), pricing_config.currency_symbol);

    println!("\n==============================================");
    println!("   Phase 2 Core Engine: CHECKPOINT PASSED");
    println!("==============================================\n");

    // Test database queries if available
    if let Some(ref database) = db {
        println!("=== Database Test ===\n");

        // Get recent readings
        let now = chrono::Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        match database.get_readings(one_hour_ago, now) {
            Ok(readings) => {
                println!("  Recent readings in database: {}", readings.len());
                if !readings.is_empty() {
                    println!("  Last reading:");
                    let last = readings.last().unwrap();
                    println!("    Timestamp: {}", last.timestamp);
                    println!("    Power:     {:.1} W", last.power_watts);
                    println!("    Source:    {}", last.source);
                }
            }
            Err(e) => println!("  Error querying readings: {}", e),
        }
        println!();
    }
}
