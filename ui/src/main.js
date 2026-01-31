/**
 * PowerCost Tracker - Main JavaScript
 *
 * Lightweight frontend for the Tauri-based power monitoring application.
 * Uses vanilla JS for minimal footprint.
 */

// Tauri API
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ===== Widget Registry =====
// Note: titleKey is used for translations - actual title is retrieved via getWidgetTitle()
const WIDGET_REGISTRY = {
    power: {
        id: 'power',
        titleKey: 'dashboard.current_power',
        shortTitleKey: 'dashboard.current_power_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>`,
        defaultSize: 'large',
        defaultColSpan: 2,
        defaultRowSpan: 2,  // Needs height for chart
        render: (data) => `
            <div class="widget-value power-value">${formatNumber(data.power_watts, 1)}<span class="unit">W</span></div>
            <div class="power-graph"><canvas id="power-chart"></canvas></div>
        `,
    },
    session_energy: {
        id: 'session_energy',
        titleKey: 'dashboard.session_energy',
        shortTitleKey: 'dashboard.session_energy_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>`,
        defaultSize: 'small',
        render: (data) => {
            const energyWh = data.cumulative_wh;
            const display = energyWh >= 1000 ? `${formatNumber(energyWh / 1000, 2)} kWh` : `${formatNumber(energyWh, 1)} Wh`;
            return `<div class="widget-value small">${display}</div>`;
        },
    },
    session_cost: {
        id: 'session_cost',
        titleKey: 'dashboard.session_cost',
        shortTitleKey: 'dashboard.session_cost_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6"/></svg>`,
        defaultSize: 'small',
        render: (data) => `<div class="widget-value small cost-value">${state.currencySymbol}${formatNumber(data.current_cost, 4)}</div>`,
    },
    hourly_estimate: {
        id: 'hourly_estimate',
        titleKey: 'dashboard.hourly_estimate',
        shortTitleKey: 'dashboard.hourly_estimate_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12"/></svg>`,
        defaultSize: 'small',
        render: (data, displayMode, widgetConfig) => renderEstimationWidget(data, widgetConfig, {
            costValue: data.hourly_cost_estimate,
            costDecimals: 4,
            unitKey: 'unit.per_hour',
            whMultiplier: 1,
        }),
    },
    daily_estimate: {
        id: 'daily_estimate',
        titleKey: 'dashboard.daily_estimate',
        shortTitleKey: 'dashboard.daily_estimate_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>`,
        defaultSize: 'small',
        render: (data, displayMode, widgetConfig) => renderEstimationWidget(data, widgetConfig, {
            costValue: data.daily_cost_estimate,
            costDecimals: 2,
            unitKey: 'unit.per_day',
            whMultiplier: 24,
        }),
    },
    monthly_estimate: {
        id: 'monthly_estimate',
        titleKey: 'dashboard.monthly_estimate',
        shortTitleKey: 'dashboard.monthly_estimate_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/></svg>`,
        defaultSize: 'small',
        render: (data, displayMode, widgetConfig) => renderEstimationWidget(data, widgetConfig, {
            costValue: data.monthly_cost_estimate,
            costDecimals: 2,
            unitKey: 'unit.per_month',
            whMultiplier: 720,
        }),
    },
    session_duration: {
        id: 'session_duration',
        titleKey: 'dashboard.session_duration',
        shortTitleKey: 'dashboard.session_duration_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>`,
        defaultSize: 'small',
        render: (data) => `<div class="widget-value small">${formatDuration(data.session_duration_secs)}</div>`,
    },
    cpu: {
        id: 'cpu',
        titleKey: 'widget.cpu',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2" ry="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/></svg>`,
        defaultSize: 'medium',
        defaultColSpan: 2,
        defaultRowSpan: 2,  // Needs height for chart/radial modes
        supportsDisplayModes: true,
        render: (data, displayMode = 'bar') => {
            const cpu = data.systemMetrics?.cpu;
            if (!cpu) return `<div class="widget-loading">${t('widget.loading')}</div>`;
            const hasTemp = cpu.temperature_celsius != null;
            const temp = hasTemp ? `${formatNumber(cpu.temperature_celsius, 0)}°C` : '';
            const globalDisplay = state.dashboardConfig?.global_display || 'normal';

            // Per-core clock range (only available when extended metrics collected)
            const perCoreFreq = cpu.per_core_frequency_mhz;
            const hasClockRange = perCoreFreq && perCoreFreq.length > 0;
            const clockRange = hasClockRange
                ? `${Math.min(...perCoreFreq)}-${Math.max(...perCoreFreq)} MHz`
                : '';

            // System fan info
            const fans = data.systemMetrics?.fans;
            const hasFans = fans && fans.fans && fans.fans.length > 0;
            const fanStr = hasFans
                ? fans.fans.map(f => {
                    const rpm = f.speed_rpm != null ? `${f.speed_rpm} RPM` : (f.speed_percent != null ? `${f.speed_percent}%` : '');
                    return f.name && rpm ? `${f.name}: ${rpm}` : rpm;
                }).filter(Boolean).join(' \u00b7 ')
                : '';

            // Per-core temperature info
            const perCoreTemp = cpu.per_core_temperature;
            const hasPerCoreTemp = perCoreTemp && perCoreTemp.length > 1;

            // Voltage info
            const voltages = data.systemMetrics?.voltages;
            const hasVoltages = voltages && voltages.length > 0;

            // Radial mode
            if (displayMode === 'radial') {
                const cpuBars = [];
                if (hasPerCoreTemp) {
                    const maxCoreTemp = Math.max(...perCoreTemp);
                    const tempColor = maxCoreTemp > 80 ? '#ef4444' : maxCoreTemp > 60 ? '#f59e0b' : '#fb923c';
                    cpuBars.push({ value: maxCoreTemp, max: 100, label: `<span style="font-weight:700;color:${tempColor}">${formatNumber(maxCoreTemp, 0)}°</span>`, color: tempColor, name: 'TMAX' });
                } else if (hasTemp && cpu.temperature_celsius != null) {
                    const tempColor = cpu.temperature_celsius > 80 ? '#ef4444' : cpu.temperature_celsius > 60 ? '#f59e0b' : '#fb923c';
                    cpuBars.push({ value: cpu.temperature_celsius, max: 100, label: `<span style="font-weight:700;color:${tempColor}">${formatNumber(cpu.temperature_celsius, 0)}°</span>`, color: tempColor, name: 'TEMP' });
                }
                if (hasClockRange && perCoreFreq.length > 0) {
                    const avgFreq = perCoreFreq.reduce((a, b) => a + b, 0) / perCoreFreq.length;
                    const maxFreq = Math.max(...perCoreFreq) * 1.2 || 5000;
                    cpuBars.push({ value: avgFreq, max: maxFreq, label: `<span style="font-weight:700;color:#6366f1">${formatNumber(avgFreq / 1000, 1)}G</span>`, color: '#6366f1', name: 'CLK' });
                }
                return `
                    <div class="radial-container">
                        ${renderRadialProgress(cpu.usage_percent, 'CPU', '#6366f1')}
                        ${renderChargeBars(cpuBars)}
                    </div>
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${cpu.name.slice(0, 30)}</div>
                `;
            }

            // Chart mode
            if (displayMode === 'chart') {
                return `
                    <div class="mini-chart-container">
                        <div class="mini-chart-header ">
                            <span class="metric-label">CPU</span>
                            <span class="metric-value">${formatNumber(cpu.usage_percent, 0)}%</span>
                        </div>
                        <canvas id="cpu-mini-chart" class="mini-chart"></canvas>
                    </div>
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${cpu.name.slice(0, 30)}</div>
                `;
            }

            // Text mode
            if (displayMode === 'text') {
                return `
                    <div class="widget-value">${formatNumber(cpu.usage_percent, 0)}<span class="unit">%</span></div>
                    ${hasTemp ? `<div class="metric-row ">
                        <span class="metric-label">${t('widget.temp')}</span>
                        <span class="metric-value">${temp}</span>
                    </div>` : ''}
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${cpu.name.slice(0, 30)}</div>
                `;
            }

            // Default bar mode
            return `
                <div class="metric-row">
                    <span class="metric-label ">${t('widget.usage')}</span>
                    <div class="progress-bar"><div class="progress-fill" style="width: ${cpu.usage_percent}%"></div></div>
                    <span class="metric-value">${formatNumber(cpu.usage_percent, 0)}%</span>
                </div>
                ${hasTemp ? `<div class="metric-row ">
                    <span class="metric-label">${t('widget.temp')}</span>
                    <span class="metric-value">${temp}</span>
                </div>` : ''}
                ${hasPerCoreTemp && globalDisplay === 'normal' ? `<div class="metric-row">
                    <span class="metric-label">Core temps</span>
                    <span class="metric-value">${Math.min(...perCoreTemp).toFixed(0)}-${Math.max(...perCoreTemp).toFixed(0)}°C</span>
                </div>` : ''}
                ${hasClockRange && globalDisplay === 'normal' ? `<div class="metric-row">
                    <span class="metric-label">${t('widget.clock')}</span>
                    <span class="metric-value">${clockRange}</span>
                </div>` : ''}
                ${hasFans && fanStr && globalDisplay === 'normal' ? `<div class="metric-row">
                    <span class="metric-label">${t('widget.fan')}</span>
                    <span class="metric-value">${fanStr}</span>
                </div>` : ''}
                ${hasVoltages && globalDisplay === 'normal' ? `<div class="metric-row">
                    <span class="metric-label">Voltages</span>
                    <span class="metric-value">${voltages.slice(0, 3).map(v => `${v.name}: ${v.value_volts.toFixed(2)}V`).join(', ')}</span>
                </div>` : ''}
                <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${cpu.name.slice(0, 30)}</div>
            `;
        },
    },
    gpu: {
        id: 'gpu',
        titleKey: 'widget.gpu',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="6" width="20" height="12" rx="2"/><line x1="6" y1="10" x2="6" y2="14"/><line x1="10" y1="10" x2="10" y2="14"/><line x1="14" y1="10" x2="14" y2="14"/><line x1="18" y1="10" x2="18" y2="14"/></svg>`,
        defaultSize: 'medium',
        defaultColSpan: 2,
        defaultRowSpan: 2,  // Needs height for chart/radial modes
        supportsDisplayModes: true,
        render: (data, displayMode = 'bar') => {
            const gpu = data.systemMetrics?.gpu;
            if (!gpu) return `<div class="widget-na">${t('widget.no_gpu')}</div>`;
            const usage = gpu.usage_percent != null ? gpu.usage_percent : 0;
            const usageStr = gpu.usage_percent != null ? `${formatNumber(gpu.usage_percent, 0)}%` : '--';
            const temp = gpu.temperature_celsius != null ? `${formatNumber(gpu.temperature_celsius, 0)}°C` : '--';
            const power = gpu.power_watts != null ? `${formatNumber(gpu.power_watts, 0)}W` : '--';
            const vram = gpu.vram_used_mb != null && gpu.vram_total_mb != null
                ? `${formatNumber(gpu.vram_used_mb / 1024, 1)}/${formatNumber(gpu.vram_total_mb / 1024, 1)} GB`
                : '--';
            const hasFan = gpu.fan_speed_percent != null;
            const hasMemClock = gpu.memory_clock_mhz != null;
            const globalDisplay = state.dashboardConfig?.global_display || 'normal';

            // Radial mode
            if (displayMode === 'radial') {
                const gpuBars = [];
                if (gpu.temperature_celsius != null) {
                    const tempColor = gpu.temperature_celsius > 80 ? '#ef4444' : gpu.temperature_celsius > 60 ? '#f59e0b' : '#fb923c';
                    gpuBars.push({ value: gpu.temperature_celsius, max: 100, label: `<span style="font-weight:700;color:${tempColor}">${formatNumber(gpu.temperature_celsius, 0)}°</span>`, color: tempColor, name: 'TEMP' });
                }
                if (gpu.power_watts != null) {
                    const maxPower = gpu.power_limit_watts || 300;
                    gpuBars.push({ value: gpu.power_watts, max: maxPower, label: `<span style="font-weight:700;color:#eab308">${formatNumber(gpu.power_watts, 0)}W</span>`, color: '#eab308', name: 'PWR' });
                }
                if (gpu.vram_used_mb != null && gpu.vram_total_mb != null) {
                    const vramPct = (gpu.vram_used_mb / gpu.vram_total_mb) * 100;
                    gpuBars.push({ value: vramPct, max: 100, label: `<span style="font-weight:700;color:#a855f7">${formatNumber(gpu.vram_used_mb / 1024, 1)}G</span>`, color: '#a855f7', name: 'VRAM' });
                }
                return `
                    <div class="radial-container">
                        ${renderRadialProgress(usage, 'GPU', '#22c55e')}
                        ${renderChargeBars(gpuBars)}
                    </div>
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${gpu.name.slice(0, 25)}</div>
                `;
            }

            // Chart mode
            if (displayMode === 'chart') {
                return `
                    <div class="mini-chart-container">
                        <div class="mini-chart-header ">
                            <span class="metric-label">GPU</span>
                            <span class="metric-value">${usageStr}</span>
                        </div>
                        <canvas id="gpu-mini-chart" class="mini-chart"></canvas>
                    </div>
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${gpu.name.slice(0, 25)}</div>
                `;
            }

            // Text mode
            if (displayMode === 'text') {
                return `
                    <div class="widget-value">${usageStr}</div>
                    <div class="metric-row ">
                        <span class="metric-label">${t('widget.temp')}</span>
                        <span class="metric-value">${temp}</span>
                    </div>
                    <div class="metric-row ">
                        <span class="metric-label">${t('widget.power')}</span>
                        <span class="metric-value">${power}</span>
                    </div>
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${gpu.name.slice(0, 25)}</div>
                `;
            }

            // Default bar mode
            return `
                <div class="metric-row">
                    <span class="metric-label ">${t('widget.usage')}</span>
                    <div class="progress-bar"><div class="progress-fill gpu-fill" style="width: ${usage}%"></div></div>
                    <span class="metric-value">${usageStr}</span>
                </div>
                <div class="metric-row ">
                    <span class="metric-label">${t('widget.temp')}</span>
                    <span class="metric-value">${temp}</span>
                </div>
                <div class="metric-row ">
                    <span class="metric-label">${t('widget.power')}</span>
                    <span class="metric-value">${power}</span>
                </div>
                <div class="metric-row ${globalDisplay !== 'normal' ? 'hidden' : ''}">
                    <span class="metric-label">VRAM</span>
                    <span class="metric-value">${vram}</span>
                </div>
                ${hasFan && globalDisplay === 'normal' ? `<div class="metric-row">
                    <span class="metric-label">${t('widget.fan')}</span>
                    <span class="metric-value">${gpu.fan_speed_percent}%</span>
                </div>` : ''}
                ${hasMemClock && globalDisplay === 'normal' ? `<div class="metric-row">
                    <span class="metric-label">${t('widget.mem_clock')}</span>
                    <span class="metric-value">${gpu.memory_clock_mhz} MHz</span>
                </div>` : ''}
                <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${gpu.name.slice(0, 25)}${(() => { const sl = gpu.source === 'nvml' ? 'NVML' : gpu.source === 'nvidia-smi' ? 'CLI' : gpu.source === 'amdgpu-sysfs' ? 'sysfs' : ''; return sl ? ` <span class="metric-source">(${sl})</span>` : ''; })()}</div>
            `;
        },
    },
    ram: {
        id: 'ram',
        titleKey: 'widget.ram',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="6" width="20" height="12" rx="2"/><line x1="6" y1="2" x2="6" y2="6"/><line x1="10" y1="2" x2="10" y2="6"/><line x1="14" y1="2" x2="14" y2="6"/><line x1="18" y1="2" x2="18" y2="6"/></svg>`,
        defaultSize: 'large',
        defaultColSpan: 2,
        defaultRowSpan: 2,
        supportsDisplayModes: true,
        render: (data, displayMode = 'bar') => {
            const mem = data.systemMetrics?.memory;
            if (!mem) return `<div class="widget-loading">${t('widget.loading')}</div>`;
            const usedGB = mem.used_bytes / (1024 * 1024 * 1024);
            const totalGB = mem.total_bytes / (1024 * 1024 * 1024);
            const globalDisplay = state.dashboardConfig?.global_display || 'normal';

            // Radial mode
            if (displayMode === 'radial') {
                const ramBars = [];
                ramBars.push({ value: usedGB, max: totalGB, label: `<span style="font-weight:700;color:#f59e0b">${formatNumber(usedGB, 1)}</span><span style="font-weight:400;color:#f59e0b">/${formatNumber(totalGB, 0)}G</span>`, color: '#f59e0b', name: 'USED' });
                if (mem.swap_total_bytes && mem.swap_total_bytes > 0) {
                    const swapUsedGB = mem.swap_used_bytes / (1024 * 1024 * 1024);
                    const swapTotalGB = mem.swap_total_bytes / (1024 * 1024 * 1024);
                    ramBars.push({ value: swapUsedGB, max: swapTotalGB, label: `<span style="font-weight:700;color:#a855f7">${formatNumber(swapUsedGB, 1)}G</span>`, color: '#a855f7', name: 'SWAP' });
                }
                return `
                    <div class="radial-container">
                        ${renderRadialProgress(mem.usage_percent, 'RAM', '#f59e0b')}
                        ${renderChargeBars(ramBars)}
                    </div>
                    ${mem.memory_speed_mhz ? `<div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${mem.memory_type ? mem.memory_type + ' ' : ''}${mem.memory_speed_mhz} MHz</div>` : ''}
                `;
            }

            // Chart mode
            if (displayMode === 'chart') {
                return `
                    <div class="mini-chart-container">
                        <div class="mini-chart-header ">
                            <span class="metric-label">RAM</span>
                            <span class="metric-value">${formatNumber(mem.usage_percent, 0)}%</span>
                        </div>
                        <canvas id="ram-mini-chart" class="mini-chart"></canvas>
                    </div>
                `;
            }

            // Text mode
            if (displayMode === 'text') {
                const hasSwap = mem.swap_total_bytes && mem.swap_total_bytes > 0;
                const swapUsedGB = hasSwap ? mem.swap_used_bytes / (1024 * 1024 * 1024) : 0;
                const swapTotalGB = hasSwap ? mem.swap_total_bytes / (1024 * 1024 * 1024) : 0;
                return `
                    <div class="widget-value">${formatNumber(mem.usage_percent, 0)}<span class="unit">%</span></div>
                    <div class="metric-info ${globalDisplay !== 'normal' ? 'hidden' : ''}">${formatNumber(usedGB, 1)} / ${formatNumber(totalGB, 1)} GB</div>
                    ${hasSwap && globalDisplay === 'normal' ? `<div class="metric-info">${t('widget.swap')}: ${formatNumber(swapUsedGB, 1)} / ${formatNumber(swapTotalGB, 1)} GB</div>` : ''}
                `;
            }

            // Default bar mode
            const hasSwap = mem.swap_total_bytes && mem.swap_total_bytes > 0;
            const swapUsedGB = hasSwap ? mem.swap_used_bytes / (1024 * 1024 * 1024) : 0;
            const swapTotalGB = hasSwap ? mem.swap_total_bytes / (1024 * 1024 * 1024) : 0;
            const swapPercent = hasSwap ? mem.swap_usage_percent : 0;
            return `
                <div class="metric-row">
                    <div class="progress-bar"><div class="progress-fill ram-fill" style="width: ${mem.usage_percent}%"></div></div>
                    <span class="metric-value">${formatNumber(mem.usage_percent, 0)}%</span>
                    <span class="text-muted">${formatNumber(usedGB, 1)}/${formatNumber(totalGB, 1)} GB</span>
                </div>
                ${mem.memory_speed_mhz ? `<div class="metric-row ${globalDisplay !== 'normal' ? 'hidden' : ''}">
                    <span class="metric-label">${t('widget.speed')}</span>
                    <span class="metric-value">${mem.memory_type ? mem.memory_type + ' ' : ''}${mem.memory_speed_mhz} MHz</span>
                </div>` : ''}
                ${hasSwap && globalDisplay === 'normal' ? `
                <div class="metric-row" style="margin-top: 4px">
                    <span class="metric-label">${t('widget.swap')}</span>
                    <div class="progress-bar" style="flex:1"><div class="progress-fill" style="width: ${swapPercent}%; background: #a855f7"></div></div>
                    <span class="metric-value">${formatNumber(swapPercent, 0)}%</span>
                </div>
                <div class="metric-info">${formatNumber(swapUsedGB, 1)} / ${formatNumber(swapTotalGB, 1)} GB</div>
                ` : ''}
            `;
        },
    },
    surplus: {
        id: 'surplus',
        titleKey: 'widget.surplus',
        shortTitleKey: 'widget.surplus_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>`,
        defaultSize: 'medium',
        render: (data) => {
            const session = data.activeSession;
            if (!session) return `
                <div class="widget-na">${t('widget.start_session_to_track')}</div>
                <button class="btn btn-sm btn-secondary set-baseline-btn" data-power="${formatNumber(data.power_watts, 1)}">${t('widget.set_baseline')} (${formatNumber(data.power_watts, 1)} W)</button>
            `;
            return `
                <div class="metric-row">
                    <span class="metric-label">${t('widget.baseline')}</span>
                    <span class="metric-value">${formatNumber(session.baseline_watts, 1)} W</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">${t('widget.current')}</span>
                    <span class="metric-value">${formatNumber(data.power_watts, 1)} W</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">${t('session.surplus')}</span>
                    <span class="metric-value surplus-value">${formatNumber(session.surplus_wh, 2)} Wh</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">${t('widget.cost')}</span>
                    <span class="metric-value">${state.currencySymbol}${formatNumber(session.surplus_cost || 0, 4)}</span>
                </div>
                <button class="btn btn-sm btn-secondary set-baseline-btn" data-power="${formatNumber(data.power_watts, 1)}">${t('widget.update_baseline')}</button>
            `;
        },
    },
    session_controls: {
        id: 'session_controls',
        titleKey: 'widget.session_controls',
        shortTitleKey: 'widget.session_controls_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>`,
        defaultSize: 'medium',
        render: (data) => {
            const session = data.activeSession;
            const elapsed = session ? Math.floor(Date.now() / 1000) - session.start_time : 0;
            const duration = session ? formatDuration(elapsed) : '--:--:--';
            const surplusWh = session ? formatNumber(session.surplus_wh, 2) : '--';
            const surplusCost = session ? `${state.currencySymbol}${formatNumber(session.surplus_cost || 0, 4)}` : '--';

            if (!session) {
                return `
                    <div class="session-widget session-widget-idle">
                        <button class="btn btn-primary session-widget-start-btn session-widget-start-big">${t('session.start')}</button>
                    </div>
                `;
            }

            const categories = state.sessionCategories || [];
            const currentCategory = session.category || '';
            const categoryOptions = categories.map(c =>
                `<option value="${c.name}" ${currentCategory === c.name ? 'selected' : ''}>${c.emoji} ${c.name}</option>`
            ).join('');

            return `
                <div class="session-widget">
                    <div class="session-widget-info">
                        <div class="session-widget-status">
                            <span class="session-widget-indicator active"></span>
                            <span class="session-widget-label">${t('widget.session_active')}</span>
                        </div>
                        <span class="session-widget-duration">${duration}</span>
                    </div>
                    <div class="session-widget-fields">
                        <input type="text" class="session-name-input" id="session-name-input"
                            placeholder="${t('session.name_placeholder')}"
                            value="${session.label || ''}"
                            data-session-id="${session.id}">
                        <select class="session-category-select" id="session-category-select" data-session-id="${session.id}">
                            <option value="">${t('session.no_category')}</option>
                            ${categoryOptions}
                        </select>
                    </div>
                    <div class="session-widget-btns">
                        <button class="btn btn-secondary btn-sm session-widget-end-btn">${t('session.end')}</button>
                    </div>
                </div>
            `;
        },
    },
    processes: {
        id: 'processes',
        titleKey: 'widget.processes',
        shortTitleKey: 'widget.processes_short',
        icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,
        defaultSize: 'large',
        defaultColSpan: 2,
        defaultRowSpan: 3,  // Needs height for process list
        render: (data) => {
            const processes = data.topProcesses;
            const advancedMode = state.processAdvancedMode || false;
            const displayList = advancedMode ? (state.allProcesses || []) : (processes || []);

            if (!displayList || displayList.length === 0) {
                return `<div class="widget-na">${t('widget.no_process_data')}</div>`;
            }

            const pinnedIcon = `<svg class="pin-icon" viewBox="0 0 24 24" fill="currentColor" stroke="none" width="12" height="12"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1 1-1v-6h5v-2l-2-2z"/></svg>`;
            const unpinnedIcon = `<svg class="pin-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1 1-1v-6h5v-2l-2-2z"/></svg>`;

            return `
                <div class="process-widget">
                    <div class="process-header">
                        <div class="process-header-row">
                            <span class="process-col-name">${t('processes.header.name')}</span>
                            <span class="process-col-cpu">${t('processes.header.cpu')}</span>
                            <span class="process-col-gpu">${t('processes.header.gpu')}</span>
                            <span class="process-col-ram">${t('processes.header.ram')}</span>
                            <span class="process-col-pin"></span>
                        </div>
                        <button class="btn-icon process-advanced-toggle ${advancedMode ? 'active' : ''}" title="${advancedMode ? t('widget.show_top') : t('widget.search_processes')}">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
                                <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
                            </svg>
                        </button>
                    </div>
                    <div class="process-list-scroll">
                        ${displayList.map(proc => {
                            const cpuVal = (proc.cpu_percent != null && !isNaN(proc.cpu_percent)) ? formatNumber(proc.cpu_percent, 1) : '--';
                            const gpuVal = (proc.gpu_percent != null && !isNaN(proc.gpu_percent)) ? formatNumber(proc.gpu_percent, 1) : '--';
                            const ramVal = (proc.memory_percent != null && !isNaN(proc.memory_percent)) ? formatNumber(proc.memory_percent, 1) : '--';
                            return `
                                <div class="process-row ${proc.is_pinned ? 'pinned' : ''}">
                                    <span class="process-name" title="${proc.name}">${proc.is_pinned ? '<span class="pinned-indicator">📌</span>' : ''}${proc.name.slice(0, 20)}</span>
                                    <span class="process-cpu">${cpuVal}%</span>
                                    <span class="process-gpu">${gpuVal}%</span>
                                    <span class="process-ram">${ramVal}%</span>
                                    <button class="process-pin-btn" data-name="${proc.name}" title="${proc.is_pinned ? t('widget.unpin') : t('widget.pin')}">
                                        ${proc.is_pinned ? pinnedIcon : unpinnedIcon}
                                    </button>
                                </div>
                            `;
                        }).join('')}
                    </div>
                </div>
            `;
        },
    },
};

