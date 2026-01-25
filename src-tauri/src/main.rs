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

use crate::core::{AppState, BaselineDetection, Config, CriticalMetrics, DetailedMetrics, ProcessMetrics, Session, SystemMetrics};
use crate::db::Database;
use crate::hardware::{BaselineDetector, PowerMonitor};
use crate::i18n::I18n;
use crate::pricing::PricingEngine;
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tokio::sync::Mutex;

/// Application state shared across all Tauri commands
pub struct TauriState {
    pub config: Arc<Mutex<Config>>,
    pub db: Arc<Mutex<Database>>,
    pub monitor: Arc<Mutex<PowerMonitor>>,
    pub pricing: Arc<Mutex<PricingEngine>>,
    pub i18n: Arc<Mutex<I18n>>,
    pub app_state: Arc<Mutex<AppState>>,
    pub baseline_detector: Arc<Mutex<BaselineDetector>>,
    pub active_session: Arc<Mutex<Option<SessionState>>>,
    /// Cached critical metrics (updated at fast rate)
    pub critical_metrics_cache: Arc<Mutex<Option<CriticalMetrics>>>,
    /// Cached detailed metrics (updated at slow rate)
    pub detailed_metrics_cache: Arc<Mutex<Option<DetailedMetrics>>>,
}

/// State for an active tracking session
pub struct SessionState {
    pub id: i64,
    pub baseline_watts: f64,
    pub total_wh: f64,
    pub surplus_wh: f64,
    pub start_time: std::time::Instant,
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

// ===== New System Metrics Commands =====

/// Get system metrics (CPU, GPU, RAM)
#[tauri::command]
async fn get_system_metrics(state: tauri::State<'_, TauriState>) -> Result<SystemMetrics, String> {
    let monitor = state.monitor.lock().await;
    monitor.get_system_metrics().map_err(|e| e.to_string())
}

/// Get top processes by CPU usage (with pinned processes)
#[tauri::command]
async fn get_top_processes(state: tauri::State<'_, TauriState>, limit: Option<usize>) -> Result<Vec<ProcessMetrics>, String> {
    let config = state.config.lock().await;
    let limit = limit.unwrap_or(config.advanced.process_list_limit);
    let pinned = config.advanced.pinned_processes.clone();
    drop(config);

    let monitor = state.monitor.lock().await;
    monitor.get_top_processes_with_pinned(limit, &pinned).map_err(|e| e.to_string())
}

/// Get all processes (for discovery mode)
#[tauri::command]
async fn get_all_processes(state: tauri::State<'_, TauriState>) -> Result<Vec<ProcessMetrics>, String> {
    let monitor = state.monitor.lock().await;
    monitor.get_all_processes().map_err(|e| e.to_string())
}

/// Pin a process for tracking
#[tauri::command]
async fn pin_process(state: tauri::State<'_, TauriState>, name: String) -> Result<Vec<String>, String> {
    let mut config = state.config.lock().await;
    if !config.advanced.pinned_processes.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
        config.advanced.pinned_processes.push(name);
        config.save().map_err(|e| e.to_string())?;
    }
    Ok(config.advanced.pinned_processes.clone())
}

/// Unpin a process
#[tauri::command]
async fn unpin_process(state: tauri::State<'_, TauriState>, name: String) -> Result<Vec<String>, String> {
    let mut config = state.config.lock().await;
    config.advanced.pinned_processes.retain(|p| !p.eq_ignore_ascii_case(&name));
    config.save().map_err(|e| e.to_string())?;
    Ok(config.advanced.pinned_processes.clone())
}

/// Get pinned processes list
#[tauri::command]
async fn get_pinned_processes(state: tauri::State<'_, TauriState>) -> Result<Vec<String>, String> {
    let config = state.config.lock().await;
    Ok(config.advanced.pinned_processes.clone())
}

/// Set process list limit
#[tauri::command]
async fn set_process_limit(state: tauri::State<'_, TauriState>, limit: usize) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.advanced.process_list_limit = limit;
    config.save().map_err(|e| e.to_string())
}

