/**
 * PowerCost Tracker - Main JavaScript
 *
 * Lightweight frontend for the Tauri-based power monitoring application.
 * Uses vanilla JS for minimal footprint.
 */

// Tauri API
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ===== State Management =====
const state = {
    translations: {},
    config: null,
    powerHistory: [],
    maxHistoryPoints: 60, // 1 minute of data at 1s refresh
    currencySymbol: '\u20AC',
};

// ===== Initialization =====
document.addEventListener('DOMContentLoaded', async () => {
    try {
        // Load translations
        await loadTranslations();

        // Load config
        state.config = await invoke('get_config');
        applyConfig(state.config);

        // Setup navigation
        setupNavigation();

        // Setup settings handlers
        setupSettings();

        // Start data updates
        startDashboardUpdates();

        // Listen for power updates from backend
        await listen('power-update', (event) => {
            updatePowerDisplay(event.payload);
        });

    } catch (error) {
        console.error('Initialization error:', error);
    }
});

// ===== Translations =====
async function loadTranslations() {
    try {
        state.translations = await invoke('get_translations');
        applyTranslations();
    } catch (error) {
        console.error('Failed to load translations:', error);
    }
}

function applyTranslations() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (state.translations[key]) {
            el.textContent = state.translations[key];
        }
    });
}

function t(key) {
    return state.translations[key] || key;
}

// ===== Navigation =====
function setupNavigation() {
    const navLinks = document.querySelectorAll('.nav-link');
    const views = document.querySelectorAll('.view');

    navLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();

            const targetView = link.getAttribute('data-view');

            // Update active states
            navLinks.forEach(l => l.classList.remove('active'));
            link.classList.add('active');

            views.forEach(v => v.classList.remove('active'));
            document.getElementById(targetView).classList.add('active');

            // Load view-specific data
            if (targetView === 'history') {
                loadHistoryData('today');
            }
        });
    });
}

// ===== Dashboard Updates =====
async function startDashboardUpdates() {
    updateDashboard();
    // Refresh every second (will be adjusted by backend based on config)
    setInterval(updateDashboard, 1000);
}

async function updateDashboard() {
    try {
        const data = await invoke('get_dashboard_data');

        // Update current power
        document.getElementById('current-power').textContent =
            formatNumber(data.power_watts, 1);

        // Update session energy
        const energyWh = data.cumulative_wh;
        if (energyWh >= 1000) {
            document.getElementById('session-energy').textContent =
                formatNumber(energyWh / 1000, 2);
            document.querySelector('#session-energy + .unit').textContent = 'kWh';
        } else {
            document.getElementById('session-energy').textContent =
                formatNumber(energyWh, 1);
        }

        // Update costs
        document.getElementById('session-cost').textContent =
            formatNumber(data.current_cost, 4);
        document.getElementById('hourly-cost').textContent =
            formatNumber(data.hourly_cost_estimate, 4);
        document.getElementById('daily-cost').textContent =
            formatNumber(data.daily_cost_estimate, 2);
        document.getElementById('monthly-cost').textContent =
            formatNumber(data.monthly_cost_estimate, 2);

        // Update currency symbols
        document.getElementById('currency-symbol').textContent = state.currencySymbol;
        document.getElementById('currency-hourly').textContent = state.currencySymbol;
        document.getElementById('currency-daily').textContent = state.currencySymbol;
        document.getElementById('currency-monthly').textContent = state.currencySymbol;

        // Update session duration
        document.getElementById('session-duration').textContent =
            formatDuration(data.session_duration_secs);

        // Update power source
        document.getElementById('power-source').textContent = data.source;

        // Update estimation warning
        const warningBanner = document.getElementById('estimation-warning');
        const statusDot = document.querySelector('.status-dot');

        if (data.is_estimated) {
            warningBanner.classList.remove('hidden');
            statusDot.classList.add('estimated');
        } else {
            warningBanner.classList.add('hidden');
            statusDot.classList.remove('estimated');
        }

        // Update power history for graph
        updatePowerHistory(data.power_watts);

    } catch (error) {
        console.error('Dashboard update error:', error);
    }
}

function updatePowerDisplay(powerWatts) {
    document.getElementById('current-power').textContent =
        formatNumber(powerWatts, 1);
    updatePowerHistory(powerWatts);
}

function updatePowerHistory(power) {
    state.powerHistory.push({
        time: Date.now(),
        power: power
    });

    // Keep only last N points
    if (state.powerHistory.length > state.maxHistoryPoints) {
        state.powerHistory.shift();
    }

    drawPowerGraph();
}

