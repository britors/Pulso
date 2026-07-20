use adw::prelude::*;
use gtk::{gio, glib};

pub fn show(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title("Preferências");
    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::new();
    group.set_title("Aparência");
    let model = gtk::StringList::new(&["Sistema", "Claro", "Escuro"]);
    let theme = adw::ComboRow::builder().title("Tema").model(&model).build();
    group.add(&theme);
    let autosave = adw::SpinRow::with_range(10.0, 600.0, 5.0);
    autosave.set_title("Salvamento automático");
    autosave.set_subtitle("Intervalo em segundos");
    group.add(&autosave);
    page.add(&group);
    dialog.add(&page);
    if let Some(settings) = settings() {
        let scheme = settings.string("color-scheme");
        theme.set_selected(match scheme.as_str() {
            "light" => 1,
            "dark" => 2,
            _ => 0,
        });
        autosave.set_value(settings.uint("autosave-interval") as f64);
        theme.connect_selected_notify(glib::clone!(
            #[weak]
            settings,
            move |row| {
                let value = ["system", "light", "dark"][row.selected() as usize];
                let _ = settings.set_string("color-scheme", value);
                apply_scheme(value);
            }
        ));
        autosave.connect_value_notify(glib::clone!(
            #[weak]
            settings,
            move |row| {
                let _ = settings.set_uint("autosave-interval", row.value() as u32);
            }
        ));
    }
    dialog.present(Some(parent));
}
pub fn settings() -> Option<gio::Settings> {
    let source = gio::SettingsSchemaSource::default()?;
    source
        .lookup("br.com.w3ti.Pulso", true)
        .map(|schema| gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None))
}
pub fn apply_saved_scheme() {
    if let Some(s) = settings() {
        apply_scheme(&s.string("color-scheme"));
    }
}
fn apply_scheme(value: &str) {
    adw::StyleManager::default().set_color_scheme(match value {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    });
}