// ===== Session Tracking Commands =====

/// Start a new tracking session
#[tauri::command]
async fn start_tracking_session(
    state: tauri::State<'_, TauriState>,
    label: Option<String>,
) -> Result<i64, String> {
    // Get baseline
    let baseline_watts = {
        let config = state.config.lock().await;
        if config.advanced.baseline_auto {
            let detector = state.baseline_detector.lock().await;
            detector.get_baseline().unwrap_or(0.0)
        } else {
            config.advanced.baseline_watts
        }
    };

    // Create session in database
    let session_id = {
        let db = state.db.lock().await;
        db.start_session(baseline_watts, label.as_deref())
            .map_err(|e| e.to_string())?
    };

    // Set active session
    {
        let mut active = state.active_session.lock().await;
        *active = Some(SessionState {
            id: session_id,
            baseline_watts,
            total_wh: 0.0,
            surplus_wh: 0.0,
            start_time: std::time::Instant::now(),
        });
    }

    Ok(session_id)
}

/// End the current tracking session
#[tauri::command]
async fn end_tracking_session(state: tauri::State<'_, TauriState>) -> Result<Option<Session>, String> {
    let session_state = {
        let mut active = state.active_session.lock().await;
        active.take()
    };

    match session_state {
        Some(session) => {
            // Calculate final surplus cost
            let surplus_cost = {
                let pricing = state.pricing.lock().await;
                pricing.calculate_cost(session.surplus_wh / 1000.0)
            };

            // End session in database
            let db = state.db.lock().await;
            db.end_session(session.id, session.total_wh, session.surplus_wh, surplus_cost)
                .map_err(|e| e.to_string())
        }
        None => Ok(None),
    }
}

/// Get current session statistics
#[tauri::command]
async fn get_session_stats(state: tauri::State<'_, TauriState>) -> Result<Option<Session>, String> {
    let active = state.active_session.lock().await;

    match active.as_ref() {
        Some(session) => {
            let pricing = state.pricing.lock().await;
            let surplus_cost = pricing.calculate_cost(session.surplus_wh / 1000.0);

            Ok(Some(Session {
                id: Some(session.id),
                start_time: chrono::Utc::now().timestamp() - session.start_time.elapsed().as_secs() as i64,
                end_time: None,
                baseline_watts: session.baseline_watts,
                total_wh: session.total_wh,
                surplus_wh: session.surplus_wh,
                surplus_cost,
                label: None, // Would need to fetch from DB for label
            }))
        }
        None => Ok(None),
    }
}

/// Get session history
#[tauri::command]
async fn get_sessions(state: tauri::State<'_, TauriState>, limit: Option<u32>) -> Result<Vec<Session>, String> {
    let db = state.db.lock().await;
    db.get_sessions(limit).map_err(|e| e.to_string())
}

// ===== Baseline Detection Commands =====

/// Detect baseline power consumption
#[tauri::command]
async fn detect_baseline(state: tauri::State<'_, TauriState>) -> Result<Option<BaselineDetection>, String> {
    let mut detector = state.baseline_detector.lock().await;
    Ok(detector.detect_baseline())
}

