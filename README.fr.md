# PowerCost Tracker

<p align="center">
  <img src="assets/logo.svg" alt="Logo PowerCost Tracker" width="120">
</p>

<p align="center">
  <strong>Une application de bureau legere pour surveiller la consommation electrique du PC et calculer les couts en temps reel.</strong>
</p>

<p align="center">
  Francais | <a href="README.md">English</a>
</p>

---

## Fonctionnalites

### Monitoring en temps reel
- **Lecture instantanee de la puissance** (Watts) depuis les capteurs materiels
- **Graphique de puissance en direct** montrant l'historique de consommation
- **Energie cumulee** depuis le demarrage de la session
- **Vues multi-periodes** (session, jour, semaine, mois)

### Configuration tarifaire flexible
- **Mode simple** : Tarif unique au kWh
- **Mode HP/HC** : Tarifs differents selon l'heure (Heures Pleines/Heures Creuses)
- **Mode saisonnier** : Differenciation ete/hiver
- **Mode Tempo** : Tarification style EDF avec couleurs de jour (bleu/blanc/rouge)
- **Support multi-devises** : EUR, USD, GBP, CHF, et plus

### Estimation des couts
- Calcul du cout en temps reel
- Projections horaires, journalieres et mensuelles
- Fonctionne sans configuration tarifaire (mode consommation uniquement)

### Empreinte minimale
- **< 50 Mo de RAM** utilisee
- **< 1% CPU** au repos
- **~5 Mo** de taille d'application
- Pas d'Electron - utilise le webview natif de l'OS

---

## Installation

### Binaires pre-compiles

Telechargez la derniere version pour votre plateforme :

| Plateforme | Format |
|------------|--------|
| Windows | Installeur `.msi` ou `.exe` portable |
| Linux | `.deb`, `.rpm`, ou `.AppImage` |

### Compilation depuis les sources

#### Prerequis

- **Rust** 1.70 ou plus recent
- **Node.js** 18 ou plus recent
- **Dependances systeme** :
  - Linux : `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`
  - Windows : WebView2 Runtime (inclus dans Windows 11)

#### Etapes de compilation

```bash
# Cloner le depot
git clone https://github.com/HLE-C0DE/PowerCost-Tracker.git
cd PowerCost-Tracker

# Installer les dependances frontend
cd ui && npm install && cd ..

# Compiler l'application
cargo tauri build
```

L'application compilee sera dans `src-tauri/target/release/`.

---

## Utilisation

### Monitoring de puissance

L'application detecte automatiquement les sources de monitoring disponibles :

| Plateforme | Source | Precision |
|------------|--------|-----------|
| Linux | Intel RAPL | Haute (mesure reelle) |
| Linux | AMD hwmon | Haute (mesure reelle) |
| Linux | Capteur batterie | Moyenne (pour portables) |
| Windows | WMI + estimation | Basse (basee sur charge CPU) |

Si aucun capteur materiel n'est disponible, l'application passe en mode estimation base sur la charge CPU.

### Configuration des tarifs

1. Ouvrez les **Parametres** depuis la barre laterale
2. Selectionnez votre **Mode de tarification** :
   - **Simple** : Entrez votre tarif au kWh
   - **HP/HC** : Definissez les tarifs et les plages horaires
3. Choisissez votre **Devise**
4. Cliquez sur **Enregistrer**

### Lecture du tableau de bord

- **Puissance actuelle** : Consommation instantanee en Watts
- **Energie de session** : Energie totale consommee depuis le lancement
- **Cout de session** : Cout cumule pour la session en cours
- **Estimations** : Couts projetes au taux de consommation actuel

---

## Configuration

La configuration est stockee dans :
- **Linux** : `~/.config/powercost-tracker/config.toml`
- **Windows** : `%APPDATA%/PowerCost-Tracker/config.toml`

### Exemple de configuration

```toml
[general]
language = "auto"        # "auto", "en", "fr"
theme = "dark"           # "dark", "light", "system"
refresh_rate_ms = 1000   # Intervalle de rafraichissement (1000-60000)
eco_mode = false         # Reduire le rafraichissement quand minimise
start_minimized = false
start_with_system = false

[pricing]
mode = "simple"          # "simple", "peak_offpeak", "seasonal", "tempo"
currency = "EUR"
currency_symbol = "\u20AC"

[pricing.simple]
rate_per_kwh = 0.2276

[pricing.peak_offpeak]
peak_rate = 0.27
offpeak_rate = 0.20
offpeak_start = "22:00"
offpeak_end = "06:00"

[widget]
enabled = true
show_cost = true
position = "bottom_right"
opacity = 0.9
```

---

## Permissions Linux

Pour lire les donnees Intel RAPL sur Linux, l'application a besoin d'acces a `/sys/class/powercap/`. Options :

### Option 1 : Executer avec privileges eleves (non recommande)
```bash
sudo powercost-tracker
```

### Option 2 : Ajouter une regle udev (recommande)
```bash
# Creer la regle udev
echo 'SUBSYSTEM=="powercap", ACTION=="add", RUN+="/bin/chmod -R a+r /sys/class/powercap/"' | \
  sudo tee /etc/udev/rules.d/99-powercap.rules

# Recharger les regles
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Option 3 : Accorder la capacite
```bash
sudo setcap cap_sys_rawio+ep /chemin/vers/powercost-tracker
```

---

## Depannage

### Badge "Estime" affiche

Cela signifie qu'aucun capteur de puissance materiel n'a ete detecte. Causes possibles :
- **Linux** : RAPL non disponible ou permission refusee
- **Windows** : Comportement normal (capteurs directs non disponibles)
- **Machine virtuelle** : Capteurs de puissance non exposes

### Valeurs semblent incorrectes

L'estimation de puissance est basee sur la charge CPU et les valeurs TDP typiques. Pour des lectures precises :
- Utilisez du materiel avec support RAPL (Intel/AMD)
- Sur Linux, assurez-vous des permissions correctes (voir ci-dessus)
- Envisagez d'utiliser des wattmetres externes pour validation

---

## Architecture

Voir [ARCHITECTURE.md](ARCHITECTURE.md) pour la documentation technique detaillee.

### Stack technique

- **Backend** : Rust + Tauri v2
- **Frontend** : Vanilla JS + CSS (sans framework)
- **Base de donnees** : SQLite (pour l'historique)
- **Configuration** : TOML

---

## Feuille de route

### v0.1 (Actuelle)
- [x] Monitoring de puissance en temps reel
- [x] Modes de tarification multiples
- [x] Interface bilingue (EN/FR)
- [x] Themes sombre/clair

### v0.2 (Prevue)
- [ ] Widget dans la barre systeme
- [ ] Suivi du surcout de session
- [ ] Export CSV

### v0.3 (Prevue)
- [ ] Repartition par composant materiel
- [ ] Profils multiples
- [ ] Alertes par notification

---

## Contribution

Les contributions sont les bienvenues ! Veuillez lire les directives de contribution avant de soumettre des PRs.

1. Forkez le depot
2. Creez une branche de fonctionnalite
3. Effectuez vos modifications
4. Soumettez une pull request

---

## Licence

Licence MIT - voir [LICENSE](LICENSE) pour les details.

---

## Remerciements

- [Tauri](https://tauri.app/) - Pour l'excellent framework
- [rusqlite](https://github.com/rusqlite/rusqlite) - Bindings SQLite pour Rust
- Documentation Intel RAPL pour les insights sur le monitoring de puissance
