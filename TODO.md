# PowerCost Tracker - Suivi de Developpement

> Application desktop cross-platform (Windows/Linux) pour mesurer la consommation electrique du PC en temps reel et calculer le cout base sur les tarifs personnalises.

---

## Progression Globale

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1 | **COMPLETE** | Setup & Architecture |
| Phase 2 | En attente | Core Engine |
| Phase 3 | **COMPLETE** | Interface utilisateur |
| Phase 4 | En attente | Cross-platform |
| Phase 5 | En attente | Bonus features |

---

## Phase 1 : Setup & Architecture

**Status: COMPLETE**

- [x] Analyser les options techniques disponibles
- [x] Choisir et documenter la stack (justifier dans ARCHITECTURE.md)
- [x] Setup du projet et structure de base
- [x] **CHECKPOINT** : Structure et ARCHITECTURE.md presentes

### Livrables Phase 1
- `ARCHITECTURE.md` - Documentation complete des choix techniques
- Structure Tauri v2 + Rust backend
- Frontend Vanilla JS/CSS
- Systeme i18n (FR/EN) configure
- README bilingues

---

## Phase 2 : Core Engine

**Status: EN ATTENTE**

- [ ] Implementer la recuperation de consommation (Linux d'abord via RAPL)
- [ ] Creer le systeme de calcul de cout
- [ ] Implementer la persistence des donnees (SQLite)
- [ ] **CHECKPOINT** : Demo CLI fonctionnelle

### Objectifs techniques
- Lecture RAPL fonctionnelle sous Linux
- Fallback estimation si pas de capteur
- Calcul temps reel des couts selon mode tarifaire
- Stockage historique dans SQLite

---

## Phase 3 : Interface utilisateur

**Status: COMPLETE**

- [x] Creer l'UI principale avec toutes les vues
- [x] Implementer le systeme i18n (FR/EN)
- [x] Creer le widget systeme
- [x] **CHECKPOINT** : App utilisable avec UI

### Vues a implementer
- Dashboard (puissance, energie, couts)
- Historique (graphiques, stats)
- Parametres (tarifs, langue, theme, widget)

---

## Phase 4 : Cross-platform

**Status: EN ATTENTE**

- [ ] Ajouter support Windows (WMI / estimation)
- [ ] Tests sur les deux plateformes
- [ ] Build et packaging (.deb, .AppImage, .msi)
- [ ] **CHECKPOINT** : Release candidates

### Formats de distribution
| Plateforme | Formats |
|------------|---------|
| Linux | `.deb`, `.rpm`, `.AppImage` |
| Windows | `.msi`, `.exe` portable |

---

## Phase 5 : Bonus features

**Status: EN ATTENTE**

### Bonus 1 : Tracking du surcout de session
- [ ] Definition d'une "baseline" (conso au repos)
  - [ ] Mode manuel
  - [ ] Mode automatique (idle detection)
- [ ] Calcul du "surplus" en temps reel
- [ ] Historique des sessions avec leur surcout

### Bonus 2 : Gestion hardware avancee
- [ ] Detection automatique des composants (CPU, GPU, RAM, disques)
- [ ] Affichage de la repartition de conso par composant
- [ ] Possibilite d'ignorer certains equipements
- [ ] Profils de monitoring (ex: "Gaming" vs "Travail")

- [ ] **CHECKPOINT** : Version complete

---

## Specifications fonctionnelles

### Core Features (MVP)

#### 1. Monitoring temps reel
- Recuperation de la consommation instantanee (Watts)
- Historique de consommation (graphiques)
- Consommation cumulee depuis le demarrage du PC
- Consommation sur differentes periodes (jour, semaine, mois)

#### 2. Configuration tarifaire flexible
- **Mode simple** : prix unique au kWh
- **Mode HP/HC** : heures pleines / heures creuses avec plages horaires configurables
- **Mode saisonnier** : tarifs differents selon la saison (ete/hiver)
- **Mode avance** : combinaison HP/HC + saisons (type Tempo EDF)
- Support des differentes devises (EUR, USD, GBP, etc.)

#### 3. Affichage des metriques
- Consommation instantanee (W)
- Consommation cumulee (Wh/kWh)
- Cout depuis demarrage du PC
- Cout estime par heure/jour/mois
- Mode "conso only" si pas de tarif configure

#### 4. Widget systeme
- Widget minimal toujours visible (optionnel)
- Affiche cout ou conso selon preference
- Clic pour ouvrir l'app complete
- Position personnalisable

---

## Contraintes techniques

### Langues
- [x] Interface bilingue : Francais et Anglais
- [x] Detection automatique de la langue systeme
- [x] Switch manuel possible dans les settings
- [x] Fichiers de traduction separes (i18n)

### Performance (CRITIQUE)
- [ ] < 50 MB RAM (cible : ~30-40 MB)
- [ ] < 1% CPU en idle
- [x] Pas de frameworks lourds (Tauri choisi vs Electron)
- [x] Refresh rate configurable (1s a 60s)
- [x] Mode "eco" avec refresh reduit

### Interface
- [x] Design moderne et minimaliste
- [x] Dark mode par defaut (+ light mode)
- [x] Responsive pour differentes resolutions
- [ ] Accessibilite (contraste, taille police)

---

## Stack technique choisie

| Composant | Choix | Justification |
|-----------|-------|---------------|
| Framework UI | **Tauri v2** | ~5MB vs 150MB Electron, 20-30MB RAM |
| Backend | **Rust** | Performance, acces systeme direct, securite memoire |
| Frontend | **Vanilla JS/CSS** | Zero overhead framework, <50KB |
| Database | **SQLite** | Embarque, leger, ACID |
| Config | **TOML** | Lisible, support natif Rust |

### Sources de monitoring
| OS | Source | Precision |
|----|--------|-----------|
| Linux | Intel RAPL (`/sys/class/powercap`) | Haute |
| Linux | AMD hwmon | Haute |
| Linux | Batterie laptop | Moyenne |
| Windows | WMI + estimation | Basse |
| Fallback | Estimation TDP | Basse |

---

## Notes importantes

### Ce qu'on evite
- Electron ou frameworks web lourds
- Dependances inutiles
- UI surchargee avec animations partout
- App qui consomme plus qu'elle ne mesure

### Ce qu'on vise
- Code propre et documente
- Gestion d'erreurs robuste (hardware non detecte, etc.)
- Configuration simple pour l'utilisateur lambda
- Mode avance pour les power users
- App utilisable meme sans configurer les tarifs

### Gestion des limitations hardware
- Mode "estimation" base sur les specs hardware
- Information claire sur la precision des donnees
- Badge "Estime" visible dans l'UI

---

## Changelog

### 2024-XX-XX - Phase 1 Complete
- Setup projet Tauri v2 + Rust
- Architecture modulaire (core, hardware, pricing, i18n, db)
- Frontend Vanilla JS avec dashboard, historique, parametres
- Systeme i18n FR/EN (70+ cles)
- Documentation complete (README, ARCHITECTURE.md)
- Scripts de build et setup permissions Linux
