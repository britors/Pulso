use crate::model::Document;
use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone)]
pub struct PresenterWindow {
    pub window: gtk::Window,
    current_area: gtk::DrawingArea,
    next_area: gtk::DrawingArea,
    notes: gtk::Label,
    current: Rc<Cell<usize>>,
    document: Rc<RefCell<Document>>,
}

impl PresenterWindow {
    pub fn new(
        document: Rc<RefCell<Document>>,
        media: Rc<RefCell<HashMap<String, Vec<u8>>>>,
        current: Rc<Cell<usize>>,
    ) -> Self {
        let window = gtk::Window::builder()
            .title("Visão do apresentador")
            .default_width(1100)
            .default_height(700)
            .build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        root.set_margin_start(18);
        root.set_margin_end(18);
        root.set_margin_top(18);
        root.set_margin_bottom(18);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let clock = gtk::Label::new(None);
        clock.add_css_class("title-2");
        let timer = gtk::Label::new(Some("00:00"));
        timer.add_css_class("title-2");
        let pause = gtk::Button::with_label("Pausar");
        let reset = gtk::Button::with_label("Zerar");
        header.append(&clock);
        header.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        header.append(&timer);
        header.append(&pause);
        header.append(&reset);
        root.append(&header);
        let previews = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        previews.set_vexpand(true);
        let current_area = gtk::DrawingArea::new();
        current_area.set_hexpand(true);
        current_area.set_vexpand(true);
        let next_area = gtk::DrawingArea::new();
        next_area.set_hexpand(true);
        next_area.set_vexpand(true);
        previews.append(&current_area);
        previews.append(&next_area);
        root.append(&previews);
        let notes = gtk::Label::new(None);
        notes.set_wrap(true);
        notes.set_xalign(0.0);
        notes.set_selectable(true);
        notes.add_css_class("title-3");
        root.append(&notes);
        window.set_child(Some(&root));
        current_area.set_draw_func(glib::clone!(
            #[strong]
            document,
            #[strong]
            media,
            #[strong]
            current,
            move |_, cr, width, height| render_preview(
                cr,
                width,
                height,
                &document,
                &media,
                current.get()
            )
        ));
        next_area.set_draw_func(glib::clone!(
            #[strong]
            document,
            #[strong]
            media,
            #[strong]
            current,
            move |_, cr, width, height| render_preview(
                cr,
                width,
                height,
                &document,
                &media,
                (current.get() + 1).min(document.borrow().slides.len() - 1)
            )
        ));
        let running = Rc::new(Cell::new(true));
        let elapsed = Rc::new(Cell::new(0_u64));
        pause.connect_clicked(glib::clone!(
            #[strong]
            running,
            move |button| {
                running.set(!running.get());
                button.set_label(if running.get() { "Pausar" } else { "Continuar" });
            }
        ));
        reset.connect_clicked(glib::clone!(
            #[strong]
            elapsed,
            #[weak]
            timer,
            move |_| {
                elapsed.set(0);
                timer.set_label("00:00");
            }
        ));
        glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak]
                window,
                #[weak]
                clock,
                #[weak]
                timer,
                #[strong]
                running,
                #[strong]
                elapsed,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let _ = &window;
                    if running.get() {
                        elapsed.set(elapsed.get() + 1);
                    }
                    let seconds = elapsed.get();
                    timer.set_label(&format!("{:02}:{:02}", seconds / 60, seconds % 60));
                    if let Ok(now) = glib::DateTime::now_local() {
                        if let Ok(value) = now.format("%H:%M:%S") {
                            clock.set_label(&value);
                        }
                    }
                    glib::ControlFlow::Continue
                }
            ),
        );
        let presenter = Self {
            window,
            current_area,
            next_area,
            notes,
            current,
            document,
        };
        presenter.refresh();
        presenter
    }
    pub fn refresh(&self) {
        self.current_area.queue_draw();
        self.next_area.queue_draw();
        self.notes
            .set_label(&self.document.borrow().slides[self.current.get()].notes);
    }
}

fn render_preview(
    cr: &cairo::Context,
    width: i32,
    height: i32,
    document: &RefCell<Document>,
    media: &RefCell<HashMap<String, Vec<u8>>>,
    index: usize,
) {
    let scale = (width as f64 / 1280.0).min(height as f64 / 720.0);
    cr.translate(
        (width as f64 - 1280.0 * scale) / 2.0,
        (height as f64 - 720.0 * scale) / 2.0,
    );
    cr.scale(scale, scale);
    let _ = crate::canvas::render::render_slide(
        cr,
        &document.borrow().slides[index],
        Some(&media.borrow()),
        None,
    );
}
