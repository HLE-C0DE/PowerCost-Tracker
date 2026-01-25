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
    historyData: [], // Historical data for the history view
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

        // Debug logging for development
        if (typeof data.power_watts !== 'number' || isNaN(data.power_watts)) {
            console.warn('Invalid power_watts received:', data.power_watts, 'Full data:', data);
        }

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
            state.historyData = [];
        } else {
            document.getElementById('no-history-data').classList.add('hidden');
            document.querySelector('.history-chart-container').classList.remove('hidden');

            // Store data for chart
            state.historyData = stats;

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

            // Draw the history chart
            drawHistoryChart();
        }

    } catch (error) {
        console.error('History load error:', error);
    }
}

// ===== History Chart =====
function drawHistoryChart() {
    const canvas = document.getElementById('history-chart');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const container = canvas.parentElement;
    const rect = container.getBoundingClientRect();

    // Set canvas size
    canvas.width = rect.width - 32; // Account for padding
    canvas.height = rect.height - 32;

    const data = state.historyData;
    if (data.length === 0) return;

    const padding = { top: 20, right: 20, bottom: 40, left: 60 };
    const width = canvas.width - padding.left - padding.right;
    const height = canvas.height - padding.top - padding.bottom;

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Find max values
    const maxWh = Math.max(...data.map(d => d.total_wh)) * 1.1 || 100;

    // Draw grid lines
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.lineWidth = 1;

    // Horizontal grid lines (5 lines)
    for (let i = 0; i <= 5; i++) {
        const y = padding.top + (height / 5) * i;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(padding.left + width, y);
        ctx.stroke();

        // Y-axis labels
        const value = maxWh - (maxWh / 5) * i;
        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.font = '11px system-ui';
        ctx.textAlign = 'right';
        ctx.fillText(formatNumber(value / 1000, 1) + ' kWh', padding.left - 8, y + 4);
    }

    // Bar width
    const barWidth = Math.min(40, (width / data.length) * 0.7);
    const barGap = (width - barWidth * data.length) / (data.length + 1);

    // Draw bars
    data.forEach((day, i) => {
        const x = padding.left + barGap + i * (barWidth + barGap);
        const barHeight = (day.total_wh / maxWh) * height;
        const y = padding.top + height - barHeight;

        // Bar gradient
        const gradient = ctx.createLinearGradient(x, y, x, padding.top + height);
        gradient.addColorStop(0, '#6366f1');
        gradient.addColorStop(1, 'rgba(99, 102, 241, 0.3)');

        ctx.fillStyle = gradient;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, barHeight, [4, 4, 0, 0]);
        ctx.fill();

        // X-axis labels (date)
        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.font = '10px system-ui';
        ctx.textAlign = 'center';
        const dateLabel = day.date ? day.date.slice(5) : `Day ${i + 1}`; // MM-DD format
        ctx.fillText(dateLabel, x + barWidth / 2, padding.top + height + 20);
    });

    // Draw average line
    const avgWh = data.reduce((sum, d) => sum + d.total_wh, 0) / data.length;
    const avgY = padding.top + height - (avgWh / maxWh) * height;

    ctx.strokeStyle = '#22c55e';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(padding.left, avgY);
    ctx.lineTo(padding.left + width, avgY);
    ctx.stroke();
    ctx.setLineDash([]);

    // Average label
    ctx.fillStyle = '#22c55e';
    ctx.font = '11px system-ui';
    ctx.textAlign = 'left';
    ctx.fillText(`Avg: ${formatNumber(avgWh / 1000, 2)} kWh`, padding.left + 5, avgY - 5);
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

    // Widget toggle button
    document.getElementById('toggle-widget-btn').addEventListener('click', async () => {
        try {
            const isOpen = await invoke('toggle_widget');
            const btn = document.getElementById('toggle-widget-btn');
            btn.textContent = isOpen ? t('settings.widget.close') || 'Close Widget' : t('settings.widget.open') || 'Open Widget';
        } catch (error) {
            console.error('Widget toggle error:', error);
        }
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

    // Seasonal
    if (config.pricing.seasonal) {
        document.getElementById('setting-summer-rate').value = config.pricing.seasonal.summer_rate;
        document.getElementById('setting-winter-rate').value = config.pricing.seasonal.winter_rate;
    }

    // Tempo
    if (config.pricing.tempo) {
        document.getElementById('setting-tempo-blue-peak').value = config.pricing.tempo.blue_peak;
        document.getElementById('setting-tempo-blue-offpeak').value = config.pricing.tempo.blue_offpeak;
        document.getElementById('setting-tempo-white-peak').value = config.pricing.tempo.white_peak;
        document.getElementById('setting-tempo-white-offpeak').value = config.pricing.tempo.white_offpeak;
        document.getElementById('setting-tempo-red-peak').value = config.pricing.tempo.red_peak;
        document.getElementById('setting-tempo-red-offpeak').value = config.pricing.tempo.red_offpeak;
    }

    // Widget
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

    const modeMap = {
        'simple': 'pricing-simple',
        'peak_offpeak': 'pricing-peak-offpeak',
        'seasonal': 'pricing-seasonal',
        'tempo': 'pricing-tempo'
    };

    if (modeMap[mode]) {
        document.getElementById(modeMap[mode]).classList.remove('hidden');
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
                seasonal: {
                    summer_rate: parseFloat(document.getElementById('setting-summer-rate').value),
                    winter_rate: parseFloat(document.getElementById('setting-winter-rate').value),
                    winter_months: [11, 12, 1, 2, 3],
                },
                tempo: {
                    blue_peak: parseFloat(document.getElementById('setting-tempo-blue-peak').value),
                    blue_offpeak: parseFloat(document.getElementById('setting-tempo-blue-offpeak').value),
                    white_peak: parseFloat(document.getElementById('setting-tempo-white-peak').value),
                    white_offpeak: parseFloat(document.getElementById('setting-tempo-white-offpeak').value),
                    red_peak: parseFloat(document.getElementById('setting-tempo-red-peak').value),
                    red_offpeak: parseFloat(document.getElementById('setting-tempo-red-offpeak').value),
                },
            },
            widget: {
                enabled: state.config?.widget?.enabled ?? true,
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

        // Show success toast
        showToast(t('settings.saved') || 'Settings saved successfully', 'success');

    } catch (error) {
        console.error('Save settings error:', error);
        showToast(t('error.save_failed') || 'Failed to save settings', 'error');
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

// ===== Toast Notifications =====
function showToast(message, type = 'info') {
    const container = document.getElementById('toast-container');

    const toast = document.createElement('div');
    toast.className = `toast ${type}`;

    const icons = {
        success: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>',
        error: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>',
        info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>'
    };

    toast.innerHTML = `
        ${icons[type] || icons.info}
        <span class="toast-message">${message}</span>
    `;

    container.appendChild(toast);

    // Remove after animation
    setTimeout(() => {
        toast.remove();
    }, 3000);
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
    if (state.historyData.length > 0) {
        drawHistoryChart();
    }
});
