---
title: "Capture de metriques GPU et solutions cross-platform"
date: 2026-01-29
topic: system-metrics/gpu-crossplatform
sources:
  - https://docs.nvidia.com/deploy/nvml-api/index.html
  - https://developer.nvidia.com/management-library-nvml
  - https://github.com/NVIDIA/nvidia-settings/blob/main/src/libXNVCtrl/NVCtrl.h
  - https://gpuopen.com/adl/
  - https://gpuopen-librariesandsdks.github.io/adl/
  - https://gpuopen.com/manuals/ADLX/adlx-cpp__perf_g_p_u_metrics/
  - https://github.com/GPUOpen-LibrariesAndSDKs/ADLX
  - https://rocm.docs.amd.com/projects/amdsmi/en/latest/
  - https://docs.kernel.org/gpu/amdgpu/thermal.html
  - https://oneapi-src.github.io/level-zero-spec/
  - https://github.com/intel/pti-gpu/blob/master/chapters/metrics_collection/LevelZero.md
  - https://docs.rs/nvml-wrapper/latest/nvml_wrapper/
  - https://docs.rs/sysinfo/latest/sysinfo/
  - https://github.com/oshi/oshi
  - https://psutil.readthedocs.io/
  - https://www.npmjs.com/package/systeminformation
  - https://libstatgrab.org/
  - https://pypi.org/project/nvidia-ml-py/
  - https://github.com/fbcotter/py3nvml
  - https://github.com/anderskm/gputil
status: final
---

# Capture de metriques GPU et solutions cross-platform

> Guide technique complet pour la lecture de metriques GPU (NVIDIA, AMD, Intel) et les bibliotheques cross-platform applicables a une application desktop Windows/Linux.

## Table des matieres