// Helper function to get widget title from translations
// Uses short title when widget is 1×1 and a short key exists
function getWidgetTitle(widgetId, widgetConfig) {
    const widget = WIDGET_REGISTRY[widgetId];
    if (!widget) return widgetId;

    // Check if widget is 1×1 and has a short title
    if (widgetConfig && widget.shortTitleKey) {
        const colSpan = widgetConfig.col_span || 1;
        const rowSpan = widgetConfig.row_span || 1;
        if (colSpan === 1 && rowSpan === 1) {
            const shortTitle = t(widget.shortTitleKey);
            if (shortTitle) return shortTitle;
        }
    }

    return t(widget.titleKey) || widget.titleKey;
}

// Helper to render estimation widgets with both cost and Wh lines
function renderEstimationWidget(data, widgetConfig, opts) {
    const { costValue, costDecimals, unitKey, whMultiplier } = opts;
    const showWh = widgetConfig?.show_wh !== false; // default true
    const whValue = (data.avg_power_watts || data.power_watts) * whMultiplier;
    const whDisplay = whValue >= 1000
        ? `${formatNumber(whValue / 1000, 1)}<span class="unit">kWh</span>`
        : `${formatNumber(whValue, 0)}<span class="unit">Wh</span>`;

    const eyeIcon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>`;
    const eyeOffIcon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>`;

    return `
        <div class="estimation-lines">
            <div class="estimation-line">
                <div class="widget-value small cost-value">${state.currencySymbol}${formatNumber(costValue, costDecimals)}<span class="unit">${t(unitKey)}</span></div>
            </div>
            ${showWh ? `<div class="estimation-line secondary">
                <div class="widget-value small">${whDisplay}</div>
            </div>` : ''}
        </div>
        <button class="estimation-toggle-btn" data-widget-id="${widgetConfig?.id}" title="${showWh ? t('widget.show_cost') : t('widget.show_energy')}">
            ${showWh ? eyeIcon : eyeOffIcon}
        </button>
    `;
}

// ===== State Management =====
const state = {
    translations: {},
    config: null,
    dashboardConfig: null,
    powerHistory: [],
    maxHistoryPoints: 60,
    currencySymbol: '\u20AC',
    historyData: [],
    // Tiered update interval IDs
    criticalIntervalId: null,  // Fast updates (power, CPU%, GPU%, cost)
    detailedIntervalId: null,  // Slow updates (processes, temps, VRAM)
    dashboardIntervalId: null, // Legacy - kept for backwards compat
    systemMetrics: null,
    activeSession: null,
    topProcesses: [],
    // Cached metrics from backend
    criticalMetrics: null,
    detailedMetrics: null,
    isEditMode: false,
    draggedWidget: null,
    draggedWidgetId: null,
    dragOffset: { x: 0, y: 0 },
    gridCols: 6,
    cellHeight: 100,  // Match CSS grid-auto-rows: 100px
    gridGap: 16,     // Match CSS gap: var(--spacing-md) = 1rem = 16px
    resizing: false,
    resizeWidgetId: null,
    resizeStartPos: null,
    resizeStartSpan: null,
    // Stream A: History arrays for mini-charts (max 60 points each)
    cpuHistory: [],
    gpuHistory: [],
    ramHistory: [],
    sessionStartTime: null,
    lastDashboardData: null,
    processAdvancedMode: false,
    allProcesses: [],
    sessionCategories: [],
    historyRange: 7,
    historyOffset: 0,
    historyMode: 'power',
    sessionCategoryFilter: [],
};

// Widget classification for tiered updates
const CRITICAL_WIDGETS = ['power', 'cpu', 'gpu', 'session_cost', 'session_energy', 'session_duration', 'hourly_estimate', 'daily_estimate', 'monthly_estimate', 'session_controls'];
const DETAILED_WIDGETS = ['processes', 'ram', 'surplus'];

// ===== Initialization =====
document.addEventListener('DOMContentLoaded', async () => {
    try {
        await loadTranslations();
        state.config = await invoke('get_config');
        state.dashboardConfig = await invoke('get_dashboard_config');
        applyConfig(state.config);

        await loadSessionCategories();
        setupNavigation();
        setupSettings();
        setupDashboard();
        setupSourceBadgeToggle();
        setupHistoryTabs();
        setupCategorySettings();

        startDashboardUpdates();

        // Listen for tiered push updates from backend
        await listen('critical-update', (event) => {
            handleCriticalUpdate(event.payload);
        });

        await listen('detailed-update', (event) => {
            handleDetailedUpdate(event.payload);
        });

        // Legacy power-update listener (for backwards compat)
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
    // Also handle placeholder translations
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        const key = el.getAttribute('data-i18n-placeholder');
        if (state.translations[key]) {
            el.placeholder = state.translations[key];
        }
    });
    // Also handle title attribute translations
    document.querySelectorAll('[data-i18n-title]').forEach(el => {
        const key = el.getAttribute('data-i18n-title');
        if (state.translations[key]) {
            el.title = state.translations[key];
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
            navLinks.forEach(l => l.classList.remove('active'));
            link.classList.add('active');
            views.forEach(v => v.classList.remove('active'));
            document.getElementById(targetView).classList.add('active');

            if (targetView === 'history') {
                // Re-init segmented controls now that the view is visible
                initSegmentedControl(document.getElementById('history-range-control'));
                initSegmentedControl(document.getElementById('history-mode-control'));
                loadHistoryForRange();
            }
        });
    });

    // Sidebar toggle functionality
    setupSidebarToggle();
}

function setupSidebarToggle() {
    const sidebar = document.getElementById('sidebar');
    const toggleBtn = document.getElementById('sidebar-toggle');

    // Restore collapsed state from localStorage
    const isCollapsed = localStorage.getItem('sidebar-collapsed') === 'true';
    if (isCollapsed) {
        sidebar.classList.add('collapsed');
    }

    // Toggle sidebar on button click
    toggleBtn.addEventListener('click', () => {
        sidebar.classList.toggle('collapsed');
        const collapsed = sidebar.classList.contains('collapsed');
        localStorage.setItem('sidebar-collapsed', collapsed);
    });
}

// ===== Source Badge Toggle =====
function setupSourceBadgeToggle() {
    const badge = document.getElementById('source-badge');
    if (!badge) return;

    // Start expanded, auto-collapse after 5s
    setTimeout(() => {
        if (!badge.classList.contains('collapsed')) {
            badge.classList.add('collapsed');
        }
    }, 5000);

    badge.addEventListener('click', () => {
        badge.classList.toggle('collapsed');
    });
}

// ===== Dashboard =====
function setupDashboard() {
    // Migrate legacy configs to new grid format
    migrateDashboardConfig();

    renderDashboard();

    // Edit dashboard button - now toggles edit mode
    document.getElementById('edit-dashboard-btn').addEventListener('click', toggleEditMode);

    // Edit mode toolbar buttons
    document.getElementById('exit-edit-mode-btn').addEventListener('click', exitEditMode);
    document.getElementById('toggle-visibility-btn').addEventListener('click', toggleVisibilityPanel);
    document.getElementById('fix-layout-btn').addEventListener('click', fixLayout);
    document.getElementById('close-visibility-panel').addEventListener('click', () => {
        document.getElementById('visibility-panel').classList.add('hidden');
    });

    // Legacy modal handlers (keep for backwards compatibility)
    document.getElementById('close-edit-modal').addEventListener('click', closeEditModal);
    document.getElementById('save-dashboard-btn').addEventListener('click', saveDashboardConfig);
    document.getElementById('reset-dashboard-btn').addEventListener('click', resetDashboard);
    document.getElementById('edit-dashboard-modal').addEventListener('click', (e) => {
        if (e.target.classList.contains('modal')) closeEditModal();
    });

    // Global mouse handlers for drag and resize
    document.addEventListener('mousemove', handleGlobalMouseMove);
    document.addEventListener('mouseup', handleGlobalMouseUp);

    // Event delegation for dynamically rendered buttons (Stream C)
    document.getElementById('dashboard-grid').addEventListener('click', handleDashboardClick);

    // Event delegation for session name input (debounced)
    let sessionNameTimer = null;
    document.getElementById('dashboard-grid').addEventListener('input', (e) => {
        const nameInput = e.target.closest('.session-name-input');
        if (nameInput) {
            const sessionId = parseInt(nameInput.dataset.sessionId);
            const label = nameInput.value;
            if (sessionNameTimer) clearTimeout(sessionNameTimer);
            sessionNameTimer = setTimeout(() => {
                if (sessionId) {
                    invoke('update_session_label', { sessionId, label }).catch(err =>
                        console.error('Failed to update label:', err)
                    );
                }
            }, 500);
        }
    });

    // Event delegation for session category select (change event)
    document.getElementById('dashboard-grid').addEventListener('change', (e) => {
        const categorySelect = e.target.closest('.session-category-select');
        if (categorySelect) {
            const sessionId = parseInt(categorySelect.dataset.sessionId);
            const category = categorySelect.value || null;
            if (sessionId) {
                invoke('update_session_category', { sessionId, category }).catch(err =>
                    console.error('Failed to update category:', err)
                );
            }
        }
    });

    // Stream A: Setup global display toggle buttons
    setupGlobalDisplayToggle();

    // Process search modal handlers
    setupProcessModal();
}

