//! English translations

use std::collections::HashMap;

pub fn get_translations() -> HashMap<String, String> {
    let mut t = HashMap::new();

    // App general
    t.insert("app.title".into(), "PowerCost Tracker".into());
    t.insert("app.version".into(), "Version".into());

    // Navigation
    t.insert("nav.dashboard".into(), "Dashboard".into());
    t.insert("nav.history".into(), "History".into());
    t.insert("nav.settings".into(), "Settings".into());
    t.insert("nav.about".into(), "About".into());

    // Dashboard
    t.insert("dashboard.current_power".into(), "Current Power".into());
    t.insert("dashboard.session_energy".into(), "Session Energy".into());
    t.insert("dashboard.session_cost".into(), "Session Cost".into());
    t.insert("dashboard.hourly_estimate".into(), "Hourly Estimate".into());
    t.insert("dashboard.daily_estimate".into(), "Daily Estimate".into());
    t.insert("dashboard.monthly_estimate".into(), "Monthly Estimate".into());
    t.insert("dashboard.session_duration".into(), "Session Duration".into());
    t.insert("dashboard.power_source".into(), "Power Source".into());
    t.insert("dashboard.estimated".into(), "Estimated".into());
    t.insert("dashboard.measured".into(), "Measured".into());

    // Units
    t.insert("unit.watts".into(), "W".into());
    t.insert("unit.kilowatts".into(), "kW".into());
    t.insert("unit.watt_hours".into(), "Wh".into());
    t.insert("unit.kilowatt_hours".into(), "kWh".into());
    t.insert("unit.per_hour".into(), "/hour".into());
    t.insert("unit.per_day".into(), "/day".into());
    t.insert("unit.per_month".into(), "/month".into());

    // Settings - General
    t.insert("settings.general".into(), "General".into());
    t.insert("settings.language".into(), "Language".into());
    t.insert("settings.language.auto".into(), "Auto-detect".into());
    t.insert("settings.theme".into(), "Theme".into());
    t.insert("settings.theme.dark".into(), "Dark".into());
    t.insert("settings.theme.light".into(), "Light".into());
    t.insert("settings.theme.system".into(), "System".into());
    t.insert("settings.refresh_rate".into(), "Refresh Rate".into());
    t.insert("settings.eco_mode".into(), "Eco Mode".into());
    t.insert("settings.eco_mode.description".into(), "Reduce refresh rate when minimized".into());
    t.insert("settings.start_minimized".into(), "Start Minimized".into());
    t.insert("settings.start_with_system".into(), "Start with System".into());

    // Settings - Pricing
    t.insert("settings.pricing".into(), "Pricing".into());
    t.insert("settings.pricing.mode".into(), "Pricing Mode".into());
    t.insert("settings.pricing.mode.simple".into(), "Simple (flat rate)".into());
    t.insert("settings.pricing.mode.peak_offpeak".into(), "Peak/Off-peak".into());
    t.insert("settings.pricing.mode.seasonal".into(), "Seasonal".into());
    t.insert("settings.pricing.mode.tempo".into(), "Tempo (EDF-style)".into());
    t.insert("settings.pricing.currency".into(), "Currency".into());
    t.insert("settings.pricing.rate".into(), "Rate per kWh".into());
    t.insert("settings.pricing.peak_rate".into(), "Peak Rate".into());
    t.insert("settings.pricing.offpeak_rate".into(), "Off-peak Rate".into());
    t.insert("settings.pricing.offpeak_start".into(), "Off-peak Start".into());
    t.insert("settings.pricing.offpeak_end".into(), "Off-peak End".into());
    t.insert("settings.pricing.summer_rate".into(), "Summer Rate".into());
    t.insert("settings.pricing.winter_rate".into(), "Winter Rate".into());
    t.insert("settings.pricing.not_configured".into(), "Pricing not configured".into());
    t.insert("settings.pricing.configure_hint".into(), "Configure pricing to see cost estimates".into());

    // Settings - Widget
    t.insert("settings.widget".into(), "Widget".into());
    t.insert("settings.widget.enabled".into(), "Enable Widget".into());
    t.insert("settings.widget.show_cost".into(), "Show Cost".into());
    t.insert("settings.widget.show_power".into(), "Show Power Only".into());
    t.insert("settings.widget.position".into(), "Position".into());
    t.insert("settings.widget.position.top_left".into(), "Top Left".into());
    t.insert("settings.widget.position.top_right".into(), "Top Right".into());
    t.insert("settings.widget.position.bottom_left".into(), "Bottom Left".into());
    t.insert("settings.widget.position.bottom_right".into(), "Bottom Right".into());
    t.insert("settings.widget.opacity".into(), "Opacity".into());
    t.insert("settings.widget.open".into(), "Open Widget".into());
    t.insert("settings.widget.close".into(), "Close Widget".into());

    // Settings - Pricing Tempo
    t.insert("settings.pricing.tempo.blue".into(), "Blue Days".into());
    t.insert("settings.pricing.tempo.white".into(), "White Days".into());
    t.insert("settings.pricing.tempo.red".into(), "Red Days".into());
    t.insert("settings.pricing.tempo.peak".into(), "Peak".into());
    t.insert("settings.pricing.tempo.offpeak".into(), "Off-peak".into());
    t.insert("settings.pricing.winter_months".into(), "Winter Months".into());

    // Settings - Status
    t.insert("settings.saved".into(), "Settings saved successfully".into());

    // Settings - Advanced
    t.insert("settings.advanced".into(), "Advanced".into());
    t.insert("settings.advanced.baseline".into(), "Baseline Power".into());
    t.insert("settings.advanced.baseline.auto".into(), "Auto-detect".into());
    t.insert("settings.advanced.baseline.manual".into(), "Manual".into());
    t.insert("settings.advanced.baseline.description".into(), "Baseline power for surplus tracking".into());

    // History
    t.insert("history.title".into(), "Consumption History".into());
    t.insert("history.today".into(), "Today".into());
    t.insert("history.this_week".into(), "This Week".into());
    t.insert("history.this_month".into(), "This Month".into());
    t.insert("history.custom_range".into(), "Custom Range".into());
    t.insert("history.total_consumption".into(), "Total Consumption".into());
    t.insert("history.total_cost".into(), "Total Cost".into());
    t.insert("history.average_power".into(), "Average Power".into());
    t.insert("history.peak_power".into(), "Peak Power".into());
    t.insert("history.no_data".into(), "No data available for this period".into());

    // About
    t.insert("about.title".into(), "About PowerCost Tracker".into());
    t.insert("about.description".into(), "A lightweight desktop application for monitoring PC power consumption and calculating electricity costs in real-time.".into());
    t.insert("about.license".into(), "License: MIT".into());
    t.insert("about.source".into(), "Source Code".into());

    // Errors and warnings
    t.insert("error.hardware_not_detected".into(), "Power monitoring hardware not detected".into());
    t.insert("error.using_estimation".into(), "Using power estimation mode".into());
    t.insert("error.permission_denied".into(), "Permission denied".into());
    t.insert("error.save_failed".into(), "Failed to save settings".into());
    t.insert("warning.estimated_values".into(), "Power values are estimated (no direct sensor detected)".into());

    // Actions
    t.insert("action.save".into(), "Save".into());
    t.insert("action.cancel".into(), "Cancel".into());
    t.insert("action.reset".into(), "Reset".into());
    t.insert("action.close".into(), "Close".into());
    t.insert("action.minimize".into(), "Minimize".into());
    t.insert("action.quit".into(), "Quit".into());

    // Time
    t.insert("time.hours".into(), "hours".into());
    t.insert("time.minutes".into(), "minutes".into());
    t.insert("time.seconds".into(), "seconds".into());

    // Dashboard - Display modes
    t.insert("dashboard.mode.normal".into(), "Normal".into());
    t.insert("dashboard.mode.minimal".into(), "Minimal".into());


    // Dashboard - Edit mode
    t.insert("dashboard.edit_mode".into(), "Edit Mode".into());
    t.insert("dashboard.default_layout".into(), "Default Layout".into());
    t.insert("dashboard.toggle_widgets".into(), "Widgets Parameters".into());
    t.insert("dashboard.done".into(), "Done".into());
    t.insert("dashboard.toggle_visibility".into(), "Toggle Widget Visibility".into());
    t.insert("dashboard.edit".into(), "Edit Dashboard".into());
    t.insert("dashboard.edit_hint".into(), "Toggle widgets visibility and drag to reorder".into());
    t.insert("dashboard.reset_default".into(), "Reset to Default".into());
    t.insert("dashboard.saved".into(), "Dashboard saved".into());
    t.insert("dashboard.save_failed".into(), "Failed to save dashboard".into());
    t.insert("dashboard.reset_success".into(), "Dashboard reset to default".into());
    t.insert("dashboard.edit_activated".into(), "Edit mode activated".into());
    t.insert("dashboard.changes_saved".into(), "Changes saved".into());
    t.insert("dashboard.default_applied".into(), "Default layout applied".into());
    t.insert("dashboard.display_mode".into(), "Display mode".into());

    // Session tracking
    t.insert("session.no_active".into(), "No active session".into());
    t.insert("session.start".into(), "Start Session".into());
    t.insert("session.end".into(), "End Session".into());
    t.insert("session.started".into(), "Session started".into());
    t.insert("session.start_failed".into(), "Failed to start session".into());
    t.insert("session.ended".into(), "Session ended".into());
    t.insert("session.end_failed".into(), "Failed to end session".into());
    t.insert("session.surplus".into(), "surplus".into());

    // Process list
    t.insert("processes.all".into(), "All Processes".into());
    t.insert("processes.search_placeholder".into(), "Search processes...".into());
    t.insert("processes.header.name".into(), "Process".into());
    t.insert("processes.header.cpu".into(), "CPU %".into());
    t.insert("processes.header.gpu".into(), "GPU %".into());
    t.insert("processes.header.ram".into(), "RAM %".into());
    t.insert("processes.pinned".into(), "Pinned".into());
    t.insert("processes.unpinned".into(), "Unpinned".into());
    t.insert("processes.pin_failed".into(), "Failed to update pin".into());

    // Settings - Baseline detection
    t.insert("settings.baseline".into(), "Baseline Detection".into());
    t.insert("settings.baseline.auto".into(), "Auto-detect Baseline".into());
    t.insert("settings.baseline.manual".into(), "Manual Baseline (W)".into());
    t.insert("settings.baseline.detected".into(), "Detected Baseline".into());
    t.insert("settings.baseline.detect_now".into(), "Detect Now".into());
    t.insert("settings.baseline.detected_value".into(), "Baseline detected".into());
    t.insert("settings.baseline.not_enough_data".into(), "Not enough data to detect baseline".into());
    t.insert("settings.baseline.detect_failed".into(), "Failed to detect baseline".into());
    t.insert("settings.baseline.set_success".into(), "Baseline set to".into());
    t.insert("settings.baseline.set_failed".into(), "Failed to set baseline".into());
    t.insert("settings.process_limit".into(), "Process List Limit".into());
    t.insert("settings.refresh_rate_detailed".into(), "Refresh Rate (Detailed)".into());
    t.insert("settings.refresh_rate_critical".into(), "Refresh Rate (Critical)".into());

    // History - Daily breakdown
    t.insert("history.daily_breakdown".into(), "Daily Breakdown".into());
    t.insert("history.date".into(), "Date".into());
    t.insert("history.energy".into(), "Energy".into());
    t.insert("history.cost".into(), "Cost".into());
    t.insert("history.rate".into(), "Rate".into());
    t.insert("history.avg".into(), "Avg".into());
    t.insert("history.peak".into(), "Peak".into());

    // History - Tabs
    t.insert("history.tab.power".into(), "Power".into());
    t.insert("history.tab.sessions".into(), "Sessions".into());
    t.insert("history.no_sessions".into(), "No sessions recorded yet".into());

    // Tray menu
    t.insert("tray.show".into(), "Show".into());
    t.insert("tray.exit".into(), "Exit".into());

    // Widget titles and labels
    t.insert("widget.cpu".into(), "CPU".into());
    t.insert("widget.gpu".into(), "GPU".into());
    t.insert("widget.ram".into(), "RAM".into());
    t.insert("widget.surplus".into(), "Surplus".into());
    t.insert("widget.session_controls".into(), "Session".into());
    t.insert("widget.processes".into(), "Top Processes".into());
    t.insert("widget.loading".into(), "Loading...".into());
    t.insert("widget.no_gpu".into(), "No GPU detected".into());
    t.insert("widget.no_process_data".into(), "No process data available".into());
    t.insert("widget.temp".into(), "Temp".into());
    t.insert("widget.power".into(), "Power".into());
    t.insert("widget.usage".into(), "Usage".into());
    t.insert("widget.cost".into(), "Cost".into());
    t.insert("widget.baseline".into(), "Baseline".into());
    t.insert("widget.current".into(), "Current".into());
    t.insert("widget.set_baseline".into(), "Set Baseline".into());
    t.insert("widget.update_baseline".into(), "Update Baseline".into());
    t.insert("widget.start_session_to_track".into(), "Start a session to track surplus".into());
    t.insert("widget.session_active".into(), "Session Active".into());
    t.insert("widget.show_top".into(), "Show top".into());
    t.insert("widget.search_processes".into(), "Search processes".into());
    t.insert("widget.pin".into(), "Pin".into());
    t.insert("widget.unpin".into(), "Unpin".into());
    t.insert("widget.size.small".into(), "Small".into());
    t.insert("widget.size.medium".into(), "Medium".into());
    t.insert("widget.size.large".into(), "Large".into());
    t.insert("widget.no_processes_found".into(), "No processes found".into());
    t.insert("widget.hide".into(), "Hide widget".into());
    t.insert("widget.display.bar".into(), "Bar".into());
    t.insert("widget.display.text".into(), "Text".into());
    t.insert("widget.display.radial".into(), "Radial".into());
    t.insert("widget.display.chart".into(), "Chart".into());
    t.insert("dashboard.display_mode_title".into(), "Display Mode".into());
    t.insert("dashboard.edit_title".into(), "Edit Dashboard".into());

    // Short widget titles (for 1×1 widgets)
    t.insert("dashboard.hourly_estimate_short".into(), "Hourly".into());
    t.insert("dashboard.daily_estimate_short".into(), "Daily".into());
    t.insert("dashboard.monthly_estimate_short".into(), "Monthly".into());
    t.insert("dashboard.session_energy_short".into(), "Energy".into());
    t.insert("dashboard.session_cost_short".into(), "Cost".into());
    t.insert("dashboard.session_duration_short".into(), "Duration".into());
    t.insert("dashboard.current_power_short".into(), "Power".into());
    t.insert("widget.processes_short".into(), "Procs".into());
    t.insert("widget.session_controls_short".into(), "Session".into());
    t.insert("widget.surplus_short".into(), "Surplus".into());

    // Estimation widget toggle labels
    t.insert("widget.show_cost".into(), "Cost".into());
    t.insert("widget.show_energy".into(), "Energy".into());

    // Extended hardware metrics
    t.insert("widget.fan".into(), "Fan".into());
    t.insert("widget.clock".into(), "Clock".into());
    t.insert("widget.mem_clock".into(), "Mem Clock".into());
    t.insert("widget.swap".into(), "Swap".into());
    t.insert("widget.speed".into(), "Speed".into());

    t
}