// ===== Power Graph =====
function drawPowerGraph() {
    const canvas = document.getElementById('power-chart');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const rect = canvas.parentElement.getBoundingClientRect();

    // Set canvas size
    canvas.width = rect.width;
    canvas.height = rect.height;

    const data = state.powerHistory;
    if (data.length < 2) return;

    const padding = 10;
    const width = canvas.width - padding * 2;
    const height = canvas.height - padding * 2;

    // Find min/max
    const powers = data.map(d => d.power);
    const maxPower = Math.max(...powers) * 1.1 || 100;
    const minPower = Math.min(...powers) * 0.9 || 0;

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw gradient background
    const gradient = ctx.createLinearGradient(0, padding, 0, height + padding);
    gradient.addColorStop(0, 'rgba(99, 102, 241, 0.3)');
    gradient.addColorStop(1, 'rgba(99, 102, 241, 0)');

    // Draw filled area
    ctx.beginPath();
    ctx.moveTo(padding, height + padding);

    data.forEach((point, i) => {
        const x = padding + (i / (data.length - 1)) * width;
        const y = padding + height - ((point.power - minPower) / (maxPower - minPower)) * height;
        ctx.lineTo(x, y);
    });

    ctx.lineTo(padding + width, height + padding);
    ctx.closePath();
    ctx.fillStyle = gradient;
    ctx.fill();

    // Draw line
    ctx.beginPath();
    data.forEach((point, i) => {
        const x = padding + (i / (data.length - 1)) * width;
        const y = padding + height - ((point.power - minPower) / (maxPower - minPower)) * height;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    });

    ctx.strokeStyle = '#6366f1';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw current value dot
    if (data.length > 0) {
        const lastPoint = data[data.length - 1];
        const x = padding + width;
        const y = padding + height - ((lastPoint.power - minPower) / (maxPower - minPower)) * height;

        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fillStyle = '#6366f1';
        ctx.fill();
    }
}

// ===== History =====
async function loadHistoryData(range) {
    try {
        const now = new Date();
        let startDate, endDate;

        switch (range) {
            case 'today':
                startDate = formatDate(now);
                endDate = startDate;
                break;
            case 'week':
                const weekAgo = new Date(now);
                weekAgo.setDate(weekAgo.getDate() - 7);
                startDate = formatDate(weekAgo);
                endDate = formatDate(now);
                break;
            case 'month':
                const monthAgo = new Date(now);
                monthAgo.setMonth(monthAgo.getMonth() - 1);
                startDate = formatDate(monthAgo);
                endDate = formatDate(now);
                break;
        }

        const stats = await invoke('get_history', { startDate, endDate });

        if (stats.length === 0) {
            document.getElementById('no-history-data').classList.remove('hidden');
            document.querySelector('.history-chart-container').classList.add('hidden');
        } else {
            document.getElementById('no-history-data').classList.add('hidden');
            document.querySelector('.history-chart-container').classList.remove('hidden');

            // Update stats
            const totalWh = stats.reduce((sum, s) => sum + s.total_wh, 0);
            const totalCost = stats.reduce((sum, s) => sum + (s.total_cost || 0), 0);
            const avgPower = stats.reduce((sum, s) => sum + s.avg_watts, 0) / stats.length;
            const maxPower = Math.max(...stats.map(s => s.max_watts));

            document.getElementById('history-total-wh').textContent =
                `${formatNumber(totalWh / 1000, 2)} kWh`;
            document.getElementById('history-total-cost').textContent =
                `${state.currencySymbol}${formatNumber(totalCost, 2)}`;
            document.getElementById('history-avg-power').textContent =
                `${formatNumber(avgPower, 0)} W`;
            document.getElementById('history-peak-power').textContent =
                `${formatNumber(maxPower, 0)} W`;
        }

    } catch (error) {
        console.error('History load error:', error);
    }
}

// Setup history range buttons
document.querySelectorAll('.range-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.range-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        loadHistoryData(btn.getAttribute('data-range'));
    });
});

// ===== Settings =====
function setupSettings() {
    // Pricing mode toggle
    const pricingModeSelect = document.getElementById('setting-pricing-mode');
    pricingModeSelect.addEventListener('change', () => {
        updatePricingModeUI(pricingModeSelect.value);
    });

    // Save button
    document.getElementById('save-settings').addEventListener('click', saveSettings);

    // Reset button
    document.getElementById('reset-settings').addEventListener('click', async () => {
        state.config = await invoke('get_config');
        applyConfig(state.config);
    });
}