// ===== Process Search Modal =====
function setupProcessModal() {
    const modal = document.getElementById('process-search-modal');
    const closeBtn = document.getElementById('close-process-modal');
    const searchInput = document.getElementById('process-search-input');

    closeBtn.addEventListener('click', closeProcessModal);
    modal.addEventListener('click', (e) => {
        if (e.target.classList.contains('modal')) closeProcessModal();
    });

    searchInput.addEventListener('input', (e) => {
        filterProcessList(e.target.value);
    });

    // Handle pin clicks in modal
    document.getElementById('process-modal-list').addEventListener('click', async (e) => {
        const pinBtn = e.target.closest('.process-modal-pin-btn');
        if (pinBtn) {
            const name = pinBtn.dataset.name;
            if (!name) return;

            try {
                const pinnedList = await invoke('get_pinned_processes');
                const isPinned = pinnedList.some(p => p.toLowerCase() === name.toLowerCase());

                if (isPinned) {
                    await invoke('unpin_process', { name });
                    showToast(`${t('processes.unpinned')}: ${name}`, 'info');
                } else {
                    await invoke('pin_process', { name });
                    showToast(`${t('processes.pinned')}: ${name}`, 'success');
                }

                // Refresh the modal list
                await refreshProcessModalList();
            } catch (error) {
                console.error('Failed to toggle pin:', error);
                showToast(t('processes.pin_failed'), 'error');
            }
        }
    });
}

async function openProcessModal() {
    const modal = document.getElementById('process-search-modal');
    const loadingBar = document.getElementById('process-loading-bar');
    const searchInput = document.getElementById('process-search-input');

    modal.classList.remove('hidden');
    searchInput.value = '';
    loadingBar.classList.remove('hidden');

    try {
        state.allProcesses = await invoke('get_all_processes');
        loadingBar.classList.add('hidden');
        renderProcessModalList(state.allProcesses);
    } catch (error) {
        console.error('Failed to get all processes:', error);
        loadingBar.classList.add('hidden');
        state.allProcesses = [];
        renderProcessModalList([]);
    }

    searchInput.focus();
}

function closeProcessModal() {
    const modal = document.getElementById('process-search-modal');
    modal.classList.add('hidden');
    state.allProcesses = [];
}

async function refreshProcessModalList() {
    try {
        state.allProcesses = await invoke('get_all_processes');
        const searchInput = document.getElementById('process-search-input');
        filterProcessList(searchInput.value);
    } catch (error) {
        console.error('Failed to refresh processes:', error);
    }
}

function filterProcessList(query) {
    const filtered = query
        ? state.allProcesses.filter(p => p.name.toLowerCase().includes(query.toLowerCase()))
        : state.allProcesses;
    renderProcessModalList(filtered);
}

function renderProcessModalList(processes) {
    const list = document.getElementById('process-modal-list');

    if (processes.length === 0) {
        list.innerHTML = `<div class="process-modal-empty">${t('widget.no_processes_found')}</div>`;
        return;
    }

    const pinnedIcon = `<svg viewBox="0 0 24 24" fill="currentColor" stroke="none" width="14" height="14"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1 1-1v-6h5v-2l-2-2z"/></svg>`;
    const unpinnedIcon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1 1-1v-6h5v-2l-2-2z"/></svg>`;

    list.innerHTML = processes.map(proc => {
        const cpuVal = (proc.cpu_percent != null && !isNaN(proc.cpu_percent)) ? formatNumber(proc.cpu_percent, 1) : '--';
        const gpuVal = (proc.gpu_percent != null && !isNaN(proc.gpu_percent)) ? formatNumber(proc.gpu_percent, 1) : '--';
        const ramVal = (proc.memory_percent != null && !isNaN(proc.memory_percent)) ? formatNumber(proc.memory_percent, 1) : '--';
        return `
            <div class="process-modal-row ${proc.is_pinned ? 'pinned' : ''}">
                <span class="process-modal-name" title="${proc.name}">${proc.is_pinned ? '<span class="pinned-indicator">📌</span>' : ''}${proc.name}</span>
                <span class="process-modal-cpu">${cpuVal}%</span>
                <span class="process-modal-gpu">${gpuVal}%</span>
                <span class="process-modal-ram">${ramVal}%</span>
                <button class="process-modal-pin-btn" data-name="${proc.name}" title="${proc.is_pinned ? t('widget.unpin') : t('widget.pin')}">
                    ${proc.is_pinned ? pinnedIcon : unpinnedIcon}
                </button>
            </div>
        `;
    }).join('');
}

/**
 * Stream A: Setup global display toggle button group
 */
function setupGlobalDisplayToggle() {
    const toggleContainer = document.getElementById('global-display-toggle');
    if (!toggleContainer) return;

    // Set initial state
    const currentMode = state.dashboardConfig?.global_display || 'normal';
    updateGlobalDisplayToggle(currentMode);

    // Add click handlers to toggle buttons
    const buttons = toggleContainer.querySelectorAll('.global-display-btn');
    buttons.forEach(btn => {
        btn.addEventListener('click', () => {
            const mode = btn.dataset.mode;
            setGlobalDisplay(mode);
        });
    });
}

// Handle clicks on dynamically rendered elements in the dashboard
async function handleDashboardClick(e) {
    // Handle estimation widget Wh toggle
    const toggleBtn = e.target.closest('.estimation-toggle-btn');
    if (toggleBtn) {
        e.stopPropagation();
        const widgetId = toggleBtn.dataset.widgetId;
        if (widgetId) {
            const wc = state.dashboardConfig?.widgets?.find(w => w.id === widgetId);
            if (wc) {
                wc.show_wh = !wc.show_wh;
                saveDashboardConfigQuiet();
                // Re-render just this widget
                const data = state.lastDashboardData || buildDashboardData();
                const widgetDef = WIDGET_REGISTRY[widgetId];
                const body = document.getElementById(`widget-body-${widgetId}`);
                if (widgetDef && body) {
                    body.innerHTML = widgetDef.render(data, null, wc);
                }
            }
        }
        return;
    }

    // Handle "Set Baseline" button click
    const baselineBtn = e.target.closest('.set-baseline-btn');
    if (baselineBtn) {
        e.stopPropagation();
        const powerStr = baselineBtn.dataset.power;
        const watts = parseFloat(powerStr);

        if (!isNaN(watts) && watts > 0) {
            try {
                await invoke('set_manual_baseline', { watts });
                showToast(`${t('settings.baseline.set_success')} ${formatNumber(watts, 1)} W`, 'success');
            } catch (error) {
                console.error('Failed to set baseline:', error);
                showToast(t('settings.baseline.set_failed'), 'error');
            }
        }
    }

    // Handle process search button - open modal
    const advancedToggle = e.target.closest('.process-advanced-toggle');
    if (advancedToggle) {
        e.stopPropagation();
        openProcessModal();
        return;
    }

    // Handle session widget start button
    const sessionStartBtn = e.target.closest('.session-widget-start-btn');
    if (sessionStartBtn) {
        e.stopPropagation();
        startSession();
        return;
    }

    // Handle session widget end button
    const sessionEndBtn = e.target.closest('.session-widget-end-btn');
    if (sessionEndBtn) {
        e.stopPropagation();
        endSession();
        return;
    }

    // Handle session category change
    const categorySelect = e.target.closest('.session-category-select');
    if (categorySelect) {
        e.stopPropagation();
        const sessionId = parseInt(categorySelect.dataset.sessionId);
        const category = categorySelect.value || null;
        if (sessionId) {
            invoke('update_session_category', { sessionId, category }).catch(err =>
                console.error('Failed to update category:', err)
            );
        }
        return;
    }

    // Handle process pin/unpin
    const pinBtn = e.target.closest('.process-pin-btn');
    if (pinBtn) {
        e.stopPropagation();
        const name = pinBtn.dataset.name;
        if (!name) return;

        try {
            // Check if already pinned
            const pinnedList = await invoke('get_pinned_processes');
            const isPinned = pinnedList.some(p => p.toLowerCase() === name.toLowerCase());

            if (isPinned) {
                await invoke('unpin_process', { name });
                showToast(`${t('processes.unpinned')}: ${name}`, 'info');
            } else {
                await invoke('pin_process', { name });
                showToast(`${t('processes.pinned')}: ${name}`, 'success');
            }

            // Refresh processes and update widget
            if (state.processAdvancedMode) {
                state.allProcesses = await invoke('get_all_processes');
            } else {
                state.topProcesses = await invoke('get_top_processes', {});
            }
            // Update just the process widget content
            const processWidget = document.querySelector('[data-widget-id="processes"] .widget-content');
            if (processWidget) {
                processWidget.innerHTML = WIDGETS.processes.render({ topProcesses: state.topProcesses });
            }
        } catch (error) {
            console.error('Failed to toggle pin:', error);
            showToast(t('processes.pin_failed'), 'error');
        }
    }
}

// Migrate legacy dashboard config (position-based) to new grid format
function migrateDashboardConfig() {
    if (!state.dashboardConfig?.widgets) return;

    const widgets = state.dashboardConfig.widgets;

    // Check if migration is needed by seeing if multiple visible widgets share the same position
    // This happens when serde applies default values (col=1, row=1) to all widgets
    const visibleWidgets = widgets.filter(w => w.visible);
    const positions = new Set();
    let needsMigration = false;

    for (const widget of visibleWidgets) {
        const posKey = `${widget.col || 1}-${widget.row || 1}`;
        if (positions.has(posKey)) {
            needsMigration = true;
            break;
        }
        positions.add(posKey);
    }

    if (!needsMigration) return;

    console.log('Migrating dashboard config to grid format...');

    // Sort by position and assign grid positions
    const sorted = [...widgets].sort((a, b) => (a.position || 0) - (b.position || 0));

    // Track occupied cells using a simple grid map
    const occupied = new Set();

    function isAreaFree(col, row, colSpan, rowSpan) {
        for (let r = row; r < row + rowSpan; r++) {
            for (let c = col; c < col + colSpan; c++) {
                if (occupied.has(`${c}-${r}`)) return false;
            }
        }
        return true;
    }

    function occupyArea(col, row, colSpan, rowSpan) {
        for (let r = row; r < row + rowSpan; r++) {
            for (let c = col; c < col + colSpan; c++) {
                occupied.add(`${c}-${r}`);
            }
        }
    }

    function findNextFreePosition(colSpan, rowSpan) {
        for (let row = 1; row <= 20; row++) {
            for (let col = 1; col <= state.gridCols - colSpan + 1; col++) {
                if (isAreaFree(col, row, colSpan, rowSpan)) {
                    return { col, row };
                }
            }
        }
        return { col: 1, row: 1 }; // Fallback
    }

    for (const widget of sorted) {
        // Get default spans from widget registry, falling back to size-based logic
        const widgetDef = WIDGET_REGISTRY[widget.id];
        let colSpan = widgetDef?.defaultColSpan || 1;
        let rowSpan = widgetDef?.defaultRowSpan || 1;

        // Override with size if no defaults in registry
        if (!widgetDef?.defaultColSpan && !widgetDef?.defaultRowSpan) {
            if (widget.size === 'large') {
                colSpan = 2; rowSpan = 2;
            } else if (widget.size === 'medium') {
                colSpan = 2; rowSpan = 1;
            }
        }

        // Find next available position
        const pos = findNextFreePosition(colSpan, rowSpan);

        widget.col = pos.col;
        widget.row = pos.row;
        widget.col_span = colSpan;
        widget.row_span = rowSpan;

        // Mark cells as occupied
        occupyArea(pos.col, pos.row, colSpan, rowSpan);
    }

    // Save migrated config
    saveDashboardConfigQuiet();
    console.log('Dashboard migration complete');
}

function renderDashboard() {
    const grid = document.getElementById('dashboard-grid');
    grid.innerHTML = '';

    // Remove drop preview if it exists
    const existingPreview = document.querySelector('.drop-preview');
    if (existingPreview) existingPreview.remove();

    const widgets = state.dashboardConfig?.widgets || [];

    // Sort by row then col for consistent rendering order
    const sortedWidgets = [...widgets].sort((a, b) => {
        const rowDiff = (a.row || 1) - (b.row || 1);
        if (rowDiff !== 0) return rowDiff;
        return (a.col || 1) - (b.col || 1);
    });

    for (const widgetConfig of sortedWidgets) {
        if (!widgetConfig.visible) continue;

        const widgetDef = WIDGET_REGISTRY[widgetConfig.id];
        if (!widgetDef) continue;

        const card = document.createElement('div');
        card.className = `card widget-card`;
        card.dataset.widgetId = widgetConfig.id;

        // Apply grid positioning
        const col = widgetConfig.col || 1;
        const row = widgetConfig.row || 1;
        const colSpan = widgetConfig.col_span || 1;
        const rowSpan = widgetConfig.row_span || 1;

        card.style.gridColumn = `${col} / span ${colSpan}`;
        card.style.gridRow = `${row} / span ${rowSpan}`;

        // Add size class for adaptive styling based on area
        const area = colSpan * rowSpan;
        if (area <= 1) card.classList.add('widget-1x1');
        else if (area <= 2) card.classList.add('widget-2cell');
        else if (area <= 4) card.classList.add('widget-4cell');

        // Add edit mode class if active
        if (state.isEditMode) {
            card.classList.add('edit-mode');
        }

        card.innerHTML = `
            <div class="card-header">
                ${widgetDef.icon}
                <span>${getWidgetTitle(widgetConfig.id, widgetConfig)}</span>
            </div>
            <div class="card-body" id="widget-body-${widgetConfig.id}">
                <div class="widget-loading">${t('widget.loading')}</div>
            </div>
            ${state.isEditMode ? `<button class="widget-disable-btn" title="${t('widget.hide')}">−</button>` : ''}
            ${state.isEditMode ? '<div class="resize-handle"></div>' : ''}
        `;

        // Edit mode drag handlers
        if (state.isEditMode) {
            card.addEventListener('mousedown', handleWidgetMouseDown);

            // Quick disable button handler
            const disableBtn = card.querySelector('.widget-disable-btn');
            if (disableBtn) {
                disableBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    toggleWidgetVisibility(widgetConfig.id, false);
                });
            }

            // Resize handle handler
            const resizeHandle = card.querySelector('.resize-handle');
            if (resizeHandle) {
                resizeHandle.addEventListener('mousedown', (e) => {
                    e.stopPropagation();
                    startResize(e, widgetConfig.id);
                });
            }
        }

        grid.appendChild(card);
    }

    // Update grid edit mode class
    if (state.isEditMode) {
        grid.classList.add('edit-mode');
    } else {
        grid.classList.remove('edit-mode');
    }

    // Populate with cached data during edit mode
    if (state.isEditMode && state.lastDashboardData) {
        populateWidgetsWithCachedData();
    }
}

/**
 * Populates widgets with cached data during edit mode
 * Uses state.lastDashboardData to render widget content
 */
function populateWidgetsWithCachedData() {
    const data = state.lastDashboardData;
    if (!data) return;

    for (const widgetConfig of state.dashboardConfig?.widgets || []) {
        if (!widgetConfig.visible) continue;
        const widgetDef = WIDGET_REGISTRY[widgetConfig.id];
        if (!widgetDef) continue;

        const body = document.getElementById(`widget-body-${widgetConfig.id}`);
        if (body) {
            // Pass display mode to widgets that support it
            if (widgetDef.supportsDisplayModes) {
                const displayMode = widgetConfig.display_mode || 'bar';
                body.innerHTML = widgetDef.render(data, displayMode, widgetConfig);
            } else {
                body.innerHTML = widgetDef.render(data, null, widgetConfig);
            }
        }
    }

    // Also draw mini charts if any widget is in chart mode
    drawMiniCharts();
}

// ===== Edit Mode Functions =====
function toggleEditMode() {
    if (state.isEditMode) {
        exitEditMode();
    } else {
        enterEditMode();
    }
}

function enterEditMode() {
    state.isEditMode = true;
    document.getElementById('edit-toolbar').classList.remove('hidden');
    document.getElementById('edit-dashboard-btn').classList.add('active');
    renderDashboard();
    showToast(t('dashboard.edit_activated'), 'info');
}

function exitEditMode() {
    state.isEditMode = false;
    document.getElementById('edit-toolbar').classList.add('hidden');
    document.getElementById('visibility-panel').classList.add('hidden');
    document.getElementById('edit-dashboard-btn').classList.remove('active');

    // Remove drop preview
    const preview = document.querySelector('.drop-preview');
    if (preview) preview.remove();

    renderDashboard();
    saveDashboardConfigQuiet();
    showToast(t('dashboard.changes_saved'), 'success');
}

function toggleVisibilityPanel() {
    const panel = document.getElementById('visibility-panel');
    if (panel.classList.contains('hidden')) {
        renderVisibilityPanel();
        panel.classList.remove('hidden');
    } else {
        panel.classList.add('hidden');
    }
}

async function fixLayout() {
    forceGridMigration();
    await saveDashboardConfigQuiet();
    renderDashboard();
    showToast(t('dashboard.default_applied'), 'success');
}

function renderVisibilityPanel() {
    const list = document.getElementById('visibility-list');
    list.innerHTML = '';

    const widgets = state.dashboardConfig?.widgets || [];

    for (const widgetConfig of widgets) {
        const widgetDef = WIDGET_REGISTRY[widgetConfig.id];
        if (!widgetDef) continue;

        const currentMode = widgetConfig.display_mode || 'bar';
        const displayModeSelect = widgetDef.supportsDisplayModes ? `
            <select class="display-mode-select" data-widget-id="${widgetConfig.id}">
                <option value="bar" ${currentMode === 'bar' ? 'selected' : ''}>${t('widget.display.bar')}</option>
                <option value="text" ${currentMode === 'text' ? 'selected' : ''}>${t('widget.display.text')}</option>
                <option value="radial" ${currentMode === 'radial' ? 'selected' : ''}>${t('widget.display.radial')}</option>
                <option value="chart" ${currentMode === 'chart' ? 'selected' : ''}>${t('widget.display.chart')}</option>
            </select>
        ` : '';

        const item = document.createElement('div');
        item.className = 'visibility-item';
        item.innerHTML = `
            <span class="visibility-item-label">${getWidgetTitle(widgetConfig.id, widgetConfig)}</span>
            <div class="visibility-item-controls">
                ${displayModeSelect}
                <label class="toggle">
                    <input type="checkbox" class="visibility-checkbox" data-widget-id="${widgetConfig.id}" ${widgetConfig.visible ? 'checked' : ''}>
                    <span class="toggle-slider"></span>
                </label>
            </div>
        `;

        const checkbox = item.querySelector('.visibility-checkbox');
        checkbox.addEventListener('change', (e) => {
            toggleWidgetVisibility(widgetConfig.id, e.target.checked);
        });

        // Add event handler for display mode changes
        const modeSelect = item.querySelector('.display-mode-select');
        if (modeSelect) {
            modeSelect.addEventListener('change', (e) => {
                const widgetId = e.target.dataset.widgetId;
                const widget = state.dashboardConfig.widgets.find(w => w.id === widgetId);
                if (widget) {
                    widget.display_mode = e.target.value;
                    saveDashboardConfigQuiet();
                    // Re-render the specific widget with new display mode
                    const body = document.getElementById(`widget-body-${widgetId}`);
                    if (body && state.lastDashboardData) {
                        const def = WIDGET_REGISTRY[widgetId];
                        if (def && def.supportsDisplayModes) {
                            body.innerHTML = def.render(state.lastDashboardData, e.target.value);
                            // Redraw mini chart if switching to chart mode
                            if (e.target.value === 'chart') {
                                setTimeout(drawMiniCharts, 50);
                            }
                        }
                    }
                }
            });
        }

        list.appendChild(item);
    }
}