1. [Introduction et vue d'ensemble](#1-introduction-et-vue-densemble)
2. [NVIDIA - APIs et outils](#2-nvidia---apis-et-outils)
   - [NVML (NVIDIA Management Library)](#21-nvml-nvidia-management-library)
   - [nvidia-smi](#22-nvidia-smi)
   - [NVAPI](#23-nvapi-windows-uniquement)
   - [XNVCtrl / nvidia-settings](#24-xnvctrl--nvidia-settings-linux-uniquement)
3. [AMD - APIs et outils](#3-amd---apis-et-outils)
   - [ADL (AMD Display Library)](#31-adl-amd-display-library)
   - [ADLX (AMD Device Library eXtra)](#32-adlx-amd-device-library-extra)
   - [ROCm SMI / AMD SMI](#33-rocm-smi--amd-smi)
   - [AMDGPU driver sysfs](#34-amdgpu-driver-sysfs-linux)
4. [Intel - APIs et outils](#4-intel---apis-et-outils)
   - [oneAPI Level Zero (Sysman)](#41-oneapi-level-zero-sysman)
   - [i915 driver sysfs](#42-i915-driver-sysfs-linux)
   - [intel_gpu_top](#43-intel_gpu_top)
5. [Solutions cross-platform](#5-solutions-cross-platform)
   - [psutil (Python)](#51-psutil-python)
   - [pynvml / nvidia-ml-py (Python)](#52-pynvml--nvidia-ml-py-python)
   - [GPUtil (Python)](#53-gputil-python)
   - [py3nvml (Python)](#54-py3nvml-python)
   - [sysinfo (Rust)](#55-sysinfo-rust)
   - [nvml-wrapper (Rust)](#56-nvml-wrapper-rust)
   - [OSHI (Java)](#57-oshi-java)
   - [systeminformation (Node.js)](#58-systeminformation-nodejs)
   - [libstatgrab (C)](#59-libstatgrab-c)
6. [Tableau comparatif final](#6-tableau-comparatif-final)
7. [Recommandations architecturales](#7-recommandations-architecturales)
8. [Sources](#8-sources)

---

## 1. Introduction et vue d'ensemble

La capture de metriques GPU est essentielle pour toute application de monitoring systeme, d'optimisation de performance, ou de gestion de ressources. Contrairement aux metriques CPU (largement standardisees via `/proc`, WMI, etc.), les metriques GPU sont **fortement fragmentees par vendor** : NVIDIA, AMD et Intel fournissent chacun leurs propres APIs propriertaires.

### Metriques cibles

| Metrique | Description | Unite typique |
|----------|-------------|---------------|
| GPU Clock (core) | Frequence du coeur graphique | MHz |
| GPU Clock (memory) | Frequence de la VRAM | MHz |
| GPU Clock (shader) | Frequence des shaders (si distinct) | MHz |
| GPU Utilisation | Pourcentage d'utilisation du GPU | % |
| VRAM utilisee | Memoire video consommee | MB/GB |
| VRAM totale | Memoire video disponible | MB/GB |
| Temperature GPU | Temperature du die/junction | C |
| Fan speed | Vitesse des ventilateurs | RPM ou % |
| Power draw | Consommation electrique | W |
| PCIe bandwidth | Bande passante PCIe utilisee | GB/s |
| Encoder utilisation | Utilisation du moteur d'encodage video | % |
| Decoder utilisation | Utilisation du moteur de decodage video | % |

### Panorama des APIs par vendor

```
NVIDIA                  AMD                     Intel
------                  ---                     -----
NVML (C, cross-plat)    ADL (C, cross-plat)     Level Zero (C, cross-plat)
NVAPI (C, Windows)      ADLX (C++, Windows)     IGCL (C, Windows)
nvidia-smi (CLI)        ROCm/AMD SMI (CLI+lib)  intel_gpu_top (CLI)
XNVCtrl (Linux, X11)    sysfs amdgpu (Linux)    sysfs i915 (Linux)
```

---

## 2. NVIDIA - APIs et outils

### 2.1 NVML (NVIDIA Management Library)

#### Presentation

NVML est la bibliotheque C officielle de NVIDIA pour le monitoring et la gestion des GPU. C'est l'API sous-jacente de `nvidia-smi` et la solution recommandee pour l'integration programmatique.

#### Mecanisme technique

NVML communique directement avec le driver NVIDIA via des appels ioctl au kernel driver. La bibliotheque est chargee dynamiquement (`libnvidia-ml.so` sur Linux, `nvml.dll` sur Windows). Elle fonctionne sans serveur X, ce qui la rend utilisable sur les serveurs headless.

#### Langages supportes

- **C/C++** : API native
- **Python** : `nvidia-ml-py` (officiel), `pynvml`, `py3nvml`
- **Rust** : `nvml-wrapper`
- **Go** : `go-nvml`
- **Java** : via JNI wrappers

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui (nvml.dll inclus dans le driver) |
| Linux | Oui (libnvidia-ml.so inclus dans le driver) |
| macOS | Non |

#### Privileges requis

- **Lecture de base** (temperature, clocks, utilisation) : aucun privilege special
- **Operations de controle** (set power limit, set clocks) : root/admin
- **Certaines metriques avancees** (ECC, retired pages) : root/admin recommande

#### Metriques accessibles

| Metrique | Fonction NVML | Disponible |
|----------|---------------|------------|
| GPU Clock core | `nvmlDeviceGetClockInfo(NVML_CLOCK_GRAPHICS)` | Oui |
| GPU Clock memory | `nvmlDeviceGetClockInfo(NVML_CLOCK_MEM)` | Oui |
| GPU Clock SM | `nvmlDeviceGetClockInfo(NVML_CLOCK_SM)` | Oui |
| GPU Utilisation | `nvmlDeviceGetUtilizationRates()` | Oui |
| VRAM usage | `nvmlDeviceGetMemoryInfo()` | Oui |
| Temperature | `nvmlDeviceGetTemperature()` | Oui |
| Fan speed (%) | `nvmlDeviceGetFanSpeed()` / `_v2()` | Oui |
| Fan speed (RPM) | Non disponible directement | Non (*) |
| Power draw | `nvmlDeviceGetPowerUsage()` | Oui (mW) |
| PCIe throughput | `nvmlDeviceGetPcieThroughput()` | Oui |
| Encoder utilisation | `nvmlDeviceGetEncoderUtilization()` | Oui |
| Decoder utilisation | `nvmlDeviceGetDecoderUtilization()` | Oui |

(*) Le RPM reel n'est pas expose par NVML sur les GPU grand public. NVML reporte une "vitesse intentionnelle" en %, pas la lecture tachymetre reelle. Utiliser NVAPI (Windows) ou XNVCtrl (Linux) pour le RPM reel.

#### Exemple de code C

```c
#include <nvml.h>
#include <stdio.h>

int main() {
    nvmlReturn_t result;
    nvmlDevice_t device;

    // Initialisation
    result = nvmlInit();
    if (result != NVML_SUCCESS) {
        printf("Erreur init NVML: %s\n", nvmlErrorString(result));
        return 1;
    }

    // Premier GPU
    nvmlDeviceGetHandleByIndex(0, &device);

    // Temperature
    unsigned int temp;
    nvmlDeviceGetTemperature(device, NVML_TEMPERATURE_GPU, &temp);
    printf("Temperature: %u C\n", temp);

    // Utilisation
    nvmlUtilization_t util;
    nvmlDeviceGetUtilizationRates(device, &util);
    printf("GPU: %u%%, Memoire: %u%%\n", util.gpu, util.memory);

    // Clocks
    unsigned int clockGraphics, clockMem;
    nvmlDeviceGetClockInfo(device, NVML_CLOCK_GRAPHICS, &clockGraphics);
    nvmlDeviceGetClockInfo(device, NVML_CLOCK_MEM, &clockMem);
    printf("Clock GPU: %u MHz, Clock Mem: %u MHz\n", clockGraphics, clockMem);

    // Power
    unsigned int power;
    nvmlDeviceGetPowerUsage(device, &power);
    printf("Power: %.1f W\n", power / 1000.0);

    // VRAM
    nvmlMemory_t memory;
    nvmlDeviceGetMemoryInfo(device, &memory);
    printf("VRAM: %llu / %llu MB\n",
           memory.used / (1024*1024), memory.total / (1024*1024));

    // Fan speed
    unsigned int fanSpeed;
    nvmlDeviceGetFanSpeed(device, &fanSpeed);
    printf("Fan: %u%%\n", fanSpeed);

    // PCIe throughput
    unsigned int txBytes, rxBytes;
    nvmlDeviceGetPcieThroughput(device, NVML_PCIE_UTIL_TX_BYTES, &txBytes);
    nvmlDeviceGetPcieThroughput(device, NVML_PCIE_UTIL_RX_BYTES, &rxBytes);
    printf("PCIe TX: %u KB/s, RX: %u KB/s\n", txBytes, rxBytes);

    // Encoder/Decoder
    unsigned int encUtil, encPeriod, decUtil, decPeriod;
    nvmlDeviceGetEncoderUtilization(device, &encUtil, &encPeriod);
    nvmlDeviceGetDecoderUtilization(device, &decUtil, &decPeriod);
    printf("Encoder: %u%%, Decoder: %u%%\n", encUtil, decUtil);

    nvmlShutdown();
    return 0;
}
```

**Compilation** : `gcc -o gpu_monitor gpu_monitor.c -lnvidia-ml`

#### Avantages

- API la plus stable et complete de NVIDIA
- Cross-platform (Windows + Linux)
- Pas besoin de serveur X
- Backward compatible entre versions
- Documentation officielle excellente
- Mise a jour reguliere (vR590, janvier 2026)

#### Inconvenients

- NVIDIA uniquement
- Fan RPM reel non disponible (seulement %)
- Certaines metriques avancees (GPM) necessitent des GPU datacenter
- Pas de controle overclock (utiliser NVAPI pour ca)

#### Fiabilite/Precision

**Excellente**. NVML est la reference utilisee par nvidia-smi, les systemes de monitoring Prometheus/Grafana, et la majorite des outils professionnels. Les valeurs de temperature et power sont fiables. La seule reserve concerne le fan speed qui reporte l'intention et non la mesure reelle.

---

### 2.2 nvidia-smi

#### Presentation

`nvidia-smi` (NVIDIA System Management Interface) est l'outil CLI officiel de NVIDIA, construit au-dessus de NVML. Il permet de monitorer et gerer les GPU NVIDIA depuis la ligne de commande.

#### Mecanisme technique

nvidia-smi est un wrapper CLI autour de NVML. Chaque appel `nvidia-smi` initialise NVML, effectue les requetes, et ferme la session. Pour du monitoring continu, l'option `dmon` ou `--loop` evite la reinitialisation.

#### OS supportes

- **Windows** : Oui (installe avec le driver)
- **Linux** : Oui (installe avec le driver)

#### Privileges requis

- Lecture : aucun privilege special
- Modification (clocks, power limit) : root/admin

#### Commandes utiles

```bash
# Vue d'ensemble
nvidia-smi

# Monitoring continu (1 sec interval)
nvidia-smi dmon -s pucvmet -d 1

# Format CSV pour parsing
nvidia-smi --query-gpu=timestamp,name,temperature.gpu,utilization.gpu,\
utilization.memory,memory.used,memory.total,power.draw,clocks.gr,\
clocks.mem,fan.speed,pcie.link.gen.current,encoder.stats.sessionCount,\
decoder.stats.sessionCount \
--format=csv,noheader,nounits -l 1

# Metriques specifiques
nvidia-smi --query-gpu=gpu_name,temperature.gpu,power.draw --format=csv
```

#### Avantages

- Installation zero (vient avec le driver)
- Scriptable, output CSV/XML
- Ideal pour prototypage rapide et debug
- Monitoring continu avec `dmon`

#### Inconvenients

- Overhead de lancement du processus a chaque appel
- Parsing de sortie texte/CSV fragile
- Pas d'API programmatique (fork/exec necessaire)
- Latence plus elevee que NVML direct

#### Fiabilite/Precision

**Identique a NVML** puisque c'est le meme backend. Cependant, le format de sortie peut varier entre versions du driver, ce qui rend le parsing fragile pour une application de production.

---

### 2.3 NVAPI (Windows uniquement)

#### Presentation

NVAPI est le SDK natif Windows de NVIDIA. Il fournit des fonctionnalites plus avancees que NVML, notamment le controle de l'overclocking, la lecture du tachymetre reel des ventilateurs, et l'acces a des fonctionnalites cachees via la DLL dynamique.

#### Mecanisme technique

NVAPI est distribue sous forme de bibliotheque statique (SDK public) et dynamique (`nvapi.dll` / `nvapi64.dll`). La version statique expose l'API documentee ; la version dynamique contient des fonctions supplementaires non documentees utilisees par des outils comme MSI Afterburner.

#### Langages supportes

- **C/C++** : SDK natif avec headers
- **C#/.NET** : `NvAPIWrapper` (wrapper communautaire)
- **Python** : pas de binding officiel (acces indirect via ctypes)

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui (SDK + DLL dans le driver) |
| Linux | Non |

#### Privileges requis

- Lecture : aucun privilege special
- Controle (overclock, fan control) : droits administrateur

#### Metriques accessibles

| Metrique | Fonction NVAPI | Disponible |
|----------|----------------|------------|
| GPU Clock core | `NvAPI_GPU_GetAllClockFrequencies()` | Oui |
| GPU Clock memory | `NvAPI_GPU_GetAllClockFrequencies()` | Oui |
| GPU Utilisation | `NvAPI_GPU_GetDynamicPstatesInfoEx()` | Oui |
| VRAM usage | `NvAPI_GPU_GetMemoryInfo()` | Oui |
| Temperature | `NvAPI_GPU_GetThermalSettings()` | Oui |
| Fan speed (%) | `NvAPI_GPU_GetCoolerSettings()` | Oui |
| **Fan speed (RPM)** | `NvAPI_GPU_GetTachReading()` | **Oui** |
| Power draw | Via fonctions non documentees | Partiel |
| PCIe bandwidth | `NvAPI_GPU_GetPCIEInfo()` | Oui |
| Encoder/Decoder | Non directement | Non |

**Point cle** : NVAPI est le seul moyen fiable d'obtenir le RPM reel du tachymetre sur Windows. Cependant, `NvAPI_GPU_GetTachReading()` ne supporte qu'un seul ventilateur ; sur les GPU multi-fan, le RPM des ventilateurs supplementaires peut etre incorrect.

#### Avantages

- RPM tachymetre reel
- Controle overclock et fan curves
- API stable et mature
- Fonctionnalites cachees dans la DLL dynamique

#### Inconvenients

- **Windows uniquement**
- SDK statique seulement (pas de chargement dynamique officiel)
- Documentation partielle (fonctions cachees non documentees)
- RPM multi-fan incorrect sur certains GPU
- Plus complexe que NVML pour du simple monitoring

#### Fiabilite/Precision

**Tres bonne** pour les metriques qu'il expose. Le RPM tachymetre est une mesure physique reelle, donc plus precise que le % de NVML. La temperature est identique a NVML (meme source hardware).

---

### 2.4 XNVCtrl / nvidia-settings (Linux uniquement)

#### Presentation

XNVCtrl (libXNVCtrl) est une extension X11 pour le controle des GPU NVIDIA sous Linux. C'est la bibliotheque sous-jacente de l'outil `nvidia-settings`. Elle permet le monitoring et le controle avance des GPU, y compris la lecture du RPM reel des ventilateurs.

#### Mecanisme technique

XNVCtrl communique avec le driver NVIDIA via le protocole X11. Cela implique qu'un serveur X doit etre en cours d'execution. La bibliotheque envoie des requetes via l'extension NV-CONTROL du protocole X.

#### Langages supportes

- **C** : API native via `NVCtrl.h`
- **Python** : pas de binding officiel (possible via ctypes/cffi)
- **Rust** : utilise dans le projet `nvfancontrol`

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Non |
| Linux | Oui (necessite X11) |
| Linux (Wayland) | Non (X11 requis) |

#### Privileges requis

- Lecture : acces au serveur X
- Controle fan : variable selon configuration (peut necessiter `CoolBits` dans `xorg.conf`)

#### Metriques accessibles

| Metrique | Attribut XNVCtrl | Disponible |
|----------|------------------|------------|
| GPU Clock core | `NV_CTRL_GPU_CURRENT_CLOCK_FREQS` | Oui |
| GPU Clock memory | `NV_CTRL_GPU_CURRENT_CLOCK_FREQS` | Oui |
| GPU Utilisation | `NV_CTRL_GPU_CURRENT_PROCESSOR_CLOCK_FREQS` | Partiel |
| Temperature | `NV_CTRL_GPU_CORE_TEMPERATURE` | Oui |
| Temperature seuil | `NV_CTRL_GPU_CORE_THRESHOLD` | Oui |
| Temperature ambiante | `NV_CTRL_AMBIENT_TEMPERATURE` | Oui |
| **Fan speed (RPM)** | Accessible via target fan | **Oui** |
| Fan speed (%) | Via cooler settings | Oui |
| Power draw | Non disponible directement | Non |

#### Commande CLI equivalente

```bash
# Lister tous les attributs disponibles
nvidia-settings --query all

# Temperature GPU
nvidia-settings -q GPUCoreTemp -t

# Frequence GPU courante
nvidia-settings -q GPUCurrentClockFreqs -t

# Fan speed
nvidia-settings -q [fan:0]/GPUCurrentFanSpeed -t
```

#### Avantages

- RPM reel des ventilateurs disponible
- Controle fin des ventilateurs (avec CoolBits)
- Temperature ambiante accessible
- Pas besoin de root pour la lecture

#### Inconvenients

- **Linux uniquement, necessite X11** (incompatible Wayland natif)
- Inutilisable sur serveurs headless
- Moins de metriques que NVML (pas de power draw, PCIe, encoder)
- En voie de deprecation avec la transition vers Wayland

#### Fiabilite/Precision

**Bonne** pour les metriques exposees. Les lectures de temperature et RPM sont des valeurs hardware reelles. Cependant, la dependance a X11 est un frein majeur pour les architectures modernes.

---

## 3. AMD - APIs et outils

### 3.1 ADL (AMD Display Library)

#### Presentation

ADL est le SDK historique d'AMD pour le monitoring et le controle des GPU Radeon. Il unifie plusieurs anciens SDKs (PDL, DSP, CCC COM). ADL est progressivement remplace par ADLX, mais reste utilise et maintenu.

#### Mecanisme technique

ADL est une bibliotheque C qui wrappe les APIs privees du driver AMD. Elle communique avec le driver via des interfaces specifiques a l'OS (IOCTL sur Windows, sysfs/ioctl sur Linux). Les fonctions de monitoring sont organisees en "Overdrive" generations (OD5, OD6, ODN, OD8) correspondant aux differentes generations de GPU.

#### Langages supportes

- **C/C++** : API native
- **C#** : Utilisable via P/Invoke
- **Python** : pas de binding officiel

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui (XP a 11, 32 et 64-bit) |
| Linux | Oui (support partiel) |

#### Privileges requis

- Lecture : aucun privilege special
- Controle (overclock) : droits administrateur

#### Metriques accessibles

L'API varie selon la generation Overdrive :

| Metrique | API Overdrive | Disponible |
|----------|---------------|------------|
| GPU Clock core | `ADL_Overdrive5_CurrentActivity_Get()` / OD6/ODN | Oui |
| GPU Clock memory | `ADL_Overdrive5_CurrentActivity_Get()` | Oui |
| GPU Utilisation (%) | `ADL_Overdrive5_CurrentActivity_Get()` | Oui |
| VRAM usage | Via `ADL_Adapter_MemoryInfo_Get()` | Partiel |
| Temperature | `ADL_Overdrive5_Temperature_Get()` | Oui (millidegres C) |
| Fan speed (%) | `ADL_Overdrive5_FanSpeed_Get()` | Oui |
| Fan speed (RPM) | `ADL_Overdrive5_FanSpeed_Get()` (type RPM) | Oui |
| Power draw | Via PMLOG sensors (recent) | Partiel |
| PCIe bandwidth | Non directement | Non |
| Encoder/Decoder | Non | Non |

**Note** : La temperature est retournee en **millidegres Celsius** (diviser par 1000).

#### Exemple de code C

```c
#include "adl_sdk.h"
#include <stdio.h>
#include <stdlib.h>

// Callback d'allocation memoire requis par ADL
void* __stdcall ADL_Main_Memory_Alloc(int iSize) {
    return malloc(iSize);
}

int main() {
    int status;

    // Initialisation
    status = ADL_Main_Control_Create(ADL_Main_Memory_Alloc, 1);
    if (status != ADL_OK) {
        printf("Erreur init ADL: %d\n", status);
        return 1;
    }

    // Nombre d'adaptateurs
    int numAdapters;
    ADL_Adapter_NumberOfAdapters_Get(&numAdapters);

    // Pour le premier adaptateur actif
    int adapterIndex = 0;

    // Temperature (millidegres C)
    ADLTemperature temp = {0};
    temp.iSize = sizeof(ADLTemperature);
    ADL_Overdrive5_Temperature_Get(adapterIndex, 0, &temp);
    printf("Temperature: %.1f C\n", temp.iTemperature / 1000.0);

    // Activite (clocks, utilisation)
    ADLPMActivity activity = {0};
    activity.iSize = sizeof(ADLPMActivity);
    ADL_Overdrive5_CurrentActivity_Get(adapterIndex, &activity);
    printf("GPU Clock: %d MHz\n", activity.iEngineClock / 100);
    printf("Mem Clock: %d MHz\n", activity.iMemoryClock / 100);
    printf("GPU Usage: %d%%\n", activity.iActivityPercent);

    // Fan speed
    ADLFanSpeedValue fanSpeed = {0};
    fanSpeed.iSize = sizeof(ADLFanSpeedValue);
    fanSpeed.iSpeedType = ADL_DL_FANCTRL_SPEED_TYPE_RPM;
    ADL_Overdrive5_FanSpeed_Get(adapterIndex, 0, &fanSpeed);
    printf("Fan: %d RPM\n", fanSpeed.iFanSpeed);

    ADL_Main_Control_Destroy();
    return 0;
}
```

#### Avantages

- Support Windows + Linux
- API mature et bien documentee
- Fan RPM reel disponible
- Support des anciennes et nouvelles generations GPU
- Open source sur GitHub (GPUOpen)

#### Inconvenients

- API vieillissante (remplacee par ADLX)
- API Overdrive fragmentee par generation (OD5/OD6/ODN/OD8)
- Complexite d'initialisation (callback memoire obligatoire)
- AMD uniquement
- Certaines metriques modernes absentes (encoder, PCIe bandwidth)

#### Fiabilite/Precision

**Bonne**. Les valeurs proviennent directement des capteurs hardware via le driver. La temperature en millidegres offre une bonne precision. Les clocks sont reportes en centaines de kHz (diviser par 100 pour MHz).

---

### 3.2 ADLX (AMD Device Library eXtra)

#### Presentation

ADLX est le successeur moderne d'ADL. AMD recommande fortement ADLX pour tout nouveau developpement. ADLX offre une API plus propre, basee sur des interfaces COM-like, avec un support complet du performance monitoring, du GPU tuning, et des fonctionnalites d'affichage.

#### Mecanisme technique

ADLX utilise un modele d'interfaces (similaire a COM) avec `QueryInterface` pour acceder aux differentes capacites. Le systeme de performance monitoring supporte a la fois les metriques courantes (`GetCurrentGPUMetrics`) et l'historique (`GetGPUMetricsHistory`). ADLX et ADL peuvent coexister dans la meme application.

#### Langages supportes

- **C** : API via vtables
- **C++** : API avec smart pointers et interfaces
- **C#** : via wrappers
- **Python** : bindings disponibles dans le SDK

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui (principal) |
| Linux | Support en developpement |

#### Privileges requis

- Lecture (monitoring) : aucun privilege special
- Controle (tuning, overclock) : droits administrateur

#### Metriques accessibles via IADLXGPUMetrics

| Metrique | Methode ADLX | Disponible |
|----------|--------------|------------|
| GPU Clock core | `GPUClockSpeed()` | Oui |
| GPU Clock memory | `GPUVRAMClockSpeed()` | Oui |
| GPU Utilisation | `GPUUsage()` | Oui |
| VRAM usage | `GPUVRAMUsage()` (via IADLXGPUMetrics2) | Oui |
| Temperature | `GPUTemperature()` | Oui |
| Temperature hotspot | `GPUHotspotTemperature()` | Oui |
| Fan speed (RPM) | `GPUFanSpeed()` | Oui |
| Power draw | `GPUPower()` | Oui |
| PCIe bandwidth | Non documente | Incertain |
| Encoder/Decoder | Non | Non |

**Interfaces etendues** :
- `IADLXGPUMetrics1` : ajoute NPU activity/frequency
- `IADLXGPUMetrics2` : ajoute shared memory metrics
- `IADLXGPUMetrics3` : metriques supplementaires (GPU recentes)

#### Exemple de code C++

```cpp
#include "SDK/ADLXHelper/Windows/Cpp/ADLXHelper.h"
#include "SDK/Include/IPerformanceMonitoring.h"

using namespace adlx;

int main() {
    ADLXHelper helper;
    ADLX_RESULT res = helper.Initialize();
    if (!ADLX_SUCCEEDED(res)) return 1;

    // Obtenir le service de monitoring
    IADLXPerformanceMonitoringServicesPtr perfServices;
    helper.GetSystemServices()->GetPerformanceMonitoringServices(&perfServices);

    // Obtenir le premier GPU
    IADLXGPUListPtr gpuList;
    helper.GetSystemServices()->GetGPUs(&gpuList);
    IADLXGPUPtr gpu;
    gpuList->At(0, &gpu);

    // Metriques courantes
    IADLXGPUMetricsPtr metrics;
    perfServices->GetCurrentGPUMetrics(gpu, &metrics);

    double gpuUsage, gpuTemp, gpuPower;
    int gpuClock, memClock, fanRPM;

    metrics->GPUUsage(&gpuUsage);
    metrics->GPUTemperature(&gpuTemp);
    metrics->GPUPower(&gpuPower);
    metrics->GPUClockSpeed(&gpuClock);
    metrics->GPUVRAMClockSpeed(&memClock);
    metrics->GPUFanSpeed(&fanRPM);

    printf("Usage: %.1f%%\n", gpuUsage);
    printf("Temp: %.1f C\n", gpuTemp);
    printf("Power: %.1f W\n", gpuPower);
    printf("GPU Clock: %d MHz\n", gpuClock);
    printf("Mem Clock: %d MHz\n", memClock);
    printf("Fan: %d RPM\n", fanRPM);

    helper.Terminate();
    return 0;
}
```

#### Avantages

- API moderne et propre (interfaces COM-like)
- Support historique des metriques (pas seulement courant)
- Coexistence avec ADL
- Open source sur GitHub
- Extensions pour NPU et fonctionnalites recentes
- Temperature hotspot accessible

#### Inconvenients

- Principalement Windows (support Linux en cours)
- SDK plus lourd qu'ADL
- Documentation parfois incomplete
- Necessite Radeon Software Adrenalin Edition 25.3.1+
- AMD uniquement

#### Fiabilite/Precision

**Tres bonne**. ADLX est l'API de reference utilisee par AMD Adrenalin Software pour l'affichage des metriques. Les valeurs proviennent directement des capteurs hardware.

---

### 3.3 ROCm SMI / AMD SMI

#### Presentation

ROCm SMI (System Management Interface) est la bibliotheque de monitoring GPU pour l'ecosysteme ROCm (compute/AI). AMD SMI est son successeur unifie, devenant l'outil principal pour la gestion hardware AMD.

#### Mecanisme technique

AMD SMI communique avec le driver amdgpu via sysfs et ioctl sous Linux. Sur les GPU compute (Instinct), il accede a des registres supplementaires via le firmware SMU (System Management Unit).

#### Langages supportes

- **C++** : API native
- **Python** : bindings officiels (`amdsmi` package)
- **CLI** : `amd-smi` / `rocm-smi`

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Non (ROCm est Linux-only) |
| Linux | Oui |

#### Privileges requis

- Lecture basique : membre du groupe `video` ou `render`
- Certaines metriques avancees : root
- Controle (clocks, power) : root

#### Metriques accessibles

| Metrique | Disponible |
|----------|------------|
| GPU Clock core | Oui |
| GPU Clock memory | Oui |
| GPU Utilisation | Oui |
| VRAM usage | Oui |
| Temperature | Oui (multiple : edge, junction, memory) |
| Fan speed (%) | Oui |
| Fan speed (RPM) | Oui |
| Power draw | Oui |
| PCIe speed/width | Oui |
| Encoder/Decoder | Non (pas de NVENC equivalent sur AMD via SMI) |
| ECC errors | Oui |
| Voltage | Oui |

#### Commandes CLI

```bash
# Vue d'ensemble
amd-smi static

# Metriques en temps reel
amd-smi metric

# Temperature
amd-smi metric -t

# Power
amd-smi metric -p

# Utilisation GPU + clocks
amd-smi metric -u

# Memoire
amd-smi metric -m

# Monitoring continu
amd-smi monitor -ptu
```

#### Exemple Python

```python
import amdsmi

amdsmi.amdsmi_init(amdsmi.AmdSmiInitFlags.ALL_DEVICES)

devices = amdsmi.amdsmi_get_processor_handles()
for dev in devices:
    # Temperature
    temp = amdsmi.amdsmi_get_temp_metric(
        dev, amdsmi.AmdSmiTemperatureType.EDGE,
        amdsmi.AmdSmiTemperatureMetric.CURRENT
    )
    print(f"Temp: {temp} C")

    # Utilisation
    engine = amdsmi.amdsmi_get_gpu_activity(dev)
    print(f"GPU: {engine['gfx_activity']}%")
    print(f"Mem: {engine['umc_activity']}%")

    # Power
    power = amdsmi.amdsmi_get_power_info(dev)
    print(f"Power: {power['average_socket_power']} W")

    # Clocks
    clk = amdsmi.amdsmi_get_clock_info(
        dev, amdsmi.AmdSmiClkType.GFX
    )
    print(f"GPU Clock: {clk['clk']} MHz")

amdsmi.amdsmi_shut_down()
```

#### Avantages

- API complete pour GPU AMD compute (Instinct, Radeon Pro)
- Bindings Python officiels
- Supporte les metriques multi-capteurs (edge, junction, memory temp)
- Open source
- Outil CLI puissant et bien structure

#### Inconvenients

- **Linux uniquement**
- Oriente GPU compute/datacenter (ROCm)
- Installation lourde (fait partie de la stack ROCm)
- Pas de support Windows
- Certaines fonctions absentes sur GPU grand public (Radeon RX)

#### Fiabilite/Precision

**Excellente** sur GPU supportes. Les metriques proviennent du SMU (System Management Unit) du GPU, offrant des lectures hardware directes. Precision comparable a celle de NVML pour NVIDIA.

---

### 3.4 AMDGPU driver sysfs (Linux)

#### Presentation

Le driver AMDGPU expose un grand nombre de metriques directement via le systeme de fichiers sysfs de Linux. C'est la methode la plus directe et la plus legere pour lire des metriques GPU AMD sous Linux.

#### Mecanisme technique

Le kernel Linux expose les registres du GPU via des fichiers virtuels sous `/sys/class/drm/cardN/device/`. Les fichiers hwmon fournissent les donnees de capteurs standardisees. Le fichier binaire `gpu_metrics` contient un blob de donnees structure avec toutes les metriques en un seul read.

#### Langages supportes

Tout langage capable de lire des fichiers : C, Python, Rust, Go, etc.

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Non |
| Linux | Oui (kernel 4.15+) |

#### Privileges requis

- Lecture (temperature, clocks, utilisation) : generalement aucun privilege (fichiers lisibles par tous)
- Ecriture (fan control, power limit) : root ou membre du groupe `video`

#### Chemins sysfs principaux

| Chemin | Metrique | Unite |
|--------|----------|-------|
| `/sys/class/drm/card0/device/gpu_busy_percent` | GPU utilisation | % |
| `/sys/class/drm/card0/device/mem_info_vram_used` | VRAM utilisee | bytes |
| `/sys/class/drm/card0/device/mem_info_vram_total` | VRAM totale | bytes |
| `/sys/class/drm/card0/device/pp_dpm_sclk` | GPU clock states | MHz |
| `/sys/class/drm/card0/device/pp_dpm_mclk` | Memory clock states | MHz |
| `/sys/class/drm/card0/device/hwmon/hwmon*/temp1_input` | Temperature | millidegres C |
| `/sys/class/drm/card0/device/hwmon/hwmon*/pwm1` | Fan PWM duty cycle | 0-255 |
| `/sys/class/drm/card0/device/hwmon/hwmon*/fan1_input` | Fan RPM | RPM |
| `/sys/class/drm/card0/device/hwmon/hwmon*/power1_average` | Power draw moyen | microwatts |
| `/sys/class/drm/card0/device/hwmon/hwmon*/power1_cap` | Power limit | microwatts |

#### Fichier gpu_metrics (blob binaire)

Le fichier `/sys/class/drm/card0/device/gpu_metrics` contient un dump binaire structure de toutes les metriques en un seul read. Les champs incluent :

- `temperature_gfx`, `temperature_soc`, `temperature_core[]`, `temperature_l3[]`
- `average_socket_power`, `average_cpu_power`, `average_soc_power`, `average_gfx_power`
- `average_gfxclk_frequency`, `average_socclk_frequency`, `average_uclk_frequency`
- `current_gfxclk`, `current_socclk`, `current_uclk`, `current_vclk0`, `current_dclk0`
- `fan_pwm`, `throttle_status`

#### Exemple Python

```python
import os

CARD_PATH = "/sys/class/drm/card0/device"

def read_sysfs(path):
    """Lire une valeur depuis sysfs."""
    try:
        with open(os.path.join(CARD_PATH, path)) as f:
            return f.read().strip()
    except (IOError, PermissionError):
        return None

# GPU utilisation
gpu_busy = read_sysfs("gpu_busy_percent")
print(f"GPU: {gpu_busy}%")

# VRAM
vram_used = int(read_sysfs("mem_info_vram_used")) / (1024**2)
vram_total = int(read_sysfs("mem_info_vram_total")) / (1024**2)
print(f"VRAM: {vram_used:.0f} / {vram_total:.0f} MB")

# Temperature (via hwmon)
import glob
hwmon = glob.glob(f"{CARD_PATH}/hwmon/hwmon*")[0]
temp = int(open(f"{hwmon}/temp1_input").read()) / 1000
print(f"Temp: {temp:.1f} C")

# Fan RPM
fan_rpm = open(f"{hwmon}/fan1_input").read().strip()
print(f"Fan: {fan_rpm} RPM")

# Power
power_uw = int(open(f"{hwmon}/power1_average").read())
print(f"Power: {power_uw / 1e6:.1f} W")

# Clocks (etat actif marque par *)
sclk_states = read_sysfs("pp_dpm_sclk")
for line in sclk_states.split('\n'):
    if '*' in line:
        print(f"GPU Clock: {line.strip()}")
```

#### Avantages

- **Zero dependance** (pas de bibliotheque, juste des fichiers)
- Tres leger (lecture de fichiers texte)
- Toujours disponible si le driver amdgpu est charge
- Pas besoin de root pour la lecture
- `gpu_metrics` blob : toutes les metriques en un seul syscall

#### Inconvenients

- **Linux uniquement**
- Chemins hwmon variables (`hwmon0`, `hwmon1`, etc.)
- Format non standardise entre generations GPU
- `gpu_metrics` blob : necessite le decodage de la structure binaire
- Pas d'API d'abstraction (raw filesystem access)
- `pp_dpm_sclk` montre les etats possibles, pas directement le clock courant

#### Fiabilite/Precision

**Excellente**. C'est la source la plus directe : les fichiers sysfs sont exposes par le driver kernel amdgpu a partir des registres hardware. Les outils comme `rocm-smi` utilisent ces memes fichiers en interne.

---

## 4. Intel - APIs et outils

### 4.1 oneAPI Level Zero (Sysman)

#### Presentation

oneAPI Level Zero est l'API bas-niveau d'Intel pour l'acces direct aux accelerateurs (GPU, FPGA, etc.). Le module **Sysman** (System Management) fournit les APIs de telemetrie et monitoring : temperature, frequence, puissance, ventilateurs, memoire, et plus.

#### Mecanisme technique

Level Zero charge dynamiquement le runtime Intel (`ze_loader`), qui communique avec le driver GPU Intel (i915 ou Xe). Le module Sysman expose des "handles" pour chaque domaine de telemetrie (frequence, temperature, puissance, etc.). Les metriques de performance GPU (utilisation EU, cache misses, etc.) utilisent un systeme de "metric groups" collectes en continu ou par query.

#### Langages supportes

- **C** : API native
- **C++** : wrappers disponibles
- **Python** : via ctypes ou bindings communautaires

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui (partiellement, 64-bit uniquement pour telemetrie) |
| Linux | Oui |

#### Privileges requis

- Lecture de base : membre des groupes `video` et `render`
- Metriques detaillees : `sysctl -w dev.i915.perf_stream_paranoid=0`
- Controle (overclock, fan) : root

#### Metriques accessibles (Sysman)

| Metrique | API Level Zero Sysman | Disponible |
|----------|----------------------|------------|
| GPU Clock core | `zesFrequencyGetState()` | Oui |
| GPU Utilisation | Via metric groups (`EuActive`) | Oui (complexe) |
| VRAM usage | `zesMemoryGetState()` | Oui |
| Temperature | `zesTemperatureGetState()` | Oui |
| Fan speed | `zesFanGetState()` | Oui |
| Power draw | `zesPowerGetEnergyCounter()` | Oui |
| PCIe bandwidth | `zesPciGetStats()` | Oui |
| Encoder/Decoder | Via engine activity metrics | Partiel |

**Note** : Les APIs de telemetrie (Engine/Fan/Telemetry/Frequency/Memory/Overclock/PCI/Power/Temperature) sont limitees aux applications 64-bit.

#### Avantages

- API officielle Intel, bien specifiee
- Support des GPU integres et discrets Intel
- Metriques de performance detaillees (EU active, cache, throughput)
- Cross-platform (Windows + Linux)
- Support multi-tile pour GPU datacenter

#### Inconvenients

- **Complexite elevee** (API bas-niveau, beaucoup de boilerplate)
- Intel uniquement
- Metriques de performance via metric groups : systeme lourd a configurer
- Documentation technique dense
- Support Windows parfois en retard sur Linux

#### Fiabilite/Precision

**Bonne**. Les metriques proviennent directement du driver. La precision depend du type de metrique : les compteurs de frequence et temperature sont precis, tandis que les metriques de performance (utilisation EU) dependent du sampling interval.

---

### 4.2 i915 driver sysfs (Linux)

#### Presentation

Le driver i915 du kernel Linux expose les metriques des GPU Intel integres et discrets via sysfs. C'est l'equivalent du sysfs amdgpu pour les GPU Intel.

#### Mecanisme technique

Le driver i915 ecrit les valeurs des registres GPU dans des fichiers sous `/sys/class/drm/card0/`. Les metriques de frequence utilisent le systeme RPS (Render Performance States) du GPU Intel.

#### Langages supportes

Tout langage capable de lire des fichiers.

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Non |
| Linux | Oui |

#### Privileges requis

- Lecture frequence : aucun privilege
- Metriques perf detaillees : `dev.i915.perf_stream_paranoid=0` et groupes `video`/`render`

#### Chemins sysfs principaux

| Chemin | Metrique | Unite |
|--------|----------|-------|
| `/sys/class/drm/card0/gt_cur_freq_mhz` | Frequence GPU courante | MHz |
| `/sys/class/drm/card0/gt_max_freq_mhz` | Frequence max logicielle | MHz |
| `/sys/class/drm/card0/gt_min_freq_mhz` | Frequence min logicielle | MHz |
| `/sys/class/drm/card0/gt_boost_freq_mhz` | Frequence boost | MHz |
| `/sys/class/drm/card0/gt/gt*/rps_RP0_freq_mhz` | Frequence max HW | MHz |
| `/sys/class/drm/card0/gt/gt*/rps_RPn_freq_mhz` | Frequence min HW | MHz |
| `/sys/class/drm/card0/gt_max_temp` | Temperature max | C |

**Note importante** : Les GPU Intel ont un comportement RC6 (deep power-saving). La frequence courante peut afficher 0 MHz quand le GPU est en idle profond.

#### Avantages

- Zero dependance
- Lecture instantanee
- Pas besoin de root pour les frequences
- Stable dans le temps

#### Inconvenients

- **Linux uniquement**
- Metriques limitees (surtout frequences)
- Pas d'utilisation %, pas de VRAM, pas de power draw via sysfs standard
- Temperature exposee seulement depuis kernel 7.0+ pour certains GPU
- Pas de fan speed (GPU integres n'ont pas de ventilateur dedie)

#### Fiabilite/Precision

**Bonne** pour les frequences. La valeur de `gt_cur_freq_mhz` est lue directement depuis les registres RPS du GPU.

---

### 4.3 intel_gpu_top

#### Presentation

`intel_gpu_top` est un outil CLI fourni dans le package `intel-gpu-tools` (aussi appele `igt-gpu-tools`). Il affiche en temps reel l'utilisation du GPU Intel, les frequences, la puissance et les temperatures.

#### Mecanisme technique

`intel_gpu_top` lit les compteurs de performance du driver i915 via debugfs/sysfs et les PMU (Performance Monitoring Units) du kernel. Il utilise `perf_event_open()` pour certaines metriques.

#### OS supportes

- **Linux uniquement**

#### Privileges requis

- Root ou `CAP_PERFMON` capability
- Ou : `dev.i915.perf_stream_paranoid=0`

#### Metriques affichees

- Utilisation par engine (Render/3D, Blitter, Video, VideoEnhance)
- Frequence GPU demandee et actuelle
- Puissance (si supportee par le hardware)
- Interrupts/sec

#### Commande

```bash
# Mode interactif (curses)
sudo intel_gpu_top

# Mode JSON pour parsing
sudo intel_gpu_top -J -s 1000

# Sortie dans un fichier
sudo intel_gpu_top -o output.json -s 1000 -l
```

#### Avantages

- Vision complete de l'utilisation GPU Intel
- Utilisation par engine (pas juste un % global)
- Output JSON pour integration programmatique
- Inclus dans les packages standard des distributions

#### Inconvenients

- Linux uniquement
- Necessite des privileges eleves
- Intel uniquement
- Pas d'API programmatique (CLI seulement)
- Pas de temperature ni VRAM directement

#### Fiabilite/Precision

**Bonne**. Utilise les memes compteurs hardware que les outils de profiling Intel (VTune). Les pourcentages d'utilisation par engine sont fiables.

---

## 5. Solutions cross-platform

### 5.1 psutil (Python)

#### Presentation

psutil (process and system utilities) est la bibliotheque Python de reference pour le monitoring systeme cross-platform. Elle couvre CPU, memoire, disques, reseau, capteurs, et processus.

#### GPU : ce que psutil peut et ne peut PAS faire

**psutil ne supporte PAS les metriques GPU.** Il n'y a aucune fonction pour :
- L'utilisation GPU
- La temperature GPU
- Les clocks GPU
- La VRAM
- Le fan speed GPU

Il existe un [issue ouvert (#526)](https://github.com/giampaolo/psutil/issues/526) depuis des annees demandant l'ajout de metriques GPU, mais ce n'est pas implemente.

**Ce que psutil PEUT faire en lien avec le GPU** :
- Detecter les processus utilisant le GPU (via leur consommation memoire/CPU)
- Lire les capteurs de temperature du systeme (qui peuvent inclure le GPU sur certaines cartes meres, via hwmon)

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui |
| Linux | Oui |
| macOS | Oui |
| FreeBSD/OpenBSD | Oui |

#### Exemple (capteurs temperature systeme)

```python
import psutil

# Capteurs de temperature (peut inclure GPU via hwmon)
temps = psutil.sensors_temperatures()
for name, entries in temps.items():
    print(f"{name}:")
    for entry in entries:
        print(f"  {entry.label}: {entry.current}C")

# Resultat possible sur Linux :
# amdgpu:
#   edge: 45.0C
#   junction: 48.0C
# coretemp:
#   Core 0: 55.0C
```

**Attention** : `sensors_temperatures()` n'est disponible que sur Linux. Sur Windows, cette fonction n'existe pas.

#### Avantages

- Bibliotheque Python de reference pour le monitoring systeme
- Tres mature, stable, bien documentee
- Cross-platform
- Capteurs temperature sur Linux (peut inclure GPU indirectement)

#### Inconvenients

- **Aucune metrique GPU native**
- `sensors_temperatures()` Linux uniquement
- Les donnees GPU via hwmon sont incompletes et non structurees
- Pas de VRAM, clocks, utilisation, power GPU

#### Fiabilite/Precision

**Excellente** pour ce qu'elle couvre (CPU, RAM, disque, reseau). Non applicable pour GPU.

---

### 5.2 pynvml / nvidia-ml-py (Python)

#### Presentation

`nvidia-ml-py` est le package Python officiel de NVIDIA fournissant des bindings pour NVML. Le module s'importe sous le nom `pynvml`. C'est la methode recommandee pour acceder aux metriques GPU NVIDIA depuis Python.

**Note sur la confusion des noms** : Il existe plusieurs packages historiques :
- `nvidia-ml-py` (officiel, recommande) - `pip install nvidia-ml-py`
- `pynvml` (ancien, **deprece**) - redirigeait vers nvidia-ml-py
- `py3nvml` (fork Python 3, maintenu independamment)

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui |
| Linux | Oui |

#### Exemple complet

```python
from pynvml import *

nvmlInit()

device_count = nvmlDeviceGetCount()
for i in range(device_count):
    handle = nvmlDeviceGetHandleByIndex(i)

    # Nom du GPU
    name = nvmlDeviceGetName(handle)
    print(f"=== {name} ===")

    # Temperature
    temp = nvmlDeviceGetTemperature(handle, NVML_TEMPERATURE_GPU)
    print(f"  Temp: {temp} C")

    # Utilisation
    util = nvmlDeviceGetUtilizationRates(handle)
    print(f"  GPU: {util.gpu}%  Mem: {util.memory}%")

    # VRAM
    mem = nvmlDeviceGetMemoryInfo(handle)
    print(f"  VRAM: {mem.used // (1024**2)} / {mem.total // (1024**2)} MB")

    # Clocks
    gfx_clk = nvmlDeviceGetClockInfo(handle, NVML_CLOCK_GRAPHICS)
    mem_clk = nvmlDeviceGetClockInfo(handle, NVML_CLOCK_MEM)
    print(f"  Clock GPU: {gfx_clk} MHz, Mem: {mem_clk} MHz")

    # Power
    power = nvmlDeviceGetPowerUsage(handle) / 1000
    print(f"  Power: {power:.1f} W")

    # Fan
    try:
        fan = nvmlDeviceGetFanSpeed(handle)
        print(f"  Fan: {fan}%")
    except NVMLError:
        print("  Fan: N/A")

    # Encoder/Decoder
    enc_util, _ = nvmlDeviceGetEncoderUtilization(handle)
    dec_util, _ = nvmlDeviceGetDecoderUtilization(handle)
    print(f"  Encoder: {enc_util}%, Decoder: {dec_util}%")

    # PCIe
    tx = nvmlDeviceGetPcieThroughput(handle, NVML_PCIE_UTIL_TX_BYTES)
    rx = nvmlDeviceGetPcieThroughput(handle, NVML_PCIE_UTIL_RX_BYTES)
    print(f"  PCIe TX: {tx} KB/s, RX: {rx} KB/s")

nvmlShutdown()
```

#### Avantages

- Bindings officiels NVIDIA
- Memes metriques que NVML C
- Installation simple (`pip install nvidia-ml-py`)
- Gestion des erreurs via exceptions Python

#### Inconvenients

- NVIDIA uniquement
- Necessite le driver NVIDIA installe
- Fan speed en % uniquement (pas RPM reel)

#### Fiabilite/Precision

**Identique a NVML** (c'est un wrapper direct).

---

### 5.3 GPUtil (Python)

#### Presentation

GPUtil est un module Python simple pour obtenir le statut des GPU NVIDIA en utilisant `nvidia-smi` en arriere-plan.

#### Mecanisme technique

GPUtil lance `nvidia-smi --query-gpu=... --format=csv` en subprocess et parse la sortie CSV. C'est donc un wrapper autour de nvidia-smi, pas un binding NVML.

#### Metriques accessibles

| Metrique | Disponible |
|----------|------------|
| GPU Utilisation | Oui |
| VRAM usage | Oui |
| Temperature | Oui |
| Clocks | Non |
| Fan speed | Non |
| Power | Non |

#### Exemple

```python
import GPUtil

gpus = GPUtil.getGPUs()
for gpu in gpus:
    print(f"{gpu.name}")
    print(f"  Load: {gpu.load * 100:.1f}%")
    print(f"  Temp: {gpu.temperature} C")
    print(f"  VRAM: {gpu.memoryUsed} / {gpu.memoryTotal} MB")
    print(f"  VRAM Free: {gpu.memoryFree} MB")
```

#### Avantages

- Installation triviale (`pip install gputil`)
- API tres simple
- Fonction `getAvailable()` pour selectionner le GPU le moins charge

#### Inconvenients

- **NVIDIA uniquement**
- Metriques limitees (pas de clocks, fan, power)
- Overhead subprocess (nvidia-smi) a chaque appel
- Plus maintenu activement
- Latence elevee (fork+exec a chaque requete)

#### Fiabilite/Precision

**Moyenne**. La precision depend de nvidia-smi, mais le parsing CSV peut casser entre versions du driver. La latence de subprocess rend cette solution inadaptee au monitoring haute frequence.

---

### 5.4 py3nvml (Python)

#### Presentation

py3nvml est un fork independant des bindings NVML pour Python 3, maintenu separement de nvidia-ml-py. Il ajoute des utilitaires supplementaires comme la selection automatique de GPU libres.

#### Fonctionnalites supplementaires

```python
from py3nvml.py3nvml import *

# Selection automatique de GPU libres
from py3nvml.utils import grab_gpus
num_grabbed = grab_gpus(num_gpus=2, gpu_fraction=0.95)
# Configure CUDA_VISIBLE_DEVICES automatiquement
```

#### Avantages

- Memes metriques que pynvml/nvidia-ml-py
- Utilitaire `grab_gpus()` pour la selection de GPU
- Maintenu activement

#### Inconvenients

- Fonctionnellement equivalent a nvidia-ml-py
- NVIDIA uniquement
- Potentielle confusion avec les autres packages NVML Python

---

### 5.5 sysinfo (Rust)

#### Presentation

`sysinfo` est le crate Rust le plus populaire pour les informations systeme : CPU, memoire, disques, reseau, processus, et composants (capteurs temperature).

#### GPU : capacites

**`sysinfo` ne supporte PAS les metriques GPU.** Le crate ne fournit aucune information sur :
- L'utilisation GPU
- Les clocks GPU
- La VRAM
- La temperature GPU (sauf indirectement via les composants hwmon)

#### Ce que sysinfo fournit en lien avec le GPU

```rust
use sysinfo::Components;

// Composants (capteurs temperature, peut inclure GPU via hwmon)
let components = Components::new_with_refreshed_list();
for component in &components {
    println!("{}: {}C", component.label(), component.temperature());
}
// Peut afficher "amdgpu edge: 45.0C" sur Linux avec GPU AMD
```

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui |
| Linux | Oui |
| macOS | Oui |
| FreeBSD | Oui |

#### Avantages

- Crate Rust de reference pour le monitoring systeme
- Tres mature (millions de telechargements)
- Cross-platform
- API ergonomique et safe

#### Inconvenients

- **Aucune metrique GPU**
- Metriques temperature GPU limitees et indirectes
- Pas de VRAM, clocks, utilisation

#### Fiabilite/Precision

**Excellente** pour CPU, RAM, disque. Non applicable pour GPU.

---

### 5.6 nvml-wrapper (Rust)

#### Presentation

`nvml-wrapper` est un wrapper Rust safe et ergonomique autour de NVML. C'est le crate recommande pour acceder aux metriques GPU NVIDIA depuis Rust.

#### Mecanisme technique

Le crate utilise `libloading` pour charger dynamiquement la bibliotheque NVML au runtime. Il ne necessite pas de linkage statique au build time.

#### Exemple

```rust
use nvml_wrapper::Nvml;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nvml = Nvml::init()?;

    let device_count = nvml.device_count()?;
    for i in 0..device_count {
        let device = nvml.device_by_index(i)?;

        let name = device.name()?;
        println!("=== {} ===", name);

        // Temperature
        let temp = device.temperature(
            nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu
        )?;
        println!("  Temp: {} C", temp);

        // Utilisation
        let util = device.utilization_rates()?;
        println!("  GPU: {}%, Mem: {}%", util.gpu, util.memory);

        // VRAM
        let mem = device.memory_info()?;
        println!("  VRAM: {} / {} MB",
                 mem.used / (1024 * 1024),
                 mem.total / (1024 * 1024));

        // Fan speed
        match device.fan_speed(0) {
            Ok(speed) => println!("  Fan: {}%", speed),
            Err(_) => println!("  Fan: N/A"),
        }

        // Power
        let power = device.power_usage()?;
        println!("  Power: {:.1} W", power as f64 / 1000.0);

        // Encoder
        let (enc_util, _) = device.encoder_utilization()?;
        println!("  Encoder: {}%", enc_util);
    }

    Ok(())
}
```

**Cargo.toml** :
```toml
[dependencies]
nvml-wrapper = "0.11"
```

#### Avantages

- API safe et ergonomique (Rust idiomatique)
- Chargement dynamique (pas besoin de NVML au build time)
- Support serde (serialisation/deserialisation)
- Backward compatible
- Support Windows + Linux

#### Inconvenients

- NVIDIA uniquement
- Necessite le driver NVIDIA installe
- MSRV : Rust 1.60+

#### Fiabilite/Precision

**Identique a NVML** (wrapper direct avec types Rust safe).

---

### 5.7 OSHI (Java)

#### Presentation

OSHI (Operating System and Hardware Information) est une bibliotheque Java pour recuperer les informations systeme et hardware sans dependance native. Elle utilise JNA (Java Native Access) pour appeler les APIs systeme.

#### GPU : capacites

OSHI fournit les informations **statiques** des cartes graphiques via `GraphicsCard` :
- Nom du GPU
- Vendor
- VRAM (taille)
- Version du driver
- Device ID

**OSHI ne fournit PAS de metriques GPU en temps reel** (utilisation, temperature, clocks, power, fan speed).

#### Exemple

```java
import oshi.SystemInfo;
import oshi.hardware.GraphicsCard;
import oshi.hardware.HardwareAbstractionLayer;

public class GpuInfo {
    public static void main(String[] args) {
        SystemInfo si = new SystemInfo();
        HardwareAbstractionLayer hal = si.getHardware();

        for (GraphicsCard gpu : hal.getGraphicsCards()) {
            System.out.println("GPU: " + gpu.getName());
            System.out.println("  Vendor: " + gpu.getVendor());
            System.out.println("  VRAM: " + gpu.getVRam() / (1024*1024) + " MB");
            System.out.println("  Version: " + gpu.getVersionInfo());
            System.out.println("  Device ID: " + gpu.getDeviceId());
        }
    }
}
```

**Maven** :
```xml
<dependency>
    <groupId>com.github.oshi</groupId>
    <artifactId>oshi-core</artifactId>
    <version>6.6.5</version>
</dependency>
```

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui |
| Linux | Oui |
| macOS | Oui |
| Solaris | Oui |

#### Avantages

- Pure Java (JNA, pas de compilation native)
- Cross-platform
- API unifiee pour tout le hardware
- Tres mature et activement maintenu
- Nouveau module FFM (Java 25+)

#### Inconvenients

- **Pas de metriques GPU en temps reel**
- Informations GPU statiques uniquement (nom, VRAM, vendor)
- Pour du monitoring GPU runtime, il faut combiner avec des outils natifs
- Capteurs de temperature dependants du hardware

#### Fiabilite/Precision

**Excellente** pour les informations statiques. Non applicable pour les metriques dynamiques GPU.

---

### 5.8 systeminformation (Node.js)

#### Presentation

`systeminformation` est la bibliotheque Node.js la plus complete pour les informations systeme. Contrairement a psutil et sysinfo, elle inclut un support **natif** pour les informations graphiques/GPU.

#### GPU : capacites

```javascript
const si = require('systeminformation');

async function gpuInfo() {
    const graphics = await si.graphics();

    for (const ctrl of graphics.controllers) {
        console.log(`GPU: ${ctrl.model}`);
        console.log(`  Vendor: ${ctrl.vendor}`);
        console.log(`  VRAM: ${ctrl.vram} MB`);
        console.log(`  Temp: ${ctrl.temperatureGpu} C`);
        console.log(`  Fan: ${ctrl.fanSpeed}%`);
        console.log(`  Clock Core: ${ctrl.clockCore} MHz`);
        console.log(`  Clock Memory: ${ctrl.clockMemory} MHz`);
        console.log(`  Utilization: ${ctrl.utilizationGpu}%`);
        console.log(`  Mem Util: ${ctrl.utilizationMemory}%`);
        console.log(`  Power Draw: ${ctrl.powerDraw} W`);
        console.log(`  Power Limit: ${ctrl.powerLimit} W`);
    }

    for (const display of graphics.displays) {
        console.log(`Display: ${display.model} (${display.resolutionX}x${display.resolutionY})`);
    }
}

gpuInfo();
```

#### Metriques GPU disponibles

| Metrique | Propriete | NVIDIA | AMD | Intel |
|----------|-----------|--------|-----|-------|
| GPU model | `model` | Oui | Oui | Oui |
| VRAM | `vram` | Oui | Oui | Oui |
| Temperature | `temperatureGpu` | Oui | Partiel | Partiel |
| Fan speed | `fanSpeed` | Oui | Partiel | Non |
| Clock core | `clockCore` | Oui | Partiel | Non |
| Clock memory | `clockMemory` | Oui | Partiel | Non |
| Utilisation | `utilizationGpu` | Oui | Partiel | Non |
| Power draw | `powerDraw` | Oui | Partiel | Non |

**Implementation interne** : systeminformation utilise nvidia-smi (NVIDIA), le driver AMD (sysfs/WMI), et WMI/sysfs (Intel) en arriere-plan. La completude des metriques depend donc du vendor.

#### OS supportes

| OS | Support |
|----|---------|
| Windows | Oui |
| Linux | Oui |
| macOS | Oui (partiel) |

#### Avantages

- **Support GPU natif** (contrairement a psutil/sysinfo/OSHI)
- Metriques dynamiques (temperature, utilisation, clocks)
- Cross-platform
- API Promise/async moderne
- Tres activement maintenu

#### Inconvenients

- Metriques GPU via subprocess (nvidia-smi, etc.) donc overhead
- Support AMD et Intel incomplet (meilleur sur NVIDIA)
- Valeurs parfois `-1` quand le hardware ne supporte pas
- Ecosysteme Node.js (pas ideal pour app desktop native)
- Temperature parfois non disponible sur Windows/macOS

#### Fiabilite/Precision

**Bonne pour NVIDIA**, variable pour AMD/Intel. La precision depend de l'outil backend utilise. Les valeurs `-1` indiquent une metrique non disponible.

---

### 5.9 libstatgrab (C)

#### Presentation

libstatgrab est une bibliotheque C cross-platform pour les statistiques systeme (CPU, memoire, disque, reseau, processus).

#### GPU : capacites

**libstatgrab ne supporte PAS les metriques GPU.** L'API couvre :
- `sg_get_cpu_stats()` - CPU
- `sg_get_mem_stats()` - Memoire
- `sg_get_disk_io_stats()` - Disque I/O
- `sg_get_network_io_stats()` - Reseau
- `sg_get_process_stats()` - Processus

Aucune fonction GPU n'est fournie.

#### OS supportes

Linux, FreeBSD, NetBSD, OpenBSD, Solaris, DragonFly BSD, HP-UX, AIX. **Pas de support Windows natif.**

#### Avantages

- Bibliotheque C legere et mature
- Large support Unix/BSD
- License LGPL
- Bindings Python (pystatgrab) et Perl

#### Inconvenients

- **Aucune metrique GPU**
- **Pas de support Windows**
- Derniere release : juillet 2021 (maintenance minimale)
- Focus exclusif sur les metriques systeme classiques

---

## 6. Tableau comparatif final

### APIs vendor-specifiques

| Critere | NVML | NVAPI | XNVCtrl | ADL | ADLX | AMD SMI | Level Zero | i915 sysfs |
|---------|------|-------|---------|-----|------|---------|------------|------------|
| **Vendor** | NVIDIA | NVIDIA | NVIDIA | AMD | AMD | AMD | Intel | Intel |
| **OS** | Win+Lin | Win | Lin (X11) | Win+Lin | Win (Lin WIP) | Lin | Win+Lin | Lin |
| **Langage** | C | C | C | C | C/C++ | C++/Python | C | Tout |
| **GPU Clock** | Oui | Oui | Oui | Oui | Oui | Oui | Oui | Oui |
| **Utilisation** | Oui | Oui | Partiel | Oui | Oui | Oui | Oui* | Non** |
| **VRAM** | Oui | Oui | Non | Partiel | Oui | Oui | Oui | Non |
| **Temperature** | Oui | Oui | Oui | Oui | Oui | Oui | Oui | Partiel |
| **Fan (%)** | Oui | Oui | Oui | Oui | Oui | Oui | Oui | Non |
| **Fan (RPM)** | Non | **Oui** | **Oui** | **Oui** | **Oui** | **Oui** | Oui | Non |
| **Power** | Oui | Partiel | Non | Partiel | Oui | Oui | Oui | Non |
| **PCIe** | Oui | Oui | Non | Non | Non | Oui | Oui | Non |
| **Encoder/Dec** | Oui | Non | Non | Non | Non | Non | Partiel | Non |
| **Privileges** | User | User | X11 | User | User | video grp | video grp | User |
| **Complexite** | Faible | Moyenne | Moyenne | Elevee | Moyenne | Faible | Elevee | Tres faible |

(*) Via metric groups, complexe a mettre en place
(**) Necessite intel_gpu_top ou perf

### Bibliotheques cross-platform

| Critere | psutil | pynvml | GPUtil | sysinfo (Rust) | nvml-wrapper | OSHI | systeminformation | libstatgrab |
|---------|--------|--------|--------|----------------|--------------|------|-------------------|-------------|
| **Langage** | Python | Python | Python | Rust | Rust | Java | Node.js | C |
| **OS** | All | Win+Lin | Win+Lin | All | Win+Lin | All | All | Unix |
| **GPU Metrics** | **Non** | NVIDIA | NVIDIA | **Non** | NVIDIA | **Non** | **Oui*** | **Non** |
| **GPU Clock** | Non | Oui | Non | Non | Oui | Non | Oui* | Non |
| **GPU Util** | Non | Oui | Oui | Non | Oui | Non | Oui* | Non |
| **VRAM** | Non | Oui | Oui | Non | Oui | Statique | Oui | Non |
| **Temperature** | Indirect** | Oui | Oui | Indirect** | Oui | Non | Oui* | Non |
| **Fan Speed** | Non | Oui (%) | Non | Non | Oui (%) | Non | Oui* | Non |
| **Power** | Non | Oui | Non | Non | Oui | Non | Oui* | Non |
| **Multi-vendor** | N/A | Non | Non | N/A | Non | N/A | **Oui** | N/A |
| **Maturite** | Excellente | Bonne | Faible | Excellente | Bonne | Excellente | Bonne | Bonne |

(*) Meilleur support NVIDIA, partiel AMD/Intel
(**) Via capteurs hwmon sur Linux, peut inclure temperature GPU

---

## 7. Recommandations architecturales

### Pour une application desktop Windows + Linux multi-vendor

L'absence de solution cross-platform unifiee pour les metriques GPU impose une architecture a couches d'abstraction :

```
┌──────────────────────────────────────────┐
│         Application Layer                │
│    (interface uniforme : GpuMetrics)     │
├──────────────────────────────────────────┤
│         Abstraction Layer                │
│   detect_vendor() → backend adapter      │
├──────┬──────────┬────────────────────────┤
│NVIDIA│   AMD    │        Intel           │
├──────┼──────────┼────────────────────────┤
│ NVML │ ADLX(W)  │ Level Zero (Sysman)   │
│      │ sysfs(L) │ sysfs (Linux)         │
│      │ ADL(both)│                        │
└──────┴──────────┴────────────────────────┘
```

### Stack recommandee par langage

#### C/C++ (natif desktop)

```
NVIDIA : NVML (cross-platform) + NVAPI (Windows, RPM)
AMD    : ADLX (Windows) + sysfs amdgpu (Linux)
Intel  : Level Zero Sysman (cross-platform) + sysfs i915 (Linux)
```

#### Rust

```
NVIDIA : nvml-wrapper (cross-platform)
AMD    : lecture sysfs directe (Linux) + ADL via FFI (Windows)
Intel  : Level Zero via FFI + sysfs (Linux)
```

#### Python

```
NVIDIA : nvidia-ml-py / pynvml
AMD    : amdsmi (Linux) + subprocess amd-smi / ADL ctypes (Windows)
Intel  : subprocess intel_gpu_top (Linux)
Fallback: psutil sensors_temperatures() pour temperature indirecte
```

### Points d'attention

1. **Detection dynamique du vendor** : utiliser `lspci` (Linux) ou WMI/SetupAPI (Windows) pour determiner le GPU present avant de charger le backend appropriate.

2. **Chargement dynamique des libraries** : toujours charger NVML, ADL, Level Zero dynamiquement (`dlopen`/`LoadLibrary`) pour eviter les erreurs si la bibliotheque n'est pas installee.

3. **Fallback gracieux** : si une metrique n'est pas disponible, retourner une valeur sentinelle (`-1` ou `None`) plutot que de crasher.

4. **Frequence de polling** : la plupart des APIs GPU supportent un polling a 1-10 Hz sans overhead notable. Au-dela de 100 Hz, utiliser NVML `dmon` ou des compteurs hardware continus.

5. **Fan RPM** : c'est la metrique la plus problematique. Sur NVIDIA, le RPM reel necessite NVAPI (Windows) ou XNVCtrl (Linux). NVML ne donne que le % d'intention.

6. **Encoder/Decoder** : uniquement NVML supporte ces metriques de facon fiable. AMD et Intel n'exposent pas cette information via leurs APIs de management.

---

## 8. Sources

### NVIDIA
- [NVML API Reference](https://docs.nvidia.com/deploy/nvml-api/index.html)
- [NVIDIA Management Library (NVML)](https://developer.nvidia.com/management-library-nvml)
- [NVML Examples (GitHub)](https://github.com/mnicely/nvml_examples)
- [NVIDIA NVML GPU Statistics - Lei Mao](https://leimao.github.io/blog/NVIDIA-NVML-GPU-Statistics/)
- [NVML sur GPU Glossary (Modal)](https://modal.com/gpu-glossary/host-software/nvml)
- [NVCtrl.h (XNVCtrl)](https://github.com/NVIDIA/nvidia-settings/blob/main/src/libXNVCtrl/NVCtrl.h)
- [NVAPI GPU Cooler API](https://docs.nvidia.com/gameworks/content/gameworkslibrary/coresdk/nvapi/group__gpucooler.html)
- [Monitoring NVIDIA GPUs using API - Medium](https://medium.com/devoops-and-universe/monitoring-nvidia-gpus-cd174bf89311)

### AMD
- [ADL Documentation](https://gpuopen-librariesandsdks.github.io/adl/)
- [AMD Display Library (GPUOpen)](https://gpuopen.com/adl/)
- [ADLX SDK (GPUOpen)](https://gpuopen.com/adlx/)
- [ADLX GPU Metrics C++ Sample](https://gpuopen.com/manuals/ADLX/adlx-cpp__perf_g_p_u_metrics/)
- [ADLX GitHub Repository](https://github.com/GPUOpen-LibrariesAndSDKs/ADLX)
- [AMD SMI Documentation](https://rocm.docs.amd.com/projects/amdsmi/en/latest/)
- [ROCm SMI Library Documentation](https://rocm.docs.amd.com/projects/rocm_smi_lib/en/latest/)
- [AMDGPU Thermal - Kernel Docs](https://docs.kernel.org/gpu/amdgpu/thermal.html)
- [AMDGPU gpu_metrics decoder (GitHub Gist)](https://gist.github.com/leuc/e45f4dc64dc1db870e4bad1c436228bb)
- [AMD SMI Deep Dive - ROCm Blog](https://rocm.blogs.amd.com/software-tools-optimization/amd-smi-overview/README.html)

### Intel
- [Level Zero Specification](https://oneapi-src.github.io/level-zero-spec/)
- [Intel PTI GPU - Metrics Collection](https://github.com/intel/pti-gpu/blob/master/chapters/metrics_collection/LevelZero.md)
- [IGCL Control Library](https://intel.github.io/drivers.gpu.control-library/Control/INTRO.html)
- [Intel Compute Runtime (GitHub)](https://github.com/intel/compute-runtime)
- [i915 sysfs source (kernel)](https://github.com/torvalds/linux/blob/master/drivers/gpu/drm/i915/i915_sysfs.c)
- [Intel GPU Frequency Blog](https://bwidawsk.net/blog/2015/5/a-bit-on-intel-gpu-frequency/)
- [Linux Kernel 7.0 Intel GPU Temp](https://www.webpronews.com/linux-kernel-7-0-enhances-intel-gpu-temp-monitoring-with-i915-driver/)

### Bibliotheques cross-platform
- [psutil Documentation](https://psutil.readthedocs.io/)
- [psutil GPU Issue #526](https://github.com/giampaolo/psutil/issues/526)
- [nvidia-ml-py (PyPI)](https://pypi.org/project/nvidia-ml-py/)
- [py3nvml (GitHub)](https://github.com/fbcotter/py3nvml)
- [GPUtil (GitHub)](https://github.com/anderskm/gputil)
- [nvml-wrapper Rust crate](https://docs.rs/nvml-wrapper/latest/nvml_wrapper/)
- [sysinfo Rust crate](https://docs.rs/sysinfo/latest/sysinfo/)
- [OSHI (GitHub)](https://github.com/oshi/oshi)
- [systeminformation (npm)](https://www.npmjs.com/package/systeminformation)
- [libstatgrab](https://libstatgrab.org/)
