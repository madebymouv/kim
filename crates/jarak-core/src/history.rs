//! Transactions and undo/redo (`JARAK-FR-013`).
//!
//! An [`Edit`] replaces `removed` at `at` with `inserted`; a transaction is a list of edits
//! applied in order, each to the text the previous one left. Its inverse is the list reversed
//! with `removed` and `inserted` swapped. Every [`Entry`] remembers the selections before and
//! after, so undo and redo restore the carets too, and carries a state id: the document is clean
//! when the id of the newest undo entry equals the id at the last save.

use crate::selection::Selection;

/// One replacement in char positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub at: usize,
    pub removed: String,
    pub inserted: String,
}

impl Edit {
    pub fn inverse(&self) -> Edit {
        Edit {
            at: self.at,
            removed: self.inserted.clone(),
            inserted: self.removed.clone(),
        }
    }
}

/// What made an entry, for grouping typed runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Typed characters (merged while the typing continues in place).
    Typing,
    /// Backspace / Delete of single characters (merged likewise).
    Deleting,
    /// Anything else (paste, line operations, replace): one entry each.
    Other,
}

/// One undo step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub edits: Vec<Edit>,
    pub before: Selection,
    pub after: Selection,
    pub kind: Kind,
    pub id: u64,
}

/// The undo and redo stacks.
#[derive(Debug, Clone, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// The next typed or deleted run starts a new entry.
    sealed: bool,
    next_id: u64,
}

impl History {
    fn fresh_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// The id of the current state (0 for the state the document was opened in).
    pub fn state(&self) -> u64 {
        self.undo.last().map_or(0, |e| e.id)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The next edit starts a new undo step (a caret move, a save, a focus change).
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Records a transaction that was just applied. Typed (or deleted) runs merge into the
    /// previous entry while they continue where it ended and `merge` allows it; anything
    /// recorded clears the redo stack.
    pub fn record(
        &mut self,
        edits: Vec<Edit>,
        before: Selection,
        after: Selection,
        kind: Kind,
        merge: bool,
    ) {
        if edits.is_empty() {
            return;
        }
        self.redo.clear();
        let id = self.fresh_id();
        if merge
            && !self.sealed
            && kind != Kind::Other
            && let Some(last) = self.undo.last_mut()
            && last.kind == kind
        {
            last.edits.extend(edits);
            last.after = after;
            // The merged entry is a different state: a save point at it no longer matches.
            last.id = id;
            return;
        }
        self.sealed = false;
        self.undo.push(Entry {
            edits,
            before,
            after,
            kind,
            id,
        });
    }

    /// The last entry for undoing (moved to the redo stack).
    pub fn undo(&mut self) -> Option<&Entry> {
        let e = self.undo.pop()?;
        self.sealed = true;
        self.redo.push(e);
        self.redo.last()
    }

    /// The last undone entry for redoing (moved back to the undo stack).
    pub fn redo(&mut self) -> Option<&Entry> {
        let e = self.redo.pop()?;
        self.sealed = true;
        self.undo.push(e);
        self.undo.last()
    }

    /// The newest undo entry (for merge decisions).
    pub fn last(&self) -> Option<&Entry> {
        self.undo.last()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }
}