function toggleWidgetVisibility(widgetId, visible) {
    const widget = state.dashboardConfig.widgets.find(w => w.id === widgetId);
    if (widget) {
        widget.visible = visible;
        renderDashboard();
        renderVisibilityPanel();
        saveDashboardConfigQuiet();
    }
}

// ===== Widget Drag Handlers =====

// Get actual grid columns based on viewport width (matching CSS media queries)
function getActualGridCols() {
    const width = window.innerWidth;
    if (width <= 768) return 1;
    if (width <= 900) return 2;
    if (width <= 1200) return 4;
    return 6;
}

// Calculate cell dimensions accounting for CSS grid gaps and scroll position
function getGridCellDimensions(grid) {
    const gridRect = grid.getBoundingClientRect();
    const cols = getActualGridCols();
    const gap = state.gridGap;

    // Cell width = (grid width - total gap space) / number of columns
    // Total gap space = gap * (cols - 1)
    const cellWidth = (gridRect.width - gap * (cols - 1)) / cols;
    const cellHeight = state.cellHeight;

    // Get scroll offset from the main content area
    const mainContent = document.querySelector('.main-content') || document.documentElement;
    const scrollTop = mainContent.scrollTop || 0;
    const scrollLeft = mainContent.scrollLeft || 0;

    return { cellWidth, cellHeight, cols, gap, gridRect, scrollTop, scrollLeft };
}

function handleWidgetMouseDown(e) {
    if (!state.isEditMode) return;
    if (e.target.closest('.widget-disable-btn') || e.target.closest('.resize-handle')) return;

    const card = e.target.closest('.widget-card');
    if (!card) return;

    state.draggedWidget = card;
    state.draggedWidgetId = card.dataset.widgetId;
    card.classList.add('dragging');

    // Save original position for swap functionality
    saveDragOriginalPosition(state.draggedWidgetId);

    // Calculate offset from card top-left
    const rect = card.getBoundingClientRect();
    state.dragOffset = {
        x: e.clientX - rect.left,
        y: e.clientY - rect.top
    };

    // Update gridCols to current viewport value
    state.gridCols = getActualGridCols();

    // Create drop preview
    createDropPreview();

    e.preventDefault();
}

function handleGlobalMouseMove(e) {
    if (state.resizing) {
        handleResize(e);
        return;
    }

    if (!state.draggedWidget || !state.isEditMode) return;

    const grid = document.getElementById('dashboard-grid');
    const { cellWidth, cellHeight, cols, gap, gridRect, scrollTop, scrollLeft } = getGridCellDimensions(grid);

    // Calculate relative position from grid top-left, accounting for scroll and click offset
    const relX = e.clientX - gridRect.left + scrollLeft - state.dragOffset.x;
    const relY = e.clientY - gridRect.top + scrollTop - state.dragOffset.y;

    // Convert pixel position to grid column/row (1-indexed)
    // Each cell occupies (cellWidth + gap) except the last column
    const cellWithGap = cellWidth + gap;
    const targetCol = Math.max(1, Math.min(cols, Math.floor(relX / cellWithGap) + 1));
    const targetRow = Math.max(1, Math.floor(relY / (cellHeight + gap)) + 1);

    // Get widget config for span
    const widget = state.dashboardConfig.widgets.find(w => w.id === state.draggedWidgetId);
    if (!widget) return;

    const colSpan = Math.min(widget.col_span || 1, cols);  // Clamp span to available columns
    const rowSpan = widget.row_span || 1;

    // Clamp to grid bounds
    const finalCol = Math.min(targetCol, cols - colSpan + 1);
    const finalRow = Math.max(1, targetRow);

    // Update drop preview position
    updateDropPreview(finalCol, finalRow, colSpan, rowSpan);
}

function handleGlobalMouseUp(e) {
    if (state.resizing) {
        endResize();
        return;
    }

    if (!state.draggedWidget || !state.isEditMode) return;

    const grid = document.getElementById('dashboard-grid');
    const { cellWidth, cellHeight, cols, gap, gridRect, scrollTop, scrollLeft } = getGridCellDimensions(grid);

    // Calculate final position, accounting for scroll
    const relX = e.clientX - gridRect.left + scrollLeft - state.dragOffset.x;
    const relY = e.clientY - gridRect.top + scrollTop - state.dragOffset.y;

    // Convert pixel position to grid column/row (1-indexed)
    const cellWithGap = cellWidth + gap;
    const targetCol = Math.max(1, Math.min(cols, Math.floor(relX / cellWithGap) + 1));
    const targetRow = Math.max(1, Math.floor(relY / (cellHeight + gap)) + 1);

    // Get widget config
    const widget = state.dashboardConfig.widgets.find(w => w.id === state.draggedWidgetId);
    if (widget) {
        const colSpan = Math.min(widget.col_span || 1, cols);  // Clamp span to available columns
        widget.col = Math.min(targetCol, cols - colSpan + 1);
        widget.row = Math.max(1, targetRow);

        // Resolve collisions (swaps positions instead of pushing down)
        resolveCollisions(widget);
    }

    // Cleanup
    state.draggedWidget.classList.remove('dragging');
    state.draggedWidget = null;
    state.draggedWidgetId = null;

    // Remove drop preview
    const preview = document.querySelector('.drop-preview');
    if (preview) preview.remove();

    renderDashboard();
    saveDashboardConfigQuiet();
}

function createDropPreview() {
    const existing = document.querySelector('.drop-preview');
    if (existing) existing.remove();

    const preview = document.createElement('div');
    preview.className = 'drop-preview';
    document.getElementById('dashboard-grid').appendChild(preview);
}

function updateDropPreview(col, row, colSpan, rowSpan) {
    const preview = document.querySelector('.drop-preview');
    if (!preview) return;

    preview.style.gridColumn = `${col} / span ${colSpan}`;
    preview.style.gridRow = `${row} / span ${rowSpan}`;
}

// ===== Collision Detection =====
// Store original position before drag for swapping
let dragOriginalPosition = null;

function saveDragOriginalPosition(widgetId) {
    const widget = state.dashboardConfig.widgets.find(w => w.id === widgetId);
    if (widget) {
        dragOriginalPosition = {
            id: widgetId,
            col: widget.col || 1,
            row: widget.row || 1
        };
    }
}

function resolveCollisions(movedWidget) {
    const widgets = state.dashboardConfig.widgets.filter(w => w.visible && w.id !== movedWidget.id);
    const cols = getActualGridCols();

    // Phase 1: Find all overlapping widgets
    const overlapping = widgets.filter(w => widgetsOverlap(movedWidget, w));

    if (overlapping.length === 0) {
        dragOriginalPosition = null;
        return;
    }

    // Phase 2: Find primary overlap (closest center to moved widget's center)
    const movedCenterCol = (movedWidget.col || 1) + (movedWidget.col_span || 1) / 2;
    const movedCenterRow = (movedWidget.row || 1) + (movedWidget.row_span || 1) / 2;

    let primaryOverlap = null;
    let minDistance = Infinity;

    for (const widget of overlapping) {
        const widgetCenterCol = (widget.col || 1) + (widget.col_span || 1) / 2;
        const widgetCenterRow = (widget.row || 1) + (widget.row_span || 1) / 2;
        const distance = Math.abs(movedCenterCol - widgetCenterCol) + Math.abs(movedCenterRow - widgetCenterRow);

        if (distance < minDistance) {
            minDistance = distance;
            primaryOverlap = widget;
        }
    }

    // Phase 3: Handle primary swap
    const swappedIds = new Set();

    if (primaryOverlap && dragOriginalPosition) {
        // Check if the swapped widget can fit at the original position
        const origCol = dragOriginalPosition.col;
        const origRow = dragOriginalPosition.row;
        const widgetColSpan = primaryOverlap.col_span || 1;

        // Ensure swap position is within grid bounds
        const clampedCol = Math.min(origCol, cols - widgetColSpan + 1);

        if (canFitAtPosition(primaryOverlap, clampedCol, origRow, [movedWidget.id])) {
            primaryOverlap.col = clampedCol;
            primaryOverlap.row = origRow;
            swappedIds.add(primaryOverlap.id);
        } else {
            // Can't fit at swap position, find an empty spot
            const emptySpot = findEmptySpot(primaryOverlap, movedWidget);
            primaryOverlap.col = emptySpot.col;
            primaryOverlap.row = emptySpot.row;
            swappedIds.add(primaryOverlap.id);
        }
    }

    // Phase 4: Handle secondary overlaps (widgets that overlap but weren't swapped)
    for (const widget of overlapping) {
        if (swappedIds.has(widget.id)) continue;

        // Find an empty spot for this widget
        const emptySpot = findEmptySpot(widget, movedWidget);
        widget.col = emptySpot.col;
        widget.row = emptySpot.row;
    }

    // Clear original position after swap
    dragOriginalPosition = null;

    // Final pass: resolve any remaining collisions by compacting
    compactGrid();

    // Verify no collisions remain
    verifyNoCollisions();
}

// Check if a widget can fit at the given position without overlapping others
function canFitAtPosition(widget, col, row, excludeIds) {
    const cols = getActualGridCols();
    const colSpan = widget.col_span || 1;
    const rowSpan = widget.row_span || 1;

    // Check grid bounds
    if (col < 1 || col + colSpan - 1 > cols || row < 1) {
        return false;
    }

    const testWidget = { col, row, col_span: colSpan, row_span: rowSpan };
    const otherWidgets = state.dashboardConfig.widgets.filter(
        w => w.visible && w.id !== widget.id && !excludeIds.includes(w.id)
    );

    for (const other of otherWidgets) {
        if (widgetsOverlap(testWidget, other)) {
            return false;
        }
    }

    return true;
}

// Verify no widgets overlap and log warning if they do
function verifyNoCollisions() {
    const widgets = state.dashboardConfig.widgets.filter(w => w.visible);

    for (let i = 0; i < widgets.length; i++) {
        for (let j = i + 1; j < widgets.length; j++) {
            if (widgetsOverlap(widgets[i], widgets[j])) {
                console.warn(`Collision detected between ${widgets[i].id} and ${widgets[j].id}`);
                // Try to fix by moving the second widget
                const emptySpot = findEmptySpot(widgets[j], widgets[i]);
                widgets[j].col = emptySpot.col;
                widgets[j].row = emptySpot.row;
            }
        }
    }
}

// Find an empty spot for a widget, avoiding the excluded widget(s)
// excludeWidget can be a single widget or an array of widgets
// Uses column-load balancing to avoid left-side pile-up
function findEmptySpot(widget, excludeWidget) {
    const cols = getActualGridCols();
    const colSpan = Math.min(widget.col_span || 1, cols);
    const rowSpan = widget.row_span || 1;

    // Normalize excludeWidget to an array
    const excludeWidgets = Array.isArray(excludeWidget) ? excludeWidget : [excludeWidget];
    const excludeIds = excludeWidgets.map(w => w.id);

    const allWidgets = state.dashboardConfig.widgets.filter(
        w => w.visible && w.id !== widget.id && !excludeIds.includes(w.id)
    );

    // Combine allWidgets and excludeWidgets for overlap checking
    const checkWidgets = [...allWidgets, ...excludeWidgets.filter(Boolean)];

    for (let row = 1; row <= 50; row++) {
        let bestCol = null;
        let bestLoad = Infinity;

        for (let col = 1; col <= cols - colSpan + 1; col++) {
            const testWidget = { col, row, col_span: colSpan, row_span: rowSpan };
            let hasOverlap = false;

            for (const other of checkWidgets) {
                if (widgetsOverlap(testWidget, other)) {
                    hasOverlap = true;
                    break;
                }
            }

            if (!hasOverlap) {
                const load = getColumnLoad(allWidgets, col, colSpan, cols);
                if (load < bestLoad) {
                    bestLoad = load;
                    bestCol = col;
                }
            }
        }

        if (bestCol !== null) {
            return { col: bestCol, row };
        }
    }

    // Fallback: just put it at the bottom
    return { col: 1, row: 50 };
}

// Calculate the total row-span load on a column band [startCol, startCol+colSpan)
function getColumnLoad(placedWidgets, startCol, colSpan, totalCols) {
    let load = 0;
    for (const w of placedWidgets) {
        const wCol = w.col || 1;
        const wColSpan = w.col_span || 1;
        const wRowSpan = w.row_span || 1;
        // Check if the widget's column band overlaps with [startCol, startCol+colSpan)
        if (wCol < startCol + colSpan && wCol + wColSpan > startCol) {
            load += wRowSpan;
        }
    }
    return load;
}

// Compact the grid by re-placing widgets to fill gaps with column balancing
function compactGrid() {
    const cols = getActualGridCols();
    const widgets = state.dashboardConfig.widgets
        .filter(w => w.visible)
        .sort((a, b) => (a.row || 1) - (b.row || 1) || (a.col || 1) - (b.col || 1));

    const placed = [];

    for (const widget of widgets) {
        const colSpan = Math.min(widget.col_span || 1, cols);
        const rowSpan = widget.row_span || 1;
        let bestPos = null;
        let bestScore = Infinity;

        // Scan positions row by row
        for (let row = 1; row <= 50; row++) {
            for (let col = 1; col <= cols - colSpan + 1; col++) {
                const testWidget = { col, row, col_span: colSpan, row_span: rowSpan };
                let hasOverlap = false;

                for (const other of placed) {
                    if (widgetsOverlap(testWidget, other)) {
                        hasOverlap = true;
                        break;
                    }
                }

                if (!hasOverlap) {
                    // Score: prefer lowest row, then least-loaded columns
                    const load = getColumnLoad(placed, col, colSpan, cols);
                    const score = row * 1000 + load;
                    if (score < bestScore) {
                        bestScore = score;
                        bestPos = { col, row };
                    }
                }
            }
            // If we found a valid position in this row, no need to check further rows
            if (bestPos && bestPos.row === row) break;
        }

        if (bestPos) {
            widget.col = bestPos.col;
            widget.row = bestPos.row;
            widget.col_span = colSpan;
        }
        placed.push(widget);
    }
}

// Track last known grid column count for resize detection
let lastGridCols = getActualGridCols();

// Reflow dashboard grid when column count changes on resize
function reflowDashboardGrid() {
    const newCols = getActualGridCols();
    if (newCols === lastGridCols) return;
    lastGridCols = newCols;

    // Clamp col_span and col to fit new column count
    const widgets = state.dashboardConfig.widgets.filter(w => w.visible);
    for (const widget of widgets) {
        if ((widget.col_span || 1) > newCols) {
            widget.col_span = newCols;
        }
        if ((widget.col || 1) + (widget.col_span || 1) - 1 > newCols) {
            widget.col = Math.max(1, newCols - (widget.col_span || 1) + 1);
        }
    }

    compactGrid();
    renderDashboard();
}

function widgetsOverlap(a, b) {
    const aCol = a.col || 1, aRow = a.row || 1;
    const aColSpan = a.col_span || 1, aRowSpan = a.row_span || 1;
    const bCol = b.col || 1, bRow = b.row || 1;
    const bColSpan = b.col_span || 1, bRowSpan = b.row_span || 1;

    // Check if rectangles overlap
    const aLeft = aCol, aRight = aCol + aColSpan;
    const aTop = aRow, aBottom = aRow + aRowSpan;
    const bLeft = bCol, bRight = bCol + bColSpan;
    const bTop = bRow, bBottom = bRow + bRowSpan;

    return !(aRight <= bLeft || bRight <= aLeft || aBottom <= bTop || bBottom <= aTop);
}

// ===== Resize Handlers =====
function startResize(e, widgetId) {
    state.resizing = true;
    state.resizeWidgetId = widgetId;
    state.resizeStartPos = { x: e.clientX, y: e.clientY };

    const widget = state.dashboardConfig.widgets.find(w => w.id === widgetId);
    state.resizeStartSpan = {
        col: widget.col_span || 1,
        row: widget.row_span || 1
    };

    // Save original position for collision resolution during resize
    saveDragOriginalPosition(widgetId);

    e.preventDefault();
}

function handleResize(e) {
    if (!state.resizing) return;

    const grid = document.getElementById('dashboard-grid');
    const { cellWidth, cellHeight, cols, gap } = getGridCellDimensions(grid);

    const deltaX = e.clientX - state.resizeStartPos.x;
    const deltaY = e.clientY - state.resizeStartPos.y;

    // Account for gap when calculating span deltas
    const colDelta = Math.round(deltaX / (cellWidth + gap));
    const rowDelta = Math.round(deltaY / (cellHeight + gap));

    const widget = state.dashboardConfig.widgets.find(w => w.id === state.resizeWidgetId);
    if (!widget) return;

    // Calculate new spans (min 1, max 4 for cols, max 5 for rows)
    const newColSpan = Math.max(1, Math.min(4, state.resizeStartSpan.col + colDelta));
    const newRowSpan = Math.max(1, Math.min(5, state.resizeStartSpan.row + rowDelta));

    // Ensure widget doesn't exceed grid bounds
    const maxColSpan = cols - (widget.col || 1) + 1;
    widget.col_span = Math.min(newColSpan, maxColSpan);
    widget.row_span = newRowSpan;

    // Update legacy size field for backwards compat
    if (widget.col_span >= 2 && widget.row_span >= 2) {
        widget.size = 'large';
    } else if (widget.col_span >= 2) {
        widget.size = 'medium';
    } else {
        widget.size = 'small';
    }

    renderDashboard();
}

