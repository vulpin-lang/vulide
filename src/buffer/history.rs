//! Undo/redo as rope snapshots.
//!
//! `ropey::Rope` clones share structure, so a snapshot is cheap. Consecutive
//! character inserts coalesce into one undo group; a newline, a cursor jump, or
//! a save breaks the group (`set_break`).

use ropey::Rope;

use super::Position;

#[derive(Clone)]
struct Snapshot {
    rope: Rope,
    cursor: Position,
}

pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    limit: usize,
    /// When set, the next `record` starts a fresh group even if `coalesce`.
    break_pending: bool,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: 500,
            break_pending: true,
        }
    }

    pub fn set_break(&mut self) {
        self.break_pending = true;
    }

    /// Call *before* mutating, with the pre-edit state. `coalesce` merges this
    /// edit into the current undo group when possible.
    pub fn record(&mut self, rope: &Rope, cursor: Position, coalesce: bool) {
        self.redo.clear();
        if coalesce && !self.break_pending && !self.undo.is_empty() {
            return;
        }
        self.undo.push(Snapshot {
            rope: rope.clone(),
            cursor,
        });
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.break_pending = false;
    }

    /// Returns the state to restore, given the current state to stash for redo.
    pub fn undo(&mut self, current: &Rope, cursor: Position) -> Option<(Rope, Position)> {
        let snap = self.undo.pop()?;
        self.redo.push(Snapshot {
            rope: current.clone(),
            cursor,
        });
        self.break_pending = true;
        Some((snap.rope, snap.cursor))
    }

    pub fn redo(&mut self, current: &Rope, cursor: Position) -> Option<(Rope, Position)> {
        let snap = self.redo.pop()?;
        self.undo.push(Snapshot {
            rope: current.clone(),
            cursor,
        });
        self.break_pending = true;
        Some((snap.rope, snap.cursor))
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}
