use std::env;
use std::path::PathBuf;

pub const GETTEXT_PACKAGE: &str = "bigrecorder";

pub fn init() {
    let _ = gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    apply_language_fallback();
    let _ = gettextrs::bindtextdomain(GETTEXT_PACKAGE, locale_dir());
    let _ = gettextrs::bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8");
    let _ = gettextrs::textdomain(GETTEXT_PACKAGE);
}

pub fn gettext(message: &str) -> String {
    gettextrs::gettext(message)
}

pub fn format_message(message: &str, values: &[(&str, &str)]) -> String {
    let mut output = message.to_string();
    for (key, value) in values {
        output = output.replace(key, value);
    }
    output
}

fn locale_dir() -> PathBuf {
    let local_dir = PathBuf::from("locale");
    if local_dir.exists() {
        return local_dir;
    }

    if let Ok(executable) = env::current_exe()
        && let Some(parent) = executable.parent()
    {
        for directory in parent.ancestors() {
            let candidate = directory.join("locale");
            if candidate.exists() {
                return candidate;
            }

            let installed_candidate = directory.join("share/locale");
            if installed_candidate.exists() {
                return installed_candidate;
            }
        }
    }

    PathBuf::from("/usr/share/locale")
}

fn apply_language_fallback() {
    if !locale_environment_is_c() {
        return;
    }

    for locale in language_locale_candidates() {
        if gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, locale.as_str()).is_some() {
            return;
        }

        if !locale.contains('.') {
            let utf8_locale = format!("{locale}.UTF-8");
            if gettextrs::setlocale(gettextrs::LocaleCategory::LcMessages, utf8_locale.as_str())
                .is_some()
            {
                return;
            }
        }
    }
}

fn locale_environment_is_c() -> bool {
    let locale = env::var("LC_ALL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("LC_MESSAGES")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            env::var("LANG")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    locale.is_some_and(|value| is_c_locale(&value))
}

fn is_c_locale(value: &str) -> bool {
    let locale = value.trim().to_ascii_lowercase();
    locale == "c" || locale == "posix" || locale.starts_with("c.")
}

fn language_locale_candidates() -> Vec<String> {
    env::var("LANGUAGE")
        .ok()
        .into_iter()
        .flat_map(|value| {
            value
                .split(':')
                .map(str::trim)
                .filter(|locale| !locale.is_empty() && !is_c_locale(locale))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}