function endResize() {
    if (!state.resizing) return;

    // Resolve any collisions caused by resize
    const widget = state.dashboardConfig.widgets.find(w => w.id === state.resizeWidgetId);
    if (widget) {
        resolveCollisions(widget);
    }

    state.resizing = false;
    state.resizeWidgetId = null;
    state.resizeStartPos = null;
    state.resizeStartSpan = null;

    renderDashboard();
    saveDashboardConfigQuiet();
}

// Legacy drag handlers (keep for fallback)
function handleDragStart(e) {
    state.draggedWidget = e.target.closest('.widget-card');
    state.draggedWidget.classList.add('dragging');
    e.dataTransfer.effectAllowed = 'move';
}

function handleDragOver(e) {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
}

function handleDrop(e) {
    e.preventDefault();
}

function handleDragEnd(e) {
    if (state.draggedWidget) {
        state.draggedWidget.classList.remove('dragging');
        state.draggedWidget = null;
    }
}

async function saveDashboardConfigQuiet() {
    try {
        await invoke('save_dashboard_config', { dashboard: state.dashboardConfig });
    } catch (error) {
        console.error('Failed to save dashboard config:', error);
    }
}

function openEditModal() {
    const modal = document.getElementById('edit-dashboard-modal');
    const list = document.getElementById('widget-toggle-list');
    list.innerHTML = '';

    const sortedWidgets = [...state.dashboardConfig.widgets].sort((a, b) => {
        const rowDiff = (a.row || 1) - (b.row || 1);
        if (rowDiff !== 0) return rowDiff;
        return (a.col || 1) - (b.col || 1);
    });

    for (const widgetConfig of sortedWidgets) {
        const widgetDef = WIDGET_REGISTRY[widgetConfig.id];
        if (!widgetDef) continue;

        // Determine current size from spans
        let currentSize = 'small';
        const colSpan = widgetConfig.col_span || 1;
        const rowSpan = widgetConfig.row_span || 1;
        if (colSpan >= 2 && rowSpan >= 2) currentSize = 'large';
        else if (colSpan >= 2) currentSize = 'medium';

        const item = document.createElement('div');
        item.className = 'widget-toggle-item';
        item.draggable = true;
        item.dataset.widgetId = widgetConfig.id;
        item.innerHTML = `
            <span class="drag-handle">&#x2630;</span>
            <span class="widget-toggle-title">${getWidgetTitle(widgetConfig.id, widgetConfig)}</span>
            <select class="widget-size-select" data-widget-id="${widgetConfig.id}">
                <option value="small" ${currentSize === 'small' ? 'selected' : ''}>${t('widget.size.small')}</option>
                <option value="medium" ${currentSize === 'medium' ? 'selected' : ''}>${t('widget.size.medium')}</option>
                <option value="large" ${currentSize === 'large' ? 'selected' : ''}>${t('widget.size.large')}</option>
            </select>
            <label class="toggle">
                <input type="checkbox" class="widget-toggle-checkbox" data-widget-id="${widgetConfig.id}" ${widgetConfig.visible ? 'checked' : ''}>
                <span class="toggle-slider"></span>
            </label>
        `;

        item.addEventListener('dragstart', (e) => {
            e.target.classList.add('dragging');
            e.dataTransfer.effectAllowed = 'move';
        });
        item.addEventListener('dragover', (e) => {
            e.preventDefault();
            const dragging = list.querySelector('.dragging');
            const target = e.target.closest('.widget-toggle-item');
            if (target && target !== dragging) {
                const items = [...list.querySelectorAll('.widget-toggle-item')];
                const draggedIndex = items.indexOf(dragging);
                const targetIndex = items.indexOf(target);
                if (draggedIndex < targetIndex) {
                    target.parentNode.insertBefore(dragging, target.nextSibling);
                } else {
                    target.parentNode.insertBefore(dragging, target);
                }
            }
        });
        item.addEventListener('dragend', (e) => e.target.classList.remove('dragging'));

        list.appendChild(item);
    }

    modal.classList.remove('hidden');
}

function closeEditModal() {
    document.getElementById('edit-dashboard-modal').classList.add('hidden');
}

async function saveDashboardConfig() {
    const list = document.getElementById('widget-toggle-list');
    const items = [...list.querySelectorAll('.widget-toggle-item')];

    items.forEach((item, index) => {
        const widgetId = item.dataset.widgetId;
        const widget = state.dashboardConfig.widgets.find(w => w.id === widgetId);
        if (widget) {
            widget.position = index;
            widget.visible = item.querySelector('.widget-toggle-checkbox').checked;
            const newSize = item.querySelector('.widget-size-select').value;
            widget.size = newSize;

            // Update spans based on size
            if (newSize === 'large') {
                widget.col_span = 2;
                widget.row_span = 2;
            } else if (newSize === 'medium') {
                widget.col_span = 2;
                widget.row_span = 1;
            } else {
                widget.col_span = 1;
                widget.row_span = 1;
            }
        }
    });

    try {
        await invoke('save_dashboard_config', { dashboard: state.dashboardConfig });
        renderDashboard();
        closeEditModal();
        showToast(t('dashboard.saved'), 'success');
    } catch (error) {
        console.error('Failed to save dashboard:', error);
        showToast(t('dashboard.save_failed'), 'error');
    }
}

async function resetDashboard() {
    try {
        // Get fresh config from backend (includes new default grid positions)
        const freshConfig = await invoke('get_config');
        state.dashboardConfig = freshConfig.dashboard;

        // Force re-migration to ensure proper grid layout
        forceGridMigration();

        await invoke('save_dashboard_config', { dashboard: state.dashboardConfig });
        renderDashboard();
        closeEditModal();
        showToast(t('dashboard.reset_success'), 'success');
    } catch (error) {
        console.error('Failed to reset dashboard:', error);
    }
}

// Force grid migration regardless of current state
function forceGridMigration() {
    if (!state.dashboardConfig?.widgets) return;

    const widgets = state.dashboardConfig.widgets;
    const sorted = [...widgets].sort((a, b) => (a.position || 0) - (b.position || 0));

    // Track occupied cells
    const occupied = new Set();

    function isAreaFree(col, row, colSpan, rowSpan) {
        for (let r = row; r < row + rowSpan; r++) {
            for (let c = col; c < col + colSpan; c++) {
                if (occupied.has(`${c}-${r}`)) return false;
            }
        }
        return true;
    }

    function occupyArea(col, row, colSpan, rowSpan) {
        for (let r = row; r < row + rowSpan; r++) {
            for (let c = col; c < col + colSpan; c++) {
                occupied.add(`${c}-${r}`);
            }
        }
    }

    function findNextFreePosition(colSpan, rowSpan) {
        for (let row = 1; row <= 20; row++) {
            for (let col = 1; col <= state.gridCols - colSpan + 1; col++) {
                if (isAreaFree(col, row, colSpan, rowSpan)) {
                    return { col, row };
                }
            }
        }
        return { col: 1, row: 1 };
    }

    for (const widget of sorted) {
        // Get default spans from widget registry, falling back to size-based logic
        const widgetDef = WIDGET_REGISTRY[widget.id];
        let colSpan = widgetDef?.defaultColSpan || 1;
        let rowSpan = widgetDef?.defaultRowSpan || 1;

        // Override with size if no defaults in registry
        if (!widgetDef?.defaultColSpan && !widgetDef?.defaultRowSpan) {
            if (widget.size === 'large') {
                colSpan = 2; rowSpan = 2;
            } else if (widget.size === 'medium') {
                colSpan = 2; rowSpan = 1;
            }
        }

        const pos = findNextFreePosition(colSpan, rowSpan);
        widget.col = pos.col;
        widget.row = pos.row;
        widget.col_span = colSpan;
        widget.row_span = rowSpan;

        occupyArea(pos.col, pos.row, colSpan, rowSpan);
    }
}

// ===== Dashboard Updates (Tiered Architecture) =====

// Widget classification for which widgets update at which rate
function isCriticalWidget(widgetId) {
    return CRITICAL_WIDGETS.includes(widgetId);
}

function isDetailedWidget(widgetId) {
    return DETAILED_WIDGETS.includes(widgetId);
}

async function startDashboardUpdates() {
    // Initial full update
    await updateDashboard();

    // Start tiered update timers
    const fastRate = state.config?.general?.refresh_rate_ms || 1000;
    const slowRate = state.config?.general?.slow_refresh_rate_ms || 5000;

    // Critical updates (power, CPU%, GPU%, cost) - fast rate
    state.criticalIntervalId = setInterval(updateCriticalWidgets, fastRate);

    // Detailed updates (processes, temps, VRAM) - slow rate
    state.detailedIntervalId = setInterval(updateDetailedWidgets, slowRate);

    // Keep legacy interval for backwards compat (but it does nothing now)
    state.dashboardIntervalId = null;

    console.log(`Dashboard updates started: critical=${fastRate}ms, detailed=${slowRate}ms`);
}

function restartDashboardUpdates() {
    // Clear all intervals
    if (state.criticalIntervalId) {
        clearInterval(state.criticalIntervalId);
        state.criticalIntervalId = null;
    }
    if (state.detailedIntervalId) {
        clearInterval(state.detailedIntervalId);
        state.detailedIntervalId = null;
    }
    if (state.dashboardIntervalId) {
        clearInterval(state.dashboardIntervalId);
        state.dashboardIntervalId = null;
    }

    // Restart with current config
    const fastRate = state.config?.general?.refresh_rate_ms || 1000;
    const slowRate = state.config?.general?.slow_refresh_rate_ms || 5000;

    state.criticalIntervalId = setInterval(updateCriticalWidgets, fastRate);
    state.detailedIntervalId = setInterval(updateDetailedWidgets, slowRate);

    console.log(`Dashboard updates restarted: critical=${fastRate}ms, detailed=${slowRate}ms`);
}

// Handle push updates from backend (critical-update event)
function handleCriticalUpdate(metrics) {
    if (state.isEditMode) return;

    state.criticalMetrics = metrics;
    state.activeSession = metrics.active_session;

    // Build data object compatible with widget renderers
    const data = buildDashboardData();

    // Update only critical widgets
    renderWidgetsByType(data, 'critical');

    // Update power source badge
    const powerSource = document.getElementById('power-source');
    if (powerSource) {
        powerSource.textContent = metrics.source;
    }

    // Update estimation warning
    const warningBanner = document.getElementById('estimation-warning');
    const statusDot = document.querySelector('.status-dot');
    if (warningBanner && statusDot) {
        if (metrics.is_estimated) {
            warningBanner.classList.remove('hidden');
            statusDot.classList.add('estimated');
        } else {
            warningBanner.classList.add('hidden');
            statusDot.classList.remove('estimated');
        }
    }

    // Update power history for graph
    updatePowerHistory(metrics.power_watts);

}

// Handle push updates from backend (detailed-update event)
function handleDetailedUpdate(metrics) {
    if (state.isEditMode) return;

    state.detailedMetrics = metrics;
    state.systemMetrics = metrics.system_metrics;
    state.topProcesses = metrics.top_processes;

    // Update metrics history for mini-charts
    updateMetricsHistory(metrics.system_metrics);

    // Build data object compatible with widget renderers
    const data = buildDashboardData();

    // Update only detailed widgets
    renderWidgetsByType(data, 'detailed');

    // Draw mini-charts after DOM is updated
    drawMiniCharts();
}

// Build dashboard data object from cached metrics
function buildDashboardData() {
    const cm = state.criticalMetrics;
    const dm = state.detailedMetrics;

    return {
        power_watts: cm?.power_watts || 0,
        avg_power_watts: cm?.avg_power_watts ?? 0,
        cumulative_wh: cm?.cumulative_wh || 0,
        current_cost: cm?.current_cost || 0,
        hourly_cost_estimate: cm?.hourly_cost_estimate || 0,
        daily_cost_estimate: cm?.daily_cost_estimate || 0,
        monthly_cost_estimate: cm?.monthly_cost_estimate || 0,
        session_duration_secs: cm?.session_duration_secs || 0,
        source: cm?.source || '--',
        is_estimated: cm?.is_estimated || false,
        systemMetrics: dm?.system_metrics || state.systemMetrics || {
            cpu: { usage_percent: cm?.cpu_usage_percent || 0 },
            gpu: cm?.gpu_usage_percent != null ? { usage_percent: cm.gpu_usage_percent, power_watts: cm.gpu_power_watts } : null,
            memory: dm?.system_metrics?.memory || null,
        },
        activeSession: cm?.active_session || state.activeSession,
        topProcesses: dm?.top_processes || state.topProcesses || [],
    };
}

// Render widgets by type (critical or detailed)
function renderWidgetsByType(data, type) {
    for (const widgetConfig of state.dashboardConfig?.widgets || []) {
        if (!widgetConfig.visible) continue;

        const isCritical = isCriticalWidget(widgetConfig.id);
        const isDetailed = isDetailedWidget(widgetConfig.id);

        // Only update widgets of the requested type
        if (type === 'critical' && !isCritical) continue;
        if (type === 'detailed' && !isDetailed) continue;

        const widgetDef = WIDGET_REGISTRY[widgetConfig.id];
        if (!widgetDef) continue;

        const body = document.getElementById(`widget-body-${widgetConfig.id}`);
        if (body) {
            // Skip re-rendering session_controls when idle (no active session) to avoid hover flicker
            if (widgetConfig.id === 'session_controls' && !data.activeSession && body.querySelector('.session-widget-idle')) continue;

            // For session_controls with active session, only update dynamic text (timer, surplus) without replacing the whole DOM
            // This prevents input/select fields from being destroyed while the user is typing or selecting
            if (widgetConfig.id === 'session_controls' && data.activeSession && body.querySelector('.session-widget')) {
                const session = data.activeSession;
                const elapsed = Math.floor(Date.now() / 1000) - session.start_time;
                const durationEl = body.querySelector('.session-widget-duration');
                if (durationEl) durationEl.textContent = formatDuration(elapsed);
                const surplusVal = body.querySelector('.session-surplus-wh');
                if (surplusVal) surplusVal.textContent = formatNumber(session.surplus_wh, 2);
                const surplusCostVal = body.querySelector('.session-surplus-cost');
                if (surplusCostVal) surplusCostVal.textContent = `${state.currencySymbol}${formatNumber(session.surplus_cost || 0, 4)}`;
                continue;
            }

            if (widgetDef.supportsDisplayModes) {
                const displayMode = widgetConfig.display_mode || 'text';
                body.innerHTML = widgetDef.render(data, displayMode, widgetConfig);
            } else {
                body.innerHTML = widgetDef.render(data, null, widgetConfig);
            }
        }
    }
}

// Polling fallback for critical widgets (if push events aren't working)
async function updateCriticalWidgets() {
    if (state.isEditMode) return;

    try {
        // Try to get cached critical metrics from backend
        const metrics = await invoke('get_critical_metrics').catch(() => null);

        if (metrics) {
            handleCriticalUpdate(metrics);
        } else {
            // Fallback to legacy dashboard data call
            const dashboardData = await invoke('get_dashboard_data');
            const sessionStats = await invoke('get_session_stats').catch(() => null);

            state.activeSession = sessionStats;

            const data = {
                ...dashboardData,
                systemMetrics: state.systemMetrics,
                activeSession: sessionStats,
                topProcesses: state.topProcesses,
            };

            state.lastDashboardData = data;
            renderWidgetsByType(data, 'critical');

            // Update power source badge
            const powerSource = document.getElementById('power-source');
            if (powerSource) {
                powerSource.textContent = dashboardData.source;
            }

            updatePowerHistory(dashboardData.power_watts);
        }
    } catch (error) {
        console.error('Critical update error:', error);
    }
}

// Polling fallback for detailed widgets (if push events aren't working)
async function updateDetailedWidgets() {
    if (state.isEditMode) return;

    try {
        // Try to get cached detailed metrics from backend
        const metrics = await invoke('get_detailed_metrics').catch(() => null);

        if (metrics) {
            handleDetailedUpdate(metrics);
        } else {
            // Fallback to individual calls
            const [systemMetrics, topProcesses] = await Promise.all([
                invoke('get_system_metrics').catch(() => null),
                invoke('get_top_processes', {}).catch(() => []),
            ]);

            state.systemMetrics = systemMetrics;
            state.topProcesses = topProcesses;

            updateMetricsHistory(systemMetrics);

            const data = buildDashboardData();
            renderWidgetsByType(data, 'detailed');

            drawMiniCharts();
        }
    } catch (error) {
        console.error('Detailed update error:', error);
    }
}

