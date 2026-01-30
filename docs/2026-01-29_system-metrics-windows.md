---
title: "Capture de métriques système - Windows"
date: 2026-01-29
topic: system-metrics/windows
sources:
  - https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-processor
  - https://learn.microsoft.com/en-us/windows/win32/perfctrs/using-the-pdh-functions-to-consume-counter-data
  - https://learn.microsoft.com/en-us/windows/win32/power/processor-power-information-str
  - https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation
  - https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex
  - https://github.com/LibreHardwareMonitor/LibreHardwareMonitor
  - https://github.com/openhardwaremonitor/openhardwaremonitor
  - https://github.com/intel/pcm
  - https://github.com/erenpinaz/CallNtPowerInformation
  - https://github.com/cyring/WinMSR
  - https://github.com/cocafe/msr-utility
  - https://www.geoffchappell.com/studies/windows/km/ntoskrnl/events/microsoft-windows-kernel-processor-power.htm
  - https://www.dima.to/blog/calculating-the-core-frequencies-of-a-modern-intel-cpu-with-clock-varying-features-in-visual-c-on-a-windows-machine/
  - https://github.com/intel/intel-cpu-frequency-library
  - https://wiki.osdev.org/Model_Specific_Registers
  - https://en.wikipedia.org/wiki/Serial_presence_detect
  - https://www.passmark.com/products/rammon/index.php
  - https://maidavale.org/blog/investigating-asus-wmi-sensors-from-powershell/
  - https://deepwiki.com/openhardwaremonitor/openhardwaremonitor/5.2-lpc-and-superio-communication
status: final
---

# Capture de métriques système sous Windows

## Table des matières

