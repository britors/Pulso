use crate::model::{Document, TransitionType};
use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

pub fn present(
    document: Rc<RefCell<Document>>,
    media: Rc<RefCell<HashMap<String, Vec<u8>>>>,
    start: usize,
    on_close: impl Fn(usize) + 'static,
) {
    let window = gtk::Window::builder().title("Pulso").build();
    let area = gtk::DrawingArea::new();
    window.set_child(Some(&area));
    let current = Rc::new(Cell::new(start));
    let previous = Rc::new(Cell::new(start));
    let progress = Rc::new(Cell::new(1.0));
    let direction = Rc::new(Cell::new(1.0));
    let animation = Rc::new(RefCell::new(None::<adw::TimedAnimation>));
    let black = Rc::new(Cell::new(false));
    let display = gtk::gdk::Display::default();
    let monitors = display.as_ref().map(gtk::gdk::Display::monitors);
    let primary_monitor = monitors
        .as_ref()
        .and_then(|items| items.item(0))
        .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok());
    let secondary_monitor = monitors
        .as_ref()
        .filter(|items| items.n_items() > 1)
        .and_then(|items| items.item(1))
        .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok());
    let presenter = secondary_monitor.as_ref().map(|_| {
        crate::show::presenter_window::PresenterWindow::new(
            document.clone(),
            media.clone(),
            current.clone(),
        )
    });
    if let Some(monitor) = secondary_monitor.as_ref() {
        window.fullscreen_on_monitor(monitor);
    } else {
        window.fullscreen();
    }
    if let Some(presenter) = presenter.as_ref() {
        if let Some(monitor) = primary_monitor.as_ref() {
            presenter.window.fullscreen_on_monitor(monitor);
        }
        presenter.window.present();
    }
    area.set_draw_func(glib::clone!(
        #[strong]
        document,
        #[strong]
        media,
        #[strong]
        current,
        #[strong]
        previous,
        #[strong]
        progress,
        #[strong]
        direction,
        #[strong]
        black,
        move |_, cr, width, height| {
            if black.get() {
                cr.set_source_rgb(0.0, 0.0, 0.0);
                let _ = cr.paint();
                return;
            }
            let sx = width as f64 / 1280.0;
            let sy = height as f64 / 720.0;
            let scale = sx.min(sy);
            cr.translate(
                (width as f64 - 1280.0 * scale) / 2.0,
                (height as f64 - 720.0 * scale) / 2.0,
            );
            cr.scale(scale, scale);
            let document = document.borrow();
            let media = media.borrow();
            let value = progress.get();
            let transition = document.slides[current.get()].transition.kind;
            if value < 1.0 && transition != TransitionType::None {
                let _ = crate::canvas::render::render_slide(
                    cr,
                    &document.slides[previous.get()],
                    Some(&media),
                    None,
                );
                match transition {
                    TransitionType::Fade => {
                        cr.push_group();
                        let _ = crate::canvas::render::render_slide(
                            cr,
                            &document.slides[current.get()],
                            Some(&media),
                            None,
                        );
                        let _ = cr.pop_group_to_source();
                        let _ = cr.paint_with_alpha(value);
                    }
                    TransitionType::Slide => {
                        let _ = cr.save();
                        cr.translate(direction.get() * 1280.0 * (1.0 - value), 0.0);
                        let _ = crate::canvas::render::render_slide(
                            cr,
                            &document.slides[current.get()],
                            Some(&media),
                            None,
                        );
                        let _ = cr.restore();
                    }
                    TransitionType::None => {}
                }
            } else {
                let _ = crate::canvas::render::render_slide(
                    cr,
                    &document.slides[current.get()],
                    Some(&media),
                    None,
                );
            }
        }
    ));
    let navigate: Rc<dyn Fn(isize)> = Rc::new(glib::clone!(
        #[weak]
        area,
        #[strong]
        document,
        #[strong]
        current,
        #[strong]
        previous,
        #[strong]
        progress,
        #[strong]
        direction,
        #[strong]
        animation,
        #[strong]
        presenter,
        move |delta| {
            let old = current.get();
            let last = document.borrow().slides.len().saturating_sub(1);
            let next = if delta > 0 {
                (old + 1).min(last)
            } else {
                old.saturating_sub(1)
            };
            if old == next {
                return;
            }
            previous.set(old);
            current.set(next);
            if let Some(presenter) = presenter.as_ref() {
                presenter.refresh();
            }
            direction.set(if delta > 0 { 1.0 } else { -1.0 });
            let transition = document.borrow().slides[next].transition.clone();
            if transition.kind == TransitionType::None || transition.duration_ms == 0 {
                progress.set(1.0);
                area.queue_draw();
                return;
            }
            progress.set(0.0);
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak]
                area,
                #[strong]
                progress,
                move |value| {
                    progress.set(value);
                    area.queue_draw();
                }
            ));
            let timed = adw::TimedAnimation::new(&area, 0.0, 1.0, transition.duration_ms, target);
            timed.play();
            *animation.borrow_mut() = Some(timed);
        }
    ));
    let click = gtk::GestureClick::new();
    click.connect_released(glib::clone!(
        #[strong]
        navigate,
        move |_, _, _, _| navigate(1)
    ));
    area.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        window,
        #[weak]
        area,
        #[strong]
        black,
        #[strong]
        navigate,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            use gtk::gdk::Key;
            match key {
                Key::Escape => window.close(),
                Key::b | Key::B => {
                    black.set(!black.get());
                    area.queue_draw();
                }
                Key::Right | Key::Down | Key::space | Key::Page_Down => {
                    navigate(1);
                }
                Key::Left | Key::Up | Key::Page_Up => {
                    navigate(-1);
                }
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        }
    ));
    window.add_controller(keys);
    window.connect_close_request(move |_| {
        if let Some(presenter) = presenter.as_ref() {
            presenter.window.close();
        }
        on_close(current.get());
        glib::Propagation::Proceed
    });
    window.present();
}
