use crate::model::Document;
use gtk::prelude::*;
pub fn rebuild(list: &gtk::ListBox, document: &Document, active: usize) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, slide) in document.slides.iter().enumerate() {
        let row = gtk::ListBoxRow::new();
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let preview = gtk::DrawingArea::builder()
            .content_width(180)
            .content_height(101)
            .build();
        let slide = slide.clone();
        preview.set_draw_func(move |_, cr, w, _| {
            let scale = w as f64 / 1280.0;
            let _ = cr.save();
            cr.scale(scale, scale);
            let _ = crate::canvas::render::render_slide(cr, &slide, None, None);
            let _ = cr.restore();
        });
        let label = gtk::Label::new(Some(&(i + 1).to_string()));
        box_.append(&preview);
        box_.append(&label);
        row.set_child(Some(&box_));
        list.append(&row);
        if i == active {
            list.select_row(Some(&row));
        }
    }
}
