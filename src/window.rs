use crate::{
    canvas::{hit, stage, text_overlay},
    io::{export_pdf, pulso_file, recovery},
    model::{
        commands::{
            AddElement, AddSlide, ChangeFrames, ChangeTransition, ChangeZ, DeleteElements,
            DeleteSlide, EditElement, ReorderSlide,
        },
        undo::UndoStack,
        Document, Element, ElementKind, ShapeType, Slide, TransitionType,
    },
    panels, preferences, show,
};
use adw::prelude::*;
use gtk::{gio, glib, subclass::prelude::*, CompositeTemplate};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone, Copy)]
enum DragMode {
    Move,
    Resize(f64, f64),
    Rotate,
}
#[derive(Clone, Copy)]
enum Guide {
    Vertical(f64),
    Horizontal(f64),
}

type DragSnapshot = (f64, f64, Vec<(String, crate::model::Frame)>, DragMode);

fn snap_group(
    elements: &[Element],
    selected: &[String],
    frames: &[(String, crate::model::Frame)],
    dx: f64,
    dy: f64,
) -> (f64, f64, Vec<Guide>) {
    let left = frames
        .iter()
        .map(|(_, f)| f.x)
        .fold(f64::INFINITY, f64::min)
        + dx;
    let top = frames
        .iter()
        .map(|(_, f)| f.y)
        .fold(f64::INFINITY, f64::min)
        + dy;
    let right = frames
        .iter()
        .map(|(_, f)| f.x + f.width)
        .fold(f64::NEG_INFINITY, f64::max)
        + dx;
    let bottom = frames
        .iter()
        .map(|(_, f)| f.y + f.height)
        .fold(f64::NEG_INFINITY, f64::max)
        + dy;
    let points_x = [left, (left + right) / 2.0, right];
    let points_y = [top, (top + bottom) / 2.0, bottom];
    let mut targets_x = vec![640.0];
    let mut targets_y = vec![360.0];
    for element in elements.iter().filter(|e| !selected.contains(&e.id)) {
        targets_x.extend([
            element.frame.x,
            element.frame.x + element.frame.width / 2.0,
            element.frame.x + element.frame.width,
        ]);
        targets_y.extend([
            element.frame.y,
            element.frame.y + element.frame.height / 2.0,
            element.frame.y + element.frame.height,
        ]);
    }
    let best_x = targets_x
        .iter()
        .flat_map(|target| points_x.map(|point| (*target - point, *target)))
        .filter(|(offset, _)| offset.abs() <= 6.0)
        .min_by(|a, b| a.0.abs().total_cmp(&b.0.abs()));
    let best_y = targets_y
        .iter()
        .flat_map(|target| points_y.map(|point| (*target - point, *target)))
        .filter(|(offset, _)| offset.abs() <= 6.0)
        .min_by(|a, b| a.0.abs().total_cmp(&b.0.abs()));
    let mut guides = Vec::new();
    if let Some((_, value)) = best_x {
        guides.push(Guide::Vertical(value));
    }
    if let Some((_, value)) = best_y {
        guides.push(Guide::Horizontal(value));
    }
    (
        dx + best_x.map_or(0.0, |value| value.0),
        dy + best_y.map_or(0.0, |value| value.0),
        guides,
    )
}

struct EditorState {
    document: RefCell<Document>,
    media: RefCell<HashMap<String, Vec<u8>>>,
    undo: RefCell<UndoStack>,
    active: Cell<usize>,
    selected: RefCell<Vec<String>>,
    path: RefCell<Option<PathBuf>>,
    dirty: Cell<bool>,
    allow_close: Cell<bool>,
    zoom: Cell<f64>,
    drag_origin: RefCell<Option<DragSnapshot>>,
    guides: RefCell<Vec<Guide>>,
}
impl Default for EditorState {
    fn default() -> Self {
        Self {
            document: RefCell::new(Document::default()),
            media: RefCell::new(HashMap::new()),
            undo: RefCell::new(UndoStack::default()),
            active: Cell::new(0),
            selected: RefCell::new(Vec::new()),
            path: RefCell::new(None),
            dirty: Cell::new(false),
            allow_close: Cell::new(false),
            zoom: Cell::new(1.0),
            drag_origin: RefCell::new(None),
            guides: RefCell::new(Vec::new()),
        }
    }
}