// Legacy full dashboard update (used for initial load)
async function updateDashboard() {
    // Stream I: Skip updates in edit mode to prevent data flickering during drag/resize
    if (state.isEditMode) return;

    try {
        const [dashboardData, systemMetrics, sessionStats, topProcesses] = await Promise.all([
            invoke('get_dashboard_data'),
            invoke('get_system_metrics').catch(() => null),
            invoke('get_session_stats').catch(() => null),
            invoke('get_top_processes', {}).catch(() => []),
        ]);

        state.systemMetrics = systemMetrics;
        state.activeSession = sessionStats;
        state.topProcesses = topProcesses;

        // Stream A: Update metrics history for mini-charts
        updateMetricsHistory(systemMetrics);

        // Track session start time for chart temporal axis
        if (sessionStats && !state.sessionStartTime) {
            state.sessionStartTime = sessionStats.start_time * 1000;
        } else if (!sessionStats) {
            state.sessionStartTime = null;
        }

        const data = {
            ...dashboardData,
            systemMetrics,
            activeSession: sessionStats,
            topProcesses,
        };

        // Cache for use during edit mode
        state.lastDashboardData = data;

        // Update each visible widget with display mode support
        for (const widgetConfig of state.dashboardConfig?.widgets || []) {
            if (!widgetConfig.visible) continue;
            const widgetDef = WIDGET_REGISTRY[widgetConfig.id];
            if (!widgetDef) continue;

            const body = document.getElementById(`widget-body-${widgetConfig.id}`);
            if (body) {
                // Pass display mode to widgets that support it
                if (widgetDef.supportsDisplayModes) {
                    const displayMode = widgetConfig.display_mode || 'text';
                    body.innerHTML = widgetDef.render(data, displayMode);
                } else {
                    body.innerHTML = widgetDef.render(data);
                }
            }
        }

        // Update power source badge
        document.getElementById('power-source').textContent = dashboardData.source;

        // Update estimation warning
        const warningBanner = document.getElementById('estimation-warning');
        const statusDot = document.querySelector('.status-dot');
        if (dashboardData.is_estimated) {
            warningBanner.classList.remove('hidden');
            statusDot.classList.add('estimated');
        } else {
            warningBanner.classList.add('hidden');
            statusDot.classList.remove('estimated');
        }

        // Update power history for graph
        updatePowerHistory(dashboardData.power_watts);

        // Stream A: Draw mini-charts after DOM is updated
        drawMiniCharts();

    } catch (error) {
        console.error('Dashboard update error:', error);
    }
}

function updatePowerDisplay(powerWatts) {
    updatePowerHistory(powerWatts);
}

function updatePowerHistory(power) {
    state.powerHistory.push({ time: Date.now(), power });
    if (state.powerHistory.length > state.maxHistoryPoints) {
        state.powerHistory.shift();
    }
    drawPowerGraph();
}

function drawPowerGraph() {
    const canvas = document.getElementById('power-chart');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const container = canvas.parentElement;
    const rect = container.getBoundingClientRect();

    const dpr = window.devicePixelRatio || 1;
    const logicalWidth = Math.max(rect.width, 100);
    const logicalHeight = Math.max(rect.height, 60);
    canvas.width = logicalWidth * dpr;
    canvas.height = logicalHeight * dpr;
    canvas.style.width = logicalWidth + 'px';
    canvas.style.height = logicalHeight + 'px';
    ctx.scale(dpr, dpr);

    const data = state.powerHistory;
    if (data.length < 2) return;

    // Stream A: Add bottom padding for time axis labels
    const padding = { top: 10, right: 10, bottom: 20, left: 10 };
    const width = logicalWidth - padding.left - padding.right;
    const height = logicalHeight - padding.top - padding.bottom;
    const powers = data.map(d => d.power);
    const maxPower = Math.max(...powers) * 1.1 || 100;
    const minPower = Math.min(...powers) * 0.9 || 0;

    ctx.clearRect(0, 0, logicalWidth, logicalHeight);

    const gradient = ctx.createLinearGradient(0, padding.top, 0, height + padding.top);
    gradient.addColorStop(0, 'rgba(99, 102, 241, 0.3)');
    gradient.addColorStop(1, 'rgba(99, 102, 241, 0)');

    ctx.beginPath();
    ctx.moveTo(padding.left, height + padding.top);
    data.forEach((point, i) => {
        const x = padding.left + (i / (data.length - 1)) * width;
        const y = padding.top + height - ((point.power - minPower) / (maxPower - minPower)) * height;
        ctx.lineTo(x, y);
    });
    ctx.lineTo(padding.left + width, height + padding.top);
    ctx.closePath();
    ctx.fillStyle = gradient;
    ctx.fill();

    ctx.beginPath();
    data.forEach((point, i) => {
        const x = padding.left + (i / (data.length - 1)) * width;
        const y = padding.top + height - ((point.power - minPower) / (maxPower - minPower)) * height;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.strokeStyle = '#6366f1';
    ctx.lineWidth = 2;
    ctx.stroke();

    if (data.length > 0) {
        const lastPoint = data[data.length - 1];
        const x = padding.left + width;
        const y = padding.top + height - ((lastPoint.power - minPower) / (maxPower - minPower)) * height;
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fillStyle = '#6366f1';
        ctx.fill();
    }

    // Stream A: Draw time axis labels (HH:MM format)
    if (data.length > 5) {
        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.font = '9px system-ui';
        ctx.textAlign = 'center';

        // Draw 3-5 time labels along the x-axis
        const labelCount = Math.min(5, Math.floor(data.length / 10));
        const step = Math.floor((data.length - 1) / labelCount);

        for (let i = 0; i <= labelCount; i++) {
            const dataIndex = Math.min(i * step, data.length - 1);
            const point = data[dataIndex];
            const x = padding.left + (dataIndex / (data.length - 1)) * width;
            const timeLabel = formatTimeHHMM(point.time);
            ctx.fillText(timeLabel, x, logicalHeight - 4);
        }
    }
}

// ===== Session Controls =====

async function startSession() {
    // Guard: don't start a new session if one is already active
    if (state.activeSession) return;

    try {
        await invoke('start_tracking_session', { label: null });
        // Fetch fresh session data
        state.activeSession = await invoke('get_session_stats').catch(() => null);
        // Re-render session widget
        refreshSessionWidget();
        showToast(t('session.started'), 'success');
    } catch (error) {
        console.error('Failed to start session:', error);
        showToast(t('session.start_failed'), 'error');
    }
}

async function endSession() {
    try {
        const session = await invoke('end_tracking_session');
        state.activeSession = null;
        // Re-render session widget
        refreshSessionWidget();

        if (session) {
            showToast(`${t('session.ended')}: ${formatNumber(session.surplus_wh, 2)} Wh ${t('session.surplus')}`, 'success');
        }
    } catch (error) {
        console.error('Failed to end session:', error);
        showToast(t('session.end_failed'), 'error');
    }
}

function refreshSessionWidget() {
    const widgetBody = document.getElementById('widget-body-session_controls');
    if (widgetBody && state.lastDashboardData) {
        state.lastDashboardData.activeSession = state.activeSession;
        widgetBody.innerHTML = WIDGET_REGISTRY.session_controls.render(state.lastDashboardData);
    }
}

// ===== History =====
// ===== Segmented Control Helper =====
function updateSegmentedControl(container, activeBtn) {
    if (!container || !activeBtn) return;
    const buttons = Array.from(container.querySelectorAll('button'));
    const indicator = container.querySelector('.segmented-indicator');
    if (!indicator || buttons.length === 0) return;

    buttons.forEach(b => b.classList.remove('active'));
    activeBtn.classList.add('active');

    const index = buttons.indexOf(activeBtn);
    const btnWidth = activeBtn.offsetWidth;
    const offset = activeBtn.offsetLeft - 3; // 3px = container padding
    indicator.style.width = `${btnWidth}px`;
    indicator.style.transform = `translateX(${offset}px)`;
}

function initSegmentedControl(container) {
    if (!container) return;
    const activeBtn = container.querySelector('button.active');
    if (activeBtn) {
        // Defer to ensure layout is computed
        requestAnimationFrame(() => updateSegmentedControl(container, activeBtn));
    }
}

function setupHistoryTabs() {
    const rangeControl = document.getElementById('history-range-control');
    const modeControl = document.getElementById('history-mode-control');

    // Initialize segmented control indicators
    initSegmentedControl(rangeControl);
    initSegmentedControl(modeControl);

    // Period range buttons
    rangeControl.querySelectorAll('button').forEach(btn => {
        btn.addEventListener('click', () => {
            const range = btn.dataset.range;
            updateSegmentedControl(rangeControl, btn);

            const customPanel = document.getElementById('history-custom-range');
            if (range === 'custom') {
                customPanel.classList.remove('hidden');
            } else {
                customPanel.classList.add('hidden');
                state.historyRange = parseInt(range);
                state.historyOffset = 0;
                loadHistoryForRange();
            }
        });
    });

    // Custom range apply
    const applyBtn = document.getElementById('history-range-apply');
    if (applyBtn) {
        applyBtn.addEventListener('click', () => {
            state.historyRange = 'custom';
            state.historyOffset = 0;
            loadHistoryForRange();
        });
    }

    // Nav prev/next
    const prevBtn = document.getElementById('history-nav-prev');
    const nextBtn = document.getElementById('history-nav-next');
    if (prevBtn) {
        prevBtn.addEventListener('click', () => {
            if (state.historyRange === 'custom') return;
            state.historyOffset++;
            loadHistoryForRange();
        });
    }
    if (nextBtn) {
        nextBtn.addEventListener('click', () => {
            if (state.historyRange === 'custom') return;
            if (state.historyOffset > 0) {
                state.historyOffset--;
                loadHistoryForRange();
            }
        });
    }

    // Mode toggle: Power / Sessions
    modeControl.querySelectorAll('button').forEach(btn => {
        btn.addEventListener('click', () => {
            const mode = btn.dataset.mode;
            updateSegmentedControl(modeControl, btn);
            state.historyMode = mode;

            const powerContent = document.getElementById('history-power-content');
            const sessionsContent = document.getElementById('history-sessions-content');
            const categoryFilter = document.getElementById('session-category-filter');
            if (mode === 'power') {
                powerContent.classList.remove('hidden');
                sessionsContent.classList.add('hidden');
                if (categoryFilter) categoryFilter.classList.add('hidden');
            } else {
                powerContent.classList.add('hidden');
                sessionsContent.classList.remove('hidden');
            }
            loadHistoryForRange();
        });
    });
}

// Compute start/end dates from shared state
function getHistoryDateRange() {
    const now = new Date();

    if (state.historyRange === 'custom') {
        const startInput = document.getElementById('history-range-start');
        const endInput = document.getElementById('history-range-end');
        if (!startInput || !startInput.value || !endInput || !endInput.value) return null;
        const startDate = new Date(startInput.value);
        const endDate = new Date(endInput.value);
        endDate.setHours(23, 59, 59);
        return { startDate, endDate };
    }

    const days = state.historyRange;
    const endDate = new Date(now);
    endDate.setDate(endDate.getDate() - (state.historyOffset * days));
    const startDate = new Date(endDate);
    startDate.setDate(startDate.getDate() - days);
    return { startDate, endDate };
}

function loadHistoryForRange() {
    const range = getHistoryDateRange();
    if (!range) return;

    // Update range label
    const label = document.getElementById('history-range-label');
    if (label) {
        label.textContent = `${range.startDate.toLocaleDateString()} - ${range.endDate.toLocaleDateString()}`;
    }

    if (state.historyMode === 'power') {
        loadHistoryData(range.startDate, range.endDate);
    } else {
        loadSessionHistoryView(range.startDate, range.endDate);
    }
}

async function loadHistoryData(startDate, endDate) {
    try {
        const startStr = formatDate(startDate);
        const endStr = formatDate(endDate);

        const stats = await invoke('get_history', { startDate: startStr, endDate: endStr });

        // Fill date gaps so chart shows every day in range
        const filledStats = fillDateGaps(stats, startDate, endDate);

        const breakdownEl = document.getElementById('daily-breakdown');
        if (filledStats.length === 0 || filledStats.every(d => d.total_wh === 0)) {
            document.getElementById('no-history-data').classList.remove('hidden');
            document.querySelector('.history-chart-container').classList.add('hidden');
            if (breakdownEl) breakdownEl.classList.add('hidden');
            state.historyData = [];
        } else {
            document.getElementById('no-history-data').classList.add('hidden');
            document.querySelector('.history-chart-container').classList.remove('hidden');
            if (breakdownEl) breakdownEl.classList.remove('hidden');
            state.historyData = filledStats;

            const nonEmpty = filledStats.filter(d => d.total_wh > 0);
            const totalWh = filledStats.reduce((sum, s) => sum + s.total_wh, 0);
            const totalCost = filledStats.reduce((sum, s) => sum + (s.total_cost || 0), 0);
            const avgPower = nonEmpty.length > 0 ? nonEmpty.reduce((sum, s) => sum + s.avg_watts, 0) / nonEmpty.length : 0;
            const maxPower = nonEmpty.length > 0 ? Math.max(...nonEmpty.map(s => s.max_watts)) : 0;

            document.getElementById('history-total-wh').textContent = `${formatNumber(totalWh / 1000, 2)} kWh`;
            document.getElementById('history-total-cost').textContent = `${state.currencySymbol}${formatNumber(totalCost, 2)}`;
            document.getElementById('history-avg-power').textContent = `${formatNumber(avgPower, 0)} W`;
            document.getElementById('history-peak-power').textContent = `${formatNumber(maxPower, 0)} W`;

            // Show rate badge
            const rateBadge = document.getElementById('history-rate-badge');
            if (rateBadge && totalWh > 0 && totalCost > 0) {
                const avgRate = totalCost / (totalWh / 1000);
                rateBadge.textContent = `${state.currencySymbol}${formatNumber(avgRate, 4)}/kWh`;
            }

            // Populate daily breakdown table (only rows with data)
            const tbody = document.getElementById('breakdown-table-body');
            if (tbody) {
                tbody.innerHTML = nonEmpty.map(day => `
                    <tr>
                        <td>${day.date}</td>
                        <td class="energy-cell">${formatNumber(day.total_wh / 1000, 3)} kWh</td>
                        <td>${formatNumber(day.avg_watts, 0)} W</td>
                        <td class="peak-cell">${formatNumber(day.max_watts, 0)} W</td>
                        <td class="cost-cell">${day.total_cost != null ? state.currencySymbol + formatNumber(day.total_cost, 4) : '--'}</td>
                        <td>${day.usage_seconds ? formatDuration(day.usage_seconds) : '--'}</td>
                    </tr>
                `).join('');
            }

            drawHistoryChart();
        }
    } catch (error) {
        console.error('History load error:', error);
    }
}

// Fill missing dates in stats array with zero-value entries
function fillDateGaps(stats, startDate, endDate) {
    const dateMap = {};
    for (const s of stats) {
        dateMap[s.date] = s;
    }

    const filled = [];
    const d = new Date(startDate);
    d.setHours(0, 0, 0, 0);
    const end = new Date(endDate);
    end.setHours(0, 0, 0, 0);

    while (d <= end) {
        const key = formatDate(d);
        if (dateMap[key]) {
            filled.push(dateMap[key]);
        } else {
            filled.push({
                date: key,
                total_wh: 0,
                avg_watts: 0,
                max_watts: 0,
                total_cost: 0,
                usage_seconds: 0,
            });
        }
        d.setDate(d.getDate() + 1);
    }
    return filled;
}

async function loadSessionHistory() {
    const range = getHistoryDateRange();
    if (range) loadSessionHistoryView(range.startDate, range.endDate);
}

function renderCategoryFilterChips(sessions) {
    const container = document.getElementById('session-category-filter');
    if (!container) return;

    // Collect unique categories from sessions
    const categoriesInData = new Set();
    for (const s of sessions) {
        categoriesInData.add(s.category || '');
    }

    if (categoriesInData.size <= 1) {
        container.classList.add('hidden');
        container.innerHTML = '';
        return;
    }

    container.classList.remove('hidden');
    const filter = state.sessionCategoryFilter;

    container.innerHTML = Array.from(categoriesInData).map(cat => {
        const isActive = filter.length === 0 || filter.includes(cat);
        const display = cat ? getCategoryDisplay(cat) : (state.translations['session.no_category'] || 'No category');
        const color = cat ? getCategoryColor(cat) : '#888';
        return `<button class="category-filter-chip ${isActive ? 'active' : ''}" data-category="${cat}" style="--chip-color: ${color}">${display}</button>`;
    }).join('');

    // Click handler for chips
    container.onclick = (e) => {
        const chip = e.target.closest('.category-filter-chip');
        if (!chip) return;

        const cat = chip.dataset.category;
        const allChips = container.querySelectorAll('.category-filter-chip');
        const allCategories = Array.from(allChips).map(c => c.dataset.category);

        if (filter.length === 0) {
            // First click: select only this category (deselect others)
            state.sessionCategoryFilter = [cat];
        } else if (filter.includes(cat)) {
            // Deselect this category
            state.sessionCategoryFilter = filter.filter(c => c !== cat);
            // If none selected, show all
            if (state.sessionCategoryFilter.length === 0) {
                state.sessionCategoryFilter = [];
            }
        } else {
            // Select this category too
            state.sessionCategoryFilter = [...filter, cat];
            // If all selected, reset to show all
            if (state.sessionCategoryFilter.length === allCategories.length) {
                state.sessionCategoryFilter = [];
            }
        }

        // Re-render with current date range
        const range = getHistoryDateRange();
        if (range) loadSessionHistoryView(range.startDate, range.endDate);
    };
}

async function loadSessionHistoryView(startDate, endDate) {
    try {
        const startTs = Math.floor(startDate.getTime() / 1000);
        const endTs = Math.floor(endDate.getTime() / 1000);

        const allSessions = await invoke('get_sessions_in_range', { start: startTs, end: endTs });

        // Build category filter chips
        renderCategoryFilterChips(allSessions);

        // Apply category filter
        const filter = state.sessionCategoryFilter;
        const sessions = filter.length === 0 ? allSessions : allSessions.filter(s => {
            const cat = s.category || '';
            return filter.includes(cat);
        });

        const list = document.getElementById('session-list');
        const empty = document.getElementById('no-sessions');

        if (sessions.length === 0) {
            list.innerHTML = '';
            empty.classList.remove('hidden');
        } else {
            empty.classList.add('hidden');
            const tr = state.translations;
            const categories = state.sessionCategories || [];
            list.innerHTML = sessions.map(s => {
                const sDate = new Date(s.start_time * 1000);
                const duration = s.end_time ? s.end_time - s.start_time : 0;
                const categoryOptions = categories.map(c =>
                    `<option value="${c.name}" ${s.category === c.name ? 'selected' : ''}>${c.emoji} ${c.name}</option>`
                ).join('');
                return `
                    <div class="session-item" data-session-id="${s.id}">
                        <div class="session-item-header">
                            <div class="session-item-header-left">
                                <input type="text" class="session-history-name-input"
                                    data-session-id="${s.id}"
                                    value="${(s.label || '').replace(/"/g, '&quot;')}"
                                    placeholder="${tr['session.name_placeholder'] || 'Session name...'}"
                                    title="${tr['session.edit_name'] || 'Edit name'}">
                                <select class="session-history-category-select" data-session-id="${s.id}">
                                    <option value="">${tr['session.no_category'] || 'No category'}</option>
                                    ${categoryOptions}
                                </select>
                                <span class="session-date">${sDate.toLocaleDateString()} ${sDate.toLocaleTimeString()}</span>
                                <span class="session-duration">${formatDuration(duration)}</span>
                            </div>
                            <div class="session-item-header-right">
                                <span class="session-status completed">
                                    <span class="session-status-dot"></span>
                                    ${s.end_time ? (tr['session.ended'] || 'Completed') : (tr['widget.session_active'] || 'Active')}
                                </span>
                                <button class="session-history-delete-btn" data-session-id="${s.id}" title="${tr['session.delete'] || 'Delete'}">✕</button>
                            </div>
                        </div>
                        <div class="session-item-stats">
                            <div class="session-stat">
                                <span class="session-stat-label">${tr['widget.baseline'] || 'Baseline'}</span>
                                <span class="session-stat-value">${formatNumber(s.baseline_watts, 1)} W</span>
                            </div>
                            <div class="session-stat">
                                <span class="session-stat-label">${tr['history.energy'] || 'Energy'}</span>
                                <span class="session-stat-value">${formatNumber(s.total_wh, 2)} Wh</span>
                            </div>
                            <div class="session-stat">
                                <span class="session-stat-label">${tr['session.surplus'] || 'Surplus'}</span>
                                <span class="session-stat-value surplus">${formatNumber(s.surplus_wh, 2)} Wh</span>
                            </div>
                            <div class="session-stat">
                                <span class="session-stat-label">${tr['history.cost'] || 'Cost'}</span>
                                <span class="session-stat-value cost">${state.currencySymbol}${formatNumber(s.surplus_cost, 4)}</span>
                            </div>
                        </div>
                    </div>
                `;
            }).join('');

            // Set up event delegation for session editing
            setupSessionListEvents(list, sessions);
        }

        // Update session summary stats
        const totalCount = sessions.length;
        const totalEnergy = sessions.reduce((sum, s) => sum + (s.total_wh || 0), 0);
        const totalSurplus = sessions.reduce((sum, s) => sum + (s.surplus_wh || 0), 0);
        const totalCost = sessions.reduce((sum, s) => sum + (s.surplus_cost || 0), 0);

        document.getElementById('session-total-count').textContent = totalCount;
        document.getElementById('session-total-energy').textContent = totalEnergy >= 1000
            ? `${formatNumber(totalEnergy / 1000, 2)} kWh`
            : `${formatNumber(totalEnergy, 1)} Wh`;
        document.getElementById('session-total-surplus').textContent = totalSurplus >= 1000
            ? `${formatNumber(totalSurplus / 1000, 2)} kWh`
            : `${formatNumber(totalSurplus, 1)} Wh`;
        document.getElementById('session-total-cost').textContent = `${state.currencySymbol}${formatNumber(totalCost, 4)}`;

        // Draw histogram
        renderSessionHistogram(sessions, startDate, endDate);

    } catch (error) {
        console.error('Failed to load sessions:', error);
    }
}

function setupSessionListEvents(list, sessions) {
    // Debounce timer for name input
    let nameDebounceTimers = {};

    // Name input - debounced save
    list.addEventListener('input', (e) => {
        if (!e.target.classList.contains('session-history-name-input')) return;
        const sessionId = parseInt(e.target.dataset.sessionId);
        if (nameDebounceTimers[sessionId]) clearTimeout(nameDebounceTimers[sessionId]);
        nameDebounceTimers[sessionId] = setTimeout(async () => {
            try {
                await invoke('update_session_label', { sessionId, label: e.target.value });
            } catch (err) {
                console.error('Failed to update session label:', err);
            }
        }, 500);
    });

    // Category select - immediate save
    list.addEventListener('change', async (e) => {
        if (!e.target.classList.contains('session-history-category-select')) return;
        const sessionId = parseInt(e.target.dataset.sessionId);
        const category = e.target.value || null;
        try {
            await invoke('update_session_category', { sessionId, category });
        } catch (err) {
            console.error('Failed to update session category:', err);
        }
    });

    // Delete button - inline confirm (click once to arm, click again to delete)
    let deleteConfirmTimer = null;
    list.addEventListener('click', async (e) => {
        const btn = e.target.closest('.session-history-delete-btn');
        if (!btn) return;

        // First click: arm the button for confirmation
        if (!btn.classList.contains('confirm')) {
            // Reset any other armed buttons
            list.querySelectorAll('.session-history-delete-btn.confirm').forEach(b => {
                b.classList.remove('confirm');
                b.textContent = '✕';
            });
            btn.classList.add('confirm');
            btn.textContent = '?';
            // Auto-reset after 3 seconds
            if (deleteConfirmTimer) clearTimeout(deleteConfirmTimer);
            deleteConfirmTimer = setTimeout(() => {
                btn.classList.remove('confirm');
                btn.textContent = '✕';
            }, 3000);
            return;
        }

        // Second click: confirmed, proceed with delete
        if (deleteConfirmTimer) clearTimeout(deleteConfirmTimer);
        btn.classList.remove('confirm');
        const sessionId = parseInt(btn.dataset.sessionId);
        try {
            await invoke('delete_session', { sessionId });
            // Remove from DOM
            const item = btn.closest('.session-item');
            if (item) item.remove();
            // Update summary stats
            const remaining = sessions.filter(s => s.id !== sessionId);
            sessions.length = 0;
            sessions.push(...remaining);
            const totalCount = remaining.length;
            const totalEnergy = remaining.reduce((sum, s) => sum + (s.total_wh || 0), 0);
            const totalSurplus = remaining.reduce((sum, s) => sum + (s.surplus_wh || 0), 0);
            const totalCost = remaining.reduce((sum, s) => sum + (s.surplus_cost || 0), 0);
            document.getElementById('session-total-count').textContent = totalCount;
            document.getElementById('session-total-energy').textContent = totalEnergy >= 1000
                ? `${formatNumber(totalEnergy / 1000, 2)} kWh`
                : `${formatNumber(totalEnergy, 1)} Wh`;
            document.getElementById('session-total-surplus').textContent = totalSurplus >= 1000
                ? `${formatNumber(totalSurplus / 1000, 2)} kWh`
                : `${formatNumber(totalSurplus, 1)} Wh`;
            document.getElementById('session-total-cost').textContent = `${state.currencySymbol}${formatNumber(totalCost, 4)}`;
            // Show empty state if no sessions left
            if (totalCount === 0) {
                list.innerHTML = '';
                document.getElementById('no-sessions')?.classList.remove('hidden');
            }
        } catch (err) {
            console.error('Failed to delete session:', err);
        }
    });
}

function getCategoryDisplay(categoryName) {
    const cat = (state.sessionCategories || []).find(c => c.name === categoryName);
    return cat ? `${cat.emoji} ${cat.name}` : categoryName;
}

function getCategoryColor(categoryName) {
    const colors = ['#6366f1', '#22c55e', '#f59e0b', '#ef4444', '#a855f7', '#06b6d4', '#ec4899', '#84cc16'];
    const categories = state.sessionCategories || [];
    const idx = categories.findIndex(c => c.name === categoryName);
    return colors[idx >= 0 ? idx % colors.length : Math.abs(categoryName.charCodeAt(0)) % colors.length];
}

function renderSessionHistogram(sessions, startDate, endDate) {
    const canvas = document.getElementById('session-histogram');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const container = canvas.parentElement;
    const rect = container.getBoundingClientRect();

    const dpr = window.devicePixelRatio || 1;
    const logicalWidth = rect.width - 16;
    const logicalHeight = 200;
    canvas.width = logicalWidth * dpr;
    canvas.height = logicalHeight * dpr;
    canvas.style.width = logicalWidth + 'px';
    canvas.style.height = logicalHeight + 'px';
    ctx.scale(dpr, dpr);

    ctx.clearRect(0, 0, logicalWidth, logicalHeight);

    const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
    const gridColor = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.08)';
    const labelColor = isDark ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.5)';

    // Generate all days in range
    const days = [];
    const d = new Date(startDate);
    d.setHours(0, 0, 0, 0);
    const end = new Date(endDate);
    end.setHours(23, 59, 59);
    while (d <= end) {
        days.push(new Date(d));
        d.setDate(d.getDate() + 1);
    }

    if (days.length === 0) return;

    // Group sessions by day and category
    const dayData = days.map(day => {
        const dayStart = day.getTime() / 1000;
        const dayEnd = dayStart + 86400;
        const daySessions = sessions.filter(s => s.start_time >= dayStart && s.start_time < dayEnd);

        const categoryHours = {};
        for (const s of daySessions) {
            const cat = s.category || t('session.no_category');
            const duration = (s.end_time || Math.floor(Date.now() / 1000)) - s.start_time;
            categoryHours[cat] = (categoryHours[cat] || 0) + duration / 3600;
        }
        return { date: day, categoryHours, totalHours: Object.values(categoryHours).reduce((a, b) => a + b, 0) };
    });

    const maxHours = Math.max(...dayData.map(d => d.totalHours), 1);
    const padding = { top: 10, right: 20, bottom: 30, left: 40 };
    const width = logicalWidth - padding.left - padding.right;
    const height = logicalHeight - padding.top - padding.bottom;
    const barWidth = Math.max(4, Math.min(40, (width / days.length) - 2));
    const barGap = (width - barWidth * days.length) / (days.length + 1);

    // Y-axis grid
    ctx.font = '10px system-ui';
    ctx.fillStyle = labelColor;
    ctx.strokeStyle = gridColor;
    ctx.lineWidth = 1;

    const ySteps = Math.min(5, Math.ceil(maxHours));
    for (let i = 0; i <= ySteps; i++) {
        const val = (maxHours / ySteps) * i;
        const y = padding.top + height - (val / maxHours) * height;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(padding.left + width, y);
        ctx.stroke();
        ctx.textAlign = 'right';
        ctx.fillText(`${val.toFixed(1)}h`, padding.left - 4, y + 3);
    }

    // Collect unique categories for legend
    const allCategories = new Set();

    // Draw bars
    dayData.forEach((day, i) => {
        const x = padding.left + barGap + i * (barWidth + barGap);
        let yOffset = 0;

        const sortedCats = Object.entries(day.categoryHours).sort((a, b) => b[1] - a[1]);
        for (const [cat, hours] of sortedCats) {
            allCategories.add(cat);
            const barH = (hours / maxHours) * height;
            const y = padding.top + height - yOffset - barH;

            ctx.fillStyle = getCategoryColor(cat);
            ctx.beginPath();
            // Rounded top corners
            const r = Math.min(3, barWidth / 2);
            ctx.moveTo(x, y + barH);
            ctx.lineTo(x, y + r);
            ctx.quadraticCurveTo(x, y, x + r, y);
            ctx.lineTo(x + barWidth - r, y);
            ctx.quadraticCurveTo(x + barWidth, y, x + barWidth, y + r);
            ctx.lineTo(x + barWidth, y + barH);
            ctx.fill();

            yOffset += barH;
        }

        // X-axis label (show every Nth label to avoid crowding)
        const showLabel = days.length <= 14 || i % Math.ceil(days.length / 14) === 0;
        if (showLabel) {
            ctx.fillStyle = labelColor;
            ctx.textAlign = 'center';
            const dateStr = `${day.date.getMonth() + 1}/${day.date.getDate()}`;
            ctx.fillText(dateStr, x + barWidth / 2, logicalHeight - 4);
        }
    });

}

