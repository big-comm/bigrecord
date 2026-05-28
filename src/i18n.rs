use std::env;
use std::path::PathBuf;

pub const GETTEXT_PACKAGE: &str = "bigrecorder";

pub fn init() {
    let _ = gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
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
