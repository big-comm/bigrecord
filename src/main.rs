mod app;
mod audio;
mod i18n;
mod ui;

use adw::prelude::*;

fn main() -> glib::ExitCode {
    if let Err(err) = adw::init() {
        eprintln!("Failed to initialize libadwaita: {err}");
        return glib::ExitCode::FAILURE;
    }

    if let Err(err) = gst::init() {
        eprintln!("Failed to initialize GStreamer: {err}");
        return glib::ExitCode::FAILURE;
    }

    i18n::init();

    app::new().run()
}