function drawHistoryChart() {
    const canvas = document.getElementById('history-chart');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const container = canvas.parentElement;
    const rect = container.getBoundingClientRect();

    const dpr = window.devicePixelRatio || 1;
    const logicalWidth = rect.width - 32;
    const logicalHeight = rect.height - 64; // Account for chart header
    canvas.width = logicalWidth * dpr;
    canvas.height = logicalHeight * dpr;
    canvas.style.width = logicalWidth + 'px';
    canvas.style.height = logicalHeight + 'px';
    ctx.scale(dpr, dpr);

    const data = state.historyData;
    if (data.length === 0) return;

    const hasCostData = data.some(d => d.total_cost != null && d.total_cost > 0);
    const rightPad = hasCostData ? 60 : 20;
    const padding = { top: 20, right: rightPad, bottom: 40, left: 60 };
    const width = logicalWidth - padding.left - padding.right;
    const height = logicalHeight - padding.top - padding.bottom;

    ctx.clearRect(0, 0, logicalWidth, logicalHeight);

    const maxWh = Math.max(...data.map(d => d.total_wh)) * 1.1 || 100;

    // Grid lines and left Y-axis (energy)
    const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
    const gridColor = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.08)';
    const labelColor = isDark ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.5)';

    ctx.strokeStyle = gridColor;
    ctx.lineWidth = 1;

    for (let i = 0; i <= 5; i++) {
        const y = padding.top + (height / 5) * i;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(padding.left + width, y);
        ctx.stroke();

        const value = maxWh - (maxWh / 5) * i;
        ctx.fillStyle = labelColor;
        ctx.font = '11px system-ui';
        ctx.textAlign = 'right';
        ctx.fillText(formatNumber(value / 1000, 1) + ' kWh', padding.left - 8, y + 4);
    }

    // Right Y-axis labels (cost) if cost data exists
    let maxCost = 0;
    if (hasCostData) {
        maxCost = Math.max(...data.map(d => d.total_cost || 0)) * 1.1 || 1;
        for (let i = 0; i <= 5; i++) {
            const y = padding.top + (height / 5) * i;
            const value = maxCost - (maxCost / 5) * i;
            ctx.fillStyle = 'rgba(34, 197, 94, 0.6)';
            ctx.font = '11px system-ui';
            ctx.textAlign = 'left';
            ctx.fillText(state.currencySymbol + formatNumber(value, 2), padding.left + width + 8, y + 4);
        }
    }

    const barWidth = Math.min(40, (width / data.length) * 0.7);
    const barGap = (width - barWidth * data.length) / (data.length + 1);

    // Draw energy bars
    data.forEach((day, i) => {
        const x = padding.left + barGap + i * (barWidth + barGap);
        const barHeight = (day.total_wh / maxWh) * height;
        const y = padding.top + height - barHeight;

        const gradient = ctx.createLinearGradient(x, y, x, padding.top + height);
        gradient.addColorStop(0, '#6366f1');
        gradient.addColorStop(1, 'rgba(99, 102, 241, 0.3)');

        ctx.fillStyle = gradient;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, barHeight, [4, 4, 0, 0]);
        ctx.fill();

        // Date labels
        ctx.fillStyle = labelColor;
        ctx.font = '10px system-ui';
        ctx.textAlign = 'center';
        const dateLabel = day.date ? day.date.slice(5) : `Day ${i + 1}`;
        ctx.fillText(dateLabel, x + barWidth / 2, padding.top + height + 20);
    });

    // Cost line overlay
    if (hasCostData && data.length > 1) {
        ctx.strokeStyle = '#22c55e';
        ctx.lineWidth = 2;
        ctx.setLineDash([]);
        ctx.beginPath();

        data.forEach((day, i) => {
            const x = padding.left + barGap + i * (barWidth + barGap) + barWidth / 2;
            const costY = padding.top + height - ((day.total_cost || 0) / maxCost) * height;
            if (i === 0) ctx.moveTo(x, costY);
            else ctx.lineTo(x, costY);
        });
        ctx.stroke();

        // Cost dots
        data.forEach((day, i) => {
            const x = padding.left + barGap + i * (barWidth + barGap) + barWidth / 2;
            const costY = padding.top + height - ((day.total_cost || 0) / maxCost) * height;
            ctx.fillStyle = '#22c55e';
            ctx.beginPath();
            ctx.arc(x, costY, 3, 0, Math.PI * 2);
            ctx.fill();
        });
    } else {
        // No cost data - show energy average line instead
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

        ctx.fillStyle = '#22c55e';
        ctx.font = '11px system-ui';
        ctx.textAlign = 'left';
        ctx.fillText(`Avg: ${formatNumber(avgWh / 1000, 2)} kWh`, padding.left + 5, avgY - 5);
    }
}

// ===== Settings =====
function setupSettings() {
    const pricingModeSelect = document.getElementById('setting-pricing-mode');
    pricingModeSelect.addEventListener('change', () => {
        updatePricingModeUI(pricingModeSelect.value);
    });

    document.getElementById('save-settings').addEventListener('click', saveSettings);
    document.getElementById('reset-settings').addEventListener('click', async () => {
        state.config = await invoke('get_config');
        applyConfig(state.config);
    });

    document.getElementById('toggle-widget-btn').addEventListener('click', async () => {
        try {
            const isOpen = await invoke('toggle_widget');
            const btn = document.getElementById('toggle-widget-btn');
            btn.textContent = isOpen ? t('settings.widget.close') : t('settings.widget.open');
        } catch (error) {
            console.error('Widget toggle error:', error);
        }
    });

    document.getElementById('detect-baseline-btn').addEventListener('click', async () => {
        try {
            const detection = await invoke('detect_baseline');
            if (detection) {
                document.getElementById('detected-baseline').textContent =
                    `${formatNumber(detection.detected_watts, 1)} W (${Math.round(detection.confidence * 100)}% confidence)`;
                showToast(`${t('settings.baseline.detected_value')}: ${formatNumber(detection.detected_watts, 1)} W`, 'success');
            } else {
                showToast(t('settings.baseline.not_enough_data'), 'info');
            }
        } catch (error) {
            console.error('Baseline detection error:', error);
            showToast(t('settings.baseline.detect_failed'), 'error');
        }
    });

    document.getElementById('setting-baseline-auto').addEventListener('change', (e) => {
        document.getElementById('manual-baseline-row').style.display = e.target.checked ? 'none' : 'flex';
    });
}

