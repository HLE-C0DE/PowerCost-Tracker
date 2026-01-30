---
title: "Capture de métriques système - Linux"
date: 2026-01-29
topic: system-metrics/linux
sources:
  - https://docs.kernel.org/cpu-freq/cpufreq-stats.html
  - https://wiki.archlinux.org/title/CPU_frequency_scaling
  - https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-devices-system-cpu
  - https://docs.kernel.org/hwmon/sysfs-interface.html
  - https://man7.org/linux/man-pages/man2/perf_event_open.2.html
  - https://web.eece.maine.edu/~vweaver/projects/rapl/
  - https://docs.kernel.org/power/powercap/powercap.html
  - https://man7.org/linux/man-pages/man4/msr.4.html
  - https://linux.die.net/man/3/libsensors
  - https://linux.die.net/man/1/ipmitool
  - https://github.com/sysstat/sysstat
  - https://wiki.archlinux.org/title/Fan_speed_control
  - https://linux.die.net/man/8/dmidecode
  - https://github.com/powercap/powercap
  - https://www.brendangregg.com/perf.html
  - https://github.com/torvalds/linux/blob/master/Documentation/hwmon/sysfs-interface.rst
status: final
---

# Capture de métriques système sous Linux

## Table des matières

1. [Introduction](#1-introduction)
2. [CPU - Fréquence par coeur](#2-cpu---fréquence-par-coeur)
3. [CPU - Utilisation par coeur](#3-cpu---utilisation-par-coeur)
4. [CPU - Température par coeur](#4-cpu---température-par-coeur)
5. [CPU - Consommation énergétique (RAPL)](#5-cpu---consommation-énergétique-rapl)
6. [RAM - Utilisation totale/disponible](#6-ram---utilisation-totaledisponible)
7. [RAM - Fréquence mémoire](#7-ram---fréquence-mémoire)
8. [RAM - Latence et timings](#8-ram---latence-et-timings)
9. [RAM - Bandwidth utilisé](#9-ram---bandwidth-utilisé)
10. [Ventilateurs - RPM et contrôle](#10-ventilateurs---rpm-et-contrôle)
11. [Système - Températures carte mère](#11-système---températures-carte-mère)
12. [Système - Voltages](#12-système---voltages)
13. [Tableau comparatif global](#13-tableau-comparatif-global)
14. [Sources](#14-sources)

---

## 1. Introduction

Ce document recense de manière exhaustive les méthodes disponibles sous Linux pour capturer les métriques matérielles d'un système desktop : fréquences CPU, utilisation, températures, consommation, RAM, ventilateurs et voltages. Pour chaque métrique, toutes les interfaces accessibles sont documentées avec leurs mécanismes techniques, exemples de code, privilèges requis, et compromis.

**Contexte** : application desktop cross-platform (Windows/Linux) nécessitant la lecture temps réel de métriques système. Ce document couvre exclusivement la partie Linux.

**Convention** : les chemins sysfs sont relatifs à `/sys/`. Les unités sont celles du kernel : millidegrés Celsius, millivolts, kilohertz, microjoules.

---

## 2. CPU - Fréquence par coeur

### 2.1 sysfs / cpufreq

**Chemin** : `/sys/devices/system/cpu/cpuN/cpufreq/`

**Mécanisme** : le sous-système cpufreq du noyau Linux expose les informations de fréquence de chaque coeur via le système de fichiers virtuel sysfs. Les fichiers sont mis à jour par le driver cpufreq actif (intel_pstate, acpi-cpufreq, amd-pstate, etc.).

**Fichiers clés** :

| Fichier | Description | Permission | Unité |
|---------|-------------|------------|-------|
| `scaling_cur_freq` | Fréquence courante (vue governor) | lecture user | kHz |
| `cpuinfo_cur_freq` | Fréquence courante (registre hardware) | lecture root | kHz |
| `cpuinfo_min_freq` | Fréquence minimum supportée | lecture user | kHz |
| `cpuinfo_max_freq` | Fréquence maximum supportée (boost) | lecture user | kHz |
| `scaling_min_freq` | Fréquence minimum autorisée | lecture/écriture root | kHz |
| `scaling_max_freq` | Fréquence maximum autorisée | lecture/écriture root | kHz |
| `base_frequency` | Fréquence de base (sans boost) | lecture user | kHz |
| `scaling_governor` | Governor actif | lecture/écriture root | - |
| `stats/time_in_state` | Temps passé par fréquence | lecture user | jiffies |

**Privilèges** : `scaling_cur_freq` lisible par tout utilisateur. `cpuinfo_cur_freq` requiert root ou capability `CAP_SYS_RAWIO`.

**Exemples de code** :

```c
// C - Lecture fréquence coeur 0
#include <stdio.h>
int main(void) {
    FILE *f = fopen("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq", "r");
    unsigned long freq_khz;
    fscanf(f, "%lu", &freq_khz);
    fclose(f);
    printf("CPU0: %lu kHz (%.2f GHz)\n", freq_khz, freq_khz / 1e6);
    return 0;
}
```

```python
# Python - Toutes les fréquences
import os, glob

for path in sorted(glob.glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")):
    cpu = path.split("/")[5]
    with open(path) as f:
        freq_khz = int(f.read().strip())
    print(f"{cpu}: {freq_khz/1e6:.2f} GHz")
```

```rust
// Rust - Lecture fréquence
use std::fs;

fn read_cpu_freq(core: u32) -> std::io::Result<u64> {
    let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq", core);
    let content = fs::read_to_string(path)?;
    Ok(content.trim().parse().unwrap())
}
```

```go
// Go - Lecture fréquence
package main

import (
    "fmt"
    "os"
    "strconv"
    "strings"
)

func readCPUFreq(core int) (uint64, error) {
    path := fmt.Sprintf("/sys/devices/system/cpu/cpu%d/cpufreq/scaling_cur_freq", core)
    data, err := os.ReadFile(path)
    if err != nil {
        return 0, err
    }
    return strconv.ParseUint(strings.TrimSpace(string(data)), 10, 64)
}
```

**Avantages** :
- Aucune dépendance externe, interface stable du noyau
- Très faible overhead (simple lecture de fichier)
- Accessible sans root pour `scaling_cur_freq`
- Disponible sur tous les systèmes Linux modernes

**Inconvénients** :
- `scaling_cur_freq` peut ne pas refléter la fréquence réelle (c'est la dernière valeur demandée par le governor)
- `cpuinfo_cur_freq` (hardware) requiert root
- Sur certains drivers (intel_pstate), la fréquence réelle n'est pas toujours exposée fidèlement
- Résolution temporelle limitée par la fréquence de mise à jour du driver

**Fiabilité** : Bonne pour `cpuinfo_cur_freq` (lecture hardware directe). Moyenne pour `scaling_cur_freq` (peut diverger de la fréquence réelle, surtout avec turbo boost dynamique).

---

### 2.2 MSR (Model-Specific Registers)

**Interface** : `/dev/cpu/N/msr`

**Mécanisme** : les MSR sont des registres internes au processeur accessibles via les instructions `rdmsr`/`wrmsr`. Sous Linux, le module noyau `msr` expose ces registres via des fichiers caractère. On accède à un registre par un `pread()` à l'offset correspondant au numéro du MSR.

**Registres pertinents (Intel)** :

| MSR | Adresse | Description |
|-----|---------|-------------|
| `IA32_PERF_STATUS` | 0x198 | Ratio de fréquence actuel |
| `IA32_PERF_CTL` | 0x199 | Contrôle du ratio de fréquence |
| `MSR_TURBO_RATIO_LIMIT` | 0x1AD | Limites turbo par nombre de coeurs actifs |
| `MSR_PLATFORM_INFO` | 0xCE | Ratio de base et limites |

**Privilèges** : Root requis (`CAP_SYS_RAWIO`). Module `msr` doit être chargé : `modprobe msr`.

**Exemple de code** :

```c
// C - Lecture MSR fréquence (Intel)
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdint.h>

int main(void) {
    int fd = open("/dev/cpu/0/msr", O_RDONLY);
    if (fd < 0) { perror("open msr"); return 1; }

    uint64_t val;
    // IA32_PERF_STATUS = 0x198
    if (pread(fd, &val, sizeof(val), 0x198) != sizeof(val)) {
        perror("pread"); return 1;
    }

    // Bits 15:8 = Current P-State ratio
    uint64_t ratio = (val >> 8) & 0xFF;
    double freq_ghz = ratio * 100.0 / 1000.0; // ratio * BCLK (100MHz)
    printf("CPU0 freq réelle: %.2f GHz (ratio=%lu)\n", freq_ghz, ratio);

    close(fd);
    return 0;
}
```

```bash
# Shell - via msr-tools
sudo modprobe msr
# Lire IA32_PERF_STATUS sur cpu0
sudo rdmsr -p 0 0x198
# Lire MSR_PLATFORM_INFO (ratio de base)
sudo rdmsr -p 0 0xCE -f 15:8
```

**Avantages** :
- Lecture hardware directe, précision maximale
- Accès aux registres non exposés via sysfs (limites turbo, etc.)
- Très faible latence de lecture

**Inconvénients** :
- Root obligatoire
- Spécifique au modèle de CPU (Intel vs AMD, et variations par micro-architecture)
- Module `msr` doit être chargé manuellement
- Risque de crash si écriture incorrecte
- Non portable entre constructeurs

**Fiabilité** : Excellente (lecture directe des registres hardware). C'est la source la plus fiable pour la fréquence temps réel.

---

### 2.3 perf_event (aperçu fréquence)

**Mécanisme** : le syscall `perf_event_open()` permet d'accéder aux compteurs de performance hardware. Bien que ce ne soit pas sa vocation première, on peut dériver la fréquence effective en mesurant le nombre de cycles CPU par seconde.

```c
// C - Mesure de cycles/seconde (proxy de fréquence)
#include <linux/perf_event.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>

static long perf_event_open(struct perf_event_attr *attr, pid_t pid,
                            int cpu, int group_fd, unsigned long flags) {
    return syscall(SYS_perf_event_open, attr, pid, cpu, group_fd, flags);
}

int main(void) {
    struct perf_event_attr pe;
    memset(&pe, 0, sizeof(pe));
    pe.type = PERF_TYPE_HARDWARE;
    pe.config = PERF_COUNT_HW_CPU_CYCLES;
    pe.disabled = 1;
    pe.exclude_kernel = 1;
    pe.exclude_hv = 1;

    int fd = perf_event_open(&pe, 0, -1, -1, 0);
    ioctl(fd, PERF_EVENT_IOC_RESET, 0);
    ioctl(fd, PERF_EVENT_IOC_ENABLE, 0);

    // Charge de travail de calibration
    volatile int sum = 0;
    for (int i = 0; i < 100000000; i++) sum += i;

    ioctl(fd, PERF_EVENT_IOC_DISABLE, 0);
    long long count;
    read(fd, &count, sizeof(count));
    printf("Cycles mesurés: %lld\n", count);
    close(fd);
    return 0;
}
```

**Privilèges** : dépend de `/proc/sys/kernel/perf_event_paranoid` (0 = user, 1 = restreint, 2 = root uniquement pour CPU-wide).

**Avantages** : pas besoin de parser sysfs, mesure dynamique réelle
**Inconvénients** : mesure indirecte de la fréquence, overhead plus élevé, complexité de configuration
**Fiabilité** : Bonne comme proxy, mais pas une mesure directe de fréquence.

---

## 3. CPU - Utilisation par coeur

### 3.1 /proc/stat

**Chemin** : `/proc/stat`

**Mécanisme** : le noyau maintient des compteurs de temps CPU cumulés depuis le boot, exprimés en jiffies (USER_HZ, typiquement 100 Hz = 10ms). Chaque ligne `cpuN` contient les compteurs par coeur. Pour obtenir un pourcentage d'utilisation, il faut lire deux fois avec un intervalle et calculer le delta.

**Format** :
```
cpu  user nice system idle iowait irq softirq steal guest guest_nice
cpu0 ...
cpu1 ...
```

**Formule** :
```
idle_time = idle + iowait
total_time = user + nice + system + idle + iowait + irq + softirq + steal
usage_% = 100 * (delta_total - delta_idle) / delta_total
```

**Privilèges** : aucun (lecture user).

**Exemples de code** :

```c
// C - Utilisation CPU par coeur
#include <stdio.h>
#include <unistd.h>
#include <string.h>

typedef struct {
    char name[16];
    unsigned long long user, nice, system, idle, iowait, irq, softirq, steal;
} CpuStat;

void read_stat(CpuStat *stats, int max_cpus, int *count) {
    FILE *f = fopen("/proc/stat", "r");
    char line[256];
    *count = 0;
    while (fgets(line, sizeof(line), f) && *count < max_cpus) {
        if (strncmp(line, "cpu", 3) != 0) break;
        CpuStat *s = &stats[(*count)++];
        sscanf(line, "%s %llu %llu %llu %llu %llu %llu %llu %llu",
               s->name, &s->user, &s->nice, &s->system, &s->idle,
               &s->iowait, &s->irq, &s->softirq, &s->steal);
    }
    fclose(f);
}

int main(void) {
    CpuStat prev[128], curr[128];
    int count;

    read_stat(prev, 128, &count);
    sleep(1);
    read_stat(curr, 128, &count);

    for (int i = 1; i < count; i++) { // skip i=0 (aggregate)
        unsigned long long prev_idle = prev[i].idle + prev[i].iowait;
        unsigned long long curr_idle = curr[i].idle + curr[i].iowait;

        unsigned long long prev_total = prev[i].user + prev[i].nice + prev[i].system +
            prev[i].idle + prev[i].iowait + prev[i].irq + prev[i].softirq + prev[i].steal;
        unsigned long long curr_total = curr[i].user + curr[i].nice + curr[i].system +
            curr[i].idle + curr[i].iowait + curr[i].irq + curr[i].softirq + curr[i].steal;

        double usage = 100.0 * (1.0 - (double)(curr_idle - prev_idle) /
                                       (double)(curr_total - prev_total));
        printf("%s: %.1f%%\n", curr[i].name, usage);
    }
    return 0;
}
```

```python
# Python - Utilisation par coeur
import time

def read_proc_stat():
    cpus = {}
    with open("/proc/stat") as f:
        for line in f:
            if not line.startswith("cpu"):
                break
            parts = line.split()
            name = parts[0]
            values = list(map(int, parts[1:]))
            cpus[name] = values
    return cpus

prev = read_proc_stat()
time.sleep(1)
curr = read_proc_stat()

for cpu in sorted(curr):
    if cpu == "cpu":
        continue  # skip aggregate
    p, c = prev[cpu], curr[cpu]
    p_idle = p[3] + p[4]
    c_idle = c[3] + c[4]
    p_total = sum(p[:8])
    c_total = sum(c[:8])
    usage = 100 * (1 - (c_idle - p_idle) / (c_total - p_total))
    print(f"{cpu}: {usage:.1f}%")
```

**Avantages** :
- Universel, présent sur tout système Linux
- Aucun privilège requis
- Très faible overhead
- Fonctionne dans les conteneurs et VMs

**Inconvénients** :
- Résolution temporelle limitée (jiffies = 10ms typiquement)
- Nécessite deux lectures avec un intervalle (pas d'instantané)
- Parser les lignes soi-même (pas d'API structurée)

**Fiabilité** : Excellente. C'est la source utilisée par `top`, `htop`, et tous les outils de monitoring standard.

---

### 3.2 sysstat (mpstat, sar)

**Mécanisme** : le package `sysstat` fournit des outils qui parsent `/proc/stat` et d'autres fichiers proc pour fournir des statistiques CPU formatées. `mpstat` est spécialisé pour les statistiques multi-processeur.

```bash
# Par coeur, toutes les secondes, 5 échantillons
mpstat -P ALL 1 5

# Sortie JSON (scriptable)
mpstat -P ALL -o JSON 1 1

# sar - historique CPU
sar -P ALL 1 5
```

**Privilèges** : aucun pour mpstat/sar. sadc (collecteur) peut nécessiter root pour certaines métriques.

**Langages** : principalement outil CLI. Pour intégration programmatique, parser la sortie JSON ou utiliser directement `/proc/stat`.

**Avantages** :
- Sortie formatée et prête à l'emploi, incluant JSON
- Historique avec sadc/sar
- Décomposition détaillée (%usr, %sys, %iowait, %irq, etc.)

**Inconvénients** :
- Dépendance au package sysstat
- Overhead de lancement de processus si utilisé depuis une app
- Pas d'API C/bibliothèque directe

**Fiabilité** : Excellente (même source que /proc/stat, avec calculs validés).

---

### 3.3 libprocps / libproc2

**Mécanisme** : bibliothèque C du projet procps-ng (qui fournit `ps`, `top`, `free`, `vmstat`). Offre une API pour lire les informations de `/proc` sans parser manuellement les fichiers.

**Installation** : `apt install libprocps-dev` (Debian/Ubuntu) ou `dnf install procps-ng-devel` (Fedora).

```c
// C - Utilisation basique de libproc2
#include <proc/readproc.h>
#include <stdio.h>

int main(void) {
    // Note: libproc2 ne calcule PAS le %CPU automatiquement.
    // Il faut lire les valeurs brutes et calculer soi-même.
    // La structure proc_t contient 'utime' et 'stime' (en ticks).
    // Pour le %CPU system-wide, /proc/stat reste recommandé.

    PROCTAB *pt = openproc(PROC_FILLSTAT | PROC_FILLSTATUS);
    proc_t proc;
    while (readproc(pt, &proc) != NULL) {
        printf("PID %d: utime=%llu stime=%llu\n",
               proc.tid, proc.utime, proc.stime);
    }
    closeproc(pt);
    return 0;
}
// Compiler: gcc -o proc_reader proc_reader.c -lprocps
```

**Note importante** : libprocps ne calcule pas le pourcentage CPU. Le champ `pcpu` existe dans la structure mais n'est pas rempli par `readproc()`. Le calcul doit être fait manuellement avec deux snapshots, exactement comme pour `/proc/stat` direct.

**Avantages** :
- API C structurée, pas de parsing manuel
- Intégration propre dans du code C/C++
- Même base de données que les outils standard (ps, top)

**Inconvénients** :
- Ne calcule pas le %CPU (il faut le faire soi-même)
- API peu documentée et changeante entre versions
- Spécifique Linux (pas portable)

**Fiabilité** : Bonne (wrapper autour de /proc).

---

## 4. CPU - Température par coeur

### 4.1 hwmon (sysfs)

**Chemin** : `/sys/class/hwmon/hwmonN/`

**Mécanisme** : le sous-système hwmon du noyau agrège les données de monitoring hardware provenant de différents drivers (coretemp pour Intel, k10temp pour AMD, etc.). Chaque chip de monitoring est exposé comme un répertoire hwmon avec des fichiers standardisés.

**Fichiers température** :

| Fichier | Description | Permission | Unité |
|---------|-------------|------------|-------|
| `temp1_input` | Température mesurée | lecture user | millidegré C |
| `temp1_max` | Seuil maximum | lecture(/écriture) | millidegré C |
| `temp1_crit` | Seuil critique | lecture | millidegré C |
| `temp1_label` | Étiquette lisible (ex: "Core 0") | lecture | texte |
| `name` | Nom du chip | lecture | texte |

**Identification du bon hwmon** : les numéros hwmon ne sont pas stables entre redémarrages. Il faut lire `name` pour identifier le chip :
- `coretemp` = températures CPU Intel
- `k10temp` = températures CPU AMD
- `nct6775` / `it8688` = chip de monitoring carte mère
- `amdgpu` / `nouveau` = GPU

```python
# Python - Découverte et lecture de toutes les températures hwmon
import os, glob

for hwmon in sorted(glob.glob("/sys/class/hwmon/hwmon*")):
    name_path = os.path.join(hwmon, "name")
    if os.path.exists(name_path):
        with open(name_path) as f:
            chip_name = f.read().strip()
    else:
        chip_name = "unknown"

    for temp in sorted(glob.glob(os.path.join(hwmon, "temp*_input"))):
        idx = os.path.basename(temp).split("_")[0]
        label_path = os.path.join(hwmon, f"{idx}_label")
        label = ""
        if os.path.exists(label_path):
            with open(label_path) as f:
                label = f.read().strip()

        with open(temp) as f:
            temp_mc = int(f.read().strip())
        print(f"[{chip_name}] {label or idx}: {temp_mc/1000:.1f}°C")
```

```c
// C - Lecture température Core 0 (coretemp)
#include <stdio.h>
#include <dirent.h>
#include <string.h>

// Trouver le hwmon pour "coretemp"
int find_coretemp_hwmon(char *path, size_t len) {
    DIR *d = opendir("/sys/class/hwmon");
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        if (ent->d_name[0] == '.') continue;
        char name_path[256];
        snprintf(name_path, sizeof(name_path),
                 "/sys/class/hwmon/%s/name", ent->d_name);
        FILE *f = fopen(name_path, "r");
        if (!f) continue;
        char name[64];
        fscanf(f, "%63s", name);
        fclose(f);
        if (strcmp(name, "coretemp") == 0) {
            snprintf(path, len, "/sys/class/hwmon/%s", ent->d_name);
            closedir(d);
            return 0;
        }
    }
    closedir(d);
    return -1;
}

int main(void) {
    char hwmon_path[256];
    if (find_coretemp_hwmon(hwmon_path, sizeof(hwmon_path)) < 0) {
        fprintf(stderr, "coretemp non trouvé\n");
        return 1;
    }
    char path[512];
    snprintf(path, sizeof(path), "%s/temp2_input", hwmon_path);
    FILE *f = fopen(path, "r");
    int temp_mc;
    fscanf(f, "%d", &temp_mc);
    fclose(f);
    printf("Core 0: %.1f°C\n", temp_mc / 1000.0);
    return 0;
}
```

**Privilèges** : lecture user pour la plupart des fichiers température. Certains fichiers d'écriture (seuils) nécessitent root.

**Avantages** :
- Interface unifiée et standardisée du noyau
- Aucune dépendance externe
- Labels humains disponibles (`temp1_label`)
- Supporte tous les drivers hwmon (CPU, GPU, carte mère, etc.)

**Inconvénients** :
- Numéros hwmon non stables entre boots (nécessite découverte dynamique)
- Les index de température (temp1, temp2...) varient selon le driver
- Pas de notification push (polling uniquement)

**Fiabilité** : Excellente. Source directe des drivers hardware du noyau.

---

### 4.2 libsensors (lm-sensors)

**Mécanisme** : bibliothèque C qui abstrait l'accès aux chips hwmon. Elle fournit une API itérative pour découvrir les chips, features et subfeatures, avec conversion automatique des valeurs.

**Installation** : `apt install libsensors-dev lm-sensors` puis `sudo sensors-detect` pour configurer les modules.

```c
// C - Lecture de toutes les températures via libsensors
#include <sensors/sensors.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    if (sensors_init(NULL) != 0) {
        fprintf(stderr, "sensors_init échoué\n");
        return 1;
    }

    const sensors_chip_name *chip;
    int chip_nr = 0;

    while ((chip = sensors_get_detected_chips(NULL, &chip_nr)) != NULL) {
        char chip_name[256];
        sensors_snprintf_chip_name(chip_name, sizeof(chip_name), chip);

        const sensors_feature *feat;
        int feat_nr = 0;
        while ((feat = sensors_get_features(chip, &feat_nr)) != NULL) {
            if (feat->type != SENSORS_FEATURE_TEMP) continue;

            char *label = sensors_get_label(chip, feat);
            const sensors_subfeature *sf = sensors_get_subfeature(
                chip, feat, SENSORS_SUBFEATURE_TEMP_INPUT);
            if (sf) {
                double val;
                sensors_get_value(chip, sf->number, &val);
                printf("[%s] %s: %.1f°C\n", chip_name, label, val);
            }
            free(label);
        }
    }

    sensors_cleanup();
    return 0;
}
// Compiler: gcc -o sensors_reader sensors_reader.c -lsensors
```

```python
# Python - via ctypes bindings (package sensors.py)
# pip install sensors
import sensors

sensors.init()
try:
    for chip in sensors.iter_detected_chips():
        for feature in chip:
            if feature.type == sensors.TEMP:
                print(f"[{chip}] {feature.label}: {feature.get_value():.1f}°C")
finally:
    sensors.cleanup()
```

**Métriques accessibles** :

| Type | Constante | Description |
|------|-----------|-------------|
| Température | `SENSORS_FEATURE_TEMP` | Températures (CPU, carte mère, GPU) |
| Voltage | `SENSORS_FEATURE_IN` | Voltages (Vcore, 3.3V, 5V, 12V) |
| Ventilateur | `SENSORS_FEATURE_FAN` | Vitesse ventilateurs (RPM) |
| Puissance | `SENSORS_FEATURE_POWER` | Consommation (W) |
| Courant | `SENSORS_FEATURE_CURR` | Courant (A) |
| Humidité | `SENSORS_FEATURE_HUMIDITY` | Humidité (%) |

**Avantages** :
- API haut niveau, abstraction complète de sysfs
- Gestion automatique de la configuration (`sensors3.conf`)
- Labels lisibles et mise à l'échelle des valeurs
- Wrappers disponibles : C++, Python, Rust (via FFI)

**Inconvénients** :
- Dépendance à libsensors (bibliothèque partagée)
- Nécessite `sensors-detect` pour configurer les modules
- Ne fournit que les chips configurés (peut manquer certains)
- Pas thread-safe par défaut

**Fiabilité** : Excellente. Utilisée en production par de nombreux outils de monitoring.

---

### 4.3 MSR (température CPU)

**Registres Intel** :
- `MSR_TEMPERATURE_TARGET` (0x1A2) : température cible (TjMax)
- `IA32_THERM_STATUS` (0x19C) : lecture thermique numérique par coeur
- `IA32_PACKAGE_THERM_STATUS` (0x1B1) : température package

**Calcul** : `T_actuelle = TjMax - Digital_Readout`

```c
// C - Température CPU via MSR (Intel)
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdint.h>

int main(void) {
    int fd = open("/dev/cpu/0/msr", O_RDONLY);
    uint64_t target, status;

    pread(fd, &target, 8, 0x1A2);  // MSR_TEMPERATURE_TARGET
    pread(fd, &status, 8, 0x19C);  // IA32_THERM_STATUS

    int tjmax = (target >> 16) & 0xFF;
    int digital_readout = (status >> 16) & 0x7F;
    int temp = tjmax - digital_readout;

    printf("TjMax: %d°C, Readout: %d, Temp: %d°C\n",
           tjmax, digital_readout, temp);
    close(fd);
    return 0;
}
```

**Privilèges** : root + module `msr` chargé.

**Avantages** : précision maximale, lecture directe du DTS (Digital Thermal Sensor)
**Inconvénients** : root, spécifique Intel (AMD utilise des MSR différents), non portable
**Fiabilité** : Excellente. Source primaire des données de température.

---

## 5. CPU - Consommation énergétique (RAPL)

### 5.1 Powercap sysfs (recommandé)

**Chemin** : `/sys/class/powercap/intel-rapl:N/`

**Mécanisme** : RAPL (Running Average Power Limit) utilise un modèle logiciel qui estime la consommation d'énergie à partir de compteurs hardware de performance et de modèles I/O. Ce n'est **pas** un wattmètre analogique, mais une estimation précise. Le noyau expose ces compteurs via le framework powercap.

**Structure** :

```
/sys/class/powercap/
├── intel-rapl:0/           # Package 0 (socket 0)
│   ├── name                # "package-0"
│   ├── energy_uj           # Énergie cumulée (microjoules)
│   ├── max_energy_range_uj # Plage max du compteur
│   ├── constraint_0_power_limit_uw  # Limite puissance (µW)
│   ├── constraint_0_time_window_us  # Fenêtre temporelle (µs)
│   ├── intel-rapl:0:0/     # Sous-zone "core"
│   │   ├── name            # "core"
│   │   └── energy_uj
│   └── intel-rapl:0:1/     # Sous-zone "uncore"
│       ├── name            # "uncore"
│       └── energy_uj
└── intel-rapl:1/           # Package 1 (si multi-socket)
```

**Calcul de la puissance** :
```
P(watts) = delta_energy_uj / delta_time_us
```

**Privilèges** : user depuis Linux 3.13, mais restrictions ajoutées depuis Linux 5.10 (lecture `energy_uj` limitée à root par défaut). Solution : ajouter une règle udev ou utiliser un groupe spécifique.

```python
# Python - Lecture puissance RAPL
import time

def read_energy(path="/sys/class/powercap/intel-rapl:0/energy_uj"):
    with open(path) as f:
        return int(f.read().strip())

e1 = read_energy()
t1 = time.monotonic()
time.sleep(1)
e2 = read_energy()
t2 = time.monotonic()

# Gestion overflow du compteur
max_range_path = "/sys/class/powercap/intel-rapl:0/max_energy_range_uj"
with open(max_range_path) as f:
    max_range = int(f.read().strip())

delta_e = e2 - e1
if delta_e < 0:
    delta_e += max_range  # overflow

delta_t = t2 - t1
power_w = delta_e / (delta_t * 1e6)
print(f"Puissance package: {power_w:.2f} W")
```

```c
// C - Lecture RAPL via libpowercap
// Bibliothèque: https://github.com/powercap/powercap
#include <powercap/powercap-rapl.h>
#include <stdio.h>

int main(void) {
    uint32_t n_pkgs = powercap_rapl_get_num_packages();
    powercap_rapl_pkg *pkgs = malloc(n_pkgs * sizeof(*pkgs));

    for (uint32_t i = 0; i < n_pkgs; i++) {
        powercap_rapl_init(i, &pkgs[i], 0);
        uint64_t energy;
        powercap_rapl_get_energy_uj(&pkgs[i], POWERCAP_RAPL_ZONE_PACKAGE, &energy);
        printf("Package %u: %lu µJ\n", i, energy);
        powercap_rapl_destroy(&pkgs[i]);
    }
    free(pkgs);
    return 0;
}
// Compiler: gcc -o rapl_reader rapl_reader.c -lpowercap
```

**Support AMD** : disponible depuis kernel 5.8 (famille 17h) et 5.11 (famille 19h). Le chemin est `amd-rapl:N` au lieu de `intel-rapl:N`.

**Avantages** :
- Interface standard du noyau, pas de dépendance externe
- Supporte Intel et AMD (kernels récents)
- Décomposition par zone (package, core, uncore, DRAM)
- Bibliothèque C disponible (libpowercap)

**Inconvénients** :
- Compteur overflow toutes les ~60 secondes (échantillonner régulièrement)
- Pas de mesure par processus (socket entier)
- Restrictions de permissions depuis Linux 5.10
- Estimation logicielle, pas mesure analogique directe

**Fiabilité** : Très bonne. Intel et AMD valident la précision de RAPL à ±5% dans les cas typiques.

---

### 5.2 MSR RAPL

**Registres** :
- `MSR_PKG_ENERGY_STATUS` (0x611) : énergie package
- `MSR_PP0_ENERGY_STATUS` (0x639) : énergie core
- `MSR_PP1_ENERGY_STATUS` (0x641) : énergie uncore/GPU intégré
- `MSR_DRAM_ENERGY_STATUS` (0x619) : énergie DRAM
- `MSR_RAPL_POWER_UNIT` (0x606) : unités de conversion

```c
// C - RAPL via MSR direct
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdint.h>
#include <math.h>

int main(void) {
    int fd = open("/dev/cpu/0/msr", O_RDONLY);
    uint64_t units_raw;
    pread(fd, &units_raw, 8, 0x606);  // MSR_RAPL_POWER_UNIT

    double energy_unit = pow(0.5, (units_raw >> 8) & 0x1F);

    uint64_t pkg_energy;
    pread(fd, &pkg_energy, 8, 0x611);  // MSR_PKG_ENERGY_STATUS

    double energy_j = (pkg_energy & 0xFFFFFFFF) * energy_unit;
    printf("Package energy: %.4f J\n", energy_j);
    close(fd);
    return 0;
}
```

**Privilèges** : root + module msr.

**Avantages** : accès direct sans framework powercap
**Inconvénients** : root, spécifique constructeur, gestion manuelle des unités
**Fiabilité** : Excellente (mêmes données que powercap, sans intermédiaire).

---

### 5.3 perf_event (RAPL)

**Mécanisme** : depuis Linux 3.14, les compteurs RAPL sont exposés via l'interface perf_event, permettant un accès programmatique unifié.

```bash
# Shell - Mesure énergie d'un programme
perf stat -e power/energy-pkg/,power/energy-cores/,power/energy-ram/ -- sleep 5
```

```c
// C - RAPL via perf_event_open()
#include <linux/perf_event.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    struct perf_event_attr pe;
    memset(&pe, 0, sizeof(pe));
    pe.type = 17;  // Type RAPL (vérifier /sys/bus/event_source/devices/power/type)
    pe.config = 0x2;  // energy-pkg (vérifier events/)
    pe.size = sizeof(pe);

    int fd = syscall(SYS_perf_event_open, &pe, -1, 0, -1, 0);
    if (fd < 0) { perror("perf_event_open"); return 1; }

    long long value;
    read(fd, &value, sizeof(value));
    // Valeur en unités définies par /sys/bus/event_source/devices/power/events/energy-pkg.scale
    printf("Energy: %lld\n", value);
    close(fd);
    return 0;
}
```

**Privilèges** : root ou `perf_event_paranoid < 1`.

**Avantages** : API unifiée avec les autres compteurs perf, support multiplexage
**Inconvénients** : complexité de configuration, permissions restrictives par défaut
**Fiabilité** : Excellente (mêmes compteurs hardware).

---

## 6. RAM - Utilisation totale/disponible

### 6.1 /proc/meminfo

**Chemin** : `/proc/meminfo`

**Mécanisme** : le noyau expose les statistiques mémoire globales dans ce fichier proc. Mis à jour en temps réel à chaque lecture.

**Champs clés** :

| Champ | Description |
|-------|-------------|
| `MemTotal` | RAM totale physique |
| `MemFree` | RAM libre (non utilisée) |
| `MemAvailable` | RAM disponible estimée (inclut caches récupérables) |
| `Buffers` | Mémoire tampon noyau |
| `Cached` | Cache de pages |
| `SwapTotal` | Swap total |
| `SwapFree` | Swap libre |
| `Active` | Pages actives |
| `Inactive` | Pages inactives |
| `Dirty` | Pages modifiées en attente d'écriture |

**Formule recommandée** (utilisation "réelle") :
```
used = MemTotal - MemAvailable
usage_% = 100 * used / MemTotal
```

> **Important** : ne PAS utiliser `MemTotal - MemFree` car cela inclut les caches et buffers qui sont récupérables. `MemAvailable` (kernel 3.14+) est l'estimation correcte.

**Privilèges** : aucun.

```c
// C - Lecture mémoire
#include <stdio.h>

int main(void) {
    FILE *f = fopen("/proc/meminfo", "r");
    char key[64];
    unsigned long value;
    char unit[8];
    unsigned long total = 0, available = 0;

    while (fscanf(f, "%63s %lu %7s", key, &value, unit) == 3) {
        if (strcmp(key, "MemTotal:") == 0) total = value;
        if (strcmp(key, "MemAvailable:") == 0) available = value;
    }
    fclose(f);

    printf("Total: %lu kB (%.1f GB)\n", total, total / 1048576.0);
    printf("Disponible: %lu kB (%.1f GB)\n", available, available / 1048576.0);
    printf("Utilisé: %.1f%%\n", 100.0 * (total - available) / total);
    return 0;
}
```

```python
# Python - Lecture mémoire
def read_meminfo():
    info = {}
    with open("/proc/meminfo") as f:
        for line in f:
            parts = line.split()
            key = parts[0].rstrip(":")
            info[key] = int(parts[1])  # en kB
    return info

mem = read_meminfo()
total_gb = mem["MemTotal"] / 1048576
avail_gb = mem["MemAvailable"] / 1048576
used_pct = 100 * (mem["MemTotal"] - mem["MemAvailable"]) / mem["MemTotal"]
print(f"Total: {total_gb:.1f} GB | Disponible: {avail_gb:.1f} GB | Utilisé: {used_pct:.1f}%")
```

```rust
// Rust - Lecture mémoire
use std::fs;
use std::collections::HashMap;

fn read_meminfo() -> HashMap<String, u64> {
    let content = fs::read_to_string("/proc/meminfo").unwrap();
    content.lines().filter_map(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let key = parts[0].trim_end_matches(':').to_string();
        let val: u64 = parts[1].parse().ok()?;
        Some((key, val))
    }).collect()
}
```

**Avantages** :
- Universel, aucune dépendance
- Très rapide, aucun overhead
- Données toujours à jour

**Inconvénients** :
- Pas de détail par processus (utiliser `/proc/[pid]/status` pour cela)
- Parser le fichier soi-même
- Pas de notion de fréquence ou latence mémoire

**Fiabilité** : Excellente. Source de référence pour toutes les statistiques mémoire.

---

### 6.2 sysinfo() (appel système)

**Mécanisme** : le syscall `sysinfo(2)` retourne une structure avec les statistiques mémoire de base.

```c
// C - sysinfo()
#include <sys/sysinfo.h>
#include <stdio.h>

int main(void) {
    struct sysinfo si;
    sysinfo(&si);

    unsigned long total = si.totalram * si.mem_unit;
    unsigned long free_mem = si.freeram * si.mem_unit;
    unsigned long used = total - free_mem;

    printf("Total: %.1f GB\n", total / 1e9);
    printf("Libre: %.1f GB\n", free_mem / 1e9);
    printf("Swap total: %.1f GB\n", (unsigned long)(si.totalswap * si.mem_unit) / 1e9);
    return 0;
}
```

**Privilèges** : aucun.

**Avantages** : API C propre, pas de parsing de fichier
**Inconvénients** : moins de détails que `/proc/meminfo`, pas de `MemAvailable`
**Fiabilité** : Bonne, mais préférer `/proc/meminfo` pour plus de détails.

---

## 7. RAM - Fréquence mémoire

### 7.1 dmidecode (SMBIOS)

**Mécanisme** : `dmidecode` lit les tables SMBIOS/DMI du BIOS, qui contiennent les informations statiques sur le matériel, dont la mémoire. Le type DMI 17 (Memory Device) contient la vitesse configurée.

```bash
# Vitesse et type de chaque barrette
sudo dmidecode --type 17 | grep -E 'Speed|Type:|Size|Locator'
```

Sortie typique :
```
Size: 16384 MB
Type: DDR5
Speed: 5600 MT/s
Configured Memory Speed: 4800 MT/s
```

**Privilèges** : root (accès aux tables SMBIOS).

**Programmatique** :

```python
# Python - Parser dmidecode
import subprocess, re

output = subprocess.check_output(["dmidecode", "--type", "17"], text=True)
for block in output.split("\n\n"):
    if "Memory Device" not in block:
        continue
    size = re.search(r"Size:\s+(.+)", block)
    speed = re.search(r"Configured Memory Speed:\s+(.+)", block)
    mtype = re.search(r"Type:\s+(\w+)", block)
    if size and "No Module" not in size.group(1):
        print(f"  Type: {mtype.group(1) if mtype else 'N/A'}")
        print(f"  Taille: {size.group(1)}")
        print(f"  Vitesse: {speed.group(1) if speed else 'N/A'}")
```

**Avantages** :
- Information statique fiable (vitesse nominale et configurée)
- Identifie le type de mémoire (DDR4, DDR5)
- Disponible sur pratiquement tous les systèmes

**Inconvénients** :
- Requiert root
- Information statique (vitesse configurée au boot, pas temps réel)
- Certains BIOS rapportent des valeurs incorrectes
- Lancement d'un processus externe si pas de bibliothèque

**Fiabilité** : Moyenne. dmidecode dépend de la qualité des tables BIOS, qui peuvent être incorrectes.

---

### 7.2 decode-dimms (SPD EEPROM)

**Mécanisme** : lit directement les données SPD (Serial Presence Detect) gravées sur les modules mémoire via le bus I2C. Ces données sont les spécifications officielles du fabricant.

```bash
# Installation et utilisation
sudo apt install i2c-tools
sudo modprobe eeprom     # ou 'ee1004' pour DDR4/DDR5
sudo decode-dimms
```

**Privilèges** : root + modules I2C chargés.

**Informations disponibles** :
- Type de mémoire (DDR4, DDR5, etc.)
- Vitesse maximale supportée
- CAS Latency (CL)
- tRCD (RAS to CAS Delay)
- tRP (Row Precharge)
- tRAS (Active to Precharge)
- Fabricant, numéro de pièce
- Tensions nominales

**Avantages** : données du fabricant, les plus fiables pour les timings
**Inconvénients** : root, nécessite modules I2C, pas toujours disponible (RAM soudée), lecture lente
**Fiabilité** : Excellente pour les spécifications nominales.

---

## 8. RAM - Latence et timings

### 8.1 CoreFreq

**Mécanisme** : module noyau + application console qui lit directement les registres du contrôleur mémoire (IMC) pour obtenir les timings actifs en temps réel.

```bash
# Installation
git clone https://github.com/cyring/CoreFreq
cd CoreFreq
make
sudo insmod corefreqk.ko
./corefreqd &
./corefreq-cli
```

**Timings disponibles** : CL, tRCD, tRP, tRAS, tRRD, tRFC, tWR, tRTP, tFAW, tCWL, et plus.

**Privilèges** : root (module noyau).

**Avantages** : timings réels actifs (pas nominaux), interface console riche
**Inconvénients** : module noyau custom (risque stabilité), pas d'API bibliothèque, limité à certains chipsets
**Fiabilité** : Excellente quand supporté.

---

### 8.2 Intel MLC (Memory Latency Checker)

**Mécanisme** : outil Intel qui mesure les latences et bande passante mémoire réelles par injection de charge.

```bash
# Latence idle
sudo ./mlc --idle_latency

# Latence sous charge
sudo ./mlc --loaded_latency

# Bande passante max
sudo ./mlc --max_bandwidth
```

**Privilèges** : root.

**Avantages** : mesure empirique réelle (pas une lecture de registre), très précis
**Inconvénients** : outil propriétaire Intel, pas d'API bibliothèque, impact sur les performances pendant la mesure
**Fiabilité** : Excellente pour la mesure empirique.

---

### 8.3 MSR (registres contrôleur mémoire)

Pour les timings actifs, certains registres MSR ou PCI config space du contrôleur mémoire intégré (IMC) contiennent les timings configurés. Cependant, ces registres sont très spécifiques à chaque plateforme (différents entre Intel et AMD, et entre générations).

**Intel** : espace PCI config du IMC (bus 0, device 0, function 0+), registres tCL, tRCD, etc.
**AMD** : registres SMN (System Management Network) accessibles via PCI config space.

**Privilèges** : root.

**Fiabilité** : Excellente mais extrêmement spécifique au matériel.

---

## 9. RAM - Bandwidth utilisé

### 9.1 perf (compteurs PMU)

**Mécanisme** : les compteurs de performance du processeur (PMU) mesurent les événements de cache et mémoire, permettant de calculer la bande passante.

```bash
# Mesure LLC misses (proxy de bandwidth)
perf stat -e LLC-loads,LLC-load-misses,LLC-stores,LLC-store-misses -a -- sleep 5

# Événements uncore pour bandwidth (Intel)
perf stat -e uncore_imc/data_reads/,uncore_imc/data_writes/ -a -- sleep 5
```

**Calcul** : `BW = (data_reads + data_writes) * 64 bytes / temps`

**Privilèges** : root ou `perf_event_paranoid < 1`.

**Avantages** : mesure en temps réel, par processus ou système complet
**Inconvénients** : noms d'événements spécifiques au matériel, complexe à configurer
**Fiabilité** : Bonne.

---

### 9.2 Intel PCM (Processor Counter Monitor)

**Mécanisme** : outil et bibliothèque Intel qui lit les compteurs uncore du IMC pour mesurer la bande passante mémoire par canal.

```bash
# Monitoring en temps réel
sudo pcm-memory

# Sortie CSV
sudo pcm-memory 1 -csv=output.csv
```

**Privilèges** : root.

**Avantages** : bande passante par canal DIMM, mise à jour en temps réel
**Inconvénients** : principalement Intel, requiert root
**Fiabilité** : Excellente.

---

### 9.3 STREAM Benchmark

**Mécanisme** : benchmark synthétique qui mesure la bande passante mémoire maximale soutenable via 4 opérations vectorielles (Copy, Scale, Add, Triad).

```bash
# Compilation et exécution
gcc -O3 -fopenmp -DSTREAM_ARRAY_SIZE=80000000 stream.c -o stream
OMP_NUM_THREADS=8 ./stream
```

**Note** : c'est un benchmark ponctuel, pas un outil de monitoring continu.

**Fiabilité** : Excellente comme mesure de référence de la bande passante maximale.

---

## 10. Ventilateurs - RPM et contrôle

### 10.1 hwmon (sysfs) - Lecture et contrôle

**Chemin** : `/sys/class/hwmon/hwmonN/`

**Fichiers ventilateurs** :

| Fichier | Description | Permission | Unité |
|---------|-------------|------------|-------|
| `fan1_input` | Vitesse actuelle | lecture user | RPM |
| `fan1_min` | Vitesse minimum | lecture/écriture | RPM |
| `fan1_max` | Vitesse maximum | lecture | RPM |
| `fan1_div` | Diviseur de mesure | lecture/écriture | puissance de 2 |
| `pwm1` | Niveau PWM (0-255) | lecture/écriture root | sans unité |
| `pwm1_enable` | Mode contrôle | lecture/écriture root | 0/1/2 |
| `pwm1_mode` | Mode signal | lecture/écriture | 0=DC, 1=PWM |

**Modes de `pwm1_enable`** :
- `0` : contrôle désactivé (ventilateur à fond)
- `1` : contrôle manuel (écrire dans `pwm1`)
- `2` : contrôle automatique (géré par le chip)

```python
# Python - Lecture RPM de tous les ventilateurs
import glob, os

for hwmon in sorted(glob.glob("/sys/class/hwmon/hwmon*")):
    name = open(os.path.join(hwmon, "name")).read().strip() if os.path.exists(os.path.join(hwmon, "name")) else "?"
    for fan in sorted(glob.glob(os.path.join(hwmon, "fan*_input"))):
        rpm = int(open(fan).read().strip())
        label_path = fan.replace("_input", "_label")
        label = open(label_path).read().strip() if os.path.exists(label_path) else os.path.basename(fan)
        print(f"[{name}] {label}: {rpm} RPM")
```

```python
# Python - Contrôle PWM (root requis)
import os

HWMON = "/sys/class/hwmon/hwmon3"  # Identifier le bon hwmon

def set_fan_manual(pwm_value):
    """Mettre le ventilateur en mode manuel et régler le PWM (0-255)."""
    # Passer en mode manuel
    with open(os.path.join(HWMON, "pwm1_enable"), "w") as f:
        f.write("1")
    # Régler le PWM
    with open(os.path.join(HWMON, "pwm1"), "w") as f:
        f.write(str(max(0, min(255, pwm_value))))

def set_fan_auto():
    """Remettre en mode automatique."""
    with open(os.path.join(HWMON, "pwm1_enable"), "w") as f:
        f.write("2")

# 50% = 128/255
set_fan_manual(128)
```

```c
// C - Lecture fan RPM
#include <stdio.h>
#include <dirent.h>
#include <string.h>

int main(void) {
    DIR *d = opendir("/sys/class/hwmon");
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        if (ent->d_name[0] == '.') continue;
        char path[512];
        snprintf(path, sizeof(path), "/sys/class/hwmon/%s/fan1_input", ent->d_name);
        FILE *f = fopen(path, "r");
        if (!f) continue;
        int rpm;
        fscanf(f, "%d", &rpm);
        fclose(f);

        // Lire le nom du chip
        snprintf(path, sizeof(path), "/sys/class/hwmon/%s/name", ent->d_name);
        f = fopen(path, "r");
        char name[64] = "?";
        if (f) { fscanf(f, "%63s", name); fclose(f); }

        printf("[%s] fan1: %d RPM\n", name, rpm);
    }
    closedir(d);
    return 0;
}
```

**Privilèges** :
- Lecture RPM : user
- Écriture PWM : root

**Avantages** :
- Interface standard du noyau
- Contrôle direct du PWM
- Supporte lecture et écriture

**Inconvénients** :
- Numéros hwmon instables
- Le contrôle manuel désactive la protection thermique automatique (dangereux)
- Certains drivers GPU (RDNA3) ne supportent pas le contrôle manuel
- Les fichiers PWM ne sont pas toujours présents

**Fiabilité** : Excellente pour la lecture. Le contrôle requiert de la prudence.

---

### 10.2 libsensors (lm-sensors)

Même API que pour les températures (section 4.2), en filtrant par `SENSORS_FEATURE_FAN`.

```c
// C - libsensors pour ventilateurs (extrait)
const sensors_feature *feat;
int feat_nr = 0;
while ((feat = sensors_get_features(chip, &feat_nr)) != NULL) {
    if (feat->type != SENSORS_FEATURE_FAN) continue;
    char *label = sensors_get_label(chip, feat);
    const sensors_subfeature *sf = sensors_get_subfeature(
        chip, feat, SENSORS_SUBFEATURE_FAN_INPUT);
    if (sf) {
        double rpm;
        sensors_get_value(chip, sf->number, &rpm);
        printf("%s: %.0f RPM\n", label, rpm);
    }
    free(label);
}
```

**Avantages** : API unifiée avec les autres capteurs, labels lisibles
**Inconvénients** : lecture seule (pas de contrôle PWM via libsensors)
**Fiabilité** : Excellente.

---

### 10.3 fancontrol (daemon)

**Mécanisme** : daemon du package lm-sensors qui gère automatiquement les ventilateurs via les fichiers hwmon/pwm, avec une configuration basée sur des courbes température-vitesse.

```bash
# Configuration initiale
sudo pwmconfig   # Assistant interactif

# Le fichier de config est /etc/fancontrol
# Lancement du daemon
sudo systemctl start fancontrol
```

**Format du fichier `/etc/fancontrol`** :
```
INTERVAL=10
DEVPATH=hwmon0=devices/platform/nct6775.2592
DEVNAME=hwmon0=nct6776
FCTEMPS=hwmon0/pwm1=hwmon0/temp1_input
FCFANS=hwmon0/pwm1=hwmon0/fan1_input
MINTEMP=hwmon0/pwm1=30
MAXTEMP=hwmon0/pwm1=60
MINSTART=hwmon0/pwm1=150
MINSTOP=hwmon0/pwm1=100
```

**Avantages** : gestion automatique, courbes configurables, intégration systemd
**Inconvénients** : pas d'API programmatique, configuration manuelle
**Fiabilité** : Bonne pour un contrôle continu.

---

### 10.4 IPMI (serveurs)

Pour les serveurs équipés d'un BMC (Baseboard Management Controller), IPMI donne accès aux ventilateurs même hors OS.

```bash
# Lire tous les capteurs fan
ipmitool sdr type fan

# Lire un capteur spécifique
ipmitool sensor get "FAN 1"

# Contrôle fan (Dell - commandes raw)
# Activer contrôle manuel
ipmitool raw 0x30 0x30 0x01 0x00
# Régler à 30%
ipmitool raw 0x30 0x30 0x02 0xff 0x1e
```

**Privilèges** : root + module IPMI (`ipmi_devintf`, `ipmi_si`).

**Avantages** : accès OOB (out-of-band), fonctionne même OS éteint, standard industriel
**Inconvénients** : uniquement serveurs/workstations avec BMC, commandes raw spécifiques au vendeur
**Fiabilité** : Excellente sur matériel supporté.

---

## 11. Système - Températures carte mère

### 11.1 hwmon (sysfs)

Les chips de monitoring carte mère (Nuvoton NCT6775/NCT6776/NCT6797, ITE IT8688E, etc.) exposent les températures VRM, chipset, et autres sondes auxiliaires via hwmon.

```python
# Python - Identifier les températures carte mère
import glob, os

for hwmon in sorted(glob.glob("/sys/class/hwmon/hwmon*")):
    name_path = os.path.join(hwmon, "name")
    if not os.path.exists(name_path):
        continue
    name = open(name_path).read().strip()

    # Chips carte mère typiques
    mb_chips = ["nct6775", "nct6776", "nct6779", "nct6797",
                "it8688", "it8689", "it8792", "f71882fg"]
    if name not in mb_chips:
        continue

    print(f"\n=== {name} ({hwmon}) ===")
    for temp in sorted(glob.glob(os.path.join(hwmon, "temp*_input"))):
        idx = os.path.basename(temp).split("_")[0]
        label_path = os.path.join(hwmon, f"{idx}_label")
        label = open(label_path).read().strip() if os.path.exists(label_path) else idx
        val = int(open(temp).read().strip()) / 1000
        if val > 0 and val < 120:  # filtrer les valeurs aberrantes
            print(f"  {label}: {val:.1f}°C")
```

**Sondes typiques** : SYSTIN (système), CPUTIN (CPU via carte mère), AUXTIN0-3 (auxiliaires), PCH (chipset).

**Privilèges** : lecture user.

**Fiabilité** : Variable selon le chip et la carte mère. Certaines sondes rapportent des valeurs erronées (-127°C, 255°C) si non connectées physiquement.

---

### 11.2 libsensors

Même utilisation que pour les températures CPU (section 4.2). Les chips carte mère sont automatiquement détectés par `sensors-detect`.

---

### 11.3 IPMI

```bash
# Températures système via IPMI
ipmitool sdr type Temperature

# Sortie typique :
# Inlet Temp       | 24 degrees C   | ok
# Exhaust Temp     | 32 degrees C   | ok
# CPU1 Temp        | 45 degrees C   | ok
# DIMM Temp        | 35 degrees C   | ok
```

**Privilèges** : root.
**Fiabilité** : Excellente sur serveurs avec BMC.

---

## 12. Système - Voltages

### 12.1 hwmon (sysfs)

**Fichiers voltage** :

| Fichier | Description | Permission | Unité |
|---------|-------------|------------|-------|
| `in0_input` | Voltage mesuré | lecture user | millivolt |
| `in0_min` | Seuil minimum | lecture/écriture | millivolt |
| `in0_max` | Seuil maximum | lecture/écriture | millivolt |
| `in0_label` | Étiquette | lecture | texte |

**Labels typiques** : Vcore, AVCC, +3.3V, +5V, +12V, Vbat, 3VSB, DRAM.

> **Attention** : les valeurs brutes ne sont pas toujours mises à l'échelle par le driver. Certains voltages (12V, 5V) nécessitent un facteur de mise à l'échelle basé sur les résistances du diviseur de tension, qui varie par carte mère.

```python
# Python - Lecture voltages
import glob, os

for hwmon in sorted(glob.glob("/sys/class/hwmon/hwmon*")):
    name = open(os.path.join(hwmon, "name")).read().strip() if os.path.exists(os.path.join(hwmon, "name")) else "?"
    for vin in sorted(glob.glob(os.path.join(hwmon, "in*_input"))):
        idx = os.path.basename(vin).split("_")[0]
        label_path = os.path.join(hwmon, f"{idx}_label")
        label = open(label_path).read().strip() if os.path.exists(label_path) else idx
        mv = int(open(vin).read().strip())
        print(f"[{name}] {label}: {mv/1000:.3f} V")
```

**Privilèges** : lecture user.

**Avantages** : interface standard, nombreux voltages disponibles
**Inconvénients** : mise à l'échelle parfois absente, labels génériques
**Fiabilité** : Variable. Dépend fortement du driver et de la carte mère.

---

### 12.2 libsensors

Filtrer par `SENSORS_FEATURE_IN` dans l'API libsensors.

```c
// C - Voltages via libsensors
if (feat->type == SENSORS_FEATURE_IN) {
    char *label = sensors_get_label(chip, feat);
    const sensors_subfeature *sf = sensors_get_subfeature(
        chip, feat, SENSORS_SUBFEATURE_IN_INPUT);
    if (sf) {
        double val;
        sensors_get_value(chip, sf->number, &val);
        printf("%s: %.3f V\n", label, val);
    }
    free(label);
}
```

**Avantages** : libsensors applique les facteurs de mise à l'échelle configurés dans `sensors3.conf`
**Fiabilité** : Meilleure que hwmon brut si la configuration est correcte.

---

### 12.3 IPMI

```bash
# Voltages via IPMI
ipmitool sdr type Voltage

# Exemple de sortie :
# Planar 3.3V     | 3.35 Volts     | ok
# Planar 5V       | 5.07 Volts     | ok
# Planar 12V      | 12.06 Volts    | ok
# Planar VBAT     | 3.12 Volts     | ok
```

**Fiabilité** : Excellente (calibré par le fabricant du serveur).

---

## 13. Tableau comparatif global

### Tableau des méthodes par métrique

| Métrique | sysfs/cpufreq | /proc/stat | /proc/meminfo | hwmon | libsensors | MSR | RAPL/powercap | perf_event | dmidecode | decode-dimms | IPMI | sysstat |
|----------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **CPU freq/coeur** | **X** | | | | | **X** | | (proxy) | | | | |
| **CPU usage/coeur** | | **X** | | | | | | **X** | | | | **X** |
| **CPU temp/coeur** | | | | **X** | **X** | **X** | | | | | **X** | |
| **CPU power** | | | | | **X** | **X** | **X** | **X** | | | **X** | |
| **RAM utilisation** | | | **X** | | | | | | | | | |
| **RAM fréquence** | | | | | | | | | **X** | **X** | | |
| **RAM timings** | | | | | | (IMC) | | | | **X** | | |
| **RAM bandwidth** | | | | | | | | **X** | | | | |
| **Fan RPM** | | | | **X** | **X** | | | | | | **X** | |
| **Fan contrôle** | | | | **X** | | | | | | | **X** | |
| **Temp carte mère** | | | | **X** | **X** | | | | | | **X** | |
| **Voltages** | | | | **X** | **X** | | | | | | **X** | |

### Tableau comparatif des méthodes

| Méthode | Privilèges | Overhead | Portabilité | Fiabilité | API native | Langages |
|---------|-----------|---------|-------------|-----------|-----------|----------|
| **sysfs/cpufreq** | user (partiel) | Très faible | Linux | Bonne | fichier texte | Tous |
| **/proc/stat** | user | Très faible | Linux | Excellente | fichier texte | Tous |
| **/proc/meminfo** | user | Très faible | Linux | Excellente | fichier texte | Tous |
| **hwmon** | user (lecture) | Très faible | Linux | Excellente | fichier texte | Tous |
| **libsensors** | user | Faible | Linux | Excellente | C | C, C++, Python, Rust (FFI) |
| **MSR** | root | Très faible | x86 Linux | Excellente | ioctl/pread | C, C++, Rust |
| **RAPL/powercap** | user/root* | Très faible | Intel/AMD Linux | Très bonne | fichier texte | Tous |
| **perf_event** | root/paranoid | Moyen | Linux | Excellente | syscall | C, C++, Rust, Go |
| **dmidecode** | root | Faible | Linux/SMBIOS | Moyenne | exec externe | Tous (via CLI) |
| **decode-dimms** | root | Moyen | Linux/I2C | Excellente | exec externe | Tous (via CLI) |
| **IPMI** | root | Moyen | Serveurs IPMI | Excellente | exec/lib | C, Python, Tous |
| **sysstat** | user | Faible | Linux | Excellente | exec/JSON | Tous (via CLI) |

*\* RAPL/powercap : user avant kernel 5.10, root depuis (sauf configuration udev spécifique).*

### Recommandations pour une application desktop

**Stratégie optimale en couches** :

1. **Couche primaire (user, aucune dépendance)** :
   - CPU freq : `sysfs/cpufreq` (`scaling_cur_freq`)
   - CPU usage : `/proc/stat`
   - RAM usage : `/proc/meminfo`
   - Températures + fans + voltages : `hwmon` sysfs

2. **Couche améliorée (avec libsensors)** :
   - Toutes les températures, fans, voltages via API C `libsensors`
   - Plus fiable grâce à la configuration automatique et mise à l'échelle

3. **Couche privilégiée (root/capabilities)** :
   - CPU freq réelle : MSR (`IA32_PERF_STATUS`)
   - CPU power : RAPL powercap ou MSR
   - RAM timings : `decode-dimms` ou CoreFreq
   - RAM fréquence : `dmidecode --type 17`

4. **Métriques avancées (optionnel)** :
   - RAM bandwidth : `perf stat` (compteurs uncore)
   - CPU power détaillée : `perf_event` RAPL

**Pour un monitoring temps réel sans root** : sysfs (cpufreq + hwmon) + /proc/stat + /proc/meminfo couvrent ~80% des besoins. L'ajout de libsensors améliore la robustesse. Les métriques restantes (power, RAM timings, bandwidth) nécessitent des privilèges élevés.

---

## 14. Sources

- [Linux Kernel - cpufreq stats](https://docs.kernel.org/cpu-freq/cpufreq-stats.html)
- [ArchWiki - CPU frequency scaling](https://wiki.archlinux.org/title/CPU_frequency_scaling)
- [Linux Kernel - sysfs-devices-system-cpu](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-devices-system-cpu)
- [Linux Kernel - hwmon sysfs interface](https://docs.kernel.org/hwmon/sysfs-interface.html)
- [GitHub - torvalds/linux hwmon docs](https://github.com/torvalds/linux/blob/master/Documentation/hwmon/sysfs-interface.rst)
- [man perf_event_open(2)](https://man7.org/linux/man-pages/man2/perf_event_open.2.html)
- [Brendan Gregg - Linux perf Examples](https://www.brendangregg.com/perf.html)
- [RAPL energy measurements from Linux](https://web.eece.maine.edu/~vweaver/projects/rapl/)
- [Linux Kernel - Power Capping Framework](https://docs.kernel.org/power/powercap/powercap.html)
- [GitHub - powercap/powercap (C library)](https://github.com/powercap/powercap)
- [man msr(4)](https://man7.org/linux/man-pages/man4/msr.4.html)
- [Intel - Reading/Writing MSRs in Linux](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/best-practices/reading-writing-msrs-in-linux.html)
- [man libsensors(3)](https://linux.die.net/man/3/libsensors)
- [GitHub - lm-sensors](https://github.com/lm-sensors/lm-sensors)
- [man ipmitool(1)](https://linux.die.net/man/1/ipmitool)
- [GitHub - ipmitool](https://github.com/ipmitool/ipmitool)
- [man dmidecode(8)](https://linux.die.net/man/8/dmidecode)
- [GitHub - sysstat](https://github.com/sysstat/sysstat)
- [ArchWiki - Fan speed control](https://wiki.archlinux.org/title/Fan_speed_control)
- [Intel Memory Latency Checker](https://www.intel.com/content/www/us/en/developer/articles/tool/intelr-memory-latency-checker.html)
- [GitHub - intel/memory-bandwidth-benchmarks](https://github.com/intel/memory-bandwidth-benchmarks)
- [GitHub - Brendan Gregg msr-cloud-tools](https://github.com/brendangregg/msr-cloud-tools)
- [GitHub - cyring/CoreFreq](https://github.com/cyring/CoreFreq)
- [Green Coding - CPU Frequency sysfs](https://docs.green-coding.io/docs/measuring/metric-providers/cpu-frequency-sysfs-core/)
- [Arm - perf_event_open tutorial](https://learn.arm.com/learning-paths/servers-and-cloud-computing/arm_pmu/perf_event_open/)
