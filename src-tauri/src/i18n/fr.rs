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
    t.insert("warning.estimated_values".into(), "Les valeurs sont estim\u{00E9}es et peuvent ne pas \u{00EA}tre pr\u{00E9}cises".into());

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

    t
}