mod imp {
    use super::*;
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/br/com/w3ti/Pulso/ui/window.ui")]
    pub struct PulsoWindow {
        #[template_child]
        pub stage: TemplateChild<gtk::DrawingArea>,
        #[template_child]
        pub stage_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub slide_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub notes_view: TemplateChild<gtk::TextView>,
        #[template_child]
        pub selection_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub new_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub open_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub present_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub recede_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub advance_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub add_slide_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub object_cards: TemplateChild<gtk::FlowBox>,
        pub(super) state: std::cell::OnceCell<Rc<EditorState>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for PulsoWindow {
        const NAME: &'static str = "PulsoWindow";
        type Type = super::PulsoWindow;
        type ParentType = adw::ApplicationWindow;
        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for PulsoWindow {}
    impl WidgetImpl for PulsoWindow {}
    impl WindowImpl for PulsoWindow {}
    impl ApplicationWindowImpl for PulsoWindow {}
    impl adw::subclass::prelude::AdwApplicationWindowImpl for PulsoWindow {}
}

glib::wrapper! { pub struct PulsoWindow(ObjectSubclass<imp::PulsoWindow>) @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow, @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager; }

impl PulsoWindow {
    pub fn new(app: &adw::Application) -> Self {
        let obj: Self = glib::Object::builder().property("application", app).build();
        obj.setup();
        obj
    }
    fn state(&self) -> Rc<EditorState> {
        self.imp().state.get().expect("state initialized").clone()
    }
    fn setup(&self) {
        preferences::apply_saved_scheme();
        let state = Rc::new(EditorState::default());
        assert!(self.imp().state.set(state.clone()).is_ok());
        self.install_actions();
        self.setup_toolbar();
        self.setup_stage();
        self.setup_inputs();
        self.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                let state = window.state();
                if !state.dirty.get() || state.allow_close.get() {
                    return glib::Propagation::Proceed;
                }
                let dialog = adw::AlertDialog::builder()
                    .heading("Descartar alterações não salvas?")
                    .build();
                dialog.add_response("cancel", "Cancelar");
                dialog.add_response("discard", "Descartar");
                dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
                dialog.choose(
                    Some(&window),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[weak]
                        window,
                        move |response| {
                            if response == "discard" {
                                window.state().allow_close.set(true);
                                window.close();
                            }
                        }
                    ),
                );
                glib::Propagation::Stop
            }
        ));
        self.refresh();
        let interval = preferences::settings()
            .map(|s| s.uint("autosave-interval"))
            .unwrap_or(30);
        glib::timeout_add_seconds_local(
            interval,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let state = window.state();
                    if state.dirty.get() {
                        let _ = recovery::save(&state.document.borrow(), &state.media.borrow());
                    }
                    glib::ControlFlow::Continue
                }
            ),
        );
        if let Some(path) = recovery::latest() {
            let dialog = adw::AlertDialog::builder()
                .heading("Uma cópia de recuperação foi encontrada")
                .build();
            dialog.add_response("cancel", "Ignorar");
            dialog.add_response("recover", "Recuperar");
            dialog.set_default_response(Some("recover"));
            dialog.choose(
                Some(self),
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |response| {
                        if response == "recover" {
                            window.open_path(&path);
                            *window.state().path.borrow_mut() = None;
                            window.state().dirty.set(true);
                            window.refresh();
                        }
                    }
                ),
            );
        }
    }
    fn setup_toolbar(&self) {
        #[derive(Clone, Copy)]
        enum InsertKind {
            Text,
            Image,
            Shape(ShapeType),
        }
        for (label, icon, kind) in [
            ("Texto", "document-edit-symbolic", InsertKind::Text),
            ("Imagem", "image-x-generic-symbolic", InsertKind::Image),
            (
                "Retângulo",
                "view-grid-symbolic",
                InsertKind::Shape(ShapeType::Rect),
            ),
            (
                "Elipse",
                "media-record-symbolic",
                InsertKind::Shape(ShapeType::Ellipse),
            ),
            (
                "Linha",
                "list-remove-symbolic",
                InsertKind::Shape(ShapeType::Line),
            ),
            (
                "Seta",
                "go-next-symbolic",
                InsertKind::Shape(ShapeType::Arrow),
            ),
        ] {
            let button = gtk::Button::new();
            button.add_css_class("flat");
            button.add_css_class("card");
            button.add_css_class("object-card");
            let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
            content.set_valign(gtk::Align::Center);
            let image = gtk::Image::from_icon_name(icon);
            image.set_pixel_size(28);
            let title = gtk::Label::new(Some(label));
            content.append(&image);
            content.append(&title);
            button.set_child(Some(&content));
            button.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    if matches!(kind, InsertKind::Image) {
                        window.insert_image();
                        return;
                    }
                    let element = match kind {
                        InsertKind::Text => Element::text(140.0, 120.0),
                        InsertKind::Shape(shape) => Element::shape(shape, 180.0, 160.0),
                        InsertKind::Image => unreachable!(),
                    };
                    window.execute(Box::new(AddElement {
                        slide: window.state().active.get(),
                        element,
                    }));
                }
            ));
            self.imp().object_cards.insert(&button, -1);
        }
        self.imp().recede_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.change_z(-1)
        ));
        self.imp().advance_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.change_z(1)
        ));
    }
    fn setup_stage(&self) {
        let state = self.state();
        self.imp().stage.set_draw_func(glib::clone!(
            #[strong]
            state,
            move |_, cr, w, h| {
                let (x, y, s) = stage::transform(w, h, state.zoom.get());
                cr.set_source_rgb(0.08, 0.08, 0.08);
                let _ = cr.paint();
                let _ = cr.save();
                cr.translate(x, y);
                cr.scale(s, s);
                let doc = state.document.borrow();
                let _ = crate::canvas::render::render_slide(
                    cr,
                    &doc.slides[state.active.get()],
                    Some(&state.media.borrow()),
                    Some(&state.selected.borrow()),
                );
                cr.set_source_rgba(1.0, 0.435, 0.38, 0.9);
                cr.set_line_width(1.5 / s);
                for guide in state.guides.borrow().iter() {
                    match guide {
                        Guide::Vertical(value) => {
                            cr.move_to(*value, 0.0);
                            cr.line_to(*value, 720.0);
                        }
                        Guide::Horizontal(value) => {
                            cr.move_to(0.0, *value);
                            cr.line_to(1280.0, *value);
                        }
                    }
                    let _ = cr.stroke();
                }
                let _ = cr.restore();
            }
        ));
        let click = gtk::GestureClick::new();
        click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |gesture, n, x, y| {
                let state = window.state();
                let (sx, sy) = stage::to_stage(
                    x,
                    y,
                    window.imp().stage.width(),
                    window.imp().stage.height(),
                    state.zoom.get(),
                );
                let doc = state.document.borrow();
                let slide = &doc.slides[state.active.get()];
                let found = slide
                    .elements
                    .iter()
                    .filter(|e| hit::point_in_frame(sx, sy, &e.frame))
                    .max_by_key(|e| e.z)
                    .map(|e| e.id.clone());
                drop(doc);
                if n == 2 {
                    if let Some(id) = found {
                        window.begin_text_edit(&id);
                    }
                    return;
                }
                let shift = gesture
                    .current_event_state()
                    .contains(gtk::gdk::ModifierType::SHIFT_MASK);
                let mut selected = state.selected.borrow_mut();
                if !shift {
                    selected.clear();
                }
                if let Some(id) = found {
                    if shift && selected.contains(&id) {
                        selected.retain(|x| x != &id);
                    } else if !selected.contains(&id) {
                        selected.push(id);
                    }
                }
                drop(selected);
                window.refresh_selection();
            }
        ));
        self.imp().stage.add_controller(click);
        let drag = gtk::GestureDrag::new();
        drag.connect_drag_begin(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, x, y| {
                let s = window.state();
                let (stage_x, stage_y) = stage::to_stage(
                    x,
                    y,
                    window.imp().stage.width(),
                    window.imp().stage.height(),
                    s.zoom.get(),
                );
                let frames = s.document.borrow().slides[s.active.get()]
                    .elements
                    .iter()
                    .filter(|e| s.selected.borrow().contains(&e.id))
                    .map(|e| (e.id.clone(), e.frame))
                    .collect::<Vec<_>>();
                let mut mode = DragMode::Move;
                if let [(_, frame)] = frames.as_slice() {
                    let (local_x, local_y) = hit::local_point(stage_x, stage_y, frame);
                    if (local_x - frame.width / 2.0).abs() <= 12.0 && (local_y + 28.0).abs() <= 12.0
                    {
                        mode = DragMode::Rotate;
                    } else {
                        'handles: for hx in [0.0, 0.5, 1.0] {
                            for hy in [0.0, 0.5, 1.0] {
                                if hx == 0.5 && hy == 0.5 {
                                    continue;
                                }
                                if (local_x - frame.width * hx).abs() <= 10.0
                                    && (local_y - frame.height * hy).abs() <= 10.0
                                {
                                    mode = DragMode::Resize(hx, hy);
                                    break 'handles;
                                }
                            }
                        }
                    }
                }
                *s.drag_origin.borrow_mut() = Some((stage_x, stage_y, frames, mode));
            }
        ));
        drag.connect_drag_update(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, dx, dy| {
                let s = window.state();
                let Some((start_x, start_y, frames, mode)) = s.drag_origin.borrow().clone() else {
                    return;
                };
                let scale = stage::transform(
                    window.imp().stage.width(),
                    window.imp().stage.height(),
                    s.zoom.get(),
                )
                .2;
                let logical_dx = dx / scale;
                let logical_dy = dy / scale;
                let (move_dx, move_dy) = if matches!(mode, DragMode::Move) {
                    let document = s.document.borrow();
                    let (snap_x, snap_y, guides) = snap_group(
                        &document.slides[s.active.get()].elements,
                        &s.selected.borrow(),
                        &frames,
                        logical_dx,
                        logical_dy,
                    );
                    *s.guides.borrow_mut() = guides;
                    (snap_x, snap_y)
                } else {
                    s.guides.borrow_mut().clear();
                    (logical_dx, logical_dy)
                };
                let mut doc = s.document.borrow_mut();
                for (id, old) in frames {
                    if let Some(e) = doc.slides[s.active.get()]
                        .elements
                        .iter_mut()
                        .find(|e| e.id == id)
                    {
                        match mode {
                            DragMode::Move => {
                                e.frame.x = old.x + move_dx;
                                e.frame.y = old.y + move_dy;
                            }
                            DragMode::Resize(hx, hy) => {
                                let angle = -old.rotation.to_radians();
                                let local_dx = logical_dx * angle.cos() - logical_dy * angle.sin();
                                let local_dy = logical_dx * angle.sin() + logical_dy * angle.cos();
                                if hx == 0.0 {
                                    e.frame.x = old.x + local_dx;
                                    e.frame.width = (old.width - local_dx).max(20.0);
                                }
                                if hx == 1.0 {
                                    e.frame.width = (old.width + local_dx).max(20.0);
                                }
                                if hy == 0.0 {
                                    e.frame.y = old.y + local_dy;
                                    e.frame.height = (old.height - local_dy).max(20.0);
                                }
                                if hy == 1.0 {
                                    e.frame.height = (old.height + local_dy).max(20.0);
                                }
                            }
                            DragMode::Rotate => {
                                let center_x = old.x + old.width / 2.0;
                                let center_y = old.y + old.height / 2.0;
                                let initial = (start_y - center_y).atan2(start_x - center_x);
                                let current = (start_y + logical_dy - center_y)
                                    .atan2(start_x + logical_dx - center_x);
                                e.frame.rotation = old.rotation + (current - initial).to_degrees();
                            }
                        }
                    }
                }
                drop(doc);
                window.imp().stage.queue_draw();
            }
        ));
        drag.connect_drag_end(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _| {
                let s = window.state();
                s.guides.borrow_mut().clear();
                let drag = s.drag_origin.borrow_mut().take();
                if let Some((_, _, before, _)) = drag {
                    let doc = s.document.borrow();
                    let changes = before
                        .into_iter()
                        .filter_map(|(id, old)| {
                            doc.slides[s.active.get()]
                                .elements
                                .iter()
                                .find(|e| e.id == id)
                                .map(|e| (id, old, e.frame))
                        })
                        .collect::<Vec<_>>();
                    drop(doc);
                    if !changes.is_empty() {
                        s.undo.borrow_mut().execute(
                            Box::new(ChangeFrames {
                                slide: s.active.get(),
                                changes,
                            }),
                            &mut s.document.borrow_mut(),
                        );
                        s.dirty.set(true);
                        window.refresh();
                    }
                }
            }
        ));
        self.imp().stage.add_controller(drag);
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |controller, _, dy| {
                if controller
                    .current_event_state()
                    .contains(gtk::gdk::ModifierType::CONTROL_MASK)
                {
                    let s = window.state();
                    s.zoom
                        .set((s.zoom.get() * (if dy < 0.0 { 1.1 } else { 0.9 })).clamp(0.25, 4.0));
                    window.imp().stage.queue_draw();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        self.imp().stage.add_controller(scroll);
    }
    fn setup_inputs(&self) {
        self.imp().add_slide_button.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.add_slide()
        ));
        self.imp().new_button.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.new_document()
        ));
        self.imp().open_button.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.open_dialog()
        ));
        self.imp().save_button.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.save()
        ));
        self.imp().present_button.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.present_from(0)
        ));
        self.imp().slide_list.connect_row_selected(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, row| if let Some(row) = row {
                let s = w.state();
                s.active.set(row.index() as usize);
                s.selected.borrow_mut().clear();
                panels::notes::set_text(
                    &w.imp().notes_view,
                    &s.document.borrow().slides[s.active.get()].notes,
                );
                w.refresh_selection();
            }
        ));
        self.imp().notes_view.buffer().connect_changed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |buffer| {
                let s = w.state();
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .to_string();
                if s.document.borrow().slides[s.active.get()].notes != text {
                    s.document.borrow_mut().slides[s.active.get()].notes = text;
                    s.dirty.set(true);
                    w.update_title();
                }
            }
        ));
        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, mods| {
                use gtk::gdk::Key;
                let ctrl = mods.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                match key {
                    Key::Delete | Key::BackSpace => w.delete_selected(),
                    Key::F5 => {
                        w.present_from(if mods.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                            w.state().active.get()
                        } else {
                            0
                        })
                    }
                    Key::z if ctrl => {
                        if mods.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                            w.redo()
                        } else {
                            w.undo()
                        }
                    }
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        w.nudge(key, mods.contains(gtk::gdk::ModifierType::SHIFT_MASK))
                    }
                    _ => return glib::Propagation::Proceed,
                }
                glib::Propagation::Stop
            }
        ));
        self.add_controller(keys);
    }
    fn install_actions(&self) {
        self.add_action_entries([
            gio::ActionEntry::builder("save")
                .activate(|w: &Self, _, _| w.save())
                .build(),
            gio::ActionEntry::builder("save-as")
                .activate(|w: &Self, _, _| w.save_as())
                .build(),
            gio::ActionEntry::builder("export-pdf")
                .activate(|w: &Self, _, _| w.export_pdf())
                .build(),
        ]);
        let app = self.application().expect("application");
        app.set_accels_for_action("win.save-as", &["<primary><shift>s"]);
        app.set_accels_for_action("win.export-pdf", &["<primary><shift>e"]);
        app.set_accels_for_action("win.save", &["<primary>s"]);
    }
    fn execute(&self, command: Box<dyn crate::model::commands::Command>) {
        let s = self.state();
        s.undo
            .borrow_mut()
            .execute(command, &mut s.document.borrow_mut());
        s.dirty.set(true);
        self.refresh();
    }
    fn add_slide(&self) {
        let s = self.state();
        let index = s.active.get() + 1;
        let bg = s.document.borrow().slides[s.active.get()]
            .background
            .clone();
        let slide = Slide::new(bg);
        self.execute(Box::new(AddSlide { index, slide }));
        s.active.set(index);
        self.refresh();
    }
    fn new_document(&self) {
        if !self.state().dirty.get() {
            self.reset_document();
            return;
        }
        let dialog = adw::AlertDialog::builder()
            .heading("Criar uma nova apresentação?")
            .body("As alterações não salvas serão descartadas.")
            .build();
        dialog.add_response("cancel", "Cancelar");
        dialog.add_response("discard", "Descartar e criar");
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.choose(
            Some(self),
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |response| {
                    if response == "discard" {
                        window.reset_document();
                    }
                }
            ),
        );
    }
    fn reset_document(&self) {
        let state = self.state();
        *state.document.borrow_mut() = Document::default();
        state.media.borrow_mut().clear();
        state.undo.borrow_mut().clear();
        state.active.set(0);
        state.selected.borrow_mut().clear();
        *state.path.borrow_mut() = None;
        state.dirty.set(false);
        recovery::clear();
        self.refresh();
    }
    fn duplicate_slide_at(&self, source: usize) {
        let state = self.state();
        if source >= state.document.borrow().slides.len() {
            return;
        }
        let index = source + 1;
        let slide = state.document.borrow().slides[source].duplicate();
        self.execute(Box::new(AddSlide { index, slide }));
        state.active.set(index);
        self.refresh();
    }
    fn delete_slide_at(&self, index: usize) {
        let state = self.state();
        let len = state.document.borrow().slides.len();
        if len <= 1 || index >= len {
            return;
        }
        let active = state.active.get();
        let slide = state.document.borrow().slides[index].clone();
        self.execute(Box::new(DeleteSlide { index, slide }));
        state.active.set(if active == index {
            index.min(len - 2)
        } else if active > index {
            active - 1
        } else {
            active
        });
        state.selected.borrow_mut().clear();
        self.refresh();
    }
    fn change_z(&self, delta: i32) {
        let state = self.state();
        let slide = &state.document.borrow().slides[state.active.get()];
        let changes = slide
            .elements
            .iter()
            .filter(|element| state.selected.borrow().contains(&element.id))
            .map(|element| {
                (
                    element.id.clone(),
                    element.z,
                    element.z.saturating_add(delta),
                )
            })
            .collect::<Vec<_>>();
        if !changes.is_empty() {
            self.execute(Box::new(ChangeZ {
                slide: state.active.get(),
                changes,
            }));
        }
    }
    fn insert_image(&self) {
        let dialog = gtk::FileDialog::builder().title("Inserir imagem").build();
        dialog.open(
            Some(self),
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |result| {
                    let Ok(file) = result else {
                        return;
                    };
                    let Some(path) = file.path() else {
                        return;
                    };
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            let hash = Sha256::digest(&bytes)
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>();
                            let extension = path
                                .extension()
                                .and_then(|value| value.to_str())
                                .unwrap_or("bin")
                                .to_ascii_lowercase();
                            let name = format!("{hash}.{extension}");
                            let state = window.state();
                            state.media.borrow_mut().insert(name.clone(), bytes);
                            window.execute(Box::new(AddElement {
                                slide: state.active.get(),
                                element: Element::image(format!("media/{name}"), 160.0, 120.0),
                            }));
                        }
                        Err(error) => window.error(&error.to_string()),
                    }
                }
            ),
        );
    }
    fn delete_selected(&self) {
        let s = self.state();
        let elements = s.document.borrow().slides[s.active.get()]
            .elements
            .iter()
            .filter(|e| s.selected.borrow().contains(&e.id))
            .cloned()
            .collect();
        if !s.selected.borrow().is_empty() {
            self.execute(Box::new(DeleteElements {
                slide: s.active.get(),
                elements,
            }));
            s.selected.borrow_mut().clear();
        }
    }
    fn nudge(&self, key: gtk::gdk::Key, big: bool) {
        let s = self.state();
        let amount = if big { 10.0 } else { 1.0 };
        let mut changes = Vec::new();
        for e in &s.document.borrow().slides[s.active.get()].elements {
            if s.selected.borrow().contains(&e.id) {
                let mut next = e.frame;
                match key {
                    gtk::gdk::Key::Left => next.x -= amount,
                    gtk::gdk::Key::Right => next.x += amount,
                    gtk::gdk::Key::Up => next.y -= amount,
                    _ => next.y += amount,
                }
                changes.push((e.id.clone(), e.frame, next));
            }
        }
        if !changes.is_empty() {
            self.execute(Box::new(ChangeFrames {
                slide: s.active.get(),
                changes,
            }));
        }
    }
    fn undo(&self) {
        let s = self.state();
        if s.undo.borrow_mut().undo(&mut s.document.borrow_mut()) {
            s.dirty.set(true);
            self.refresh();
        }
    }
    fn redo(&self) {
        let s = self.state();
        if s.undo.borrow_mut().redo(&mut s.document.borrow_mut()) {
            s.dirty.set(true);
            self.refresh();
        }
    }
    fn begin_text_edit(&self, id: &str) {
        let s = self.state();
        let Some(element) = s.document.borrow().slides[s.active.get()]
            .elements
            .iter()
            .find(|e| e.id == id)
            .cloned()
        else {
            return;
        };
        let props = match &element.kind {
            ElementKind::Text { props } => props.clone(),
            _ => return,
        };
        let before = element.kind.clone();
        let view = text_overlay::create(&text_overlay::plain_text(&props));
        view.set_halign(gtk::Align::Start);
        view.set_valign(gtk::Align::Start);
        let (offset_x, offset_y, scale) = stage::transform(
            self.imp().stage.width(),
            self.imp().stage.height(),
            s.zoom.get(),
        );
        view.set_margin_start((offset_x + element.frame.x * scale).round() as i32);
        view.set_margin_top((offset_y + element.frame.y * scale).round() as i32);
        view.set_width_request((element.frame.width * scale).round().max(40.0) as i32);
        view.set_height_request((element.frame.height * scale).round().max(30.0) as i32);
        self.imp().stage_overlay.add_overlay(&view);
        view.grab_focus();
        let key = gtk::EventControllerKey::new();
        let element_id = id.to_owned();
        key.connect_key_pressed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[weak]
            view,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    let buffer = view.buffer();
                    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                    let s = w.state();
                    let mut after = before.clone();
                    text_overlay::update_from_plain(&mut after, &text);
                    w.execute(Box::new(EditElement {
                        slide: s.active.get(),
                        id: element_id.clone(),
                        before: before.clone(),
                        after,
                    }));
                    w.imp().stage_overlay.remove_overlay(&view);
                    w.refresh();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        view.add_controller(key);
    }
    fn refresh(&self) {
        let s = self.state();
        panels::slide_panel::rebuild(&self.imp().slide_list, &s.document.borrow(), s.active.get());
        self.setup_slide_dnd();
        panels::notes::set_text(
            &self.imp().notes_view,
            &s.document.borrow().slides[s.active.get()].notes,
        );
        self.refresh_selection();
        self.update_title();
    }
    fn setup_slide_dnd(&self) {
        let mut child = self.imp().slide_list.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            let Ok(row) = widget.downcast::<gtk::ListBoxRow>() else {
                continue;
            };
            let index = row.index() as u32;
            let source = gtk::DragSource::builder()
                .actions(gtk::gdk::DragAction::MOVE)
                .build();
            source.connect_prepare(move |_, _, _| {
                Some(gtk::gdk::ContentProvider::for_value(&index.to_value()))
            });
            row.add_controller(source);
            let target = gtk::DropTarget::new(u32::static_type(), gtk::gdk::DragAction::MOVE);
            target.connect_drop(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[weak]
                row,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let Ok(from) = value.get::<u32>() else {
                        return false;
                    };
                    let to = row.index().max(0) as usize;
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        window,
                        move || window.reorder_slide(from as usize, to)
                    ));
                    true
                }
            ));
            row.add_controller(target);
            let menu = gtk::Popover::new();
            menu.set_has_arrow(true);
            menu.set_parent(&row);
            let actions = gtk::Box::new(gtk::Orientation::Vertical, 2);
            actions.set_margin_start(6);
            actions.set_margin_end(6);
            actions.set_margin_top(6);
            actions.set_margin_bottom(6);
            let transition = gtk::Button::with_label("Transição…");
            transition.add_css_class("flat");
            transition.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[weak]
                menu,
                move |_| {
                    menu.popdown();
                    window.show_transition_dialog(index as usize);
                }
            ));
            actions.append(&transition);
            actions.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            let duplicate = gtk::Button::with_label("Duplicar slide");
            duplicate.add_css_class("flat");
            duplicate.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[weak]
                menu,
                move |_| {
                    menu.popdown();
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        window,
                        move || window.duplicate_slide_at(index as usize)
                    ));
                }
            ));
            actions.append(&duplicate);
            let delete = gtk::Button::with_label("Excluir slide");
            delete.add_css_class("flat");
            delete.add_css_class("destructive-action");
            delete.set_sensitive(self.state().document.borrow().slides.len() > 1);
            delete.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[weak]
                menu,
                move |_| {
                    menu.popdown();
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        window,
                        move || window.delete_slide_at(index as usize)
                    ));
                }
            ));
            actions.append(&delete);
            menu.set_child(Some(&actions));
            row.connect_unrealize(glib::clone!(
                #[weak]
                menu,
                move |_| menu.unparent()
            ));
            let context = gtk::GestureClick::new();
            context.set_button(3);
            context.connect_pressed(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[weak]
                row,
                #[weak]
                menu,
                move |_, _, x, y| {
                    window.imp().slide_list.select_row(Some(&row));
                    menu.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                    menu.popup();
                }
            ));
            row.add_controller(context);
        }
    }
    fn show_transition_dialog(&self, slide: usize) {
        let state = self.state();
        let Some(before) = state
            .document
            .borrow()
            .slides
            .get(slide)
            .map(|item| item.transition.clone())
        else {
            return;
        };
        let dialog = adw::Dialog::builder()
            .title("Transição do slide")
            .content_width(420)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.set_margin_top(24);
        content.set_margin_bottom(24);
        let type_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let type_label = gtk::Label::new(Some("Tipo"));
        type_label.set_hexpand(true);
        type_label.set_xalign(0.0);
        let transition = gtk::DropDown::from_strings(&["Nenhuma", "Dissolver", "Deslizar"]);
        transition.set_selected(match before.kind {
            TransitionType::None => 0,
            TransitionType::Fade => 1,
            TransitionType::Slide => 2,
        });
        type_row.append(&type_label);
        type_row.append(&transition);
        content.append(&type_row);
        let duration_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let duration_label = gtk::Label::new(Some("Duração (ms)"));
        duration_label.set_hexpand(true);
        duration_label.set_xalign(0.0);
        let duration = gtk::SpinButton::with_range(0.0, 5000.0, 50.0);
        duration.set_value(f64::from(before.duration_ms));
        duration_row.append(&duration_label);
        duration_row.append(&duration);
        content.append(&duration_row);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label("Cancelar");
        cancel.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        let apply = gtk::Button::with_label("Aplicar");
        apply.add_css_class("suggested-action");
        apply.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[weak]
            transition,
            #[weak]
            duration,
            move |_| {
                let mut after = before.clone();
                after.kind = match transition.selected() {
                    0 => TransitionType::None,
                    2 => TransitionType::Slide,
                    _ => TransitionType::Fade,
                };
                after.duration_ms = duration.value() as u32;
                window.execute(Box::new(ChangeTransition {
                    slide,
                    before: before.clone(),
                    after,
                }));
                dialog.close();
            }
        ));
        buttons.append(&cancel);
        buttons.append(&apply);
        content.append(&buttons);
        dialog.set_child(Some(&content));
        dialog.present(Some(self));
    }
    fn reorder_slide(&self, from: usize, to: usize) {
        if from == to {
            return;
        }
        let state = self.state();
        let len = state.document.borrow().slides.len();
        if from >= len || to >= len {
            return;
        }
        let active = state.active.get();
        self.execute(Box::new(ReorderSlide { from, to }));
        state.active.set(if active == from {
            to
        } else if from < active && to >= active {
            active - 1
        } else if from > active && to <= active {
            active + 1
        } else {
            active
        });
        self.refresh();
    }
    fn refresh_selection(&self) {
        let s = self.state();
        panels::inspector::update(&self.imp().selection_label, &s.selected.borrow());
        self.imp().stage.queue_draw();
    }
    fn update_title(&self) {
        let s = self.state();
        let path = s.path.borrow();
        let name = path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Sem título");
        self.imp().window_title.set_title(&format!(
            "{}{} — Pulso",
            if s.dirty.get() { "• " } else { "" },
            name
        ));
    }
    pub fn open_path(&self, path: &Path) {
        match pulso_file::load(path) {
            Ok((doc, media)) => {
                let s = self.state();
                *s.document.borrow_mut() = doc;
                *s.media.borrow_mut() = media;
                *s.path.borrow_mut() = Some(path.to_owned());
                s.active.set(0);
                s.dirty.set(false);
                s.undo.borrow_mut().clear();
                recovery::clear();
                self.refresh();
            }
            Err(e) => self.error(&e.to_string()),
        }
    }
    fn open_dialog(&self) {
        let dialog = gtk::FileDialog::builder()
            .title("Abrir apresentação")
            .build();
        dialog.open(
            Some(self),
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |r| if let Ok(file) = r {
                    if let Some(path) = file.path() {
                        w.open_path(&path)
                    }
                }
            ),
        );
    }
    fn save(&self) {
        if let Some(path) = self.state().path.borrow().clone() {
            self.save_to(&path)
        } else {
            self.save_as()
        }
    }
    fn save_as(&self) {
        let dialog = gtk::FileDialog::builder()
            .title("Salvar apresentação")
            .initial_name("apresentacao.pulso")
            .build();
        dialog.save(
            Some(self),
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |r| if let Ok(file) = r {
                    if let Some(path) = file.path() {
                        w.save_to(&path)
                    }
                }
            ),
        );
    }
    fn save_to(&self, path: &Path) {
        let s = self.state();
        let result = pulso_file::save(path, &s.document.borrow(), &s.media.borrow());
        match result {
            Ok(()) => {
                *s.path.borrow_mut() = Some(path.to_owned());
                s.dirty.set(false);
                recovery::clear();
                self.update_title();
            }
            Err(e) => self.error(&e.to_string()),
        }
    }
    fn export_pdf(&self) {
        let dialog = gtk::FileDialog::builder()
            .title("Exportar PDF")
            .initial_name("apresentacao.pdf")
            .build();
        dialog.save(
            Some(self),
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |r| if let Ok(file) = r {
                    if let Some(path) = file.path() {
                        if let Err(e) = export_pdf::export(
                            &w.state().document.borrow(),
                            &w.state().media.borrow(),
                            &path,
                        ) {
                            w.error(&e.to_string())
                        }
                    }
                }
            ),
        );
    }
    fn present_from(&self, start: usize) {
        let s = self.state();
        let document = Rc::new(RefCell::new(s.document.borrow().clone()));
        let media = Rc::new(RefCell::new(s.media.borrow().clone()));
        show::show_window::present(
            document,
            media,
            start,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |index| {
                    w.state().active.set(index);
                    w.refresh();
                    w.present();
                }
            ),
        );
    }
    fn error(&self, message: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading("Erro")
            .body(message)
            .build();
        dialog.add_response("ok", "OK");
        dialog.present(Some(self));
    }
}
