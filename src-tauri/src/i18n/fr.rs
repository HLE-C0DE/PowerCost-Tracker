//! French translations / Traductions fran\u{00E7}aises

use std::collections::HashMap;

pub fn get_translations() -> HashMap<String, String> {
    let mut t = HashMap::new();

    // App general
    t.insert("app.title".into(), "PowerCost Tracker".into());
    t.insert("app.version".into(), "Version".into());

    // Navigation
    t.insert("nav.dashboard".into(), "Tableau de bord".into());
    t.insert("nav.history".into(), "Historique".into());
    t.insert("nav.settings".into(), "Param\u{00E8}tres".into());
    t.insert("nav.about".into(), "\u{00C0} propos".into());

    // Dashboard
    t.insert("dashboard.current_power".into(), "Puissance actuelle".into());
    t.insert("dashboard.session_energy".into(), "\u{00C9}nergie de session".into());
    t.insert("dashboard.session_cost".into(), "Co\u{00FB}t de session".into());
    t.insert("dashboard.hourly_estimate".into(), "Estimation horaire".into());
    t.insert("dashboard.daily_estimate".into(), "Estimation journali\u{00E8}re".into());
    t.insert("dashboard.monthly_estimate".into(), "Estimation mensuelle".into());
    t.insert("dashboard.session_duration".into(), "Dur\u{00E9}e de session".into());
    t.insert("dashboard.power_source".into(), "Source de mesure".into());
    t.insert("dashboard.estimated".into(), "Estim\u{00E9}".into());
    t.insert("dashboard.measured".into(), "Mesur\u{00E9}".into());

    // Units
    t.insert("unit.watts".into(), "W".into());
    t.insert("unit.kilowatts".into(), "kW".into());
    t.insert("unit.watt_hours".into(), "Wh".into());
    t.insert("unit.kilowatt_hours".into(), "kWh".into());
    t.insert("unit.per_hour".into(), "/heure".into());
    t.insert("unit.per_day".into(), "/jour".into());
    t.insert("unit.per_month".into(), "/mois".into());

    // Settings - General
    t.insert("settings.general".into(), "G\u{00E9}n\u{00E9}ral".into());
    t.insert("settings.language".into(), "Langue".into());
    t.insert("settings.language.auto".into(), "D\u{00E9}tection automatique".into());
    t.insert("settings.theme".into(), "Th\u{00E8}me".into());
    t.insert("settings.theme.dark".into(), "Sombre".into());
    t.insert("settings.theme.light".into(), "Clair".into());
    t.insert("settings.theme.system".into(), "Syst\u{00E8}me".into());
    t.insert("settings.refresh_rate".into(), "Fr\u{00E9}quence de rafra\u{00EE}chissement".into());
    t.insert("settings.eco_mode".into(), "Mode \u{00E9}co".into());
    t.insert("settings.eco_mode.description".into(), "R\u{00E9}duire la fr\u{00E9}quence quand minimis\u{00E9}".into());
    t.insert("settings.start_minimized".into(), "D\u{00E9}marrer minimis\u{00E9}".into());
    t.insert("settings.start_with_system".into(), "D\u{00E9}marrer avec le syst\u{00E8}me".into());
    t.insert("settings.remember_window_position".into(), "M\u{00E9}moriser la position et la taille de la fen\u{00EA}tre".into());

    // Settings - Pricing
    t.insert("settings.pricing".into(), "Tarification".into());
    t.insert("settings.pricing.mode".into(), "Mode de tarification".into());
    t.insert("settings.pricing.mode.simple".into(), "Simple (tarif unique)".into());
    t.insert("settings.pricing.mode.peak_offpeak".into(), "Heures pleines/creuses".into());
    t.insert("settings.pricing.mode.seasonal".into(), "Saisonnier".into());
    t.insert("settings.pricing.mode.tempo".into(), "Tempo (style EDF)".into());
    t.insert("settings.pricing.currency".into(), "Devise".into());
    t.insert("settings.pricing.rate".into(), "Tarif au kWh".into());
    t.insert("settings.pricing.peak_rate".into(), "Tarif heures pleines".into());
    t.insert("settings.pricing.offpeak_rate".into(), "Tarif heures creuses".into());
    t.insert("settings.pricing.offpeak_start".into(), "D\u{00E9}but heures creuses".into());
    t.insert("settings.pricing.offpeak_end".into(), "Fin heures creuses".into());
    t.insert("settings.pricing.summer_rate".into(), "Tarif \u{00E9}t\u{00E9}".into());
    t.insert("settings.pricing.winter_rate".into(), "Tarif hiver".into());
    t.insert("settings.pricing.not_configured".into(), "Tarification non configur\u{00E9}e".into());
    t.insert("settings.pricing.configure_hint".into(), "Configurez la tarification pour voir les estimations de co\u{00FB}t".into());

    // Settings - Widget
    t.insert("settings.widget".into(), "Widget".into());
    t.insert("settings.widget.enabled".into(), "Activer le widget".into());
    t.insert("settings.widget.show_cost".into(), "Afficher le co\u{00FB}t".into());
    t.insert("settings.widget.show_power".into(), "Afficher la consommation uniquement".into());
    t.insert("settings.widget.position".into(), "Position".into());
    t.insert("settings.widget.position.top_left".into(), "Haut gauche".into());
    t.insert("settings.widget.position.top_right".into(), "Haut droite".into());
    t.insert("settings.widget.position.bottom_left".into(), "Bas gauche".into());
    t.insert("settings.widget.position.bottom_right".into(), "Bas droite".into());
    t.insert("settings.widget.opacity".into(), "Opacit\u{00E9}".into());
    t.insert("settings.widget.open".into(), "Ouvrir le widget".into());
    t.insert("settings.widget.close".into(), "Fermer le widget".into());

    // Settings - Pricing Tempo
    t.insert("settings.pricing.tempo.blue".into(), "Jours bleus".into());
    t.insert("settings.pricing.tempo.white".into(), "Jours blancs".into());
    t.insert("settings.pricing.tempo.red".into(), "Jours rouges".into());
    t.insert("settings.pricing.tempo.peak".into(), "Heures pleines".into());
    t.insert("settings.pricing.tempo.offpeak".into(), "Heures creuses".into());
    t.insert("settings.pricing.winter_months".into(), "Mois d'hiver".into());

    // Settings - Status
    t.insert("settings.saved".into(), "Param\u{00E8}tres enregistr\u{00E9}s avec succ\u{00E8}s".into());

    // Settings - Advanced
    t.insert("settings.advanced".into(), "Avanc\u{00E9}".into());
    t.insert("settings.advanced.baseline".into(), "Consommation de base".into());
    t.insert("settings.advanced.baseline.auto".into(), "D\u{00E9}tection automatique".into());
    t.insert("settings.advanced.baseline.manual".into(), "Manuel".into());
    t.insert("settings.advanced.baseline.description".into(), "Consommation de base pour le suivi du surplus".into());

    // History
    t.insert("history.title".into(), "Historique de consommation".into());
    t.insert("history.today".into(), "Aujourd'hui".into());
    t.insert("history.this_week".into(), "Cette semaine".into());
    t.insert("history.this_month".into(), "Ce mois".into());
    t.insert("history.custom_range".into(), "P\u{00E9}riode personnalis\u{00E9}e".into());
    t.insert("history.total_consumption".into(), "Consommation totale".into());
    t.insert("history.total_cost".into(), "Co\u{00FB}t total".into());
    t.insert("history.average_power".into(), "Puissance moyenne".into());
    t.insert("history.peak_power".into(), "Puissance maximale".into());
    t.insert("history.no_data".into(), "Aucune donn\u{00E9}e disponible pour cette p\u{00E9}riode".into());

    // About
    t.insert("about.title".into(), "\u{00C0} propos de PowerCost Tracker".into());
    t.insert("about.description".into(), "Une application de bureau l\u{00E9}g\u{00E8}re pour surveiller la consommation \u{00E9}lectrique du PC et calculer les co\u{00FB}ts d'\u{00E9}lectricit\u{00E9} en temps r\u{00E9}el.".into());
    t.insert("about.license".into(), "Licence : MIT".into());
    t.insert("about.source".into(), "Code source".into());

    // Errors and warnings
    t.insert("error.hardware_not_detected".into(), "Mat\u{00E9}riel de mesure non d\u{00E9}tect\u{00E9}".into());
    t.insert("error.using_estimation".into(), "Utilisation du mode estimation".into());
    t.insert("error.permission_denied".into(), "Permission refus\u{00E9}e".into());
    t.insert("error.save_failed".into(), "\u{00C9}chec de l'enregistrement".into());
    t.insert("warning.estimated_values".into(), "Les valeurs de puissance sont estim\u{00E9}es (aucun capteur direct d\u{00E9}tect\u{00E9})".into());

    // Actions
    t.insert("action.save".into(), "Enregistrer".into());
    t.insert("action.cancel".into(), "Annuler".into());
    t.insert("action.reset".into(), "R\u{00E9}initialiser".into());
    t.insert("action.close".into(), "Fermer".into());
    t.insert("action.minimize".into(), "Minimiser".into());
    t.insert("action.quit".into(), "Quitter".into());

    // Time
    t.insert("time.hours".into(), "heures".into());
    t.insert("time.minutes".into(), "minutes".into());
    t.insert("time.seconds".into(), "secondes".into());

    // Dashboard - Display modes
    t.insert("dashboard.mode.normal".into(), "Normal".into());
    t.insert("dashboard.mode.minimal".into(), "Minimal".into());


    // Dashboard - Edit mode
    t.insert("dashboard.edit_mode".into(), "Mode édition".into());
    t.insert("dashboard.default_layout".into(), "Disposition par défaut".into());
    t.insert("dashboard.toggle_widgets".into(), "Paramètres des widgets".into());
    t.insert("dashboard.done".into(), "Terminé".into());
    t.insert("dashboard.toggle_visibility".into(), "Visibilité des widgets".into());
    t.insert("dashboard.edit".into(), "Modifier le tableau de bord".into());
    t.insert("dashboard.edit_hint".into(), "Activez/désactivez les widgets et glissez pour réorganiser".into());
    t.insert("dashboard.reset_default".into(), "Réinitialiser".into());
    t.insert("dashboard.saved".into(), "Tableau de bord enregistré".into());
    t.insert("dashboard.save_failed".into(), "Échec de l'enregistrement".into());
    t.insert("dashboard.reset_success".into(), "Tableau de bord réinitialisé".into());
    t.insert("dashboard.edit_activated".into(), "Mode édition activé".into());
    t.insert("dashboard.changes_saved".into(), "Modifications enregistrées".into());
    t.insert("dashboard.default_applied".into(), "Disposition par défaut appliquée".into());
    t.insert("dashboard.display_mode".into(), "Mode d'affichage".into());

    // Layout profiles
    t.insert("dashboard.profile".into(), "Profil".into());
    t.insert("dashboard.save_profile".into(), "Sauvegarder le profil".into());
    t.insert("dashboard.delete_profile".into(), "Supprimer le profil".into());
    t.insert("dashboard.profile_saved".into(), "Profil sauvegard\u{00E9}".into());
    t.insert("dashboard.profile_deleted".into(), "Profil supprim\u{00E9}".into());
    t.insert("dashboard.expand_to_edit".into(), "Agrandissez la fen\u{00EA}tre pour modifier la disposition".into());
    t.insert("dashboard.profile_name_prompt".into(), "Nom du profil :".into());
    t.insert("dashboard.custom_layout".into(), "-- Personnalis\u{00E9} --".into());

    // Session tracking
    t.insert("session.no_active".into(), "Aucune session active".into());
    t.insert("session.start".into(), "Démarrer la session".into());
    t.insert("session.end".into(), "Terminer la session".into());
    t.insert("session.started".into(), "Session démarrée".into());
    t.insert("session.start_failed".into(), "Échec du démarrage de la session".into());
    t.insert("session.ended".into(), "Session terminée".into());
    t.insert("session.end_failed".into(), "Échec de la fin de session".into());
    t.insert("session.surplus".into(), "surplus".into());

    // Process list
    t.insert("processes.all".into(), "Tous les processus".into());
    t.insert("processes.search_placeholder".into(), "Rechercher des processus...".into());
    t.insert("processes.header.name".into(), "Processus".into());
    t.insert("processes.header.cpu".into(), "CPU %".into());
    t.insert("processes.header.gpu".into(), "GPU %".into());
    t.insert("processes.header.ram".into(), "RAM %".into());
    t.insert("processes.pinned".into(), "Épinglé".into());
    t.insert("processes.unpinned".into(), "Désépinglé".into());
    t.insert("processes.pin_failed".into(), "Échec de la mise à jour de l'épingle".into());
    t.insert("processes.killed".into(), "Processus arrêté".into());
    t.insert("processes.kill_failed".into(), "Échec de l'arrêt du processus".into());
    t.insert("processes.kill_confirm".into(), "Arrêter le processus".into());

    // Settings - Baseline detection
    t.insert("settings.baseline".into(), "Détection de base".into());
    t.insert("settings.baseline.auto".into(), "Détection automatique".into());
    t.insert("settings.baseline.manual".into(), "Consommation de base (W)".into());
    t.insert("settings.baseline.detected".into(), "Base détectée".into());
    t.insert("settings.baseline.detect_now".into(), "Détecter maintenant".into());
    t.insert("settings.baseline.detected_value".into(), "Base détectée".into());
    t.insert("settings.baseline.not_enough_data".into(), "Pas assez de données pour détecter la base".into());
    t.insert("settings.baseline.detect_failed".into(), "Échec de la détection".into());
    t.insert("settings.baseline.set_success".into(), "Base définie à".into());
    t.insert("settings.baseline.set_failed".into(), "Échec de la définition de la base".into());
    t.insert("settings.process_limit".into(), "Limite de processus".into());
    t.insert("settings.refresh_rate_detailed".into(), "Fréquence (Détaillée)".into());
    t.insert("settings.refresh_rate_critical".into(), "Fréquence (Critique)".into());

    // History - Daily breakdown
    t.insert("history.daily_breakdown".into(), "D\u{00E9}tail journalier".into());
    t.insert("history.date".into(), "Date".into());
    t.insert("history.energy".into(), "\u{00C9}nergie".into());
    t.insert("history.cost".into(), "Co\u{00FB}t".into());
    t.insert("history.rate".into(), "Tarif".into());
    t.insert("history.avg".into(), "Moy.".into());
    t.insert("history.peak".into(), "Max".into());

    // History - Tabs
    t.insert("history.tab.power".into(), "Puissance".into());
    t.insert("history.tab.sessions".into(), "Sessions".into());
    t.insert("history.no_sessions".into(), "Aucune session enregistr\u{00E9}e".into());

    // Tray menu
    t.insert("tray.show".into(), "Afficher".into());
    t.insert("tray.exit".into(), "Quitter".into());

    // Widget titles and labels
    t.insert("widget.cpu".into(), "CPU".into());
    t.insert("widget.gpu".into(), "GPU".into());
    t.insert("widget.ram".into(), "RAM".into());
    t.insert("widget.surplus".into(), "Surplus".into());
    t.insert("widget.session_controls".into(), "Session".into());
    t.insert("widget.processes".into(), "Processus actifs".into());
    t.insert("widget.loading".into(), "Chargement...".into());
    t.insert("widget.no_gpu".into(), "Aucun GPU d\u{00E9}tect\u{00E9}".into());
    t.insert("widget.no_process_data".into(), "Aucune donn\u{00E9}e de processus disponible".into());
    t.insert("widget.temp".into(), "Temp".into());
    t.insert("widget.power".into(), "Puissance".into());
    t.insert("widget.usage".into(), "Utilisation".into());
    t.insert("widget.cost".into(), "Co\u{00FB}t".into());
    t.insert("widget.baseline".into(), "Base".into());
    t.insert("widget.current".into(), "Actuel".into());
    t.insert("widget.set_baseline".into(), "D\u{00E9}finir la base".into());
    t.insert("widget.update_baseline".into(), "Mettre \u{00E0} jour la base".into());
    t.insert("widget.start_session_to_track".into(), "D\u{00E9}marrez une session pour suivre le surplus".into());
    t.insert("widget.session_active".into(), "Session active".into());
    t.insert("widget.show_top".into(), "Afficher le top".into());
    t.insert("widget.search_processes".into(), "Rechercher des processus".into());
    t.insert("widget.pin".into(), "\u{00C9}pingler".into());
    t.insert("widget.unpin".into(), "D\u{00E9}s\u{00E9}pingler".into());
    t.insert("widget.size.small".into(), "Petit".into());
    t.insert("widget.size.medium".into(), "Moyen".into());
    t.insert("widget.size.large".into(), "Grand".into());
    t.insert("widget.no_processes_found".into(), "Aucun processus trouv\u{00E9}".into());
    t.insert("widget.hide".into(), "Masquer le widget".into());
    t.insert("widget.display.bar".into(), "Barre".into());
    t.insert("widget.display.text".into(), "Texte".into());
    t.insert("widget.display.radial".into(), "Radial".into());
    t.insert("widget.display.chart".into(), "Graphique".into());
    t.insert("dashboard.display_mode_title".into(), "Mode d'affichage".into());
    t.insert("dashboard.edit_title".into(), "Modifier le tableau de bord".into());

    // Short widget titles (for 1×1 widgets)
    t.insert("dashboard.hourly_estimate_short".into(), "Horaire".into());
    t.insert("dashboard.daily_estimate_short".into(), "Journalier".into());
    t.insert("dashboard.monthly_estimate_short".into(), "Mensuel".into());
    t.insert("dashboard.session_energy_short".into(), "\u{00C9}nergie".into());
    t.insert("dashboard.session_cost_short".into(), "Co\u{00FB}t".into());
    t.insert("dashboard.session_duration_short".into(), "Dur\u{00E9}e".into());
    t.insert("dashboard.current_power_short".into(), "Puissance".into());
    t.insert("widget.processes_short".into(), "Procs".into());
    t.insert("widget.session_controls_short".into(), "Session".into());
    t.insert("widget.surplus_short".into(), "Surplus".into());

    // Estimation widget toggle labels
    t.insert("widget.show_cost".into(), "Co\u{00FB}t".into());
    t.insert("widget.show_energy".into(), "\u{00C9}nergie".into());

    // Extended hardware metrics
    t.insert("widget.fan".into(), "Ventilateur".into());
    t.insert("widget.clock".into(), "Horloge".into());
    t.insert("widget.mem_clock".into(), "Horloge m\u{00E9}m.".into());
    t.insert("widget.swap".into(), "Swap".into());
    t.insert("widget.speed".into(), "Vitesse".into());

    // Session editing
    t.insert("session.delete".into(), "Supprimer".into());
    t.insert("session.delete_confirm".into(), "Supprimer cette session ?".into());
    t.insert("session.edit_name".into(), "Modifier le nom".into());

    // Session naming and categories
    t.insert("session.name_placeholder".into(), "Nom de session...".into());
    t.insert("session.no_category".into(), "Aucune cat\u{00E9}gorie".into());
    t.insert("session.category".into(), "Cat\u{00E9}gorie".into());

    // Settings - Categories
    t.insert("settings.categories".into(), "Cat\u{00E9}gories de session".into());
    t.insert("settings.categories.add".into(), "Ajouter".into());
    t.insert("settings.categories.delete".into(), "Supprimer".into());
    t.insert("settings.categories.name_placeholder".into(), "Nom de cat\u{00E9}gorie".into());

    // History - Session histogram
    t.insert("history.usage".into(), "Utilisation".into());
    t.insert("history.7_days".into(), "7 jours".into());
    t.insert("history.30_days".into(), "30 jours".into());
    t.insert("history.12_months".into(), "12 mois".into());
    t.insert("history.custom".into(), "Personnalis\u{00E9}".into());
    t.insert("history.apply".into(), "Appliquer".into());
    t.insert("history.hours".into(), "heures".into());

    t
}
