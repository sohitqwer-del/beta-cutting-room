//! Immutable document history for editors and CLI transactions.
use crate::Document;
/// Snapshot-based history. A new edit clears redo.
#[derive(Debug, Clone)]
pub struct History {
    current: Document,
    past: Vec<Document>,
    future: Vec<Document>,
}
impl History {
    /// Begin a history with a document.
    pub fn new(doc: Document) -> Self {
        Self {
            current: doc,
            past: vec![],
            future: vec![],
        }
    }
    /// Read the current immutable snapshot.
    pub fn current(&self) -> &Document {
        &self.current
    }
    /// Commit an already prepared snapshot.
    pub fn apply(&mut self, doc: Document) {
        self.past.push(std::mem::replace(&mut self.current, doc));
        self.future.clear();
    }
    /// Restore the previous snapshot; false when there is none.
    pub fn undo(&mut self) -> bool {
        if let Some(d) = self.past.pop() {
            self.future.push(std::mem::replace(&mut self.current, d));
            true
        } else {
            false
        }
    }
    /// Restore an undone snapshot; false when there is none.
    pub fn redo(&mut self) -> bool {
        if let Some(d) = self.future.pop() {
            self.past.push(std::mem::replace(&mut self.current, d));
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transactions() {
        let mut h = History::new(Document::default());
        let mut d = h.current().clone();
        d.name = "edit".into();
        h.apply(d);
        assert!(h.undo());
        assert_eq!(h.current().name, "");
        assert!(h.redo());
        assert_eq!(h.current().name, "edit");
        assert!(h.undo());
        h.apply(Document::default());
        assert!(!h.redo());
    }
}
