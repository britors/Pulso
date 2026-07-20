use super::{Document, Element, ElementKind, Frame, Slide, Transition};

pub trait Command {
    fn apply(&mut self, document: &mut Document);
    fn revert(&mut self, document: &mut Document);
}

pub struct AddElement {
    pub slide: usize,
    pub element: Element,
}
impl Command for AddElement {
    fn apply(&mut self, d: &mut Document) {
        d.slides[self.slide].elements.push(self.element.clone());
    }
    fn revert(&mut self, d: &mut Document) {
        d.slides[self.slide]
            .elements
            .retain(|e| e.id != self.element.id);
    }
}

pub struct DeleteElements {
    pub slide: usize,
    pub elements: Vec<Element>,
}
impl Command for DeleteElements {
    fn apply(&mut self, d: &mut Document) {
        let ids: Vec<_> = self.elements.iter().map(|e| &e.id).collect();
        d.slides[self.slide]
            .elements
            .retain(|e| !ids.contains(&&e.id));
    }
    fn revert(&mut self, d: &mut Document) {
        d.slides[self.slide].elements.extend(self.elements.clone());
        d.slides[self.slide].elements.sort_by_key(|e| e.z);
    }
}

pub struct ChangeFrames {
    pub slide: usize,
    pub changes: Vec<(String, Frame, Frame)>,
}
impl ChangeFrames {
    fn set(&self, d: &mut Document, after: bool) {
        for (id, before, next) in &self.changes {
            if let Some(e) = d.slides[self.slide]
                .elements
                .iter_mut()
                .find(|e| &e.id == id)
            {
                e.frame = if after { *next } else { *before };
            }
        }
    }
}
impl Command for ChangeFrames {
    fn apply(&mut self, d: &mut Document) {
        self.set(d, true);
    }
    fn revert(&mut self, d: &mut Document) {
        self.set(d, false);
    }
}

pub struct AddSlide {
    pub index: usize,
    pub slide: Slide,
}
impl Command for AddSlide {
    fn apply(&mut self, d: &mut Document) {
        d.slides
            .insert(self.index.min(d.slides.len()), self.slide.clone());
    }
    fn revert(&mut self, d: &mut Document) {
        d.slides.retain(|s| s.id != self.slide.id);
    }
}

#[allow(dead_code)]
pub struct DeleteSlide {
    pub index: usize,
    pub slide: Slide,
}
impl Command for DeleteSlide {
    fn apply(&mut self, d: &mut Document) {
        d.slides.remove(self.index);
    }
    fn revert(&mut self, d: &mut Document) {
        d.slides.insert(self.index, self.slide.clone());
    }
}

#[allow(dead_code)]
pub struct ReorderSlide {
    pub from: usize,
    pub to: usize,
}
#[allow(dead_code)]
impl ReorderSlide {
    fn move_slide(d: &mut Document, from: usize, to: usize) {
        let slide = d.slides.remove(from);
        d.slides.insert(to, slide);
    }
}
impl Command for ReorderSlide {
    fn apply(&mut self, d: &mut Document) {
        Self::move_slide(d, self.from, self.to);
    }
    fn revert(&mut self, d: &mut Document) {
        Self::move_slide(d, self.to, self.from);
    }
}

#[allow(dead_code)]
pub struct EditNotes {
    pub slide: usize,
    pub before: String,
    pub after: String,
}

pub struct ChangeZ {
    pub slide: usize,
    pub changes: Vec<(String, i32, i32)>,
}
pub struct EditElement {
    pub slide: usize,
    pub id: String,
    pub before: ElementKind,
    pub after: ElementKind,
}
pub struct ChangeTransition {
    pub slide: usize,
    pub before: Transition,
    pub after: Transition,
}
impl Command for ChangeTransition {
    fn apply(&mut self, d: &mut Document) {
        d.slides[self.slide].transition = self.after.clone();
    }
    fn revert(&mut self, d: &mut Document) {
        d.slides[self.slide].transition = self.before.clone();
    }
}
impl Command for EditElement {
    fn apply(&mut self, d: &mut Document) {
        self.set(d, true);
    }
    fn revert(&mut self, d: &mut Document) {
        self.set(d, false);
    }
}
impl EditElement {
    fn set(&self, d: &mut Document, after: bool) {
        if let Some(element) = d.slides[self.slide]
            .elements
            .iter_mut()
            .find(|e| e.id == self.id)
        {
            element.kind = if after {
                self.after.clone()
            } else {
                self.before.clone()
            };
        }
    }
}
impl Command for ChangeZ {
    fn apply(&mut self, d: &mut Document) {
        self.set(d, true);
    }
    fn revert(&mut self, d: &mut Document) {
        self.set(d, false);
    }
}
impl ChangeZ {
    fn set(&self, d: &mut Document, after: bool) {
        for (id, before, next) in &self.changes {
            if let Some(element) = d.slides[self.slide]
                .elements
                .iter_mut()
                .find(|e| &e.id == id)
            {
                element.z = if after { *next } else { *before };
            }
        }
    }
}
impl Command for EditNotes {
    fn apply(&mut self, d: &mut Document) {
        d.slides[self.slide].notes.clone_from(&self.after);
    }
    fn revert(&mut self, d: &mut Document) {
        d.slides[self.slide].notes.clone_from(&self.before);
    }
}