1. [Introduction](#1-introduction)
2. [Méthode 1 : WMI (Windows Management Instrumentation)](#2-méthode-1--wmi)
3. [Méthode 2 : PDH (Performance Data Helper)](#3-méthode-2--pdh)
4. [Méthode 3 : CallNtPowerInformation / NtQuerySystemInformation](#4-méthode-3--callntpowerinformation--ntquerysysteminformation)
5. [Méthode 4 : LibreHardwareMonitor](#5-méthode-4--librehardwaremonitor)
6. [Méthode 5 : OpenHardwareMonitor](#6-méthode-5--openhardwaremonitor)
7. [Méthode 6 : Accès direct MSR / CPUID (Ring 0)](#7-méthode-6--accès-direct-msr--cpuid-ring-0)
8. [Méthode 7 : ETW (Event Tracing for Windows)](#8-méthode-7--etw-event-tracing-for-windows)
9. [Méthode 8 : Windows Performance Counters (sans PDH)](#9-méthode-8--windows-performance-counters-sans-pdh)
10. [Méthode 9 : GlobalMemoryStatusEx / GetPerformanceInfo](#10-méthode-9--globalmemorystatusex--getperformanceinfo)
11. [Méthode 10 : Intel PCM (Performance Counter Monitor)](#11-méthode-10--intel-pcm)
12. [Méthode 11 : Accès SMBus / SPD pour timings RAM](#12-méthode-11--accès-smbus--spd)
13. [Méthode 12 : IPMI (serveurs)](#13-méthode-12--ipmi)
14. [Couverture détaillée par métrique](#14-couverture-détaillée-par-métrique)
15. [Tableau comparatif global](#15-tableau-comparatif-global)
16. [Recommandations architecturales](#16-recommandations-architecturales)
17. [Sources](#17-sources)

---

## 1. Introduction

Ce document recense de manière exhaustive les méthodes disponibles sous Windows pour capturer des métriques système hardware : CPU, RAM, ventilateurs, températures, voltages, etc. Pour chaque méthode, on documente le mécanisme technique, les langages supportés, les privilèges requis, les métriques accessibles, ainsi que les avantages et inconvénients.

**Contexte cible** : application desktop Windows devant lire en temps réel CPU freq, per-core utilization, GPU clock, fan speed, RAM, latencies, etc.

---

## 2. Méthode 1 : WMI

### 2.1 Mécanisme technique

WMI (Windows Management Instrumentation) est l'implémentation Microsoft du standard CIM (Common Information Model). Il expose des classes via des namespaces (principalement `root\CIMV2` et `root\WMI`) qui permettent de requêter des informations système via des requêtes WQL (SQL-like).

WMI fonctionne en mode client-serveur : le service `winmgmt` (WMI service) gère les requêtes et interroge les providers WMI correspondants (drivers OEM, providers système, etc.).

### 2.2 Classes principales

| Classe WMI | Namespace | Description |
|---|---|---|
| `Win32_Processor` | `root\CIMV2` | Processeur : fréquence, nb de coeurs, load |
| `Win32_PerfFormattedData_PerfOS_Processor` | `root\CIMV2` | Utilisation CPU par coeur (%) |
| `Win32_PerfFormattedData_Counters_ProcessorInformation` | `root\CIMV2` | Info processeur étendue (fréquence, parking) |
| `MSAcpi_ThermalZoneTemperature` | `root\WMI` | Température zone thermique ACPI |
| `Win32_PhysicalMemory` | `root\CIMV2` | RAM : capacité, fréquence, type |
| `Win32_OperatingSystem` | `root\CIMV2` | Mémoire totale/disponible |
| `Win32_Fan` | `root\CIMV2` | Ventilateurs (nécessite provider OEM) |
| `Win32_TemperatureProbe` | `root\CIMV2` | Sondes de température (nécessite provider OEM) |
| `Win32_VoltageProbe` | `root\CIMV2` | Sondes de voltage (nécessite provider OEM) |

### 2.3 Exemples de code

**PowerShell** :
```powershell
# CPU - Fréquence et info générale (1 instance par socket)
Get-CimInstance Win32_Processor | Select-Object Name, CurrentClockSpeed, MaxClockSpeed,
    NumberOfCores, NumberOfLogicalProcessors, LoadPercentage

# CPU - Utilisation par coeur (%)
Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor |
    Select-Object Name, PercentProcessorTime

# Température ACPI (admin requis, résultat en Kelvin × 10)
$temp = Get-CimInstance MSAcpi_ThermalZoneTemperature -Namespace "root/wmi"
$celsius = ($temp.CurrentTemperature / 10) - 273.15
Write-Host "Température zone thermique: $celsius °C"

# RAM - Info physique
Get-CimInstance Win32_PhysicalMemory |
    Select-Object Manufacturer, PartNumber, Speed, ConfiguredClockSpeed, Capacity

# RAM - Usage global
Get-CimInstance Win32_OperatingSystem |
    Select-Object TotalVisibleMemorySize, FreePhysicalMemory
```

**C# (System.Management)** :
```csharp
using System.Management;

// CPU fréquence
var searcher = new ManagementObjectSearcher(
    "SELECT CurrentClockSpeed, MaxClockSpeed, LoadPercentage FROM Win32_Processor");
foreach (ManagementObject obj in searcher.Get())
{
    Console.WriteLine($"Current: {obj["CurrentClockSpeed"]} MHz");
    Console.WriteLine($"Max: {obj["MaxClockSpeed"]} MHz");
    Console.WriteLine($"Load: {obj["LoadPercentage"]}%");
}

// CPU utilisation par coeur
var cpuSearcher = new ManagementObjectSearcher(
    "SELECT Name, PercentProcessorTime FROM Win32_PerfFormattedData_PerfOS_Processor");
foreach (ManagementObject obj in cpuSearcher.Get())
{
    Console.WriteLine($"Core {obj["Name"]}: {obj["PercentProcessorTime"]}%");
}
```

**Python (wmi)** :
```python
import wmi

c = wmi.WMI()

# CPU info
for cpu in c.Win32_Processor():
    print(f"Freq: {cpu.CurrentClockSpeed} MHz, Max: {cpu.MaxClockSpeed} MHz")
    print(f"Cores: {cpu.NumberOfCores}, Load: {cpu.LoadPercentage}%")

# Per-core usage
for core in c.Win32_PerfFormattedData_PerfOS_Processor():
    print(f"Core {core.Name}: {core.PercentProcessorTime}%")

# RAM
for mem in c.Win32_PhysicalMemory():
    print(f"Speed: {mem.Speed} MHz, Capacity: {int(mem.Capacity) // (1024**3)} GB")
```

**C++ (COM)** :
```cpp
#include <Wbemidl.h>
#include <comdef.h>
#pragma comment(lib, "wbemuuid.lib")

// Initialisation COM + connexion WMI (simplifié)
CoInitializeEx(0, COINIT_MULTITHREADED);
IWbemLocator* pLoc = nullptr;
CoCreateInstance(CLSID_WbemLocator, 0, CLSCTX_INPROC_SERVER,
    IID_IWbemLocator, (LPVOID*)&pLoc);

IWbemServices* pSvc = nullptr;
pLoc->ConnectServer(_bstr_t(L"ROOT\\CIMV2"), NULL, NULL, 0, NULL, 0, 0, &pSvc);

CoSetProxyBlanket(pSvc, RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE, NULL,
    RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE, NULL, EOAC_NONE);

IEnumWbemClassObject* pEnum = nullptr;
pSvc->ExecQuery(bstr_t("WQL"),
    bstr_t("SELECT CurrentClockSpeed FROM Win32_Processor"),
    WBEM_FLAG_FORWARD_ONLY, NULL, &pEnum);

IWbemClassObject* pObj = nullptr;
ULONG uReturn = 0;
while (pEnum->Next(WBEM_INFINITE, 1, &pObj, &uReturn) == S_OK) {
    VARIANT vtProp;
    pObj->Get(L"CurrentClockSpeed", 0, &vtProp, 0, 0);
    printf("CPU Freq: %d MHz\n", vtProp.intVal);
    VariantClear(&vtProp);
    pObj->Release();
}
```

### 2.4 Privilèges requis

| Classe | Privilège |
|---|---|
| `Win32_Processor`, `Win32_PhysicalMemory` | Utilisateur standard |
| `Win32_PerfFormattedData_*` | Utilisateur standard |
| `MSAcpi_ThermalZoneTemperature` | **Administrateur** |
| `Win32_Fan`, `Win32_TemperatureProbe` | Utilisateur (mais nécessite provider OEM) |

### 2.5 Métriques accessibles

| Métrique | Disponible | Précision |
|---|---|---|
| CPU fréquence (par socket) | Oui (`CurrentClockSpeed`) | Moyenne - reflète P-state, pas la fréquence instantanée |
| CPU fréquence par coeur | **Non** | N/A |
| CPU utilisation par coeur | Oui (`Win32_PerfFormattedData_PerfOS_Processor`) | Bonne |
| CPU température par coeur | **Non** | N/A |
| CPU température zone ACPI | Partiel (`MSAcpi_ThermalZoneTemperature`) | Faible - souvent température carte mère, pas CPU |
| CPU power | **Non** | N/A |
| RAM fréquence | Oui (`Win32_PhysicalMemory.Speed`) | Bonne (valeur SPD nominale) |
| RAM capacité/usage | Oui | Bonne |
| RAM timings (CAS, etc.) | **Non** | N/A |
| Ventilateurs RPM | Théorique (`Win32_Fan`) | Très mauvaise - retourne vide sur 95% des machines |
| Températures carte mère | Théorique | Très mauvaise - dépend provider OEM |
| Voltages | Théorique (`Win32_VoltageProbe`) | Très mauvaise |

### 2.6 Avantages

- API standard Windows, aucune dépendance externe
- Disponible sur toutes les versions de Windows (XP+)
- Supporte tous les langages via COM (C++, C#, Python, PowerShell, VBScript, Rust via COM)
- Pas de driver kernel requis pour la plupart des classes
- Bon pour les métriques de base (CPU load, RAM usage)

### 2.7 Inconvénients

- **Lent** : 20-30x plus lent que PDH pour les mêmes données
- **Pas de per-core frequency** : 1 instance = 1 socket physique
- **Pas de température CPU réelle** : `MSAcpi_ThermalZoneTemperature` retourne la zone thermique ACPI, souvent différente du CPU
- Les classes hardware (`Win32_Fan`, `Win32_TemperatureProbe`, `Win32_VoltageProbe`) nécessitent des providers OEM rarement implémentés
- Instable sur certains systèmes (timeouts, freezes)
- `CurrentClockSpeed` reflète le P-state actuel, pas la fréquence boost réelle

### 2.8 Fiabilité / Précision

**Moyenne à faible** selon la métrique. Fiable pour CPU load et RAM usage. Peu fiable pour les températures (zone ACPI != CPU). Inutilisable pour fans/voltages sur la majorité du hardware consumer.

---

## 3. Méthode 2 : PDH

### 3.1 Mécanisme technique

PDH (Performance Data Helper) est une API C/C++ Windows qui fournit un accès haut niveau aux compteurs de performance système. Elle encapsule le registry de performance (`HKEY_PERFORMANCE_DATA`) et simplifie la collecte, le formatage et le logging des données.

PDH fonctionne par **queries** : on ouvre une query, on y ajoute des compteurs (counter paths), on collecte les données périodiquement, puis on lit les valeurs formatées.

**Important** : De nombreux compteurs (type rate) nécessitent **deux échantillons** séparés d'au moins 1 seconde pour calculer une valeur significative.

### 3.2 Compteurs principaux

| Counter Path | Description |
|---|---|
| `\Processor(*)\% Processor Time` | Utilisation CPU par coeur |
| `\Processor(_Total)\% Processor Time` | Utilisation CPU totale |
| `\Processor Information(*)\Processor Frequency` | Fréquence par processeur logique (MHz) |
| `\Processor Information(*)\% Processor Performance` | Performance relative (%) |
| `\Processor Information(*)\Parking Status` | État parking du coeur |
| `\Memory\Available MBytes` | RAM disponible |
| `\Memory\% Committed Bytes In Use` | % mémoire committée |
| `\Memory\Pages/sec` | Pages mémoire par seconde |
| `\Memory\Cache Bytes` | Taille du cache fichier |
| `\Process(*)\% Processor Time` | CPU par processus |

### 3.3 Exemples de code

**C++ (natif)** :
```cpp
#include <windows.h>
#include <pdh.h>
#include <pdhmsg.h>
#pragma comment(lib, "pdh.lib")

int main() {
    PDH_HQUERY query;
    PDH_HCOUNTER counterCpuTotal;
    PDH_HCOUNTER counterFreq;
    PDH_FMT_COUNTERVALUE value;

    // Ouvrir la query
    PdhOpenQuery(NULL, 0, &query);

    // Ajouter les compteurs
    PdhAddEnglishCounter(query,
        L"\\Processor(_Total)\\% Processor Time", 0, &counterCpuTotal);
    PdhAddEnglishCounter(query,
        L"\\Processor Information(0,0)\\Processor Frequency", 0, &counterFreq);

    // Premier échantillon (baseline)
    PdhCollectQueryData(query);
    Sleep(1000);

    // Second échantillon (calcul du delta)
    PdhCollectQueryData(query);

    // Lecture CPU total
    PdhGetFormattedCounterValue(counterCpuTotal, PDH_FMT_DOUBLE, NULL, &value);
    printf("CPU Total: %.1f%%\n", value.doubleValue);

    // Lecture fréquence
    PdhGetFormattedCounterValue(counterFreq, PDH_FMT_LONG, NULL, &value);
    printf("CPU Freq core 0: %ld MHz\n", value.longValue);

    PdhCloseQuery(query);
    return 0;
}
```

**C++ - Lecture per-core** :
```cpp
// Utilisation par coeur avec wildcard
PDH_HCOUNTER counterPerCore;
PdhAddEnglishCounter(query,
    L"\\Processor(*)\\% Processor Time", 0, &counterPerCore);

// Après 2 collectes...
DWORD bufferSize = 0, itemCount = 0;
PdhGetFormattedCounterArray(counterPerCore, PDH_FMT_DOUBLE,
    &bufferSize, &itemCount, NULL);

PDH_FMT_COUNTERVALUE_ITEM* items =
    (PDH_FMT_COUNTERVALUE_ITEM*)malloc(bufferSize);
PdhGetFormattedCounterArray(counterPerCore, PDH_FMT_DOUBLE,
    &bufferSize, &itemCount, items);

for (DWORD i = 0; i < itemCount; i++) {
    wprintf(L"Core %s: %.1f%%\n",
        items[i].szName, items[i].FmtValue.doubleValue);
}
free(items);
```

**C# (via P/Invoke ou PerformanceCounter)** :
```csharp
using System.Diagnostics;

// Utilisation CPU par coeur
int coreCount = Environment.ProcessorCount;
var counters = new PerformanceCounter[coreCount];
for (int i = 0; i < coreCount; i++)
    counters[i] = new PerformanceCounter("Processor", "% Processor Time", i.ToString());

// Premier appel = baseline
foreach (var c in counters) c.NextValue();
Thread.Sleep(1000);

// Deuxième appel = valeur réelle
for (int i = 0; i < coreCount; i++)
    Console.WriteLine($"Core {i}: {counters[i].NextValue():F1}%");
```

**Python (psutil utilise PDH en interne sur Windows)** :
```python
import psutil

# Utilisation par coeur
per_core = psutil.cpu_percent(interval=1, percpu=True)
for i, usage in enumerate(per_core):
    print(f"Core {i}: {usage}%")

# Fréquence (utilise CallNtPowerInformation en interne)
freq = psutil.cpu_freq(percpu=True)
for i, f in enumerate(freq):
    print(f"Core {i}: {f.current} MHz (max: {f.max} MHz)")
```

### 3.4 Privilèges requis

| Opération | Privilège |
|---|---|
| Lecture compteurs Processor / Memory | Utilisateur standard |
| Lecture compteurs système généraux | Utilisateur standard |
| Utilisateur doit appartenir au groupe | `Performance Monitor Users` ou `Administrators` |

### 3.5 Métriques accessibles

| Métrique | Disponible | Compteur |
|---|---|---|
| CPU utilisation par coeur | **Oui** | `\Processor(*)\% Processor Time` |
| CPU fréquence par processeur logique | **Oui** | `\Processor Information(*)\Processor Frequency` |
| RAM disponible | **Oui** | `\Memory\Available MBytes` |
| RAM usage % | **Oui** | `\Memory\% Committed Bytes In Use` |
| I/O disque | **Oui** | `\PhysicalDisk(*)\*` |
| Réseau | **Oui** | `\Network Interface(*)\*` |
| CPU température | **Non** | Pas de compteur natif |
| Ventilateurs | **Non** | Pas de compteur natif |
| Voltages | **Non** | Pas de compteur natif |
| RAM timings | **Non** | N/A |

### 3.6 Avantages

- **20-30x plus rapide que WMI** pour les mêmes données
- API stable et bien documentée
- Per-core CPU utilisation et fréquence
- Supporte le logging dans des fichiers (`.blg`, `.csv`)
- Fonctionne en mode utilisateur standard
- Gère les wildcards (`*`) pour énumérer dynamiquement les instances

### 3.7 Inconvénients

- API C uniquement (nécessite P/Invoke ou wrappers pour autres langages)
- Nécessite 2 échantillons pour les compteurs de type rate
- Pas d'accès aux métriques hardware bas niveau (température, voltage, fans)
- Counter paths localisés (sauf `PdhAddEnglishCounter`)
- Ne fonctionne pas dans les apps Windows OneCore (utiliser PerfLib V2 à la place)
- Fréquence processeur rapportée peut ne pas refléter le boost instantané

### 3.8 Fiabilité / Précision

**Bonne à très bonne** pour les métriques de performance (CPU usage, mémoire). La fréquence processeur est celle rapportée par l'OS (P-state), pas nécessairement la fréquence boost instantanée.

---

## 4. Méthode 3 : CallNtPowerInformation / NtQuerySystemInformation

### 4.1 Mécanisme technique

Ces fonctions NT natives permettent d'accéder à des informations système de bas niveau.

**`CallNtPowerInformation`** (dans `powrprof.dll`) avec le paramètre `ProcessorInformation` retourne un tableau de structures `PROCESSOR_POWER_INFORMATION`, une par processeur logique.

**`NtQuerySystemInformation`** (dans `ntdll.dll`) avec `SystemProcessorPerformanceInformation` retourne un tableau de `SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION` contenant les temps idle/kernel/user par processeur.

### 4.2 Structures clés

```c
// PROCESSOR_POWER_INFORMATION (par processeur logique)
typedef struct {
    ULONG Number;           // Numéro du processeur
    ULONG MaxMhz;           // Fréquence max (MHz)
    ULONG CurrentMhz;       // Fréquence actuelle (MHz)
    ULONG MhzLimit;         // Limite de fréquence imposée
    ULONG MaxIdleState;     // État idle max supporté
    ULONG CurrentIdleState; // État idle actuel
} PROCESSOR_POWER_INFORMATION;

// SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION (par processeur logique)
typedef struct {
    LARGE_INTEGER IdleTime;     // Temps idle (100ns)
    LARGE_INTEGER KernelTime;   // Temps kernel (100ns)
    LARGE_INTEGER UserTime;     // Temps user (100ns)
    LARGE_INTEGER Reserved1[2];
    ULONG Reserved2;
} SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION;
```

### 4.3 Exemples de code

**C++ - Fréquence par coeur** :
```cpp
#include <windows.h>
#include <powrprof.h>
#pragma comment(lib, "PowrProf.lib")

typedef struct {
    ULONG Number;
    ULONG MaxMhz;
    ULONG CurrentMhz;
    ULONG MhzLimit;
    ULONG MaxIdleState;
    ULONG CurrentIdleState;
} PROCESSOR_POWER_INFORMATION;

int main() {
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    DWORD numCPU = si.dwNumberOfProcessors;

    DWORD bufSize = sizeof(PROCESSOR_POWER_INFORMATION) * numCPU;
    auto* ppi = new PROCESSOR_POWER_INFORMATION[numCPU];

    NTSTATUS status = CallNtPowerInformation(
        ProcessorInformation,   // InformationLevel
        NULL, 0,                // InputBuffer (aucun)
        ppi, bufSize            // OutputBuffer
    );

    if (status == 0) { // STATUS_SUCCESS
        for (DWORD i = 0; i < numCPU; i++) {
            printf("CPU %lu: Current=%lu MHz, Max=%lu MHz, Limit=%lu MHz\n",
                ppi[i].Number, ppi[i].CurrentMhz,
                ppi[i].MaxMhz, ppi[i].MhzLimit);
        }
    }

    delete[] ppi;
    return 0;
}
```

**C++ - Utilisation CPU par coeur (NtQuerySystemInformation)** :
```cpp
#include <windows.h>
#include <winternl.h>

typedef NTSTATUS(WINAPI* NtQuerySysInfo)(
    SYSTEM_INFORMATION_CLASS, PVOID, ULONG, PULONG);

int main() {
    auto NtQuery = (NtQuerySysInfo)GetProcAddress(
        GetModuleHandle(L"ntdll.dll"), "NtQuerySystemInformation");

    SYSTEM_INFO si;
    GetSystemInfo(&si);
    DWORD numCPU = si.dwNumberOfProcessors;

    DWORD bufSize = sizeof(SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION) * numCPU;
    auto* info1 = new SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION[numCPU];
    auto* info2 = new SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION[numCPU];

    // Premier échantillon
    NtQuery(SystemProcessorPerformanceInformation, info1, bufSize, NULL);
    Sleep(1000);
    // Second échantillon
    NtQuery(SystemProcessorPerformanceInformation, info2, bufSize, NULL);

    for (DWORD i = 0; i < numCPU; i++) {
        LONGLONG idleDelta = info2[i].IdleTime.QuadPart - info1[i].IdleTime.QuadPart;
        LONGLONG totalDelta =
            (info2[i].KernelTime.QuadPart + info2[i].UserTime.QuadPart) -
            (info1[i].KernelTime.QuadPart + info1[i].UserTime.QuadPart);

        double usage = 100.0 - ((double)idleDelta * 100.0 / (double)totalDelta);
        printf("CPU %lu: %.1f%%\n", i, usage);
    }

    delete[] info1;
    delete[] info2;
    return 0;
}
```

**Rust** :
```rust
use windows::Win32::System::Power::{CallNtPowerInformation, PROCESSOR_POWER_INFORMATION};
use windows::Win32::System::SystemInformation::GetSystemInfo;

fn get_cpu_frequencies() -> Vec<(u32, u32)> {
    let mut sys_info = Default::default();
    unsafe { GetSystemInfo(&mut sys_info) };
    let num_cpu = sys_info.dwNumberOfProcessors as usize;

    let mut buffer = vec![PROCESSOR_POWER_INFORMATION::default(); num_cpu];
    let buf_size = (std::mem::size_of::<PROCESSOR_POWER_INFORMATION>() * num_cpu) as u32;

    unsafe {
        CallNtPowerInformation(
            11, // ProcessorInformation
            None,
            0,
            Some(buffer.as_mut_ptr() as *mut _),
            buf_size,
        ).ok().unwrap();
    }

    buffer.iter().map(|p| (p.CurrentMhz, p.MaxMhz)).collect()
}
```

**Python (ctypes)** :
```python
import ctypes
from ctypes import wintypes

class PROCESSOR_POWER_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("Number", wintypes.ULONG),
        ("MaxMhz", wintypes.ULONG),
        ("CurrentMhz", wintypes.ULONG),
        ("MhzLimit", wintypes.ULONG),
        ("MaxIdleState", wintypes.ULONG),
        ("CurrentIdleState", wintypes.ULONG),
    ]

kernel32 = ctypes.windll.kernel32
powrprof = ctypes.windll.PowrProf

# Nombre de processeurs logiques
import os
num_cpu = os.cpu_count()

# Allouer le buffer
PPIArray = PROCESSOR_POWER_INFORMATION * num_cpu
buffer = PPIArray()
buf_size = ctypes.sizeof(buffer)

# Appel (ProcessorInformation = 11)
status = powrprof.CallNtPowerInformation(11, None, 0,
    ctypes.byref(buffer), buf_size)

if status == 0:
    for i in range(num_cpu):
        print(f"CPU {buffer[i].Number}: {buffer[i].CurrentMhz} MHz "
              f"(max: {buffer[i].MaxMhz} MHz)")
```

### 4.4 Privilèges requis

| Fonction | Privilège |
|---|---|
| `CallNtPowerInformation(ProcessorInformation)` | **Utilisateur standard** |
| `NtQuerySystemInformation(SystemProcessorPerformanceInformation)` | **Utilisateur standard** |

### 4.5 Métriques accessibles

| Métrique | Disponible | Fonction |
|---|---|---|
| CPU fréquence par coeur logique | **Oui** | `CallNtPowerInformation` |
| CPU fréquence max | **Oui** | `CallNtPowerInformation` |
| CPU état idle par coeur | **Oui** | `CallNtPowerInformation` |
| CPU utilisation par coeur | **Oui** | `NtQuerySystemInformation` |
| CPU température | **Non** | N/A |
| RAM / Fans / Voltages | **Non** | N/A |

### 4.6 Avantages

- Fréquence **par coeur logique** (pas par socket comme WMI)
- Très rapide (appel direct noyau, pas de couche COM)
- Aucun privilège admin requis
- Disponible sans dépendances externes
- Léger : pas besoin d'initialiser COM ou WMI

### 4.7 Inconvénients

- APIs **partiellement documentées** / semi-privées (peuvent changer entre versions Windows)
- `CurrentMhz` reflète le P-state nominal, **pas la fréquence boost instantanée**
- Sur certains systèmes, `CurrentMhz` peut retourner 0 ou des valeurs aberrantes
- Limité aux infos CPU — pas d'accès température, RAM timings, fans
- `NtQuerySystemInformation` est officiellement marquée comme pouvant être retirée

### 4.8 Fiabilité / Précision

**Moyenne**. La fréquence retournée est le P-state actuel × fréquence max, pas la fréquence réelle sous Turbo Boost. Pour obtenir la vraie fréquence instantanée, il faut utiliser les MSR APERF/MPERF (voir section Ring 0). L'utilisation CPU calculée via `NtQuerySystemInformation` est fiable et précise.

---

## 5. Méthode 4 : LibreHardwareMonitor

### 5.1 Mécanisme technique

LibreHardwareMonitor (LHM) est une bibliothèque C# open source (licence MPL 2.0) qui accède directement au hardware via :

1. **Driver kernel** intégré : charge un driver Ring 0 au démarrage pour lire les MSR, les registres I/O, et l'espace de configuration PCI
2. **Super I/O chips** : communique directement avec les puces de monitoring (ITE, Nuvoton, Winbond, Fintek) via le bus LPC (ports I/O 0x2E/0x2F ou 0x4E/0x4F)
3. **SMBus** : accède aux EEPROMs SPD des modules RAM
4. **WMI** : utilise WMI comme fallback pour certaines données
5. **NVML / ADL** : interfaces propriétaires NVIDIA et AMD pour les GPU

C'est le **successeur actif** d'OpenHardwareMonitor, avec un support hardware beaucoup plus étendu.

### 5.2 Puces Super I/O supportées

- **Nuvoton** : NCT6771F, NCT6776F, NCT6779D, NCT6791D, NCT6792D, NCT6793D, NCT6795D, NCT6796D, NCT6797D, NCT6798D
- **ITE** : IT8620E, IT8628E, IT8655E, IT8665E, IT8686E, IT8688E, IT8721F, IT8728F, IT8771E, IT8772E, IT879xE
- **Winbond** : W83627DHG, W83627EHF, W83627HF, W83667HG, W83687THF
- **Fintek** : F71858, F71862, F71869, F71882, F71889

### 5.3 Exemples de code

**C# (NuGet : LibreHardwareMonitorLib)** :
```csharp
using LibreHardwareMonitor.Hardware;

var computer = new Computer
{
    IsCpuEnabled = true,
    IsGpuEnabled = true,
    IsMemoryEnabled = true,
    IsMotherboardEnabled = true,
    IsFanControllerEnabled = true
};

computer.Open();

foreach (IHardware hardware in computer.Hardware)
{
    hardware.Update();

    Console.WriteLine($"=== {hardware.Name} ({hardware.HardwareType}) ===");

    foreach (ISensor sensor in hardware.Sensors)
    {
        Console.WriteLine($"  {sensor.Name}: {sensor.Value} " +
            $"({sensor.SensorType})");
    }

    // Sous-hardware (ex: Super I/O sur carte mère)
    foreach (IHardware sub in hardware.SubHardware)
    {
        sub.Update();
        foreach (ISensor sensor in sub.Sensors)
        {
            Console.WriteLine($"  [{sub.Name}] {sensor.Name}: " +
                $"{sensor.Value} ({sensor.SensorType})");
        }
    }
}

computer.Close();
```

**C# - Filtrage spécifique** :
```csharp
// Températures CPU par coeur
var cpuHardware = computer.Hardware
    .FirstOrDefault(h => h.HardwareType == HardwareType.Cpu);
cpuHardware?.Update();

var temps = cpuHardware?.Sensors
    .Where(s => s.SensorType == SensorType.Temperature);
foreach (var t in temps ?? Enumerable.Empty<ISensor>())
    Console.WriteLine($"{t.Name}: {t.Value}°C");

// Vitesse ventilateurs
var mobo = computer.Hardware
    .FirstOrDefault(h => h.HardwareType == HardwareType.Motherboard);
mobo?.Update();
foreach (var sub in mobo?.SubHardware ?? Array.Empty<IHardware>())
{
    sub.Update();
    var fans = sub.Sensors.Where(s => s.SensorType == SensorType.Fan);
    foreach (var f in fans)
        Console.WriteLine($"{f.Name}: {f.Value} RPM");
}
```

**C# - Contrôle ventilateur** :
```csharp
// Trouver un contrôle de ventilateur
var control = sub.Sensors
    .FirstOrDefault(s => s.SensorType == SensorType.Control);
if (control != null)
{
    // Définir la vitesse à 50%
    control.Control.SetSoftware(50);

    // Revenir au contrôle automatique
    control.Control.SetDefault();
}
```

**Interop depuis d'autres langages** :

Pour Python, Rust ou C++, LHM peut être consommé via :
- **WMI bridge** : LHM expose ses données en WMI (`root\LibreHardwareMonitor`)
- **HTTP server** : LHM peut démarrer un serveur HTTP JSON
- **Interop COM/.NET** : via Python.NET, IronPython, ou COM interop

```python
# Python via le WMI bridge (LHM doit tourner en arrière-plan)
import wmi
w = wmi.WMI(namespace="root\\LibreHardwareMonitor")
sensors = w.Sensor()
for s in sensors:
    print(f"{s.Parent}: {s.Name} = {s.Value} ({s.SensorType})")
```

### 5.4 Privilèges requis

| Opération | Privilège |
|---|---|
| Lecture températures CPU/GPU | **Administrateur** (charge un driver kernel) |
| Lecture fans / voltages | **Administrateur** |
| Contrôle ventilateurs | **Administrateur** |
| Lecture fréquences | **Administrateur** |
| Lecture RAM info | **Administrateur** |

**Note** : Le driver kernel (`WinRing0x64.sys` ou équivalent) peut être bloqué par HVCI (Hypervisor-Enforced Code Integrity) sur Windows 11 22H2+. Il faut potentiellement désactiver `VulnerableDriverBlocklistEnable` dans le registre.

### 5.5 Métriques accessibles

| Métrique | Disponible | Précision |
|---|---|---|
| CPU fréquence par coeur | **Oui** (via MSR APERF/MPERF) | **Excellente** - fréquence boost réelle |
| CPU utilisation par coeur | **Oui** | Bonne |
| CPU température par coeur | **Oui** (via MSR 0x19C) | **Excellente** |
| CPU package power (W) | **Oui** (via MSR RAPL) | Bonne |
| RAM usage | **Oui** | Bonne |
| RAM fréquence | **Oui** | Bonne |
| GPU température | **Oui** (NVML/ADL) | Excellente |
| GPU clock | **Oui** | Excellente |
| GPU fan speed | **Oui** | Excellente |
| GPU power | **Oui** | Bonne |
| Ventilateurs carte mère | **Oui** (via Super I/O) | Bonne (dépend du chipset) |
| Températures carte mère (VRM, chipset) | **Oui** (via Super I/O) | Bonne |
| Voltages (Vcore, DRAM, etc.) | **Oui** (via Super I/O) | Bonne |
| Contrôle ventilateurs | **Oui** (ITE, Nuvoton supportés) | Bonne |
| RAM timings | **Non** | N/A |
| Memory bandwidth | **Non** | N/A |

### 5.6 Avantages

- **Couverture la plus complète** de toutes les méthodes pour une seule bibliothèque
- Open source (MPL 2.0), activement maintenu
- Fréquence CPU réelle via APERF/MPERF (pas juste P-state)
- Température CPU par coeur via DTS (Digital Thermal Sensor)
- Package power via Intel RAPL
- Support GPU complet (NVIDIA + AMD)
- Support Super I/O étendu (fans, voltages, températures mobo)
- NuGet package disponible
- Expose les données en WMI et HTTP

### 5.7 Inconvénients

- **C# uniquement** (nécessite .NET Framework 4.7.2+ ou .NET 6+)
- **Administrateur obligatoire** pour la quasi-totalité des fonctionnalités
- Charge un **driver kernel** (problèmes HVCI sur Win11)
- La bibliothèque peut être lourde pour une intégration embarquée
- Pas de timings RAM (CAS, tRCD, etc.)
- Potentiel impact sur la sécurité (driver Ring 0 non signé par Microsoft)

### 5.8 Fiabilité / Précision

**Excellente** pour toutes les métriques supportées. C'est la méthode la plus fiable pour les températures CPU per-core, la fréquence boost réelle, et les lectures Super I/O. Utilisée comme référence par de nombreux outils de monitoring.

---

## 6. Méthode 5 : OpenHardwareMonitor

### 6.1 Mécanisme technique

OpenHardwareMonitor (OHM) est le prédécesseur de LibreHardwareMonitor. Il utilise le même principe : un driver kernel Ring 0 pour accéder aux MSR et aux puces Super I/O.

### 6.2 Différences avec LibreHardwareMonitor

| Aspect | OpenHardwareMonitor | LibreHardwareMonitor |
|---|---|---|
| Dernière mise à jour | ~2020 (abandonné) | Actif (2024+) |
| Framework | .NET Framework 4.0 | .NET 4.7.2 / .NET 6+ |
| Support CPU récents | Jusqu'à ~10ème gen Intel, Zen 2 | Zen 4, Intel 13th/14th gen+ |
| Support GPU récent | Jusqu'à ~RTX 2000 | RTX 4000/5000, RX 7000+ |
| Super I/O chips | Support de base | Support étendu (NCT6798D, etc.) |
| Licence | MPL 2.0 | MPL 2.0 |
| NuGet | Non officiel | Officiel (`LibreHardwareMonitorLib`) |

### 6.3 Exemple de code (C#)

```csharp
using OpenHardwareMonitor.Hardware;

var computer = new Computer()
{
    CPUEnabled = true,
    GPUEnabled = true,
    FanControllerEnabled = true,
    MainboardEnabled = true,
    RAMEnabled = true
};

computer.Open();

foreach (IHardware hw in computer.Hardware)
{
    hw.Update();
    foreach (ISensor sensor in hw.Sensors)
    {
        if (sensor.SensorType == SensorType.Temperature)
            Console.WriteLine($"{hw.Name} - {sensor.Name}: {sensor.Value}°C");
    }
}

computer.Close();
```

### 6.4 Recommandation

**Ne pas utiliser OHM pour de nouveaux projets.** Migrer vers LibreHardwareMonitor qui offre un support hardware plus large, des corrections de bugs, et une maintenance active.

---

## 7. Méthode 6 : Accès direct MSR / CPUID (Ring 0)

### 7.1 Mécanisme technique

Les Model-Specific Registers (MSR) sont des registres internes au CPU accessibles uniquement en Ring 0 (mode kernel) via les instructions x86 `RDMSR` et `WRMSR`. L'instruction `CPUID` est, elle, disponible en Ring 3 (user mode).

Pour lire les MSR depuis une application user-mode Windows, il faut un **driver kernel** qui expose cette fonctionnalité.

### 7.2 MSR importants

| MSR | Adresse | Description |
|---|---|---|
| `IA32_THERM_STATUS` | 0x19C | Température DTS par coeur |
| `IA32_TEMPERATURE_TARGET` | 0x1A2 | TjMax (température max) |
| `MSR_PERF_STATUS` | 0x198 | Ratio de fréquence actuel |
| `IA32_MPERF` | 0xE7 | Maximum performance counter |
| `IA32_APERF` | 0xE8 | Actual performance counter |
| `MSR_PLATFORM_INFO` | 0xCE | Ratio base, TDP |
| `MSR_RAPL_POWER_UNIT` | 0x606 | Unités RAPL (power) |
| `MSR_PKG_ENERGY_STATUS` | 0x611 | Énergie package (Intel RAPL) |
| `MSR_PP0_ENERGY_STATUS` | 0x639 | Énergie cores |
| `MSR_DRAM_ENERGY_STATUS` | 0x619 | Énergie DRAM |

### 7.3 Calcul de la température CPU

```
Formule : Température = TjMax - DTS_Reading

TjMax = MSR 0x1A2, bits [23:16]  (typiquement 100°C ou 85°C)
DTS   = MSR 0x19C, bits [22:16]  (Digital Thermal Sensor)
```

### 7.4 Calcul de la fréquence réelle (Turbo Boost)

```
Fréquence réelle = Base_Frequency × (APERF_delta / MPERF_delta)

Où :
- Base_Frequency = MSR 0xCE bits [15:8] × 100 MHz (bus clock)
- APERF_delta = MSR 0xE8 (t2) - MSR 0xE8 (t1)
- MPERF_delta = MSR 0xE7 (t2) - MSR 0xE7 (t1)
```

### 7.5 Drivers disponibles

| Driver | Description | Statut |
|---|---|---|
| **WinRing0** | Driver open source historique, utilisé par CoreTemp, RealTemp, CPU-Z | Bloqué par HVCI (Win11 22H2+) |
| **WinMSR** | Driver 64-bit avec exemples CPUID + température | Open source (GitHub) |
| **msr-utility (msr-cmd)** | CLI basé sur WinRing0, supporte 64+ coeurs | Open source (GitHub) |
| **Driver LHM** | Intégré dans LibreHardwareMonitor | Le plus à jour |

### 7.6 Exemples de code

**C++ avec WinRing0** :
```cpp
#include "OlsApi.h"  // WinRing0 API

// Initialiser le driver
InitializeOls();

// Lire température du coeur 0
DWORD eax, edx;
RdmsrPx(0x19C, &eax, &edx, 0);  // MSR 0x19C, coeur 0
int dts_reading = (eax >> 16) & 0x7F;

// Lire TjMax
RdmsrPx(0x1A2, &eax, &edx, 0);
int tjmax = (eax >> 16) & 0xFF;

int temperature = tjmax - dts_reading;
printf("Core 0 temp: %d°C\n", temperature);

// Lire RAPL energy (package power)
RdmsrPx(0x606, &eax, &edx, 0);
double energy_unit = pow(0.5, (eax & 0x1F00) >> 8);

DWORD energy1, energy2;
RdmsrPx(0x611, &energy1, &edx, 0);
Sleep(1000);
RdmsrPx(0x611, &energy2, &edx, 0);

double power_watts = (energy2 - energy1) * energy_unit;
printf("Package Power: %.1f W\n", power_watts);

DeinitializeOls();
```

**CPUID (user mode, pas besoin de driver)** :
```cpp
#include <intrin.h>

void get_cpu_info() {
    int cpuInfo[4];

    // CPUID leaf 0 : vendor string
    __cpuid(cpuInfo, 0);
    char vendor[13] = {};
    memcpy(vendor, &cpuInfo[1], 4);
    memcpy(vendor + 4, &cpuInfo[3], 4);
    memcpy(vendor + 8, &cpuInfo[2], 4);
    printf("Vendor: %s\n", vendor);

    // CPUID leaf 1 : features
    __cpuid(cpuInfo, 1);
    int family = ((cpuInfo[0] >> 8) & 0xF) + ((cpuInfo[0] >> 20) & 0xFF);
    int model = ((cpuInfo[0] >> 4) & 0xF) | ((cpuInfo[0] >> 12) & 0xF0);
    printf("Family: %d, Model: %d\n", family, model);

    // Check MSR support
    bool msr_supported = (cpuInfo[3] >> 5) & 1;
    printf("MSR supported: %s\n", msr_supported ? "yes" : "no");

    // CPUID leaf 0x16 : frequencies (Intel 6th gen+)
    __cpuid(cpuInfo, 0x16);
    printf("Base: %d MHz, Max Turbo: %d MHz, Bus: %d MHz\n",
        cpuInfo[0] & 0xFFFF, cpuInfo[1] & 0xFFFF, cpuInfo[2] & 0xFFFF);
}
```

### 7.7 Privilèges requis

| Opération | Privilège |
|---|---|
| `CPUID` instruction | **Utilisateur standard** (Ring 3) |
| `RDMSR` / `WRMSR` | **Administrateur + driver kernel** (Ring 0) |
| Installation du driver | **Administrateur** |

### 7.8 Métriques accessibles

| Métrique | Disponible | MSR |
|---|---|---|
| CPU température par coeur | **Oui** | 0x19C (IA32_THERM_STATUS) |
| CPU fréquence réelle par coeur | **Oui** | 0xE7/0xE8 (APERF/MPERF) |
| CPU package power (W) | **Oui** | 0x611 (Intel RAPL) |
| CPU core power | **Oui** | 0x639 (Intel PP0) |
| DRAM power | **Oui** | 0x619 (Intel DRAM RAPL) |
| CPU base/turbo ratio | **Oui** | 0xCE (PLATFORM_INFO) |
| CPU voltage (VID) | Partiel | Dépend de la génération |
| RAM / Fans / Mobo | **Non** | MSR = CPU uniquement |

### 7.9 Avantages

- **Précision maximale** : accès direct aux registres hardware
- Fréquence boost réelle (APERF/MPERF), pas le P-state
- Température par coeur (DTS)
- Intel RAPL pour la consommation énergétique
- CPUID en user-mode pour identifier le CPU

### 7.10 Inconvénients

- **Nécessite un driver kernel** (complexe à développer/maintenir)
- **Problèmes de sécurité** : driver Ring 0 = surface d'attaque
- **HVCI** (Win11 22H2+) bloque les drivers non signés Microsoft
- **Intel-spécifique** pour beaucoup de MSR (RAPL, DTS, etc.)
- AMD utilise des MSR différents (ex: package power via différents registres)
- Pas d'accès aux capteurs carte mère (fans, voltages) via MSR

### 7.11 Fiabilité / Précision

**Excellente** pour les métriques CPU. C'est la source la plus précise possible pour la température, la fréquence réelle, et la consommation. Les valeurs MSR sont celles que le hardware rapporte directement.

---

## 8. Méthode 7 : ETW (Event Tracing for Windows)

### 8.1 Mécanisme technique

ETW est le mécanisme de tracing du noyau Windows. Il permet de capturer des événements émis par le kernel et les drivers en temps réel, avec un overhead minimal. Les événements sont organisés par **providers** identifiés par des GUIDs.

Pour les métriques CPU/power, le provider principal est :
- **`Microsoft-Windows-Kernel-Processor-Power`** (GUID: `{0f67e49f-fe51-4e9f-b490-6f2948cc6027}`)

### 8.2 Événements clés

| Event ID | Nom | Description |
|---|---|---|
| 4 | Summary | Résumé des états P-states |
| 7 | LongCapInfo | Capacités étendues du processeur |
| 8 | QuickCapInfo | Capacités rapides |
| 37 | PerfWarning | Alerte throttling firmware |
| 55 | PerfCapInfo | Fréquence nominale, état idle, perf min/max |
| 56-62 | PPM events | Changements P-state (Win8+) |

### 8.3 Exemples de code

**Collecte avec xperf (ligne de commande)** :
```cmd
:: Démarrer une trace ETW incluant les événements power
xperf -on PROC_THREAD+LOADER+POWER+IDLE_STATES -stackwalk CSwitch

:: Arrêter et sauvegarder
xperf -d trace.etl

:: Analyser avec Windows Performance Analyzer (WPA)
wpa trace.etl
```

**PowerShell - Lecture des événements** :
```powershell
# Lire les événements récents du provider Kernel-Processor-Power
Get-WinEvent -FilterHashtable @{
    ProviderName = "Microsoft-Windows-Kernel-Processor-Power"
    Id = 55
} -MaxEvents 10 | ForEach-Object {
    $_.Message
}
```

**C# - Session ETW en temps réel** :
```csharp
using Microsoft.Diagnostics.Tracing;
using Microsoft.Diagnostics.Tracing.Session;

// Nécessite NuGet: Microsoft.Diagnostics.Tracing.TraceEvent
using var session = new TraceEventSession("CPUPowerSession");

session.EnableProvider(
    "Microsoft-Windows-Kernel-Processor-Power",
    TraceEventLevel.Informational);

session.Source.Dynamic.All += (TraceEvent data) =>
{
    if (data.ID == (TraceEventID)55)
    {
        Console.WriteLine($"Event 55: {data.PayloadNames}");
        // Extraire NominalFrequency, MaxPerformance, etc.
    }
};

// Démarrer l'écoute (bloquant)
session.Source.Process();
```

**C++ - Session ETW native** :
```cpp
#include <windows.h>
#include <evntrace.h>
#include <tdh.h>

// GUID du provider
static const GUID ProcessorPowerGuid =
    {0x0f67e49f, 0xfe51, 0x4e9f,
     {0xb4, 0x90, 0x6f, 0x29, 0x48, 0xcc, 0x60, 0x27}};

// Configuration simplifiée d'une session ETW
EVENT_TRACE_PROPERTIES* props = /* allocation + configuration */;
StartTrace(&hSession, L"CPUPowerTrace", props);

ENABLE_TRACE_PARAMETERS params = {};
params.Version = ENABLE_TRACE_PARAMETERS_VERSION_2;
EnableTraceEx2(hSession, &ProcessorPowerGuid,
    EVENT_CONTROL_CODE_ENABLE_PROVIDER,
    TRACE_LEVEL_INFORMATION, 0xC2, 0, 0, &params);

// Callback pour traiter les événements
// ... (voir documentation Microsoft pour le consumer pattern)
```

### 8.4 Privilèges requis

| Opération | Privilège |
|---|---|
| Créer une session ETW | **Administrateur** |
| Consommer une session existante | **Administrateur** (ou `Performance Log Users`) |
| Lire les logs `.etl` | Utilisateur standard |

### 8.5 Métriques accessibles

| Métrique | Disponible | Via |
|---|---|---|
| CPU P-state transitions | **Oui** | Events 56-62 |
| CPU fréquence nominale | **Oui** | Event 55 |
| CPU throttling alerts | **Oui** | Event 37 |
| CPU C-state usage | **Oui** | Events idle |
| Context switches | **Oui** | Kernel events |
| CPU utilisation | Indirect (calcul via events) | Thread scheduling |
| Température / Fans / RAM | **Non** | N/A |

### 8.6 Avantages

- **Overhead minimal** : conçu pour le tracing production
- Capture les **transitions** de P-state en temps réel
- Historique temporel précis (timestamps haute résolution)
- Intégration avec WPA (Windows Performance Analyzer) pour l'analyse
- Événements structurés avec metadata

### 8.7 Inconvénients

- **Complexe** à mettre en oeuvre (API verbeuse, callbacks)
- **Event-driven** : ne fournit pas de valeurs instantanées sur demande
- Principalement orienté diagnostic/profiling, pas monitoring continu
- Nécessite des outils Microsoft (WPR, WPA, xperf) pour une analyse complète
- Pas d'accès aux métriques hardware (température, fans, voltage)
- Admin requis pour créer des sessions

### 8.8 Fiabilité / Précision

**Très bonne** pour les événements qu'il capture (P-state transitions, C-states). Les timestamps sont précis au microseconde. Cependant, ce n'est pas un outil de monitoring continu — c'est un outil de profiling/diagnostic.

---

## 9. Méthode 8 : Windows Performance Counters (sans PDH)

### 9.1 Mécanisme technique

Les Performance Counters Windows peuvent être accédés directement via le registre (`HKEY_PERFORMANCE_DATA`) sans passer par PDH. C'est le mécanisme de plus bas niveau pour lire les compteurs de performance, utilisé par PDH en interne.

Deux versions existent :
- **PerfLib V1** : Via `RegQueryValueEx` sur `HKEY_PERFORMANCE_DATA`
- **PerfLib V2** : API moderne via `PerfOpenQueryHandle`, `PerfAddCounters`, etc. (recommandée pour les apps OneCore)

### 9.2 Exemples de code

**C++ - PerfLib V1 (registry direct)** :
```cpp
#include <windows.h>
#include <winperf.h>

// Lire les données brutes du compteur "Processor" (index 238)
DWORD bufSize = 0;
RegQueryValueEx(HKEY_PERFORMANCE_DATA, L"238", NULL, NULL, NULL, &bufSize);

BYTE* buffer = new BYTE[bufSize];
RegQueryValueEx(HKEY_PERFORMANCE_DATA, L"238", NULL, NULL, buffer, &bufSize);

auto* perfData = (PERF_DATA_BLOCK*)buffer;
auto* objType = (PERF_OBJECT_TYPE*)((BYTE*)perfData + perfData->HeaderLength);
auto* instDef = (PERF_INSTANCE_DEFINITION*)((BYTE*)objType + objType->DefinitionLength);

// Itérer les instances (une par coeur + _Total)
for (LONG i = 0; i < objType->NumInstances; i++) {
    auto* counterBlock = (PERF_COUNTER_BLOCK*)
        ((BYTE*)instDef + instDef->ByteLength);

    wchar_t* name = (wchar_t*)((BYTE*)instDef + instDef->NameOffset);
    // Extraire les valeurs du counter block selon les définitions...

    instDef = (PERF_INSTANCE_DEFINITION*)
        ((BYTE*)counterBlock + counterBlock->ByteLength);
}

delete[] buffer;
RegCloseKey(HKEY_PERFORMANCE_DATA);
```

**C# - PerformanceCounter** :
```csharp
using System.Diagnostics;

// Utilisation CPU totale
var cpuCounter = new PerformanceCounter("Processor", "% Processor Time", "_Total");
cpuCounter.NextValue();  // Premier appel = baseline
Thread.Sleep(1000);
float cpuUsage = cpuCounter.NextValue();
Console.WriteLine($"CPU Usage: {cpuUsage:F1}%");

// Mémoire disponible
var memCounter = new PerformanceCounter("Memory", "Available MBytes");
Console.WriteLine($"Available RAM: {memCounter.NextValue()} MB");

// Lister tous les compteurs d'une catégorie
var category = new PerformanceCounterCategory("Processor Information");
foreach (var instance in category.GetInstanceNames())
{
    var counters = category.GetCounters(instance);
    foreach (var counter in counters)
        Console.WriteLine($"  {instance}: {counter.CounterName}");
}
```

### 9.3 Compteurs disponibles (même que PDH)

Même couverture que PDH (section 3.5) — c'est le même backend. PDH est simplement un wrapper au-dessus de cette API.

### 9.4 Avantages

- Plus bas niveau que PDH = potentiellement plus rapide
- PerfLib V2 compatible avec les apps OneCore
- Pas de dépendance sur `pdh.dll`

### 9.5 Inconvénients

- **API très complexe** (structures imbriquées, offsets manuels)
- Pas de formatage automatique des valeurs
- Nécessite de connaître les index des compteurs
- PDH est préférable dans 99% des cas

---

## 10. Méthode 9 : GlobalMemoryStatusEx / GetPerformanceInfo

### 10.1 Mécanisme technique

Ces fonctions Win32 fournissent des informations rapides sur l'état de la mémoire système :

- **`GlobalMemoryStatusEx`** : rempli une structure `MEMORYSTATUSEX` avec RAM physique/virtuelle totale et disponible
- **`GetPerformanceInfo`** : retourne des informations système étendues (handles, threads, commit charge, etc.)

### 10.2 Structure MEMORYSTATUSEX

```c
typedef struct _MEMORYSTATUSEX {
    DWORD     dwLength;            // Taille de la structure
    DWORD     dwMemoryLoad;        // % mémoire utilisée (0-100)
    DWORDLONG ullTotalPhys;        // RAM physique totale (bytes)
    DWORDLONG ullAvailPhys;        // RAM physique disponible (bytes)
    DWORDLONG ullTotalPageFile;    // Page file total
    DWORDLONG ullAvailPageFile;    // Page file disponible
    DWORDLONG ullTotalVirtual;     // Espace virtuel total
    DWORDLONG ullAvailVirtual;     // Espace virtuel disponible
    DWORDLONG ullAvailExtendedVirtual; // Toujours 0
} MEMORYSTATUSEX;
```

### 10.3 Exemples de code

**C++** :
```cpp
#include <windows.h>
#include <psapi.h>

int main() {
    // GlobalMemoryStatusEx
    MEMORYSTATUSEX memInfo = {};
    memInfo.dwLength = sizeof(memInfo);
    GlobalMemoryStatusEx(&memInfo);

    printf("Memory Load: %lu%%\n", memInfo.dwMemoryLoad);
    printf("Total Physical: %.1f GB\n",
        memInfo.ullTotalPhys / (1024.0 * 1024 * 1024));
    printf("Available Physical: %.1f GB\n",
        memInfo.ullAvailPhys / (1024.0 * 1024 * 1024));
    printf("Used: %.1f GB\n",
        (memInfo.ullTotalPhys - memInfo.ullAvailPhys) / (1024.0 * 1024 * 1024));

    // GetPerformanceInfo
    PERFORMANCE_INFORMATION perfInfo = {};
    perfInfo.cb = sizeof(perfInfo);
    GetPerformanceInfo(&perfInfo, sizeof(perfInfo));

    printf("Commit Total: %zu pages\n", perfInfo.CommitTotal);
    printf("Physical Total: %zu pages\n", perfInfo.PhysicalTotal);
    printf("Physical Available: %zu pages\n", perfInfo.PhysicalAvailable);
    printf("Page Size: %zu bytes\n", perfInfo.PageSize);

    return 0;
}
```

**C#** :
```csharp
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
class MEMORYSTATUSEX {
    public uint dwLength = (uint)Marshal.SizeOf(typeof(MEMORYSTATUSEX));
    public uint dwMemoryLoad;
    public ulong ullTotalPhys;
    public ulong ullAvailPhys;
    public ulong ullTotalPageFile;
    public ulong ullAvailPageFile;
    public ulong ullTotalVirtual;
    public ulong ullAvailVirtual;
    public ulong ullAvailExtendedVirtual;
}

[DllImport("kernel32.dll", SetLastError = true)]
static extern bool GlobalMemoryStatusEx([In, Out] MEMORYSTATUSEX buffer);

var memStatus = new MEMORYSTATUSEX();
GlobalMemoryStatusEx(memStatus);
Console.WriteLine($"Memory Load: {memStatus.dwMemoryLoad}%");
Console.WriteLine($"Total: {memStatus.ullTotalPhys / (1024*1024*1024.0):F1} GB");
Console.WriteLine($"Available: {memStatus.ullAvailPhys / (1024*1024*1024.0):F1} GB");
```

**Python** :
```python
import psutil

mem = psutil.virtual_memory()
print(f"Total: {mem.total / (1024**3):.1f} GB")
print(f"Available: {mem.available / (1024**3):.1f} GB")
print(f"Used: {mem.percent}%")
```

**Rust** :
```rust
use windows::Win32::System::SystemInformation::GlobalMemoryStatusEx;
use windows::Win32::System::SystemInformation::MEMORYSTATUSEX;

fn get_memory_info() -> (u64, u64) {
    let mut mem = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut mem).unwrap() };
    (mem.ullTotalPhys, mem.ullAvailPhys)
}
```

### 10.4 Privilèges requis

**Aucun privilège spécial** — utilisateur standard suffit.

### 10.5 Métriques accessibles

| Métrique | Disponible |
|---|---|
| RAM totale (bytes) | **Oui** |
| RAM disponible (bytes) | **Oui** |
| RAM % utilisée | **Oui** |
| Page file total/disponible | **Oui** |
| Espace virtuel | **Oui** |
| RAM fréquence | **Non** |
| RAM timings | **Non** |
| RAM bandwidth | **Non** |
| RAM latence | **Non** |

### 10.6 Avantages

- **Extrêmement rapide** (appel kernel direct)
- Aucune dépendance, aucun privilège
- Disponible sur toutes les versions Windows
- Valeurs toujours à jour

### 10.7 Inconvénients

- Uniquement usage mémoire (total/disponible)
- Pas de fréquence, timings, bandwidth, ou latence
- Pas d'info par processus (utiliser `GetProcessMemoryInfo` pour ça)

### 10.8 Fiabilité / Précision

**Excellente** pour ce qu'elle mesure (capacité et usage). C'est la source de référence pour l'utilisation RAM.

---

## 11. Méthode 10 : Intel PCM (Performance Counter Monitor)

### 11.1 Mécanisme technique

Intel PCM est une bibliothèque C++ open source d'Intel qui accède aux PMU (Performance Monitoring Units) des processeurs Intel et AMD pour mesurer des métriques de performance bas niveau.

Sur Windows, PCM utilise :
- Le driver **WinRing0** (ou WinPMem) pour accéder aux MSR
- L'espace de configuration **PCI** pour les uncore counters (IMC = memory controller)

### 11.2 Métriques uniques

PCM est la seule méthode permettant de mesurer :
- **Memory bandwidth** par canal DRAM (lecture, écriture, total)
- **Cache miss rates** (L1, L2, L3)
- **Instructions Per Cycle (IPC)**
- **QPI/UPI bandwidth** (inter-socket)
- **PCIe bandwidth**

### 11.3 Exemples de code

**C++ - API programmatique** :
```cpp
#include "cpucounters.h"

int main() {
    PCM* pcm = PCM::getInstance();

    if (pcm->program() != PCM::Success) {
        printf("Erreur: impossible d'initialiser PCM\n");
        return 1;
    }

    SystemCounterState before = getSystemCounterState();
    Sleep(1000);
    SystemCounterState after = getSystemCounterState();

    // IPC (Instructions Per Cycle)
    printf("IPC: %.2f\n", getIPC(before, after));

    // Cache misses
    printf("L3 Cache Misses: %llu\n", getL3CacheMisses(before, after));
    printf("L2 Cache Misses: %llu\n", getL2CacheMisses(before, after));

    // Memory bandwidth (nécessite uncore counters)
    printf("Memory Read BW: %.1f GB/s\n",
        getBytesReadFromMC(before, after) / 1e9);
    printf("Memory Write BW: %.1f GB/s\n",
        getBytesWrittenToMC(before, after) / 1e9);

    pcm->cleanup();
    return 0;
}
```

**Outils en ligne de commande** :
```cmd
:: Monitoring basique (CPU, cache, memory BW)
pcm.exe 1

:: Memory bandwidth détaillé (par canal et DIMM)
pcm-memory.exe 1

:: Latence mémoire
pcm-latency.exe

:: Power/énergie (RAPL)
pcm-power.exe 1
```

### 11.4 Privilèges requis

| Opération | Privilège |
|---|---|
| Toute utilisation de PCM | **Administrateur** + driver kernel |
| Installation sur Windows | Nécessite le driver WinPMem ou WinRing0 |

### 11.5 Métriques accessibles

| Métrique | Disponible | Outil/API |
|---|---|---|
| Memory bandwidth (R/W/Total) par canal | **Oui** | `pcm-memory`, `getBytesReadFromMC` |
| Memory latence | **Oui** | `pcm-latency` |
| Cache miss rates (L1/L2/L3) | **Oui** | `pcm`, `getL3CacheMisses` |
| IPC | **Oui** | `pcm`, `getIPC` |
| CPU utilisation réelle | **Oui** | `pcm` |
| Package/Core power (RAPL) | **Oui** | `pcm-power` |
| QPI/UPI bandwidth | **Oui** | `pcm` |
| PCIe bandwidth | **Oui** | `pcm-pcie` |
| CPU température | **Non** (utiliser MSR direct) | N/A |
| Fans / Voltages | **Non** | N/A |

### 11.6 Avantages

- **Seule méthode** pour mesurer le memory bandwidth réel
- Métriques de performance micro-architecturales (IPC, cache)
- Supporté officiellement par Intel
- Open source (BSD-3-Clause)
- Support AMD Zen (partiel) depuis les versions récentes

### 11.7 Inconvénients

- **Intel-centric** (support AMD limité)
- Administrateur + driver kernel obligatoires
- Conflit avec d'autres outils PMU (VTune, perf)
- PMU est une ressource exclusive (un seul outil à la fois)
- Installation complexe sur Windows
- Ne couvre pas les capteurs hardware (température, fans)

### 11.8 Fiabilité / Précision

**Excellente** pour les métriques PMU. Intel PCM lit directement les compteurs hardware du CPU et du memory controller. C'est la source la plus précise pour le memory bandwidth et les cache statistics.

---

## 12. Méthode 11 : Accès SMBus / SPD

### 12.1 Mécanisme technique

Les modules RAM contiennent une EEPROM **SPD** (Serial Presence Detect) accessible via le bus **SMBus** (System Management Bus, variante I2C). Cette EEPROM contient les spécifications complètes du module : timings, tensions, profils XMP/EXPO, etc.

**Accès SMBus sur Windows** :
1. Trouver le contrôleur SMBus dans l'espace de configuration PCI du southbridge/PCH
2. Lire le BAR (Base Address Register) du contrôleur
3. Accéder aux registres I/O du contrôleur SMBus
4. Envoyer des commandes I2C aux adresses 0x50-0x57 (une par slot DIMM)

### 12.2 Données SPD disponibles

| Donnée | DDR4 | DDR5 |
|---|---|---|
| CAS Latency (CL) | Oui | Oui |
| tRCD | Oui | Oui |
| tRP | Oui | Oui |
| tRAS | Oui | Oui |
| tRC | Oui | Oui |
| Profils XMP/EXPO | Oui (XMP 2.0) | Oui (XMP 3.0, EXPO) |
| Voltage nominale | Oui | Oui |
| Fabricant / Part Number | Oui | Oui |
| Module Rank / Bank | Oui | Oui |

### 12.3 Outils existants

| Outil | Type | Description |
|---|---|---|
| **Thaiphoon Burner** | Application | Lecture/écriture SPD complète via SMBus |
| **CPU-Z** | Application | Affiche les timings et fréquence |
| **RAMMon (PassMark)** | Application / SDK | Lecture SPD, SDK `SysInfo.dll` disponible |
| **HWiNFO** | Application | Timings même sans SPD |
| **ZenTimings** | Application | Timings et sub-timings (AMD uniquement) |

### 12.4 Accès programmatique

**Pas d'API Windows standard** pour lire le SPD. Les options sont :

1. **SDK PassMark SysInfo** : DLL commerciale qui lit le SPD
2. **Accès direct au contrôleur SMBus** via un driver kernel (chipset-spécifique)
3. **LibreHardwareMonitor** : implémente partiellement la lecture SMBus
4. **Code custom** : nécessite de reverse-engineer l'accès SMBus du PCH spécifique

**C++ - Principe d'accès SMBus (pseudo-code)** :
```cpp
// 1. Trouver le contrôleur SMBus dans le PCI config space
// Intel PCH: Bus 0, Device 31, Function 4
DWORD smbusBase = ReadPciConfig(0, 31, 4, 0x20) & 0xFFE0;

// 2. Lire SPD de la DIMM au slot 0 (adresse I2C: 0x50)
BYTE ReadSPDByte(WORD smbusBase, BYTE slotAddr, BYTE offset) {
    // Attendre que le bus soit libre
    while (inb(smbusBase + 0x00) & 0x01) Sleep(1);

    // Configurer l'adresse esclave et l'offset
    outb(smbusBase + 0x04, (slotAddr << 1) | 0x01); // Slave addr + Read
    outb(smbusBase + 0x03, offset);                   // Register offset

    // Lancer la transaction (byte read)
    outb(smbusBase + 0x02, 0x48);

    // Attendre la fin
    while (!(inb(smbusBase + 0x00) & 0x02)) Sleep(1);

    // Lire le résultat
    return inb(smbusBase + 0x05);
}

// 3. Décoder le CAS Latency (DDR4, offset 24-25)
WORD casSupported = (ReadSPDByte(smbusBase, 0x50, 25) << 8) |
                     ReadSPDByte(smbusBase, 0x50, 24);
// Bit 0 = CL7, Bit 1 = CL8, etc.
```

### 12.5 Privilèges requis

**Administrateur + driver kernel** pour accéder aux ports I/O du contrôleur SMBus.

### 12.6 Avantages

- **Seule méthode** pour obtenir les timings RAM réels (CAS, tRCD, tRP, tRAS)
- Données provenant directement de l'EEPROM du module
- Profils XMP/EXPO complets

### 12.7 Inconvénients

- **Aucune API Windows standard**
- Nécessite un driver kernel et une connaissance du chipset
- Implémentation **chipset-spécifique** (Intel PCH vs AMD FCH)
- Certains BIOS bloquent l'accès SMBus en écriture
- DDR4 utilise un protocole EEPROM différent de DDR5 (SPD5118 hub)
- RAM soudée (laptops) peut ne pas avoir de données SPD

### 12.8 Fiabilité / Précision

**Excellente** si l'accès fonctionne — les données sont celles programmées dans l'EEPROM du module. La difficulté est d'obtenir l'accès.

---

## 13. Méthode 12 : IPMI

### 13.1 Mécanisme technique

IPMI (Intelligent Platform Management Interface) est un standard pour la gestion hors-bande des serveurs. Un BMC (Baseboard Management Controller) intégré à la carte mère surveille les capteurs hardware indépendamment de l'OS.

**Utilisation** : principalement sur les cartes serveur (Supermicro, Dell iDRAC, HP iLO, Lenovo XCC).

### 13.2 Exemples

```cmd
:: Lire tous les capteurs
ipmitool sensor list

:: Lire un capteur spécifique
ipmitool sensor reading "FAN 1 RPM"
ipmitool sensor reading "CPU Temp"

:: Définir la vitesse des ventilateurs (Supermicro)
ipmitool raw 0x30 0x70 0x66 0x01 0x00 0x40

:: Via ipmiutil (Windows natif)
ipmiutil sensor -v
```

### 13.3 Métriques accessibles

| Métrique | Disponible |
|---|---|
| CPU température | **Oui** |
| Fan speed (RPM) | **Oui** |
| Fan control | **Oui** (commandes raw vendor-specific) |
| Voltages | **Oui** |
| Power consumption | **Oui** (sur certaines plateformes) |
| RAM info | Partiel |

### 13.4 Limitations

- **Serveurs uniquement** — pas disponible sur les PC de bureau grand public
- Commandes de contrôle vendor-specific
- Latence plus élevée (le BMC a son propre cycle de polling)

---

## 14. Couverture détaillée par métrique

### 14.1 CPU - Fréquence par coeur

| Méthode | Base | Boost/Temps réel | Per-core | Précision |
|---|---|---|---|---|
| WMI (`Win32_Processor`) | Oui (`MaxClockSpeed`) | Partiel (`CurrentClockSpeed` = P-state) | **Non** (per-socket) | Faible |
| PDH (`Processor Information`) | Oui | Partiel (P-state) | **Oui** | Moyenne |
| `CallNtPowerInformation` | Oui (`MaxMhz`) | Partiel (`CurrentMhz` = P-state) | **Oui** | Moyenne |
| MSR (APERF/MPERF) | Oui (0xCE) | **Oui** (fréquence réelle) | **Oui** | **Excellente** |
| LibreHardwareMonitor | Oui | **Oui** (via MSR) | **Oui** | **Excellente** |
| CPUID (leaf 0x16) | Oui | Max Turbo (statique) | **Non** | Bonne (statique) |
| Intel PCM | Oui | Oui (via MSR) | **Oui** | Excellente |

**Recommandation** : LibreHardwareMonitor ou MSR direct pour la fréquence boost réelle per-core.

### 14.2 CPU - Utilisation par coeur

| Méthode | Per-core | Intervalle min | Précision |
|---|---|---|---|
| WMI (`PerfFormattedData_PerfOS_Processor`) | **Oui** | ~1s | Bonne |
| PDH (`Processor(*)\% Processor Time`) | **Oui** | ~1s | Très bonne |
| `NtQuerySystemInformation` | **Oui** | ~100ms | Très bonne |
| `GetSystemTimes` | **Non** (total) | ~100ms | Bonne |
| ETW (thread scheduling) | **Oui** | ~µs | Excellente |

**Recommandation** : PDH pour la simplicité, `NtQuerySystemInformation` pour la performance.

### 14.3 CPU - Température par coeur

| Méthode | Per-core | Réalité mesurée | Précision |
|---|---|---|---|
| WMI (`MSAcpi_ThermalZoneTemperature`) | **Non** | Zone thermique ACPI (souvent mobo) | **Faible** |
| MSR (0x19C DTS) | **Oui** | Die température CPU réelle | **Excellente** |
| LibreHardwareMonitor | **Oui** | Die température (via MSR) | **Excellente** |
| IPMI | **Non** (1 valeur CPU) | Capteur externe ou DTS | Bonne |

**Recommandation** : LibreHardwareMonitor (qui utilise MSR en interne).

### 14.4 CPU - Power (TDP, Package Power)

| Méthode | Package | Per-core | DRAM | Précision |
|---|---|---|---|---|
| MSR (Intel RAPL) | **Oui** (0x611) | **Oui** (0x639) | **Oui** (0x619) | Excellente |
| LibreHardwareMonitor | **Oui** | **Oui** | Partiel | Excellente |
| Intel PCM | **Oui** | **Oui** | **Oui** | Excellente |
| WMI / PDH | **Non** | **Non** | **Non** | N/A |

**Recommandation** : LibreHardwareMonitor ou Intel PCM.

### 14.5 RAM - Utilisation

| Méthode | Total | Disponible | % Usage | Précision |
|---|---|---|---|---|
| `GlobalMemoryStatusEx` | **Oui** | **Oui** | **Oui** | Excellente |
| WMI (`Win32_OperatingSystem`) | **Oui** | **Oui** | Calculé | Bonne |
| PDH (`Memory\*`) | **Oui** | **Oui** | **Oui** | Très bonne |
| `GetPerformanceInfo` | **Oui** | **Oui** | Calculé | Excellente |

**Recommandation** : `GlobalMemoryStatusEx` pour la rapidité.

### 14.6 RAM - Fréquence

| Méthode | Fréquence nominale | Fréquence temps réel | Précision |
|---|---|---|---|
| WMI (`Win32_PhysicalMemory.Speed`) | **Oui** | **Non** (SPD nominal) | Bonne (nominal) |
| WMI (`ConfiguredClockSpeed`) | **Oui** | **Non** | Bonne |
| LibreHardwareMonitor | **Oui** | Partiel | Bonne |
| CPU-Z / HWiNFO | **Oui** | **Oui** | Excellente |

### 14.7 RAM - Timings (CAS, tRCD, tRP, tRAS)

| Méthode | Disponible | Précision |
|---|---|---|
| WMI | **Non** | N/A |
| Accès SMBus/SPD | **Oui** | Excellente |
| PassMark SysInfo SDK | **Oui** | Excellente |
| LibreHardwareMonitor | **Non** | N/A |

**Recommandation** : Accès SMBus direct ou SDK PassMark.

### 14.8 RAM - Bandwidth

| Méthode | Disponible | Per-canal | Précision |
|---|---|---|---|
| Intel PCM | **Oui** | **Oui** | Excellente |
| WMI / PDH | **Non** | N/A | N/A |
| GlobalMemoryStatusEx | **Non** | N/A | N/A |

**Recommandation** : Intel PCM (seule option réaliste).

### 14.9 Ventilateurs - RPM

| Méthode | Disponible | Fiable | Contrôle |
|---|---|---|---|
| WMI (`Win32_Fan`) | Théorique | **Non** (vide sur 95% des machines) | Non |
| LibreHardwareMonitor (Super I/O) | **Oui** | **Oui** | **Oui** (certains chips) |
| IPMI | **Oui** (serveurs) | **Oui** | **Oui** |

**Recommandation** : LibreHardwareMonitor.

### 14.10 Système - Températures carte mère (VRM, chipset)

| Méthode | Disponible | Capteurs |
|---|---|---|
| WMI (`Win32_TemperatureProbe`) | Théorique (vide en pratique) | N/A |
| WMI ACPI | 1 zone thermique | Peu précis |
| LibreHardwareMonitor (Super I/O) | **Oui** | VRM, chipset, auxiliaire |
| ASUS WMI (cartes ASUS) | **Oui** | VRM, chipset, T-sensor |
| IPMI | **Oui** (serveurs) | Multiples |

### 14.11 Système - Voltages

| Méthode | Disponible | Capteurs |
|---|---|---|
| WMI (`Win32_VoltageProbe`) | Théorique (vide en pratique) | N/A |
| LibreHardwareMonitor (Super I/O) | **Oui** | Vcore, VDIMM, 3.3V, 5V, 12V, etc. |
| MSR (CPU VID) | Partiel | VID seulement (pas Vcore réel) |
| IPMI | **Oui** (serveurs) | Multiples |

---

## 15. Tableau comparatif global

| Méthode | Privilèges | CPU Freq/core | CPU Usage/core | CPU Temp/core | CPU Power | RAM Usage | RAM Freq | RAM Timings | RAM BW | Fans | Voltages | Temp Mobo | Fan Control |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **WMI** | User/Admin | P-state (socket) | Oui | Zone ACPI | Non | Oui | Nominal | Non | Non | Vide* | Vide* | Vide* | Non |
| **PDH** | User | P-state (core) | Oui | Non | Non | Oui | Non | Non | Non | Non | Non | Non | Non |
| **NtPowerInfo** | User | P-state (core) | Oui** | Non | Non | Non | Non | Non | Non | Non | Non | Non | Non |
| **LHM** | **Admin** | **Réelle** | Oui | **Oui** | **Oui** | Oui | Oui | Non | Non | **Oui** | **Oui** | **Oui** | **Oui** |
| **OHM** | Admin | Réelle | Oui | Oui | Oui | Oui | Oui | Non | Non | Oui | Oui | Oui | Partiel |
| **MSR direct** | **Admin+Driver** | **Réelle** | Calcul | **Oui** | **Oui** | Non | Non | Non | Non | Non | Partiel | Non | Non |
| **ETW** | Admin | P-state events | Indirect | Non | Non | Non | Non | Non | Non | Non | Non | Non | Non |
| **PerfCounters** | User | P-state | Oui | Non | Non | Oui | Non | Non | Non | Non | Non | Non | Non |
| **GlobalMemStat** | User | N/A | N/A | N/A | N/A | **Oui** | Non | Non | Non | N/A | N/A | N/A | N/A |
| **Intel PCM** | Admin+Driver | Réelle | Oui | Non | **Oui** | Non | Non | Non | **Oui** | Non | Non | Non | Non |
| **SMBus/SPD** | Admin+Driver | N/A | N/A | N/A | N/A | N/A | Oui | **Oui** | N/A | N/A | N/A | N/A | N/A |
| **IPMI** | Admin | Non | Non | Oui | Oui | Non | Non | Non | Non | **Oui** | **Oui** | **Oui** | **Oui** |

> `*` = Nécessite un provider OEM rarement implémenté
> `**` = Via `NtQuerySystemInformation`

---

## 16. Recommandations architecturales

### 16.1 Architecture recommandée pour une app desktop

```
┌─────────────────────────────────────────────────────┐
│                Application Desktop                   │
├──────────────┬──────────────┬───────────────────────┤
│  User-Mode   │  User-Mode   │    Admin-Mode         │
│  (pas admin) │  (pas admin) │    (driver kernel)    │
├──────────────┼──────────────┼───────────────────────┤
│ GlobalMemory │ PDH          │ LibreHardwareMonitor  │
│ StatusEx     │ (CPU usage/  │ (tout le reste :      │
│ (RAM usage)  │  freq, RAM)  │  temp, fans, voltage, │
│              │              │  freq réelle, power,   │
│ CPUID        │ NtPowerInfo  │  GPU, Super I/O)      │
│ (CPU ident)  │ (freq/core)  │                       │
│              │              │ Intel PCM             │
│              │              │ (memory BW, cache)    │
│              │              │                       │
│              │              │ SMBus/SPD             │
│              │              │ (RAM timings)         │
└──────────────┴──────────────┴───────────────────────┘
```

### 16.2 Stratégie en couches

1. **Couche 1 (User mode, pas admin)** : `GlobalMemoryStatusEx` + PDH + `CallNtPowerInformation` + CPUID
   - RAM usage, CPU usage per-core, CPU freq P-state, identification CPU
   - Fonctionne toujours, sans élévation

2. **Couche 2 (Admin, avec LHM)** : LibreHardwareMonitor
   - Températures, fans, voltages, freq boost réelle, GPU, power
   - Nécessite "Exécuter en tant qu'administrateur"

3. **Couche 3 (Spécialisé)** : Intel PCM + SMBus custom
   - Memory bandwidth, cache stats, RAM timings
   - Pour les fonctionnalités avancées uniquement

### 16.3 Considérations Windows 11

- **HVCI** (Hypervisor-Enforced Code Integrity) bloque les drivers non signés
- Les drivers WinRing0 utilisés par LHM, PCM, et d'autres sont vulnérables au blocage
- Solutions possibles :
  - Signer le driver via Microsoft's WHQL process
  - Demander à l'utilisateur de désactiver HVCI (non recommandé)
  - Utiliser un driver WHQL signé existant
  - Attendre que LHM implémente un driver conforme

### 16.4 Alternative multi-plateforme

Pour une app Windows + Linux :
- **CPU usage** : `NtQuerySystemInformation` (Win) / `/proc/stat` (Linux)
- **CPU freq** : `CallNtPowerInformation` (Win) / `/sys/devices/system/cpu/*/cpufreq/` (Linux)
- **RAM** : `GlobalMemoryStatusEx` (Win) / `/proc/meminfo` (Linux)
- **Hardware sensors** : LibreHardwareMonitor (Win) / `lm-sensors` + `hwmon` (Linux)
- **Abstraction** : `psutil` (Python) ou `sysinfo` crate (Rust) couvrent les deux plateformes

---

## 17. Sources

### Documentation officielle Microsoft
- [Win32_Processor class](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-processor)
- [PDH Functions](https://learn.microsoft.com/en-us/windows/win32/perfctrs/using-the-pdh-functions-to-consume-counter-data)
- [PROCESSOR_POWER_INFORMATION](https://learn.microsoft.com/en-us/windows/win32/power/processor-power-information-str)
- [NtQuerySystemInformation](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation)
- [GlobalMemoryStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex)
- [MEMORYSTATUSEX](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex)
- [Memory Performance Information](https://learn.microsoft.com/en-us/windows/win32/memory/memory-performance-information)
- [Event Tracing for Windows](https://learn.microsoft.com/en-us/windows/win32/etw/event-tracing-portal)
- [Performance Counters](https://learn.microsoft.com/en-us/windows/win32/perfctrs/about-performance-counters)
- [Win32_PhysicalMemory](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemory)

### Projets open source
- [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor) - Monitoring hardware complet (C#, MPL 2.0)
- [OpenHardwareMonitor](https://github.com/openhardwaremonitor/openhardwaremonitor) - Prédécesseur de LHM (abandonné)
- [Intel PCM](https://github.com/intel/pcm) - Performance Counter Monitor (C++, BSD-3)
- [Intel CPU Frequency Library](https://github.com/intel/intel-cpu-frequency-library) - Échantillonnage fréquence via APERF/MPERF
- [CallNtPowerInformation example](https://github.com/erenpinaz/CallNtPowerInformation) - Exemple C++
- [WinMSR](https://github.com/cyring/WinMSR) - Driver MSR Windows 64-bit
- [msr-utility](https://github.com/cocafe/msr-utility) - CLI MSR via WinRing0
- [ETW provider manifest](https://github.com/repnz/etw-providers-docs/blob/master/Manifests-Win10-17134/Microsoft-Windows-Kernel-Processor-Power.xml)

### Ressources techniques
- [Kernel Processor Power ETW Provider](https://www.geoffchappell.com/studies/windows/km/ntoskrnl/events/microsoft-windows-kernel-processor-power.htm)
- [Calculating Core Frequencies with APERF/MPERF](https://www.dima.to/blog/calculating-the-core-frequencies-of-a-modern-intel-cpu-with-clock-varying-features-in-visual-c-on-a-windows-machine/)
- [Model Specific Registers - OSDev](https://wiki.osdev.org/Model_Specific_Registers)
- [Serial Presence Detect - Wikipedia](https://en.wikipedia.org/wiki/Serial_presence_detect)
- [LPC and SuperIO Communication](https://deepwiki.com/openhardwaremonitor/openhardwaremonitor/5.2-lpc-and-superio-communication)
- [ASUS WMI Sensors](https://maidavale.org/blog/investigating-asus-wmi-sensors-from-powershell/)
- [RAMMon - PassMark](https://www.passmark.com/products/rammon/index.php)
- [Intel PCM memory bandwidth on Windows](https://community.intel.com/t5/Software-Tuning-Performance/Intel-PCM-Measure-memory-bandwidth-on-Windows/td-p/985000)
- [NtDoc - NtQuerySystemInformation](https://ntdoc.m417z.com/ntquerysysteminformation)
- [CPU usage via performance counters (CodeProject)](https://www.codeproject.com/Articles/3413/How-to-get-CPU-usage-by-performance-counters-witho)
- [How to use LibreHardwareMonitor in C# projects](https://librehardwaremonitor.com/how-to-use-librehardwaremonitor-in-c-projects/)
