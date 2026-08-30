use std::collections::HashMap;
use std::sync::OnceLock;

// On embarque les fichiers JSON existants directement dans le binaire à la compilation !
const FR_JSON: &str = include_str!("../../public/locales/fr.json");
const EN_JSON: &str = include_str!("../../public/locales/en.json");

// Traductions des 22 autres langues européennes embarquées de manière compacte et optimisée via include_str!
const DE_JSON: &str = include_str!("../../public/locales/de.json");
const ES_JSON: &str = include_str!("../../public/locales/es.json");
const IT_JSON: &str = include_str!("../../public/locales/it.json");
const PT_JSON: &str = include_str!("../../public/locales/pt.json");
const NL_JSON: &str = include_str!("../../public/locales/nl.json");
const SV_JSON: &str = include_str!("../../public/locales/sv.json");
const DA_JSON: &str = include_str!("../../public/locales/da.json");
const FI_JSON: &str = include_str!("../../public/locales/fi.json");
const PL_JSON: &str = include_str!("../../public/locales/pl.json");
const CS_JSON: &str = include_str!("../../public/locales/cs.json");
const SK_JSON: &str = include_str!("../../public/locales/sk.json");
const HU_JSON: &str = include_str!("../../public/locales/hu.json");
const EL_JSON: &str = include_str!("../../public/locales/el.json");
const RO_JSON: &str = include_str!("../../public/locales/ro.json");
const BG_JSON: &str = include_str!("../../public/locales/bg.json");
const HR_JSON: &str = include_str!("../../public/locales/hr.json");
const SL_JSON: &str = include_str!("../../public/locales/sl.json");
const ET_JSON: &str = include_str!("../../public/locales/et.json");
const LV_JSON: &str = include_str!("../../public/locales/lv.json");
const LT_JSON: &str = include_str!("../../public/locales/lt.json");
const GA_JSON: &str = include_str!("../../public/locales/ga.json");
const MT_JSON: &str = include_str!("../../public/locales/mt.json");

static TRANSLATIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Détecte la langue du système via la variable d'environnement LANG (sur Unix) ou d'autres indicators.
fn detect_locale() -> String {
    if let Ok(lang) = std::env::var("LANG") {
        let clean_lang = lang.split('.').next().unwrap_or(&lang);
        let clean_lang = clean_lang.split('_').next().unwrap_or(clean_lang);
        return clean_lang.to_ascii_lowercase();
    }
    "en".to_string() // Langue par défaut
}

/// Helper pour associer un code de langue à son contenu JSON.
fn get_json_content(locale: &str) -> &'static str {
    match locale {
        "fr" => FR_JSON,
        "de" => DE_JSON,
        "es" => ES_JSON,
        "it" => IT_JSON,
        "pt" => PT_JSON,
        "nl" => NL_JSON,
        "sv" => SV_JSON,
        "da" => DA_JSON,
        "fi" => FI_JSON,
        "pl" => PL_JSON,
        "cs" => CS_JSON,
        "sk" => SK_JSON,
        "hu" => HU_JSON,
        "el" => EL_JSON,
        "ro" => RO_JSON,
        "bg" => BG_JSON,
        "hr" => HR_JSON,
        "sl" => SL_JSON,
        "et" => ET_JSON,
        "lv" => LV_JSON,
        "lt" => LT_JSON,
        "ga" => GA_JSON,
        "mt" => MT_JSON,
        _ => EN_JSON, // Fallback vers l'anglais
    }
}

/// Initialise les traductions au démarrage de l'application (avec éventuelle surcharge de langue).
pub fn init(override_lang: Option<String>) {
    let locale = override_lang
        .map(|l| l.to_ascii_lowercase())
        .unwrap_or_else(detect_locale);
    let json_content = get_json_content(&locale);
    let parsed: HashMap<String, String> = serde_json::from_str(json_content).unwrap_or_default();
    let _ = TRANSLATIONS.set(parsed);
}

/// Récupère la traduction pour une clé donnée, avec repli (fallback) sur la clé elle-même.
pub fn t(key: &str) -> String {
    TRANSLATIONS.get()
        .and_then(|map| map.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}
