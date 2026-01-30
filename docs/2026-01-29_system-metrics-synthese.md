---
title: "Capture de métriques système - Synthèse et recommandations"
date: 2026-01-29
topic: system-metrics/synthesis
sources:
  - 2026-01-29_system-metrics-windows.md
  - 2026-01-29_system-metrics-linux.md
  - 2026-01-29_system-metrics-gpu-crossplatform.md
status: final
---

# Synthèse : Capture de métriques système pour application desktop Windows/Linux

> Ce document synthétise les recherches détaillées sur les APIs Windows, Linux et GPU/cross-platform pour la capture de métriques hardware. Il fournit un comparatif global, une architecture recommandée et un guide de décision par métrique.

## Table des matières

1. [Vue d'ensemble du problème](#1-vue-densemble-du-problème)
2. [Matrice métrique × méthode × OS](#2-matrice-métrique--méthode--os)
3. [Comparatif des approches par métrique](#3-comparatif-des-approches-par-métrique)
4. [Niveaux de privilèges et compromis](#4-niveaux-de-privilèges-et-compromis)
5. [Architecture recommandée](#5-architecture-recommandée)
6. [Choix du langage et stack technique](#6-choix-du-langage-et-stack-technique)
7. [Métriques difficiles : solutions et workarounds](#7-métriques-difficiles--solutions-et-workarounds)
8. [Tableau de décision rapide](#8-tableau-de-décision-rapide)
9. [Pièges et gotchas](#9-pièges-et-gotchas)
10. [Conclusion](#10-conclusion)

---

## 1. Vue d'ensemble du problème

### Le défi fondamental

La capture de métriques hardware est **fragmentée à trois niveaux** :

1. **Par OS** : Windows et Linux exposent les données via des mécanismes totalement différents (WMI/PDH vs sysfs/proc)
2. **Par vendor GPU** : NVIDIA (NVML/NVAPI), AMD (ADL/ADLX/sysfs), Intel (Level Zero/i915) — aucun standard commun
3. **Par niveau de profondeur** : les métriques "faciles" (CPU usage, RAM) sont accessibles sans privilèges, mais les métriques "profondes" (fréquence réelle, fan RPM, power, latence) nécessitent des drivers kernel ou des privilèges root/admin

### Cartographie des difficultés

```
Difficulté      Métriques
─────────────   ──────────────────────────────────────────
█ Facile        CPU usage/core, RAM usage totale/dispo
██ Moyenne      CPU freq (P-state), GPU utilisation, VRAM, GPU temp
███ Difficile   CPU freq réelle (boost), CPU temp/core, fan RPM, GPU clock, power draw
████ Très dur   RAM timings (CAS/tRCD), RAM bandwidth, latence mémoire, fan RPM GPU (NVIDIA)
```

---

## 2. Matrice métrique × méthode × OS

### CPU

| Métrique | Windows (User) | Windows (Admin) | Linux (User) | Linux (Root) |
|----------|---------------|-----------------|-------------|-------------|
| **Utilisation/core (%)** | PDH, WMI, NtQuerySysInfo | idem | `/proc/stat` | idem |
| **Fréquence P-state** | PDH, CallNtPowerInfo | idem | sysfs `scaling_cur_freq` | idem |
| **Fréquence réelle (boost)** | — | LHM, MSR (APERF/MPERF) | — | MSR, `cpuinfo_cur_freq` |
| **Température/core** | — | LHM, OHM, MSR | — | hwmon, libsensors, MSR |
| **Power (package W)** | — | LHM, Intel PCM | RAPL powercap* | RAPL MSR, perf_event |

> \* RAPL powercap : accessible user avant kernel 5.10, root requis depuis.

### RAM

| Métrique | Windows (User) | Windows (Admin) | Linux (User) | Linux (Root) |
|----------|---------------|-----------------|-------------|-------------|
| **Usage totale/dispo** | GlobalMemoryStatusEx | idem | `/proc/meminfo` | idem |
| **Fréquence (MHz)** | WMI Win32_PhysicalMemory | idem | — | `dmidecode --type 17` |
| **Timings (CAS, etc.)** | — | SMBus/SPD via LHM | — | `decode-dimms` (i2c-tools) |
| **Bandwidth** | — | Intel PCM | — | `perf stat` (compteurs uncore) |
| **Latence** | — | Intel MLC (benchmark) | — | Intel MLC, `perf mem` |

### GPU

| Métrique | NVIDIA (Win+Lin) | AMD Windows | AMD Linux | Intel Windows | Intel Linux |
|----------|-----------------|------------|----------|--------------|------------|
| **Clock core** | NVML | ADL / ADLX | sysfs `pp_dpm_sclk` | IGCL | sysfs i915 |
| **Clock mémoire** | NVML | ADL / ADLX | sysfs `pp_dpm_mclk` | IGCL | sysfs i915 |
| **Utilisation (%)** | NVML | ADLX | sysfs `gpu_busy_percent` | Level Zero* | intel_gpu_top |
| **VRAM** | NVML | ADLX | sysfs `mem_info_*` | Level Zero | — |
| **Température** | NVML | ADL / ADLX | hwmon | Level Zero | hwmon |
| **Fan speed (%)** | NVML | ADL / ADLX | hwmon | Level Zero | — |
| **Fan speed (RPM)** | NVAPI(Win)/XNVCtrl(Lin) | ADL / ADLX | hwmon | Level Zero | — |
| **Power draw (W)** | NVML | ADLX | hwmon | Level Zero | — |
| **PCIe BW** | NVML | — | AMD SMI | Level Zero | — |
| **Encoder/Decoder** | NVML | — | — | — | — |

> \* Level Zero Sysman : complexe à mettre en place (metric groups).

### Ventilateurs & carte mère

| Métrique | Windows (User) | Windows (Admin) | Linux (User) | Linux (Root) |
|----------|---------------|-----------------|-------------|-------------|
| **Fan RPM (CPU/chassis)** | — | LHM (Super I/O) | hwmon sysfs | IPMI |
| **Fan contrôle (set %)** | — | LHM | hwmon pwm | IPMI |
| **Temp VRM/chipset** | — | LHM (Super I/O) | hwmon | IPMI |
| **Voltages** | — | LHM (Super I/O) | hwmon, libsensors | IPMI |

---

## 3. Comparatif des approches par métrique

### 3.1 CPU Utilisation par cœur

| | Windows | Linux |
|---|---------|-------|
| **Méthode recommandée** | PDH (`\Processor(N)\% Processor Time`) | `/proc/stat` (calcul delta) |
| **Alternative** | WMI `Win32_PerfFormattedData_PerfOS_Processor` | `sysstat` (mpstat) |
| **Privilèges** | User | User |
| **Précision** | Excellente | Excellente |
| **Overhead** | Faible | Très faible |
| **Gotcha** | WMI : lent (COM), PDH préféré | Nécessite deux lectures + calcul delta |

### 3.2 CPU Fréquence par cœur

| | Windows | Linux |
|---|---------|-------|
| **Méthode simple** | `CallNtPowerInformation` | sysfs `scaling_cur_freq` |
| **Méthode précise** | LHM via MSR APERF/MPERF (admin) | MSR `IA32_PERF_STATUS` (root) |
| **Précision simple** | P-state (pas le boost réel) | P-state (vue governor) |
| **Précision MSR** | Fréquence réelle | Fréquence réelle |
| **Gotcha Win** | `CurrentClockSpeed` WMI = valeur nominale unique par socket | |
| **Gotcha Lin** | | `scaling_cur_freq` ≠ fréquence réelle avec turbo boost |

**Verdict** : pour la fréquence *réelle* en boost, il faut des privilèges élevés sur les deux OS. La méthode APERF/MPERF (ratio de compteurs MSR) est la plus fiable.

### 3.3 CPU Température par cœur

| | Windows | Linux |
|---|---------|-------|
| **Méthode** | LHM (admin) | hwmon sysfs ou libsensors |
| **Alternative** | MSR `IA32_THERM_STATUS` | MSR direct |
| **Privilèges** | Admin (driver kernel) | User (hwmon) / Root (MSR) |
| **Gotcha Win** | `MSAcpi_ThermalZoneTemperature` = zone ACPI, PAS par cœur | |
| **Gotcha Lin** | | Dépend du driver chargé (coretemp, k10temp, zenpower) |

**Avantage Linux** : température par cœur accessible en user mode via hwmon. Sur Windows, c'est admin obligatoire via LHM.

### 3.4 GPU Métriques

| | NVIDIA | AMD | Intel |
|---|--------|-----|-------|
| **API recommandée** | NVML (cross-platform) | ADLX (Win) + sysfs (Lin) | Level Zero (cross) |
| **Facilité** | Excellente | Moyenne | Difficile |
| **Couverture** | Complète | Bonne | Partielle |
| **Fan RPM** | NVAPI (Win) / XNVCtrl (Lin) | ADL/ADLX ou hwmon | Level Zero |
| **Gotcha** | NVML fan = % intention, pas RPM | ADL legacy vs ADLX moderne | API complexe (metric groups) |

**Verdict** : NVIDIA est le mieux documenté et le plus simple. AMD nécessite deux chemins (ADLX Windows + sysfs Linux). Intel est le plus complexe avec le moins de métriques.

### 3.5 RAM Timings et latence

C'est la **métrique la plus difficile** à obtenir programmatiquement :

| Approche | OS | Privilèges | Ce qu'on obtient |
|----------|-----|-----------|------------------|
| SMBus/SPD via LHM | Windows | Admin + driver | CAS, tRCD, tRP, tRAS (JEDEC) |
| `decode-dimms` (i2c-tools) | Linux | Root | CAS, tRCD, tRP, tRAS (JEDEC) |
| MSR registres IMC | Les deux | Ring 0 | Timings configurés par le BIOS |
| `dmidecode --type 20` | Linux | Root | Fréquence uniquement |
| Intel MLC | Les deux | Root/Admin | Latence mesurée (ns) |
| `perf mem` | Linux | Root | Latence mesurée (cycles) |

**Gotcha critique** : les timings SPD sont les **valeurs JEDEC du module**, pas nécessairement ceux configurés par le BIOS (XMP/DOCP). Pour les timings réels en cours d'utilisation, il faut accéder aux registres du contrôleur mémoire (IMC) — extrêmement complexe et spécifique à chaque génération de CPU.

---

## 4. Niveaux de privilèges et compromis

### Couverture par niveau de privilège

```
                     Windows                                     Linux
                     ───────                                     ─────
User mode     ┌─────────────────────────┐              ┌─────────────────────────┐
(~40%)        │ CPU usage, CPU freq      │              │ CPU usage, CPU freq      │
              │ (P-state), RAM usage,    │              │ (P-state), RAM usage,    │
              │ GPU* via NVML/ADL        │              │ GPU via sysfs, temp,     │
              │                          │              │ fans, voltages (hwmon)   │
              └─────────────────────────┘              └─────────────────────────┘

Admin/Root    ┌─────────────────────────┐              ┌─────────────────────────┐
(~85%)        │ + Tout LHM : temp CPU,   │              │ + CPU freq réelle (MSR)  │
              │   fans, voltages, freq    │              │ + RAPL power             │
              │   réelle, power, GPU      │              │ + RAM freq (dmidecode)   │
              │ + Intel PCM (RAM BW)      │              │ + RAM timings            │
              └─────────────────────────┘              └─────────────────────────┘

Ring 0/Driver ┌─────────────────────────┐              ┌─────────────────────────┐
(~95%)        │ + MSR direct             │              │ + MSR direct             │
              │ + SMBus/SPD (RAM timing) │              │ + Compteurs uncore       │
              │ + Compteurs uncore       │              │   (perf_event, RAM BW)   │
              └─────────────────────────┘              └─────────────────────────┘
```

**Observation clé** : Linux offre plus de métriques en user mode que Windows grâce à hwmon/sysfs. Sur Windows, la plupart des métriques hardware intéressantes nécessitent LHM (admin + driver kernel).

---

## 5. Architecture recommandée

### Architecture en couches avec abstraction OS

```
┌───────────────────────────────────────────────────────────────┐
│                     Application (UI / API)                     │
│          Interface uniforme : MetricsProvider                  │
├───────────────────────────────────────────────────────────────┤
│                    Abstraction Layer                           │
│   ┌─────────────┬──────────────┬───────────────────────────┐  │
│   │ CpuMetrics   │ GpuMetrics   │ SystemMetrics             │  │
│   │ (freq, usage,│ (clock, util,│ (fans, temp, voltage,     │  │
│   │  temp, power)│  vram, temp, │  RAM details)             │  │
│   │              │  fan, power) │                           │  │
│   └──────┬───────┴──────┬───────┴──────────┬────────────────┘  │
│          │              │                  │                    │
├──────────┼──────────────┼──────────────────┼────────────────────┤
│  Backend │   Backend    │  Backend         │                    │
│  Windows │   GPU        │  Windows         │   Backend Linux    │
├──────────┼──────────────┼──────────────────┼────────────────────┤
│ PDH      │ NVML         │ LHM (admin)      │ sysfs/cpufreq     │
│ NtPower  │ NVAPI (Win)  │ GlobalMemStatus  │ /proc/stat        │
│ WMI      │ ADLX (Win)   │ WMI (RAM info)   │ /proc/meminfo     │
│ LHM      │ ADL sysfs(L) │                  │ hwmon (sensors)   │
│ Intel PCM│ Level Zero   │                  │ RAPL powercap     │
│          │ XNVCtrl (L)  │                  │ libsensors        │
│          │ i915 sysfs(L)│                  │ dmidecode         │
└──────────┴──────────────┴──────────────────┴────────────────────┘
```

### Stratégie de chargement

1. **Détection OS** → charger le backend approprié
2. **Détection GPU vendor** → `lspci` (Linux), WMI/SetupAPI (Windows)
3. **Chargement dynamique** → `dlopen`/`LoadLibrary` pour toutes les libs vendor
4. **Fallback gracieux** → si une lib est absente, désactiver les métriques correspondantes (pas de crash)
5. **Détection de privilèges** → tester les permissions et adapter les méthodes disponibles

### Mode dégradé

```
Si Admin/Root disponible :
  → Stack complet (100% des métriques)

Si User seulement :
  Windows → CPU usage (PDH) + CPU freq P-state + RAM usage + GPU (NVML/ADL)
            Manque : temp, fans, voltages, freq réelle, power

  Linux   → CPU usage + CPU freq + RAM usage + temp + fans + voltages (hwmon) + GPU
            Manque : freq réelle, power, RAM timings/BW
```

---

## 6. Choix du langage et stack technique

### Comparatif par langage

| Langage | CPU/RAM | GPU NVIDIA | GPU AMD | GPU Intel | Cross-platform | Complexité |
|---------|---------|-----------|---------|-----------|---------------|------------|
| **C/C++** | APIs natives | NVML + NVAPI | ADL/ADLX | Level Zero | Manuel | Haute |
| **Rust** | sysinfo crate | nvml-wrapper | FFI ADL + sysfs | FFI L0 + sysfs | Bon | Moyenne-Haute |
| **C#** | P/Invoke + LHM | LHM intégré | LHM intégré | LHM partiel | Windows-centré | Moyenne |
| **Python** | psutil | pynvml | amdsmi/subprocess | subprocess | Bon | Faible |
| **Go** | gopsutil | go-nvml | sysfs | sysfs | Bon | Moyenne |

### Recommandation par cas d'usage

**App desktop native (performance max)** :
- **C++ ou Rust** avec chargement dynamique des APIs vendor
- Windows : LHM comme backend principal (C# interop ou port de la logique)
- Linux : sysfs/hwmon + libsensors + RAPL

**App desktop avec UI riche (C#/.NET)** :
- **C# avec LibreHardwareMonitor** (Windows) — le plus simple et complet
- Linux : wrapper P/Invoke vers libsensors, ou subprocess vers CLI tools
- GPU : LHM gère déjà NVIDIA/AMD/Intel sur Windows

**Script/prototype rapide** :
- **Python** avec psutil + pynvml + amdsmi
- Limité mais suffisant pour 70% des besoins

**App cross-platform Rust** :
- `sysinfo` crate (CPU, RAM basics)
- `nvml-wrapper` (NVIDIA GPU)
- sysfs direct (Linux sensors, AMD GPU)
- FFI vers ADL/ADLX (AMD Windows)

---

## 7. Métriques difficiles : solutions et workarounds

### 7.1 Fréquence CPU réelle (avec boost)

**Problème** : les APIs simples retournent la fréquence P-state, pas la fréquence réelle quand le turbo boost est actif.

**Solution** :
- Lire les MSR `IA32_APERF` et `IA32_MPERF` à deux instants, calculer le ratio
- Fréquence réelle = fréquence_base × (delta_APERF / delta_MPERF)
- Nécessite admin/root + driver MSR

**Workaround sans privilèges** :
- Linux : `scaling_cur_freq` donne une approximation raisonnable avec `intel_pstate` ou `amd-pstate`
- Windows : `CallNtPowerInformation` donne la fréquence P-state demandée

### 7.2 Fan RPM GPU NVIDIA

**Problème** : `nvmlDeviceGetFanSpeed()` retourne un % (target), pas les RPM réels.

**Solutions** :
- Windows : NVAPI `NvAPI_GPU_GetCoolerSettings()` → RPM réels
- Linux : XNVCtrl `NV_CTRL_THERMAL_COOLER_SPEED` → RPM réels
- Alternative Linux : `nvidia-settings -q [fan:0]/GPUCurrentFanSpeedRPM`

### 7.3 RAM Timings (CAS, tRCD, tRP, tRAS)

**Problème** : aucune API standard pour lire les timings RAM actifs.

**Solutions** :
| Méthode | Ce qu'elle lit | Précision |
|---------|---------------|-----------|
| SPD via SMBus (LHM/decode-dimms) | Timings JEDEC du module | Profil JEDEC, pas XMP actif |
| MSR registres IMC | Timings configurés par BIOS | Vrais timings actifs |
| Thaiphoon Burner (Windows) | SPD + analyse | Complet mais propriétaire |
| CoreFreq (Linux) | Registres IMC | Vrais timings, kernel module requis |

**Recommandation** : SPD via SMBus (le plus portable) + note à l'utilisateur que ce sont les valeurs JEDEC.

### 7.4 Latence mémoire

**Problème** : pas d'API pour lire la latence mémoire en temps réel.

**Solutions** :
- **Intel MLC** (Memory Latency Checker) : benchmark intrusif, mesure la latence réelle en ns. Disponible Windows/Linux. Pas adapté au monitoring continu.
- **`perf mem`** (Linux) : échantillonne les accès mémoire et mesure la latence par accès (en cycles). Root requis. Overhead non négligeable.
- **Compteurs PMC** : compteurs hardware `MEM_LOAD_RETIRED.L3_MISS` etc. via perf_event. Donnent une latence moyenne statistique.

**Réalité** : la "latence mémoire en temps réel" n'est pas vraiment faisable sans overhead significatif. Les outils de monitoring (HWiNFO, HWMonitor) ne montrent pas cette métrique pour cette raison.

### 7.5 RAM Bandwidth utilisé

**Solutions** :
- **Intel PCM** : lecture des compteurs IMC (Integrated Memory Controller). Admin + driver. Windows et Linux.
- **`perf stat`** (Linux) : compteurs uncore `uncore_imc/data_reads/`, `uncore_imc/data_writes/`. Root requis.
- **RAPL** : le sous-domaine DRAM de RAPL donne la consommation (W), pas le bandwidth directement.

---

## 8. Tableau de décision rapide

**"Quelle méthode utiliser pour [métrique] sur [OS] ?"**

| Métrique | Windows → | Linux → |
|----------|-----------|---------|
| CPU usage/core | `PDH` | `/proc/stat` (calcul delta) |
| CPU freq/core (approx) | `CallNtPowerInformation` | sysfs `scaling_cur_freq` |
| CPU freq/core (réelle) | `LHM` (admin) | MSR APERF/MPERF (root) |
| CPU temp/core | `LHM` (admin) | hwmon sysfs (user) |
| CPU power | `LHM` (admin) | RAPL powercap (root*) |
| RAM usage | `GlobalMemoryStatusEx` | `/proc/meminfo` |
| RAM fréquence | WMI `Win32_PhysicalMemory` | `dmidecode --type 17` (root) |
| RAM timings | SMBus/SPD via `LHM` (admin) | `decode-dimms` (root) |
| RAM bandwidth | `Intel PCM` (admin) | `perf stat` uncore (root) |
| GPU clock | NVML / ADLX | NVML / sysfs amdgpu / i915 |
| GPU utilisation | NVML / ADLX | NVML / sysfs `gpu_busy_percent` |
| GPU temp | NVML / ADLX | NVML / hwmon |
| GPU fan % | NVML / ADLX | NVML / hwmon |
| GPU fan RPM | NVAPI(NVIDIA) / ADLX(AMD) | XNVCtrl(NVIDIA) / hwmon(AMD) |
| GPU power | NVML / ADLX | NVML / hwmon |
| GPU VRAM | NVML / ADLX | NVML / sysfs `mem_info_*` |
| Fan RPM (CPU/chassis) | `LHM` (admin) | hwmon sysfs (user) |
| Voltages | `LHM` (admin) | hwmon / libsensors (user) |
| Temp carte mère | `LHM` (admin) | hwmon / libsensors (user) |

---

## 9. Pièges et gotchas

### Windows

| Piège | Détail |
|-------|--------|
| **WMI `CurrentClockSpeed`** | Valeur unique par socket, pas par cœur. Souvent = fréquence nominale. |
| **WMI `Win32_Fan`** | Retourne des données vides sur 99% des machines (pas de provider OEM). |
| **MSAcpi_ThermalZoneTemperature** | Zone thermique ACPI ≠ température CPU. Souvent inexacte. |
| **WMI lenteur** | Requêtes COM, 100-500ms par requête. Inadapté au polling rapide. |
| **LHM driver WinRing0** | Bloqué par HVCI (Windows 11). Nécessite driver signé WHQL. |
| **PDH freq** | Retourne un % relatif à la fréquence max, pas une fréquence absolue. |
| **ETW** | API asynchrone complexe, pas de données directes de fréquence. |

### Linux

| Piège | Détail |
|-------|--------|
| **`scaling_cur_freq`** | Vue du governor, peut différer de la fréquence réelle (turbo). |
| **RAPL post-kernel 5.10** | Permissions restreintes en user mode. Nécessite udev rule ou root. |
| **hwmon numérotation** | Les hwmonN changent entre reboots. Toujours vérifier le fichier `name`. |
| **`/proc/stat` CPU usage** | Nécessite deux lectures et calcul du delta. Pas une valeur instantanée. |
| **dmidecode** | Fréquence SPD ≠ fréquence XMP/DOCP configurée dans le BIOS. |
| **perf_event paranoid** | `/proc/sys/kernel/perf_event_paranoid` ≥ 2 bloque les compteurs en user. |

### GPU

| Piège | Détail |
|-------|--------|
| **NVML fan speed** | Retourne un % (intention), pas les RPM. Utiliser NVAPI/XNVCtrl pour RPM. |
| **ADL vs ADLX** | ADL = legacy (deprecated mais toujours utilisé). ADLX = moderne (C++). APIs incompatibles. |
| **AMD sysfs pp_dpm_sclk** | Liste les P-states disponibles avec indicateur actif. Pas une valeur directe en MHz. |
| **Intel Level Zero** | API complexe (metric groups, streamer sessions). Overkill pour du simple monitoring. |
| **GPU absent** | Toujours charger les libs GPU dynamiquement. Un PC peut n'avoir qu'un iGPU ou aucun GPU dédié. |

---

## 10. Conclusion

### Résumé des recommandations

1. **Architecture en couches** : séparer les métriques par niveau de privilège et offrir un mode dégradé

2. **Stack minimal viable** (couvre ~80% des besoins) :
   - Windows : `PDH` + `GlobalMemoryStatusEx` + `NVML` (user mode)
   - Linux : `/proc/stat` + `/proc/meminfo` + sysfs cpufreq + hwmon + `NVML` (user mode)

3. **Stack complet** (admin/root requis) :
   - Windows : `LHM` + `NVML`/`NVAPI` + `ADLX` + `Intel PCM`
   - Linux : sysfs/proc + hwmon + libsensors + RAPL + MSR + `NVML` + sysfs amdgpu + dmidecode

4. **GPU** : NVML est l'API la mieux conçue. Pour AMD, prévoyez deux chemins (ADLX Windows + sysfs Linux). Intel GPU est le plus limité.

5. **Métriques impossibles sans privilèges** : fréquence CPU réelle (boost), power draw CPU, RAM timings, RAM bandwidth, latence mémoire

6. **Métriques quasi-impossibles en temps réel** : latence mémoire (seulement via benchmark ou sampling statistique)

### Documents de référence

Pour les détails complets de chaque API avec exemples de code :

- **Windows** : `2026-01-29_system-metrics-windows.md` — 12 méthodes détaillées (WMI, PDH, LHM, MSR, ETW, Intel PCM, etc.)
- **Linux** : `2026-01-29_system-metrics-linux.md` — 12 méthodes détaillées (sysfs, proc, hwmon, libsensors, RAPL, perf_event, etc.)
- **GPU + Cross-platform** : `2026-01-29_system-metrics-gpu-crossplatform.md` — APIs NVIDIA/AMD/Intel + 9 bibliothèques cross-platform

---

## Sources

Ce document synthétise les trois recherches détaillées :
- [Capture de métriques système - Windows](./2026-01-29_system-metrics-windows.md)
- [Capture de métriques système - Linux](./2026-01-29_system-metrics-linux.md)
- [Capture de métriques GPU et solutions cross-platform](./2026-01-29_system-metrics-gpu-crossplatform.md)
