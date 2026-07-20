use crate::{preferences, window::PulsoWindow};
use adw::prelude::*;
use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
use gio::ActionEntry;

const APP_ID: &str = "br.com.w3ti.Pulso";

pub fn run() -> glib::ExitCode {
    setlocale(LocaleCategory::LcAll, "");
    let locale = option_env!("PULSO_LOCALEDIR").unwrap_or("/usr/share/locale");
    let _ = bindtextdomain("pulso", locale);
    let _ = bind_textdomain_codeset("pulso", "UTF-8");
    let _ = textdomain("pulso");
    gio::resources_register_include!("pulso.gresource").expect("resource bundle");
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_startup(|_| load_css());
    app.connect_activate(|app| {
        let window = PulsoWindow::new(app);
        window.present();
    });
    app.connect_open(|app, files, _| {
        let window = PulsoWindow::new(app);
        if let Some(path) = files.first().and_then(gio::File::path) {
            window.open_path(&path);
        }
        window.present();
    });
    app.add_action_entries([
        ActionEntry::builder("preferences")
            .activate(|app: &adw::Application, _, _| {
                if let Some(parent) = app.active_window() {
                    preferences::show(&parent);
                }
            })
            .build(),
        ActionEntry::builder("quit")
            .activate(|app: &adw::Application, _, _| app.quit())
            .build(),
    ]);
    app.set_accels_for_action("app.quit", &["<primary>q"]);
    app.run()
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/br/com/w3ti/Pulso/style.css");
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
