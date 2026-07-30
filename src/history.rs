//! Bounded undo/redo history for calculator state snapshots.
//!
//! The UI records a snapshot immediately before a user-visible calculator
//! mutation. Undo swaps the current state into the redo stack and restores the
//! previous snapshot; redo performs the inverse. Recording a new mutation after
//! undoing clears the redo branch, matching conventional desktop behavior.

const DEFAULT_LIMIT: usize = 256;

#[derive(Debug)]
pub struct History<T> {
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self::with_limit(DEFAULT_LIMIT)
    }
}

impl<T> History<T> {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Record the state that existed immediately before a successful mutation.
    /// Any previously available redo branch is invalid once a new edit occurs.
    pub fn record(&mut self, before: T) {
        self.undo.push(before);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: T) -> Option<T> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        Some(previous)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_redo_swap_states_in_order() {
        let mut history = History::with_limit(8);
        history.record(0);
        history.record(1);

        let state = history.undo(2).unwrap();
        assert_eq!(state, 1);
        let state = history.undo(state).unwrap();
        assert_eq!(state, 0);
        assert!(!history.can_undo());
        assert!(history.can_redo());

        let state = history.redo(state).unwrap();
        assert_eq!(state, 1);
        let state = history.redo(state).unwrap();
        assert_eq!(state, 2);
        assert!(!history.can_redo());
    }

    #[test]
    fn new_edit_after_undo_discards_redo_branch() {
        let mut history = History::with_limit(8);
        history.record("zero");
        let state = history.undo("one").unwrap();
        assert_eq!(state, "zero");
        assert!(history.can_redo());
        history.record(state);
        assert!(!history.can_redo());
    }

    #[test]
    fn history_is_bounded() {
        let mut history = History::with_limit(2);
        history.record(0);
        history.record(1);
        history.record(2);
        assert_eq!(history.undo(3), Some(2));
        assert_eq!(history.undo(2), Some(1));
        assert_eq!(history.undo(1), None);
    }
}
