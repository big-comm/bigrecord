use adw::prelude::*;

pub const APP_ID: &str = "org.communitybig.bigrecord";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn new() -> adw::Application {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
        crate::ui::style::load();
    });

    app.connect_activate(crate::ui::window::build);
    app.set_accels_for_action("win.close", &["<primary>q"]);

    app
}