function applyConfig(config) {
    document.getElementById('setting-language').value = config.general.language;
    document.getElementById('setting-theme').value = config.general.theme;
    document.getElementById('setting-refresh-rate').value = config.general.refresh_rate_ms;
    document.getElementById('setting-slow-refresh-rate').value = config.general.slow_refresh_rate_ms || 5000;
    document.getElementById('setting-eco-mode').checked = config.general.eco_mode;
    document.getElementById('setting-start-minimized').checked = config.general.start_minimized || false;
    document.getElementById('setting-start-with-system').checked = config.general.start_with_system || false;

    document.getElementById('setting-baseline-auto').checked = config.advanced.baseline_auto;
    document.getElementById('setting-baseline-watts').value = config.advanced.baseline_watts;
    document.getElementById('setting-process-limit').value = config.advanced.process_list_limit || 10;
    document.getElementById('manual-baseline-row').style.display = config.advanced.baseline_auto ? 'none' : 'flex';

    document.getElementById('setting-pricing-mode').value = config.pricing.mode;
    document.getElementById('setting-currency').value = config.pricing.currency;
    document.getElementById('setting-rate-kwh').value = config.pricing.simple.rate_per_kwh;
    document.getElementById('setting-peak-rate').value = config.pricing.peak_offpeak.peak_rate;
    document.getElementById('setting-offpeak-rate').value = config.pricing.peak_offpeak.offpeak_rate;
    document.getElementById('setting-offpeak-start').value = config.pricing.peak_offpeak.offpeak_start;
    document.getElementById('setting-offpeak-end').value = config.pricing.peak_offpeak.offpeak_end;

    if (config.pricing.seasonal) {
        document.getElementById('setting-summer-rate').value = config.pricing.seasonal.summer_rate;
        document.getElementById('setting-winter-rate').value = config.pricing.seasonal.winter_rate;
    }

    if (config.pricing.tempo) {
        document.getElementById('setting-tempo-blue-peak').value = config.pricing.tempo.blue_peak;
        document.getElementById('setting-tempo-blue-offpeak').value = config.pricing.tempo.blue_offpeak;
        document.getElementById('setting-tempo-white-peak').value = config.pricing.tempo.white_peak;
        document.getElementById('setting-tempo-white-offpeak').value = config.pricing.tempo.white_offpeak;
        document.getElementById('setting-tempo-red-peak').value = config.pricing.tempo.red_peak;
        document.getElementById('setting-tempo-red-offpeak').value = config.pricing.tempo.red_offpeak;
    }

    document.getElementById('setting-widget-show-cost').checked = config.widget.show_cost;
    document.getElementById('setting-widget-position').value = config.widget.position;

    document.documentElement.setAttribute('data-theme', config.general.theme);
    state.currencySymbol = config.pricing.currency_symbol;
    updatePricingModeUI(config.pricing.mode);
}

function updatePricingModeUI(mode) {
    document.querySelectorAll('.pricing-mode-config').forEach(el => el.classList.add('hidden'));
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
        const newStartWithSystem = document.getElementById('setting-start-with-system').checked;
        const oldStartWithSystem = state.config?.general?.start_with_system || false;

        const config = {
            general: {
                language: document.getElementById('setting-language').value,
                theme: document.getElementById('setting-theme').value,
                refresh_rate_ms: parseInt(document.getElementById('setting-refresh-rate').value),
                slow_refresh_rate_ms: parseInt(document.getElementById('setting-slow-refresh-rate').value),
                eco_mode: document.getElementById('setting-eco-mode').checked,
                start_minimized: document.getElementById('setting-start-minimized').checked,
                start_with_system: newStartWithSystem,
            },
            pricing: {
                mode: document.getElementById('setting-pricing-mode').value,
                currency: document.getElementById('setting-currency').value,
                currency_symbol: getCurrencySymbol(document.getElementById('setting-currency').value),
                simple: { rate_per_kwh: parseFloat(document.getElementById('setting-rate-kwh').value) },
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
                display_items: state.config?.widget?.display_items || ['power', 'cost'],
                size: state.config?.widget?.size || 'normal',
                theme: state.config?.widget?.theme || 'default',
            },
            advanced: {
                baseline_watts: parseFloat(document.getElementById('setting-baseline-watts').value) || 0,
                baseline_auto: document.getElementById('setting-baseline-auto').checked,
                active_profile: state.config?.advanced?.active_profile || 'default',
                pinned_processes: state.config?.advanced?.pinned_processes || [],
                process_list_limit: parseInt(document.getElementById('setting-process-limit').value) || 10,
                extended_metrics_threshold: state.config?.advanced?.extended_metrics_threshold || 15.0,
                session_categories: state.sessionCategories || state.config?.advanced?.session_categories || [],
            },
            dashboard: state.dashboardConfig || state.config?.dashboard,
        };

        await invoke('set_config', { config });

        // Update autostart setting if it changed
        if (newStartWithSystem !== oldStartWithSystem) {
            try {
                await invoke('set_autostart', { enabled: newStartWithSystem });
            } catch (autoErr) {
                console.error('Failed to set autostart:', autoErr);
                // Don't fail the whole save if autostart fails
            }
        }

        state.config = config;
        state.currencySymbol = config.pricing.currency_symbol;
        restartDashboardUpdates();
        document.documentElement.setAttribute('data-theme', config.general.theme);
        await loadTranslations();
        renderDashboard();
        showToast(t('settings.saved') || 'Settings saved successfully', 'success');
    } catch (error) {
        console.error('Save settings error:', error);
        showToast(t('error.save_failed') || 'Failed to save settings', 'error');
    }
}

function getCurrencySymbol(currency) {
    return { 'EUR': '\u20AC', 'USD': '$', 'GBP': '\u00A3', 'CHF': 'CHF' }[currency] || currency;
}

// ===== Session Categories =====
async function loadSessionCategories() {
    try {
        state.sessionCategories = await invoke('get_session_categories');
    } catch (error) {
        console.error('Failed to load categories:', error);
        state.sessionCategories = [];
    }
}

function setupCategorySettings() {
    renderCategorySettings();

    const addBtn = document.getElementById('add-category-btn');
    if (addBtn) {
        addBtn.addEventListener('click', addCategory);
    }
}

function renderCategorySettings() {
    const list = document.getElementById('category-list');
    if (!list) return;

    const categories = state.sessionCategories || [];
    list.innerHTML = categories.map(c => `
        <div class="category-item">
            <span class="category-emoji">${c.emoji}</span>
            <span class="category-name">${c.name}</span>
            <button class="btn btn-icon btn-sm category-delete-btn" data-name="${c.name}" title="${t('settings.categories.delete')}">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
                    <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
            </button>
        </div>
    `).join('');

    // Wire up delete buttons
    list.querySelectorAll('.category-delete-btn').forEach(btn => {
        btn.addEventListener('click', () => removeCategory(btn.dataset.name));
    });
}

async function addCategory() {
    const emojiInput = document.getElementById('category-emoji-input');
    const nameInput = document.getElementById('category-name-input');
    const emoji = emojiInput.value.trim();
    const name = nameInput.value.trim();

    if (!emoji || !name) return;

    try {
        state.sessionCategories = await invoke('add_session_category', { category: { emoji, name } });
        emojiInput.value = '';
        nameInput.value = '';
        renderCategorySettings();
    } catch (error) {
        console.error('Failed to add category:', error);
    }
}

async function removeCategory(name) {
    try {
        state.sessionCategories = await invoke('remove_session_category', { name });
        renderCategorySettings();
    } catch (error) {
        console.error('Failed to remove category:', error);
    }
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
    toast.innerHTML = `${icons[type] || icons.info}<span class="toast-message">${message}</span>`;
    container.appendChild(toast);
    setTimeout(() => toast.remove(), 3000);
}

// ===== Utility Functions =====
function formatNumber(num, decimals = 2) {
    if (num === null || num === undefined || isNaN(num)) return '--';
    return num.toFixed(decimals);
}

function formatDuration(seconds) {
    if (!seconds || isNaN(seconds)) return '--:--:--';
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    return `${hrs.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

function formatDate(date) {
    return date.toISOString().split('T')[0];
}

// ===== Stream A: Display Mode Components =====

/**
 * Renders a radial progress SVG component
 * @param {number} percent - Progress percentage (0-100)
 * @param {string} label - Label to display in center
 * @param {string} color - Stroke color for the progress arc
 * @returns {string} SVG markup
 */
function renderRadialProgress(percent, label, color = '#6366f1') {
    const radius = 36;
    const strokeWidth = 6;
    const circumference = 2 * Math.PI * radius;
    const offset = circumference - (percent / 100) * circumference;

    return `
        <svg class="radial-progress" viewBox="0 0 100 100">
            <circle
                class="radial-bg"
                cx="50"
                cy="50"
                r="${radius}"
                fill="none"
                stroke="rgba(255,255,255,0.1)"
                stroke-width="${strokeWidth}"
            />
            <circle
                class="radial-fill"
                cx="50"
                cy="50"
                r="${radius}"
                fill="none"
                stroke="${color}"
                stroke-width="${strokeWidth}"
                stroke-linecap="round"
                stroke-dasharray="${circumference}"
                stroke-dashoffset="${offset}"
                transform="rotate(-90 50 50)"
            />
            <text x="50" y="45" text-anchor="middle" class="radial-value">${formatNumber(percent, 0)}%</text>
            <text x="50" y="62" text-anchor="middle" class="radial-label">${label}</text>
        </svg>
    `;
}

/**
 * Renders a single vertical charge bar (battery-style meter)
 * @param {number} value - Current value
 * @param {number} max - Maximum value for the scale
 * @param {string} label - Value label below the bar (e.g. "58°", "3.2G")
 * @param {string} color - Fill color
 * @param {string} name - Metric name above the bar (e.g. "TEMP", "CLK")
 * @returns {string} HTML markup
 */
function renderChargeBar(value, max, label, color, name) {
    const percent = Math.min(100, Math.max(0, (value / max) * 100));
    const plainLabel = label.replace(/<[^>]*>/g, '');
    return `
        <div class="charge-bar" title="${name}: ${plainLabel}">
            <span class="charge-bar-name" style="color:${color}">${name}</span>
            <div class="charge-bar-track">
                <div class="charge-bar-fill" style="height:${percent}%;background:${color}"></div>
            </div>
            <span class="charge-bar-label">${label}</span>
        </div>
    `;
}

/**
 * Renders a container of multiple charge bars
 * @param {Array<{value: number, max: number, label: string, color: string, name: string}>} bars
 * @returns {string} HTML markup
 */
function renderChargeBars(bars) {
    if (!bars || bars.length === 0) return '';
    return `
        <div class="charge-bars">
            ${bars.map(b => renderChargeBar(b.value, b.max, b.label, b.color, b.name)).join('')}
        </div>
    `;
}

/**
 * Draws a mini sparkline chart on a canvas - auto-fits to container
 * @param {string} canvasId - Canvas element ID
 * @param {number[]} historyArray - Array of values to plot
 * @param {string} color - Line color (hex format)
 */
function renderMiniChart(canvasId, historyArray, color = '#6366f1') {
    const canvas = document.getElementById(canvasId);
    if (!canvas || historyArray.length < 2) return;

    const ctx = canvas.getContext('2d');

    // Auto-fit to widget card body
    const cardBody = canvas.closest('.card-body');
    if (!cardBody) return;

    const bodyRect = cardBody.getBoundingClientRect();
    const chartContainer = canvas.closest('.mini-chart-container');
    const header = chartContainer?.querySelector('.mini-chart-header');
    const headerHeight = header ? header.offsetHeight : 0;
    const metricInfo = cardBody.querySelector('.metric-info');
    const metricInfoHeight = metricInfo ? metricInfo.offsetHeight + 8 : 0;

    // Fill available space in card body
    const dpr = window.devicePixelRatio || 1;
    const logicalWidth = Math.max(bodyRect.width - 16, 100);
    const logicalHeight = Math.max(bodyRect.height - headerHeight - metricInfoHeight - 16, 60);
    canvas.width = logicalWidth * dpr;
    canvas.height = logicalHeight * dpr;
    canvas.style.width = logicalWidth + 'px';
    canvas.style.height = logicalHeight + 'px';
    ctx.scale(dpr, dpr);

    const data = historyArray;
    const padding = { top: 8, right: 8, bottom: 8, left: 8 };
    const width = logicalWidth - padding.left - padding.right;
    const height = logicalHeight - padding.top - padding.bottom;

    // Use 0-100 range for percentage-based metrics
    const maxVal = 100;
    const minVal = 0;
    const range = maxVal - minVal;

    ctx.clearRect(0, 0, logicalWidth, logicalHeight);

    // Parse hex color to rgba
    const hexToRgba = (hex, alpha) => {
        const r = parseInt(hex.slice(1, 3), 16);
        const g = parseInt(hex.slice(3, 5), 16);
        const b = parseInt(hex.slice(5, 7), 16);
        return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    };

    // Draw gradient fill
    const gradient = ctx.createLinearGradient(0, padding.top, 0, height + padding.top);
    gradient.addColorStop(0, hexToRgba(color, 0.4));
    gradient.addColorStop(1, hexToRgba(color, 0.05));

    ctx.beginPath();
    ctx.moveTo(padding.left, height + padding.top);
    data.forEach((val, i) => {
        const x = padding.left + (i / (data.length - 1)) * width;
        const y = padding.top + height - ((val - minVal) / range) * height;
        ctx.lineTo(x, y);
    });
    ctx.lineTo(padding.left + width, height + padding.top);
    ctx.closePath();
    ctx.fillStyle = gradient;
    ctx.fill();

    // Draw line with glow effect
    ctx.shadowColor = color;
    ctx.shadowBlur = 4;
    ctx.beginPath();
    data.forEach((val, i) => {
        const x = padding.left + (i / (data.length - 1)) * width;
        const y = padding.top + height - ((val - minVal) / range) * height;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.shadowBlur = 0;

    // Draw endpoint dot with pulse effect
    if (data.length > 0) {
        const lastVal = data[data.length - 1];
        const x = padding.left + width;
        const y = padding.top + height - ((lastVal - minVal) / range) * height;

        // Outer glow
        ctx.beginPath();
        ctx.arc(x, y, 6, 0, Math.PI * 2);
        ctx.fillStyle = hexToRgba(color, 0.3);
        ctx.fill();

        // Inner dot
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
    }

    // Draw current value indicator line
    if (data.length > 0) {
        const lastVal = data[data.length - 1];
        const y = padding.top + height - ((lastVal - minVal) / range) * height;
        ctx.setLineDash([2, 2]);
        ctx.strokeStyle = hexToRgba(color, 0.5);
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(padding.left + width - 10, y);
        ctx.stroke();
        ctx.setLineDash([]);
    }
}

/**
 * Updates CPU/GPU/RAM history arrays from system metrics
 * @param {object} systemMetrics - System metrics object
 */
function updateMetricsHistory(systemMetrics) {
    if (!systemMetrics) return;

    const maxPoints = 60;

    // CPU history
    if (systemMetrics.cpu?.usage_percent != null) {
        state.cpuHistory.push(systemMetrics.cpu.usage_percent);
        if (state.cpuHistory.length > maxPoints) state.cpuHistory.shift();
    }

    // GPU history
    if (systemMetrics.gpu?.usage_percent != null) {
        state.gpuHistory.push(systemMetrics.gpu.usage_percent);
        if (state.gpuHistory.length > maxPoints) state.gpuHistory.shift();
    }

    // RAM history
    if (systemMetrics.memory?.usage_percent != null) {
        state.ramHistory.push(systemMetrics.memory.usage_percent);
        if (state.ramHistory.length > maxPoints) state.ramHistory.shift();
    }
}

/**
 * Draws mini charts for widgets in chart display mode
 */
function drawMiniCharts() {
    // Get widget display modes
    const widgets = state.dashboardConfig?.widgets || [];

    for (const widget of widgets) {
        if (!widget.visible) continue;

        if (widget.id === 'cpu' && widget.display_mode === 'chart' && state.cpuHistory.length > 1) {
            renderMiniChart('cpu-mini-chart', state.cpuHistory, '#6366f1');
        }
        if (widget.id === 'gpu' && widget.display_mode === 'chart' && state.gpuHistory.length > 1) {
            renderMiniChart('gpu-mini-chart', state.gpuHistory, '#22c55e');
        }
        if (widget.id === 'ram' && widget.display_mode === 'chart' && state.ramHistory.length > 1) {
            renderMiniChart('ram-mini-chart', state.ramHistory, '#f59e0b');
        }
    }
}

/**
 * Formats time as HH:MM for chart axis
 * @param {number} timestamp - Unix timestamp in milliseconds
 * @returns {string} Formatted time string
 */
function formatTimeHHMM(timestamp) {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, '0');
    const mins = date.getMinutes().toString().padStart(2, '0');
    return `${hours}:${mins}`;
}

/**
 * Cycles global display mode through normal -> minimize -> normal
 */
async function cycleGlobalDisplay() {
    const modes = ['normal', 'minimize'];
    const current = state.dashboardConfig?.global_display || 'normal';
    const currentIndex = modes.indexOf(current);
    const nextIndex = (currentIndex + 1) % modes.length;
    const nextMode = modes[nextIndex];

    state.dashboardConfig.global_display = nextMode;
    await saveDashboardConfigQuiet();

    // Update toggle button state
    updateGlobalDisplayToggle(nextMode);

    showToast(`${t('dashboard.display_mode')}: ${t('dashboard.mode.' + nextMode) || nextMode}`, 'info');
}

/**
 * Sets global display mode
 * @param {string} mode - 'normal' or 'minimize'
 */
async function setGlobalDisplay(mode) {
    state.dashboardConfig.global_display = mode;
    await saveDashboardConfigQuiet();
    updateGlobalDisplayToggle(mode);
}

/**
 * Updates the global display toggle button UI
 * @param {string} mode - Current display mode
 */
function updateGlobalDisplayToggle(mode) {
    const buttons = document.querySelectorAll('.global-display-btn');
    buttons.forEach(btn => {
        btn.classList.toggle('active', btn.dataset.mode === mode);
    });
}

// ===== Window resize handler =====
let resizeReflowTimer = null;
window.addEventListener('resize', () => {
    drawPowerGraph();
    if (state.historyData.length > 0) {
        drawHistoryChart();
    }
    drawMiniCharts();

    // Debounced dashboard grid reflow on column count change
    clearTimeout(resizeReflowTimer);
    resizeReflowTimer = setTimeout(reflowDashboardGrid, 150);
});