function applyConfig(config) {
    // General
    document.getElementById('setting-language').value = config.general.language;
    document.getElementById('setting-theme').value = config.general.theme;
    document.getElementById('setting-refresh-rate').value = config.general.refresh_rate_ms;
    document.getElementById('setting-eco-mode').checked = config.general.eco_mode;

    // Pricing
    document.getElementById('setting-pricing-mode').value = config.pricing.mode;
    document.getElementById('setting-currency').value = config.pricing.currency;
    document.getElementById('setting-rate-kwh').value = config.pricing.simple.rate_per_kwh;
    document.getElementById('setting-peak-rate').value = config.pricing.peak_offpeak.peak_rate;
    document.getElementById('setting-offpeak-rate').value = config.pricing.peak_offpeak.offpeak_rate;
    document.getElementById('setting-offpeak-start').value = config.pricing.peak_offpeak.offpeak_start;
    document.getElementById('setting-offpeak-end').value = config.pricing.peak_offpeak.offpeak_end;

    // Widget
    document.getElementById('setting-widget-enabled').checked = config.widget.enabled;
    document.getElementById('setting-widget-show-cost').checked = config.widget.show_cost;
    document.getElementById('setting-widget-position').value = config.widget.position;

    // Apply theme
    document.documentElement.setAttribute('data-theme', config.general.theme);

    // Update currency symbol
    state.currencySymbol = config.pricing.currency_symbol;

    // Show correct pricing mode config
    updatePricingModeUI(config.pricing.mode);
}

function updatePricingModeUI(mode) {
    document.querySelectorAll('.pricing-mode-config').forEach(el => {
        el.classList.add('hidden');
    });

    if (mode === 'simple') {
        document.getElementById('pricing-simple').classList.remove('hidden');
    } else if (mode === 'peak_offpeak') {
        document.getElementById('pricing-peak-offpeak').classList.remove('hidden');
    }
}

async function saveSettings() {
    try {
        const config = {
            general: {
                language: document.getElementById('setting-language').value,
                theme: document.getElementById('setting-theme').value,
                refresh_rate_ms: parseInt(document.getElementById('setting-refresh-rate').value),
                eco_mode: document.getElementById('setting-eco-mode').checked,
                start_minimized: state.config?.general?.start_minimized || false,
                start_with_system: state.config?.general?.start_with_system || false,
            },
            pricing: {
                mode: document.getElementById('setting-pricing-mode').value,
                currency: document.getElementById('setting-currency').value,
                currency_symbol: getCurrencySymbol(document.getElementById('setting-currency').value),
                simple: {
                    rate_per_kwh: parseFloat(document.getElementById('setting-rate-kwh').value),
                },
                peak_offpeak: {
                    peak_rate: parseFloat(document.getElementById('setting-peak-rate').value),
                    offpeak_rate: parseFloat(document.getElementById('setting-offpeak-rate').value),
                    offpeak_start: document.getElementById('setting-offpeak-start').value,
                    offpeak_end: document.getElementById('setting-offpeak-end').value,
                },
                seasonal: state.config?.pricing?.seasonal || {
                    summer_rate: 0.20,
                    winter_rate: 0.25,
                    winter_months: [11, 12, 1, 2, 3],
                },
                tempo: state.config?.pricing?.tempo || {
                    blue_peak: 0.16,
                    blue_offpeak: 0.13,
                    white_peak: 0.19,
                    white_offpeak: 0.15,
                    red_peak: 0.76,
                    red_offpeak: 0.16,
                },
            },
            widget: {
                enabled: document.getElementById('setting-widget-enabled').checked,
                show_cost: document.getElementById('setting-widget-show-cost').checked,
                position: document.getElementById('setting-widget-position').value,
                opacity: state.config?.widget?.opacity || 0.9,
            },
            advanced: state.config?.advanced || {
                baseline_watts: 0,
                baseline_auto: true,
                active_profile: 'default',
            },
        };

        await invoke('set_config', { config });
        state.config = config;
        state.currencySymbol = config.pricing.currency_symbol;

        // Apply theme immediately
        document.documentElement.setAttribute('data-theme', config.general.theme);

        // Reload translations if language changed
        await loadTranslations();

    } catch (error) {
        console.error('Save settings error:', error);
    }
}

function getCurrencySymbol(currency) {
    const symbols = {
        'EUR': '\u20AC',
        'USD': '$',
        'GBP': '\u00A3',
        'CHF': 'CHF',
    };
    return symbols[currency] || currency;
}

// ===== Utility Functions =====
function formatNumber(num, decimals = 2) {
    if (num === null || num === undefined || isNaN(num)) return '--';
    return num.toFixed(decimals);
}

function formatDuration(seconds) {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    return `${hrs.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

function formatDate(date) {
    return date.toISOString().split('T')[0];
}

// ===== Window resize handler for canvas =====
window.addEventListener('resize', () => {
    drawPowerGraph();
});
