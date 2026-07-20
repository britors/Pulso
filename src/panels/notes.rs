use gtk::prelude::*;
pub fn set_text(view: &gtk::TextView, text: &str) {
    view.buffer().set_text(text);
}
