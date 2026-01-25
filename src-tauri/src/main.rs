//! PowerCost Tracker - Main entry point
//!
//! A lightweight desktop application for monitoring PC power consumption
//! and calculating electricity costs in real-time.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod db;
mod hardware;
mod i18n;
mod pricing;

use crate::core::{AppState, Config};
use crate::db::Database;
use crate::hardware::PowerMonitor;
use crate::i18n::I18n;
use crate::pricing::PricingEngine;
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;

/// Application state shared across all Tauri commands
pub struct TauriState {
    pub config: Arc<Mutex<Config>>,
    pub db: Arc<Mutex<Database>>,
    pub monitor: Arc<Mutex<PowerMonitor>>,
    pub pricing: Arc<Mutex<PricingEngine>>,
    pub i18n: Arc<Mutex<I18n>>,
    pub app_state: Arc<Mutex<AppState>>,
}

// Tauri commands exposed to the frontend

/// Get current power consumption in watts
#[tauri::command]
async fn get_power_watts(state: tauri::State<'_, TauriState>) -> Result<f64, String> {
    let monitor = state.monitor.lock().await;
    monitor.get_power_watts().map_err(|e| e.to_string())
}

/// Get current power reading with full details
#[tauri::command]
async fn get_power_reading(state: tauri::State<'_, TauriState>) -> Result<core::PowerReading, String> {
    let monitor = state.monitor.lock().await;
    monitor.get_reading().map_err(|e| e.to_string())
}

/// Get cumulative energy consumption since tracking started
#[tauri::command]
async fn get_energy_wh(state: tauri::State<'_, TauriState>) -> Result<f64, String> {
    let app_state = state.app_state.lock().await;
    Ok(app_state.cumulative_wh)
}

/// Get current cost based on energy consumed
#[tauri::command]
async fn get_current_cost(state: tauri::State<'_, TauriState>) -> Result<f64, String> {
    let app_state = state.app_state.lock().await;
    Ok(app_state.current_cost)
}

/// Get full dashboard data in one call (more efficient)
#[tauri::command]
async fn get_dashboard_data(state: tauri::State<'_, TauriState>) -> Result<core::DashboardData, String> {
    let app_state = state.app_state.lock().await;
    let monitor = state.monitor.lock().await;
    let pricing = state.pricing.lock().await;

    let power_watts = monitor.get_power_watts().unwrap_or_else(|e| {
        log::warn!("Failed to get power reading: {}", e);
        0.0
    });
    let hourly_cost = pricing.calculate_hourly_cost(power_watts);
    let daily_cost = pricing.calculate_daily_cost(power_watts);
    let monthly_cost = pricing.calculate_monthly_cost(power_watts);

    Ok(core::DashboardData {
        power_watts,
        cumulative_wh: app_state.cumulative_wh,
        current_cost: app_state.current_cost,
        hourly_cost_estimate: hourly_cost,
        daily_cost_estimate: daily_cost,
        monthly_cost_estimate: monthly_cost,
        session_duration_secs: app_state.session_start.elapsed().as_secs(),
        source: monitor.get_source_name().to_string(),
        is_estimated: monitor.is_estimated(),
    })
}

/// Get application configuration
#[tauri::command]
async fn get_config(state: tauri::State<'_, TauriState>) -> Result<Config, String> {
    let config = state.config.lock().await;
    Ok(config.clone())
}

/// Update application configuration
#[tauri::command]
async fn set_config(state: tauri::State<'_, TauriState>, config: Config) -> Result<(), String> {
    let mut current_config = state.config.lock().await;
    *current_config = config.clone();
    current_config.save().map_err(|e| e.to_string())?;

    // Update pricing engine with new config
    let mut pricing = state.pricing.lock().await;
    pricing.update_config(&config.pricing);

    // Update i18n with new language
    let mut i18n = state.i18n.lock().await;
    i18n.set_language(&config.general.language);

    Ok(())
}

/// Get translated string
#[tauri::command]
async fn translate(state: tauri::State<'_, TauriState>, key: String) -> Result<String, String> {
    let i18n = state.i18n.lock().await;
    Ok(i18n.get(&key))
}

/// Get all translations for current language
#[tauri::command]
async fn get_translations(state: tauri::State<'_, TauriState>) -> Result<std::collections::HashMap<String, String>, String> {
    let i18n = state.i18n.lock().await;
    Ok(i18n.get_all())
}

/// Get historical data for a date range
#[tauri::command]
async fn get_history(
    state: tauri::State<'_, TauriState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<db::DailyStats>, String> {
    let db = state.db.lock().await;
    let config = state.config.lock().await;
    let pricing_mode = config.pricing.mode.clone();
    drop(config);

    // Update today's stats before fetching to ensure fresh data
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if start_date <= today && end_date >= today {
        let _ = db.update_today_stats(Some(&pricing_mode));
    }

    db.get_daily_stats(&start_date, &end_date)
        .map_err(|e| e.to_string())
}

/// Get power readings for a time range (for graphs)
#[tauri::command]
async fn get_readings(
    state: tauri::State<'_, TauriState>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<db::PowerReadingRecord>, String> {
    let db = state.db.lock().await;
    db.get_readings(start_timestamp, end_timestamp)
        .map_err(|e| e.to_string())
}

