use super::{commands::Command, Document};

#[derive(Default)]
pub struct UndoStack {
    undo: Vec<Box<dyn Command>>,
    redo: Vec<Box<dyn Command>>,
}
impl UndoStack {
    pub fn execute(&mut self, mut command: Box<dyn Command>, document: &mut Document) {
        command.apply(document);
        self.undo.push(command);
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    pub fn undo(&mut self, document: &mut Document) -> bool {
        if let Some(mut c) = self.undo.pop() {
            c.revert(document);
            self.redo.push(c);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self, document: &mut Document) -> bool {
        if let Some(mut c) = self.redo.pop() {
            c.apply(document);
            self.undo.push(c);
            true
        } else {
            false
        }
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
