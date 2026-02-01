//! PowerCost Tracker - Main entry point
//!
//! A lightweight desktop application for monitoring PC power consumption
//! and calculating electricity costs in real-time.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod db;
mod elevation;
mod hardware;
mod i18n;
mod pricing;

use crate::core::{AppState, BaselineDetection, Config, CriticalMetrics, DetailedMetrics, LayoutProfile, ProcessMetrics, Session, SessionCategory, SystemMetrics};
use crate::db::Database;
use crate::hardware::{BaselineDetector, PowerMonitor};
use crate::i18n::I18n;
use crate::pricing::PricingEngine;
use std::sync::Arc;
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;
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
    pub label: Option<String>,
    pub category: Option<String>,
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

    // Use session average power for estimates instead of instantaneous
    let session_duration_secs = app_state.session_start.elapsed().as_secs();
    let avg_power_watts = if session_duration_secs > 0 {
        app_state.cumulative_wh / (session_duration_secs as f64 / 3600.0)
    } else {
        power_watts
    };

    let hourly_cost = pricing.calculate_hourly_cost(avg_power_watts);
    let daily_cost = pricing.calculate_daily_cost(avg_power_watts);
    let monthly_cost = pricing.calculate_monthly_cost(avg_power_watts);

    Ok(core::DashboardData {
        power_watts,
        avg_power_watts,
        cumulative_wh: app_state.cumulative_wh,
        current_cost: app_state.current_cost,
        hourly_cost_estimate: hourly_cost,
        daily_cost_estimate: daily_cost,
        monthly_cost_estimate: monthly_cost,
        session_duration_secs,
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

    // Get current rate from pricing engine
    let rate_per_kwh = {
        let pricing = state.pricing.lock().await;
        pricing.get_current_rate()
    };

    // Update today's stats before fetching to ensure fresh data
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if start_date <= today && end_date >= today {
        let _ = db.update_today_stats(Some(&pricing_mode), Some(rate_per_kwh));
    }

    let mut stats = db.get_daily_stats(&start_date, &end_date)
        .map_err(|e| e.to_string())?;

    // Backfill cost for any days that have NULL total_cost
    for stat in stats.iter_mut() {
        if stat.total_cost.is_none() && stat.total_wh > 0.0 {
            stat.total_cost = Some((stat.total_wh / 1000.0) * rate_per_kwh);
        }
    }

    Ok(stats)
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

/// Kill a process by name
#[tauri::command]
async fn kill_process(name: String) -> Result<(), String> {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes();

    let mut found = false;
    let mut killed = false;
    for (_pid, process) in sys.processes() {
        if process.name().eq_ignore_ascii_case(&name) {
            found = true;
            if process.kill() {
                killed = true;
            }
        }
    }

    if killed {
        Ok(())
    } else if found {
        Err(format!("ACCESS_DENIED:{}", name))
    } else {
        Err(format!("NOT_FOUND:{}", name))
    }
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
    // Guard: don't start a new session if one is already active
    {
        let active = state.active_session.lock().await;
        if active.is_some() {
            return Err("A session is already active".to_string());
        }
    }

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
            label: label.clone(),
            category: None,
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
                label: session.label.clone(),
                category: session.category.clone(),
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

// ===== Session Category & Label Commands =====

/// Update a session's label
#[tauri::command]
async fn update_session_label(state: tauri::State<'_, TauriState>, session_id: i64, label: String) -> Result<(), String> {
    // Update in-memory state if this is the active session
    {
        let mut active = state.active_session.lock().await;
        if let Some(ref mut session) = *active {
            if session.id == session_id {
                session.label = Some(label.clone());
            }
        }
    }
    let db = state.db.lock().await;
    db.update_session_label(session_id, &label).map_err(|e| e.to_string())
}

/// Update a session's category
#[tauri::command]
async fn update_session_category(state: tauri::State<'_, TauriState>, session_id: i64, category: Option<String>) -> Result<(), String> {
    // Update in-memory state if this is the active session
    {
        let mut active = state.active_session.lock().await;
        if let Some(ref mut session) = *active {
            if session.id == session_id {
                session.category = category.clone();
            }
        }
    }
    let db = state.db.lock().await;
    db.update_session_category(session_id, category.as_deref()).map_err(|e| e.to_string())
}

/// Get session categories from config
#[tauri::command]
async fn get_session_categories(state: tauri::State<'_, TauriState>) -> Result<Vec<SessionCategory>, String> {
    let config = state.config.lock().await;
    Ok(config.advanced.session_categories.clone())
}

/// Add a new session category
#[tauri::command]
async fn add_session_category(state: tauri::State<'_, TauriState>, category: SessionCategory) -> Result<Vec<SessionCategory>, String> {
    let mut config = state.config.lock().await;
    if !config.advanced.session_categories.iter().any(|c| c.name == category.name) {
        config.advanced.session_categories.push(category);
        config.save().map_err(|e| e.to_string())?;
    }
    Ok(config.advanced.session_categories.clone())
}

/// Remove a session category by name
#[tauri::command]
async fn remove_session_category(state: tauri::State<'_, TauriState>, name: String) -> Result<Vec<SessionCategory>, String> {
    let mut config = state.config.lock().await;
    config.advanced.session_categories.retain(|c| c.name != name);
    config.save().map_err(|e| e.to_string())?;
    Ok(config.advanced.session_categories.clone())
}

/// Delete a session
#[tauri::command]
async fn delete_session(state: tauri::State<'_, TauriState>, session_id: i64) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_session(session_id).map_err(|e| e.to_string())
}

/// Get sessions in a date range
#[tauri::command]
async fn get_sessions_in_range(state: tauri::State<'_, TauriState>, start: i64, end: i64) -> Result<Vec<Session>, String> {
    let db = state.db.lock().await;
    db.get_sessions_in_range(start, end).map_err(|e| e.to_string())
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

// ===== Elevation commands =====

/// Check if the app is running with elevated (admin) privileges
#[tauri::command]
fn is_elevated() -> bool {
    elevation::is_elevated()
}

/// Relaunch the app with elevated privileges, then exit
#[tauri::command]
fn relaunch_elevated() -> bool {
    if elevation::relaunch_elevated() {
        std::process::exit(0);
    }
    false
}

// ===== Layout Profile Commands =====

/// Get all saved layout profiles
#[tauri::command]
async fn get_layout_profiles(state: tauri::State<'_, TauriState>) -> Result<Vec<LayoutProfile>, String> {
    let config = state.config.lock().await;
    Ok(config.dashboard.profiles.clone())
}

/// Save current layout as a named profile (upsert)
#[tauri::command]
async fn save_layout_profile(state: tauri::State<'_, TauriState>, name: String) -> Result<Vec<LayoutProfile>, String> {
    let mut config = state.config.lock().await;
    let profile = LayoutProfile {
        name: name.clone(),
        widgets: config.dashboard.widgets.clone(),
        global_display: config.dashboard.global_display.clone(),
    };

    // Upsert: replace existing profile with same name, or add new
    if let Some(existing) = config.dashboard.profiles.iter_mut().find(|p| p.name == name) {
        *existing = profile;
    } else {
        config.dashboard.profiles.push(profile);
    }

    config.dashboard.active_profile = name;
    config.save().map_err(|e| e.to_string())?;
    Ok(config.dashboard.profiles.clone())
}

/// Load a named profile, applying its widgets to the active config
#[tauri::command]
async fn load_layout_profile(state: tauri::State<'_, TauriState>, name: String) -> Result<crate::core::DashboardConfig, String> {
    let mut config = state.config.lock().await;
    let profile = config.dashboard.profiles.iter().find(|p| p.name == name).cloned();

    match profile {
        Some(p) => {
            config.dashboard.widgets = p.widgets;
            config.dashboard.global_display = p.global_display;
            config.dashboard.active_profile = name;
            config.save().map_err(|e| e.to_string())?;
            Ok(config.dashboard.clone())
        }
        None => Err(format!("Profile '{}' not found", name)),
    }
}

/// Delete a named profile
#[tauri::command]
async fn delete_layout_profile(state: tauri::State<'_, TauriState>, name: String) -> Result<Vec<LayoutProfile>, String> {
    let mut config = state.config.lock().await;
    config.dashboard.profiles.retain(|p| p.name != name);

    // Clear active profile if it was the deleted one
    if config.dashboard.active_profile == name {
        config.dashboard.active_profile = String::new();
    }

    config.save().map_err(|e| e.to_string())?;
    Ok(config.dashboard.profiles.clone())
}

// ===== Update Check =====

#[derive(serde::Serialize, Clone)]
struct UpdateCheckResult {
    update_available: bool,
    current_version: String,
    latest_version: String,
    release_url: String,
    release_notes: String,
}

/// Compare two semver strings, returns true if `latest` is newer than `current`
fn version_is_newer(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> (u64, u64, u64) {
        let v = v.trim_start_matches('v');
        let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    parse(latest) > parse(current)
}

#[tauri::command]
async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    app.opener().open_url(&url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let client = reqwest::Client::builder()
        .user_agent(format!("PowerCost-Tracker/{}", current_version))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get("https://api.github.com/repos/HLE-C0DE/PowerCost-Tracker/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let tag = json["tag_name"].as_str().unwrap_or("").to_string();
    let release_url = json["html_url"].as_str().unwrap_or("").to_string();
    let release_notes = json["body"].as_str().unwrap_or("").to_string();

    let update_available = version_is_newer(&current_version, &tag);

    Ok(UpdateCheckResult {
        update_available,
        current_version,
        latest_version: tag.trim_start_matches('v').to_string(),
        release_url,
        release_notes,
    })
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

    // Auto-relaunch elevated if configured (Windows only)
    if config.general.run_as_admin && !elevation::is_elevated() {
        log::info!("Run as admin is enabled but not elevated, requesting elevation...");
        if elevation::relaunch_elevated() {
            log::info!("Elevated process launched, exiting current instance");
            std::process::exit(0);
        }
        log::warn!("UAC was denied or elevation failed, continuing without elevation");
    }

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
        .plugin(tauri_plugin_opener::init())
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
            kill_process,
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
            // Layout profile commands
            get_layout_profiles,
            save_layout_profile,
            load_layout_profile,
            delete_layout_profile,
            // Autostart command
            set_autostart,
            // Tiered monitoring API (fast/slow refresh)
            get_critical_metrics,
            get_detailed_metrics,
            // Session category & label commands
            update_session_label,
            update_session_category,
            get_session_categories,
            add_session_category,
            remove_session_category,
            get_sessions_in_range,
            delete_session,
            // Elevation commands
            is_elevated,
            relaunch_elevated,
            // Update check
            get_app_version,
            check_for_updates,
            open_url,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Check if start_minimized is enabled and hide the main window
            let state: tauri::State<'_, TauriState> = app.state();
            let (start_minimized, remember_pos, win_x, win_y, win_w, win_h) = {
                // Use block_on since we're in sync context
                let config = tauri::async_runtime::block_on(state.config.lock());
                (
                    config.general.start_minimized,
                    config.general.remember_window_position,
                    config.general.window_x,
                    config.general.window_y,
                    config.general.window_width,
                    config.general.window_height,
                )
            };

            // Restore saved window position and size
            if remember_pos {
                if let Some(main_window) = app.get_webview_window("main") {
                    if let (Some(w), Some(h)) = (win_w, win_h) {
                        let _ = main_window.set_size(LogicalSize::new(w, h));
                    }
                    if let (Some(x), Some(y)) = (win_x, win_y) {
                        // Verify position is within available screen area
                        let is_visible = main_window.available_monitors().ok()
                            .map(|monitors| {
                                monitors.iter().any(|m| {
                                    let pos = m.position();
                                    let size = m.size();
                                    let scale = m.scale_factor();
                                    let mx = pos.x as f64 / scale;
                                    let my = pos.y as f64 / scale;
                                    let mw = size.width as f64 / scale;
                                    let mh = size.height as f64 / scale;
                                    // Check if at least part of the window is visible on this monitor
                                    x < mx + mw && x + win_w.unwrap_or(900.0) > mx &&
                                    y < my + mh && y + win_h.unwrap_or(600.0) > my
                                })
                            })
                            .unwrap_or(false);

                        if is_visible {
                            let _ = main_window.set_position(LogicalPosition::new(x, y));
                            log::info!("Restored window position: ({}, {})", x, y);
                        } else {
                            log::info!("Saved window position ({}, {}) is off-screen, using default", x, y);
                        }
                    }
                }
            }

            if !start_minimized {
                if let Some(main_window) = app.get_webview_window("main") {
                    let _ = main_window.show();
                    log::info!("Main window shown on startup");
                }
            } else {
                log::info!("Started minimized - main window stays hidden");
            }

            // Create tray menu with translated labels
            let i18n = tauri::async_runtime::block_on(state.i18n.lock());
            let quit_item = MenuItem::with_id(app, "quit", i18n.get("tray.exit"), true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", i18n.get("tray.show"), true, None::<&str>)?;
            let restart_item = MenuItem::with_id(app, "restart", i18n.get("tray.restart"), true, None::<&str>)?;
            drop(i18n);
            let menu = Menu::with_items(app, &[&show_item, &restart_item, &quit_item])?;

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
                        "restart" => {
                            log::info!("Restart requested from tray menu");
                            tauri::process::restart(&app.env());
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

            // Check for updates at startup if enabled
            {
                let check_updates = {
                    let config = tauri::async_runtime::block_on(state.config.lock());
                    config.general.check_updates_at_startup
                };
                if check_updates {
                    let app_handle_updates = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Delay to avoid slowing down startup
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        match check_for_updates().await {
                            Ok(result) if result.update_available => {
                                let _ = app_handle_updates.emit("update-available", result);
                                log::info!("Update available, notified frontend");
                            }
                            Ok(_) => log::info!("App is up to date"),
                            Err(e) => log::warn!("Startup update check failed: {}", e),
                        }
                    });
                }
            }

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
            if window.label() != "main" {
                return;
            }

            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // Save window geometry before hiding
                    let app = window.app_handle().clone();
                    let win = window.clone();
                    tauri::async_runtime::spawn(async move {
                        save_window_geometry(&app, &win).await;
                    });

                    // Hide window instead of closing
                    let _ = window.hide();
                    api.prevent_close();
                    log::info!("Main window hidden to tray");
                }
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                    // Save geometry on move/resize (in case of crash before close)
                    let app = window.app_handle().clone();
                    let win = window.clone();
                    tauri::async_runtime::spawn(async move {
                        save_window_geometry(&app, &win).await;
                    });
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Save window position and size to config
async fn save_window_geometry(app: &tauri::AppHandle, window: &tauri::Window) {
    let state: tauri::State<'_, TauriState> = app.state();
    let mut config = state.config.lock().await;

    if !config.general.remember_window_position {
        return;
    }

    let scale = window.scale_factor().unwrap_or(1.0);

    if let Ok(pos) = window.outer_position() {
        config.general.window_x = Some(pos.x as f64 / scale);
        config.general.window_y = Some(pos.y as f64 / scale);
    }

    if let Ok(size) = window.outer_size() {
        config.general.window_width = Some(size.width as f64 / scale);
        config.general.window_height = Some(size.height as f64 / scale);
    }

    if let Err(e) = config.save() {
        log::warn!("Failed to save window geometry: {}", e);
    }
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

        // Use session average power for estimates instead of instantaneous
        let avg_power_watts = if session_duration_secs > 0 {
            cumulative_wh / (session_duration_secs as f64 / 3600.0)
        } else {
            power_watts // fallback to instantaneous at start
        };

        // Calculate cost estimates
        let (hourly_cost, daily_cost, monthly_cost) = {
            let pricing = state.pricing.lock().await;
            (
                pricing.calculate_hourly_cost(avg_power_watts),
                pricing.calculate_daily_cost(avg_power_watts),
                pricing.calculate_monthly_cost(avg_power_watts),
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
                    label: session.label.clone(),
                    category: session.category.clone(),
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
            avg_power_watts,
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
                    let rate = {
                        let pricing = state.pricing.lock().await;
                        pricing.get_current_rate()
                    };
                    let _ = db.update_today_stats(Some(&pricing_mode), Some(rate));

                    // Track app usage time (accumulate 60 seconds per minute)
                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    let _ = db.add_usage_seconds(&today, 60);
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

        // Determine if we should collect extended metrics (per-core freq, fans)
        // based on whether CPU or GPU load exceeds the configured threshold
        let should_collect_extended = {
            let critical = state.critical_metrics_cache.lock().await;
            let config = state.config.lock().await;
            let threshold = config.advanced.extended_metrics_threshold;
            if let Some(ref cm) = *critical {
                cm.cpu_usage_percent >= threshold
                    || cm.gpu_usage_percent.map_or(false, |g| g >= threshold)
            } else {
                false
            }
        };

        // Collect detailed metrics in a blocking task to avoid blocking async runtime
        // This is where slow GPU commands (nvidia-smi) and process enumeration happen
        let detailed_metrics = {
            let monitor = state.monitor.lock().await;
            // Use spawn_blocking for the slow operations
            let limit_clone = limit;
            let pinned_clone = pinned.clone();

            // We need to clone what we need since spawn_blocking requires 'static
            match monitor.collect_detailed_metrics(limit_clone, &pinned_clone, should_collect_extended) {
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
                        extended_collected: false,
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