/// Open the widget window
#[tauri::command]
async fn open_widget(app: tauri::AppHandle, state: tauri::State<'_, TauriState>) -> Result<(), String> {
    // Check if widget is already open
    if app.get_webview_window("widget").is_some() {
        return Ok(());
    }

    // Get widget position from config
    let config = state.config.lock().await;
    let position = &config.widget.position;

    // Calculate position based on config
    let (x, y) = match position.as_str() {
        "top_left" => (20.0, 20.0),
        "top_right" => (1200.0, 20.0),  // Will be adjusted by screen size
        "bottom_left" => (20.0, 700.0),
        "bottom_right" => (1200.0, 700.0),
        _ => (20.0, 20.0),
    };

    // Create widget window
    let _widget = WebviewWindowBuilder::new(&app, "widget", WebviewUrl::App("widget.html".into()))
        .title("PowerCost Widget")
        .inner_size(180.0, 70.0)
        .position(x, y)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .transparent(true)
        .skip_taskbar(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Close the widget window
#[tauri::command]
async fn close_widget(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(widget) = app.get_webview_window("widget") {
        widget.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Toggle widget visibility
#[tauri::command]
async fn toggle_widget(app: tauri::AppHandle, state: tauri::State<'_, TauriState>) -> Result<bool, String> {
    if let Some(widget) = app.get_webview_window("widget") {
        widget.close().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        open_widget(app, state).await?;
        Ok(true)
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting PowerCost Tracker v{}", env!("CARGO_PKG_VERSION"));

    // Load or create configuration
    let config = Config::load().unwrap_or_else(|e| {
        log::warn!("Failed to load config, using defaults: {}", e);
        Config::default()
    });

    // Initialize database
    let db = Database::new().unwrap_or_else(|e| {
        log::error!("Failed to initialize database: {}", e);
        std::process::exit(1);
    });

    // Initialize power monitor
    let monitor = PowerMonitor::new().unwrap_or_else(|e| {
        log::warn!("Failed to initialize power monitor: {}", e);
        PowerMonitor::estimation_fallback()
    });

    // Initialize pricing engine
    let pricing = PricingEngine::new(&config.pricing);

    // Initialize i18n
    let i18n = I18n::new(&config.general.language);

    // Create application state
    let app_state = AppState::new();

    // Wrap in Arc<Mutex> for thread-safe sharing
    let state = TauriState {
        config: Arc::new(Mutex::new(config)),
        db: Arc::new(Mutex::new(db)),
        monitor: Arc::new(Mutex::new(monitor)),
        pricing: Arc::new(Mutex::new(pricing)),
        i18n: Arc::new(Mutex::new(i18n)),
        app_state: Arc::new(Mutex::new(app_state)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_power_watts,
            get_power_reading,
            get_energy_wh,
            get_current_cost,
            get_dashboard_data,
            get_config,
            set_config,
            translate,
            get_translations,
            get_history,
            get_readings,
            open_widget,
            close_widget,
            toggle_widget,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Start background monitoring task
            tauri::async_runtime::spawn(async move {
                monitoring_loop(app_handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Background task that periodically reads power and updates state
async fn monitoring_loop(app: tauri::AppHandle) {
    log::info!("Starting monitoring loop");
    let state: tauri::State<'_, TauriState> = app.state();

    let mut last_reading_time = std::time::Instant::now();

    // Get initial refresh rate
    let initial_refresh_ms = {
        let config = state.config.lock().await;
        config.general.refresh_rate_ms
    };
    let mut current_refresh_ms = initial_refresh_ms;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(current_refresh_ms));

    log::info!("Monitoring loop initialized with {}ms refresh rate", current_refresh_ms);

    loop {
        interval.tick().await;

        // Get current refresh rate from config
        let refresh_ms = {
            let config = state.config.lock().await;
            config.general.refresh_rate_ms
        };

        // Only recreate interval if refresh rate changed
        if refresh_ms != current_refresh_ms {
            current_refresh_ms = refresh_ms;
            interval = tokio::time::interval(tokio::time::Duration::from_millis(refresh_ms));
        }

        // Read power
        let power_watts = {
            let monitor = state.monitor.lock().await;
            monitor.get_power_watts().unwrap_or(0.0)
        };

        // Calculate energy consumed since last reading
        let elapsed_hours = last_reading_time.elapsed().as_secs_f64() / 3600.0;
        let energy_wh = power_watts * elapsed_hours;
        last_reading_time = std::time::Instant::now();

        // Update app state
        {
            let mut app_state = state.app_state.lock().await;
            app_state.cumulative_wh += energy_wh;

            // Update cost
            let pricing = state.pricing.lock().await;
            app_state.current_cost = pricing.calculate_cost(app_state.cumulative_wh / 1000.0);
        }

        // Store reading in database (every 10 readings to reduce writes)
        static READING_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = READING_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count % 10 == 0 {
            let monitor = state.monitor.lock().await;
            if let Ok(reading) = monitor.get_reading() {
                let db = state.db.lock().await;
                let _ = db.insert_reading(&reading);

                // Update daily stats every 60 readings (~every minute at 1s refresh)
                if count % 60 == 0 {
                    let config = state.config.lock().await;
                    let pricing_mode = config.pricing.mode.clone();
                    drop(config);
                    let _ = db.update_today_stats(Some(&pricing_mode));
                }
            }
        }

        // Emit event to frontend
        let _ = app.emit("power-update", power_watts);
    }
}