/// Set manual baseline
#[tauri::command]
async fn set_manual_baseline(state: tauri::State<'_, TauriState>, watts: f64) -> Result<(), String> {
    // Update detector
    {
        let mut detector = state.baseline_detector.lock().await;
        detector.set_manual_baseline(watts);
    }

    // Update config
    {
        let mut config = state.config.lock().await;
        config.advanced.baseline_watts = watts;
        config.advanced.baseline_auto = false;
        config.save().map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Enable auto baseline detection
#[tauri::command]
async fn enable_auto_baseline(state: tauri::State<'_, TauriState>) -> Result<(), String> {
    // Clear manual baseline
    {
        let mut detector = state.baseline_detector.lock().await;
        detector.clear_manual_baseline();
    }

    // Update config
    {
        let mut config = state.config.lock().await;
        config.advanced.baseline_auto = true;
        config.save().map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get dashboard config for UI
#[tauri::command]
async fn get_dashboard_config(state: tauri::State<'_, TauriState>) -> Result<crate::core::DashboardConfig, String> {
    let config = state.config.lock().await;
    Ok(config.dashboard.clone())
}

/// Save dashboard config
#[tauri::command]
async fn save_dashboard_config(
    state: tauri::State<'_, TauriState>,
    dashboard: crate::core::DashboardConfig,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.dashboard = dashboard;
    config.save().map_err(|e| e.to_string())
}

/// Set autostart (start with system) enabled/disabled
#[tauri::command]
async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let autostart_manager = app.autolaunch();

    if enabled {
        autostart_manager.enable().map_err(|e| e.to_string())?;
        log::info!("Autostart enabled");
    } else {
        autostart_manager.disable().map_err(|e| e.to_string())?;
        log::info!("Autostart disabled");
    }

    Ok(())
}

// ===== Tiered Monitoring API (Fast/Slow refresh) =====

/// Get critical metrics (cached, updated at fast rate)
/// Returns power, CPU%, GPU%, cost, session data - always responsive
#[tauri::command]
async fn get_critical_metrics(state: tauri::State<'_, TauriState>) -> Result<Option<CriticalMetrics>, String> {
    let cache = state.critical_metrics_cache.lock().await;
    Ok(cache.clone())
}

/// Get detailed metrics (cached, updated at slow rate)
/// Returns processes, temps, VRAM - may be slightly stale
#[tauri::command]
async fn get_detailed_metrics(state: tauri::State<'_, TauriState>) -> Result<Option<DetailedMetrics>, String> {
    let cache = state.detailed_metrics_cache.lock().await;
    Ok(cache.clone())
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

    // Initialize baseline detector with config
    let mut baseline_detector = BaselineDetector::new();
    if !config.advanced.baseline_auto && config.advanced.baseline_watts > 0.0 {
        baseline_detector.set_manual_baseline(config.advanced.baseline_watts);
    }

    // Wrap in Arc<Mutex> for thread-safe sharing
    let state = TauriState {
        config: Arc::new(Mutex::new(config)),
        db: Arc::new(Mutex::new(db)),
        monitor: Arc::new(Mutex::new(monitor)),
        pricing: Arc::new(Mutex::new(pricing)),
        i18n: Arc::new(Mutex::new(i18n)),
        app_state: Arc::new(Mutex::new(app_state)),
        baseline_detector: Arc::new(Mutex::new(baseline_detector)),
        active_session: Arc::new(Mutex::new(None)),
        critical_metrics_cache: Arc::new(Mutex::new(None)),
        detailed_metrics_cache: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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
            // New system metrics commands
            get_system_metrics,
            get_top_processes,
            get_all_processes,
            pin_process,
            unpin_process,
            get_pinned_processes,
            set_process_limit,
            // Session tracking commands
            start_tracking_session,
            end_tracking_session,
            get_session_stats,
            get_sessions,
            // Baseline detection commands
            detect_baseline,
            set_manual_baseline,
            enable_auto_baseline,
            // Dashboard config commands
            get_dashboard_config,
            save_dashboard_config,
            // Autostart command
            set_autostart,
            // Tiered monitoring API (fast/slow refresh)
            get_critical_metrics,
            get_detailed_metrics,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Check if start_minimized is enabled and hide the main window
            let state: tauri::State<'_, TauriState> = app.state();
            let start_minimized = {
                // Use block_on since we're in sync context
                let config = tauri::async_runtime::block_on(state.config.lock());
                config.general.start_minimized
            };

            if start_minimized {
                if let Some(main_window) = app.get_webview_window("main") {
                    let _ = main_window.hide();
                    log::info!("Started minimized - main window hidden");
                }
            }

            // Create tray menu
            let quit_item = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Build tray icon with menu
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "quit" => {
                            log::info!("Quit requested from tray menu");
                            std::process::exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                log::info!("Window shown from tray menu");
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            log::info!("Window shown from tray icon click");
                        }
                    }
                })
                .build(app)?;

            // Start critical monitoring loop (fast rate: power, CPU%, GPU%, cost)
            let app_handle_critical = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                critical_monitoring_loop(app_handle_critical).await;
            });

            // Start detailed monitoring loop (slow rate: processes, temps, VRAM)
            let app_handle_detailed = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                detailed_monitoring_loop(app_handle_detailed).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Only intercept close for main window
                if window.label() == "main" {
                    // Hide window instead of closing
                    let _ = window.hide();
                    api.prevent_close();
                    log::info!("Main window hidden to tray");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Critical monitoring loop - runs at fast rate (user's refresh_rate_ms)
/// Updates: power, CPU%, GPU% (from cache), cost, session tracking
/// NEVER blocks on GPU commands - uses cached values for GPU metrics
async fn critical_monitoring_loop(app: tauri::AppHandle) {
    log::info!("Starting critical monitoring loop");
    let state: tauri::State<'_, TauriState> = app.state();

    let mut last_reading_time = std::time::Instant::now();

    // Get initial refresh rate
    let initial_refresh_ms = {
        let config = state.config.lock().await;
        config.general.refresh_rate_ms
    };
    let mut current_refresh_ms = initial_refresh_ms;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(current_refresh_ms));

    log::info!("Critical monitoring loop initialized with {}ms refresh rate", current_refresh_ms);

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
            log::info!("Critical monitoring loop rate changed to {}ms", refresh_ms);
        }

        // Read power using FAST path (CPU-only + cached GPU, no blocking commands)
        let (power_watts, cpu_usage, gpu_usage, gpu_power) = {
            let monitor = state.monitor.lock().await;
            monitor.get_power_watts_fast().unwrap_or((0.0, 0.0, None, None))
        };

        // Calculate energy consumed since last reading
        let elapsed_hours = last_reading_time.elapsed().as_secs_f64() / 3600.0;
        let energy_wh = power_watts * elapsed_hours;
        last_reading_time = std::time::Instant::now();

        // Update app state and get values for critical metrics
        let (cumulative_wh, current_cost, session_duration_secs) = {
            let mut app_state = state.app_state.lock().await;
            app_state.cumulative_wh += energy_wh;
            app_state.last_power_watts = power_watts;

            // Update cost
            let pricing = state.pricing.lock().await;
            app_state.current_cost = pricing.calculate_cost(app_state.cumulative_wh / 1000.0);

            (
                app_state.cumulative_wh,
                app_state.current_cost,
                app_state.session_start.elapsed().as_secs(),
            )
        };

        // Calculate cost estimates
        let (hourly_cost, daily_cost, monthly_cost) = {
            let pricing = state.pricing.lock().await;
            (
                pricing.calculate_hourly_cost(power_watts),
                pricing.calculate_daily_cost(power_watts),
                pricing.calculate_monthly_cost(power_watts),
            )
        };

        // Update baseline detector with new sample
        {
            let mut detector = state.baseline_detector.lock().await;
            detector.add_sample(power_watts);
        }

        // Update active session and get session data
        let active_session = {
            let mut active = state.active_session.lock().await;

            if let Some(ref mut session) = *active {
                session.total_wh += energy_wh;

                // Calculate surplus (power above baseline)
                let surplus_watts = (power_watts - session.baseline_watts).max(0.0);
                let surplus_energy = surplus_watts * elapsed_hours;
                session.surplus_wh += surplus_energy;

                // Build session data for frontend
                let pricing = state.pricing.lock().await;
                let surplus_cost = pricing.calculate_cost(session.surplus_wh / 1000.0);

                Some(Session {
                    id: Some(session.id),
                    start_time: chrono::Utc::now().timestamp() - session.start_time.elapsed().as_secs() as i64,
                    end_time: None,
                    baseline_watts: session.baseline_watts,
                    total_wh: session.total_wh,
                    surplus_wh: session.surplus_wh,
                    surplus_cost,
                    label: None,
                })
            } else {
                None
            }
        };

        // Get source info
        let (source, is_estimated) = {
            let monitor = state.monitor.lock().await;
            (monitor.get_source_name().to_string(), monitor.is_estimated())
        };

        // Build and cache critical metrics
        let critical_metrics = CriticalMetrics {
            power_watts,
            cpu_usage_percent: cpu_usage,
            gpu_usage_percent: gpu_usage,
            gpu_power_watts: gpu_power,
            cumulative_wh,
            current_cost,
            hourly_cost_estimate: hourly_cost,
            daily_cost_estimate: daily_cost,
            monthly_cost_estimate: monthly_cost,
            session_duration_secs,
            active_session,
            source,
            is_estimated,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Update cache
        {
            let mut cache = state.critical_metrics_cache.lock().await;
            *cache = Some(critical_metrics.clone());
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

        // Emit critical update event to frontend
        let _ = app.emit("critical-update", critical_metrics);
    }
}

/// Detailed monitoring loop - runs at slow rate (slow_refresh_rate_ms, default 5s)
/// Updates: top processes, temperatures, VRAM details
/// This loop uses spawn_blocking for GPU commands to avoid blocking the async runtime
async fn detailed_monitoring_loop(app: tauri::AppHandle) {
    log::info!("Starting detailed monitoring loop");
    let state: tauri::State<'_, TauriState> = app.state();

    // Get initial slow refresh rate
    let initial_slow_refresh_ms = {
        let config = state.config.lock().await;
        config.general.slow_refresh_rate_ms
    };
    let mut current_slow_refresh_ms = initial_slow_refresh_ms;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(current_slow_refresh_ms));

    log::info!("Detailed monitoring loop initialized with {}ms refresh rate", current_slow_refresh_ms);

    loop {
        interval.tick().await;

        // Get current slow refresh rate from config
        let slow_refresh_ms = {
            let config = state.config.lock().await;
            config.general.slow_refresh_rate_ms
        };

        // Only recreate interval if refresh rate changed
        if slow_refresh_ms != current_slow_refresh_ms {
            current_slow_refresh_ms = slow_refresh_ms;
            interval = tokio::time::interval(tokio::time::Duration::from_millis(slow_refresh_ms));
            log::info!("Detailed monitoring loop rate changed to {}ms", slow_refresh_ms);
        }

        // Get config for process limit and pinned processes
        let (limit, pinned) = {
            let config = state.config.lock().await;
            (
                config.advanced.process_list_limit,
                config.advanced.pinned_processes.clone(),
            )
        };

        // Collect detailed metrics in a blocking task to avoid blocking async runtime
        // This is where slow GPU commands (nvidia-smi) and process enumeration happen
        let detailed_metrics = {
            let monitor = state.monitor.lock().await;
            // Use spawn_blocking for the slow operations
            let limit_clone = limit;
            let pinned_clone = pinned.clone();

            // We need to clone what we need since spawn_blocking requires 'static
            match monitor.collect_detailed_metrics(limit_clone, &pinned_clone) {
                Ok(metrics) => Some(metrics),
                Err(e) => {
                    log::debug!("Failed to collect detailed metrics: {}", e);
                    // Fallback: try to get metrics individually
                    let system_metrics = monitor.get_system_metrics().ok();
                    let top_processes = monitor.get_top_processes_with_pinned(limit_clone, &pinned_clone).unwrap_or_default();

                    Some(DetailedMetrics {
                        system_metrics,
                        top_processes,
                        timestamp: chrono::Utc::now().timestamp(),
                    })
                }
            }
        };

        // Update cache
        if let Some(metrics) = detailed_metrics.clone() {
            let mut cache = state.detailed_metrics_cache.lock().await;
            *cache = Some(metrics);
        }

        // Emit detailed update event to frontend
        if let Some(metrics) = detailed_metrics {
            let _ = app.emit("detailed-update", metrics);
        }
    }
}
