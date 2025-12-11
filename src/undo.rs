//! Undo/Redo system for annotation operations.
//!
//! This module implements the Command pattern to enable undo/redo functionality
//! for annotation operations. Each undoable action is represented as a Command
//! that knows how to undo and redo itself.

use crate::annotation::{Annotation, AnnotationStore, Category, Shape};

// ============================================================================
// Command Types
// ============================================================================

/// A command that can be undone and redone.
/// Each command stores enough information to reverse its effect.
#[derive(Debug, Clone)]
pub enum Command {
    /// Add an annotation
    AddAnnotation {
        /// The image key this annotation belongs to
        image_key: String,
        /// The annotation that was added
        annotation: Annotation,
    },
    /// Remove an annotation
    RemoveAnnotation {
        /// The image key this annotation belonged to
        image_key: String,
        /// The annotation that was removed (stored for undo)
        annotation: Annotation,
    },
    /// Modify an annotation's shape
    ModifyShape {
        /// The image key
        image_key: String,
        /// The annotation ID
        annotation_id: u64,
        /// The shape before modification
        old_shape: Shape,
        /// The shape after modification
        new_shape: Shape,
    },
    /// Modify an annotation's category
    ModifyCategory {
        /// The image key
        image_key: String,
        /// The annotation ID
        annotation_id: u64,
        /// The category before modification
        old_category_id: u32,
        /// The category after modification
        new_category_id: u32,
    },
    /// Clear all annotations from an image
    ClearAnnotations {
        /// The image key
        image_key: String,
        /// All annotations that were cleared (stored for undo)
        annotations: Vec<Annotation>,
    },
    /// Add a category
    AddCategory {
        /// The category that was added
        category: Category,
    },
    /// Batch command - groups multiple commands into one undo step
    Batch {
        /// Description of the batch operation
        description: String,
        /// The commands in this batch
        commands: Vec<Command>,
    },
}

impl Command {
    /// Get a human-readable description of this command
    pub fn description(&self) -> String {
        match self {
            Command::AddAnnotation { .. } => "Add annotation".to_string(),
            Command::RemoveAnnotation { .. } => "Delete annotation".to_string(),
            Command::ModifyShape { .. } => "Move/resize annotation".to_string(),
            Command::ModifyCategory { .. } => "Change category".to_string(),
            Command::ClearAnnotations { annotations, .. } => {
                format!("Clear {} annotations", annotations.len())
            }
            Command::AddCategory { category } => format!("Add category '{}'", category.name),
            Command::Batch { description, .. } => description.clone(),
        }
    }
}

// ============================================================================
// Undo Stack
// ============================================================================

/// Configuration for the undo stack
#[derive(Debug, Clone)]
pub struct UndoConfig {
    /// Maximum number of commands to keep in history
    pub max_history: usize,
}

impl Default for UndoConfig {
    fn default() -> Self {
        Self { max_history: 100 }
    }
}

/// The undo/redo history stack.
///
/// Maintains two stacks:
/// - `undo_stack`: Commands that can be undone (most recent at the end)
/// - `redo_stack`: Commands that can be redone (most recent at the end)
///
/// When a new command is executed, it's pushed to undo_stack and redo_stack is cleared.
/// When undo is called, the command is moved from undo_stack to redo_stack.
/// When redo is called, the command is moved from redo_stack to undo_stack.
#[derive(Debug, Clone, Default)]
pub struct UndoStack {
    /// Stack of commands that can be undone
    undo_stack: Vec<Command>,
    /// Stack of commands that can be redone
    redo_stack: Vec<Command>,
    /// Configuration
    config: UndoConfig,
}

impl UndoStack {
    /// Create a new empty undo stack
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom configuration
    pub fn with_config(config: UndoConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Push a command to the undo stack.
    /// This clears the redo stack (can't redo after a new action).
    pub fn push(&mut self, command: Command) {
        log::debug!("📝 Undo: pushed '{}'", command.description());
        self.undo_stack.push(command);
        self.redo_stack.clear();

        // Limit history size
        while self.undo_stack.len() > self.config.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Pop a command from the undo stack for undoing.
    /// The command is moved to the redo stack.
    /// Returns the command to undo, or None if stack is empty.
    pub fn pop_undo(&mut self) -> Option<Command> {
        let cmd = self.undo_stack.pop()?;
        log::debug!("⏪ Undo: '{}'", cmd.description());
        self.redo_stack.push(cmd.clone());
        Some(cmd)
    }

    /// Pop a command from the redo stack for redoing.
    /// The command is moved back to the undo stack.
    /// Returns the command to redo, or None if stack is empty.
    pub fn pop_redo(&mut self) -> Option<Command> {
        let cmd = self.redo_stack.pop()?;
        log::debug!("⏩ Redo: '{}'", cmd.description());
        self.undo_stack.push(cmd.clone());
        Some(cmd)
    }

    /// Get the description of the command that would be undone
    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.last().map(|c| c.description())
    }

    /// Get the description of the command that would be redone
    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.last().map(|c| c.description())
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        log::debug!("🗑️ Undo history cleared");
    }

    /// Get the number of commands in undo history
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of commands in redo history
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

// ============================================================================
// Undo/Redo Execution
// ============================================================================

/// Apply (execute) a command - this is called when initially doing an action
/// Note: The action itself should already be done; this just records it for undo.
/// For redo, use `redo_command` instead.
pub fn record_command(stack: &mut UndoStack, command: Command) {
    stack.push(command);
}

/// Undo a command by reversing its effect on the annotation stores.
/// Returns true if the undo was successful, false if there was nothing to undo.
pub fn undo_command(
    stack: &mut UndoStack,
    annotations_map: &mut std::collections::HashMap<String, AnnotationStore>,
    categories: &mut std::collections::HashMap<u32, Category>,
) -> bool {
    let Some(cmd) = stack.pop_undo() else {
        return false;
    };

    apply_undo(&cmd, annotations_map, categories);
    true
}

/// Redo a command by re-applying its effect.
/// Returns true if the redo was successful, false if there was nothing to redo.
pub fn redo_command(
    stack: &mut UndoStack,
    annotations_map: &mut std::collections::HashMap<String, AnnotationStore>,
    categories: &mut std::collections::HashMap<u32, Category>,
) -> bool {
    let Some(cmd) = stack.pop_redo() else {
        return false;
    };

    apply_redo(&cmd, annotations_map, categories);
    true
}

/// Apply the undo operation for a command
fn apply_undo(
    cmd: &Command,
    annotations_map: &mut std::collections::HashMap<String, AnnotationStore>,
    categories: &mut std::collections::HashMap<u32, Category>,
) {
    match cmd {
        Command::AddAnnotation {
            image_key,
            annotation,
        } => {
            // Undo add = remove
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.remove(annotation.id);
                log::debug!("⏪ Undid add annotation {}", annotation.id);
            }
        }
        Command::RemoveAnnotation {
            image_key,
            annotation,
        } => {
            // Undo remove = add back
            if let Some(store) = annotations_map.get_mut(image_key) {
                // Need to restore with the same ID
                restore_annotation(store, annotation.clone());
                log::debug!("⏪ Undid remove annotation {}", annotation.id);
            }
        }
        Command::ModifyShape {
            image_key,
            annotation_id,
            old_shape,
            ..
        } => {
            // Undo modify = restore old shape
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.update_shape(*annotation_id, old_shape.clone());
                log::debug!("⏪ Undid shape modification on {}", annotation_id);
            }
        }
        Command::ModifyCategory {
            image_key,
            annotation_id,
            old_category_id,
            ..
        } => {
            // Undo category change = restore old category
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.set_category(*annotation_id, *old_category_id);
                log::debug!("⏪ Undid category change on {}", annotation_id);
            }
        }
        Command::ClearAnnotations {
            image_key,
            annotations,
        } => {
            // Undo clear = restore all annotations
            if let Some(store) = annotations_map.get_mut(image_key) {
                for ann in annotations {
                    restore_annotation(store, ann.clone());
                }
                log::debug!("⏪ Undid clear, restored {} annotations", annotations.len());
            }
        }
        Command::AddCategory { category } => {
            // Undo add category = remove it
            categories.remove(&category.id);
            log::debug!("⏪ Undid add category '{}'", category.name);
        }
        Command::Batch { commands, .. } => {
            // Undo batch in reverse order
            for cmd in commands.iter().rev() {
                apply_undo(cmd, annotations_map, categories);
            }
        }
    }
}

/// Apply the redo operation for a command
fn apply_redo(
    cmd: &Command,
    annotations_map: &mut std::collections::HashMap<String, AnnotationStore>,
    categories: &mut std::collections::HashMap<u32, Category>,
) {
    match cmd {
        Command::AddAnnotation {
            image_key,
            annotation,
        } => {
            // Redo add = add again
            if let Some(store) = annotations_map.get_mut(image_key) {
                restore_annotation(store, annotation.clone());
                log::debug!("⏩ Redid add annotation {}", annotation.id);
            }
        }
        Command::RemoveAnnotation {
            image_key,
            annotation,
        } => {
            // Redo remove = remove again
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.remove(annotation.id);
                log::debug!("⏩ Redid remove annotation {}", annotation.id);
            }
        }
        Command::ModifyShape {
            image_key,
            annotation_id,
            new_shape,
            ..
        } => {
            // Redo modify = apply new shape
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.update_shape(*annotation_id, new_shape.clone());
                log::debug!("⏩ Redid shape modification on {}", annotation_id);
            }
        }
        Command::ModifyCategory {
            image_key,
            annotation_id,
            new_category_id,
            ..
        } => {
            // Redo category change = apply new category
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.set_category(*annotation_id, *new_category_id);
                log::debug!("⏩ Redid category change on {}", annotation_id);
            }
        }
        Command::ClearAnnotations { image_key, .. } => {
            // Redo clear = clear again
            if let Some(store) = annotations_map.get_mut(image_key) {
                store.clear();
                log::debug!("⏩ Redid clear annotations");
            }
        }
        Command::AddCategory { category } => {
            // Redo add category = add again
            categories.insert(category.id, category.clone());
            log::debug!("⏩ Redid add category '{}'", category.name);
        }
        Command::Batch { commands, .. } => {
            // Redo batch in forward order
            for cmd in commands {
                apply_redo(cmd, annotations_map, categories);
            }
        }
    }
}

/// Restore an annotation to a store with its original ID.
/// This is needed because AnnotationStore.add() generates new IDs.
fn restore_annotation(store: &mut AnnotationStore, annotation: Annotation) {
    // We need to directly insert the annotation with its original ID
    // This requires access to the internal HashMap, so we'll use a workaround:
    // Add via the normal method, then update if needed
    // Actually, we need a special method on AnnotationStore for this
    store.restore(annotation);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{BoundingBox, Point};

    #[test]
    fn test_undo_stack_basic() {
        let mut stack = UndoStack::new();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        let cmd = Command::AddAnnotation {
            image_key: "test.jpg".to_string(),
            annotation: Annotation::new(1, 0, Shape::Point(Point::new(10.0, 10.0))),
        };

        stack.push(cmd);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        let undone = stack.pop_undo();
        assert!(undone.is_some());
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        let redone = stack.pop_redo();
        assert!(redone.is_some());
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_push_clears_redo() {
        let mut stack = UndoStack::new();

        stack.push(Command::AddAnnotation {
            image_key: "test.jpg".to_string(),
            annotation: Annotation::new(1, 0, Shape::Point(Point::new(10.0, 10.0))),
        });
        stack.pop_undo();
        assert!(stack.can_redo());

        // Push new command should clear redo
        stack.push(Command::AddAnnotation {
            image_key: "test.jpg".to_string(),
            annotation: Annotation::new(2, 0, Shape::Point(Point::new(20.0, 20.0))),
        });
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_max_history() {
        let mut stack = UndoStack::with_config(UndoConfig { max_history: 3 });

        for i in 0..5 {
            stack.push(Command::AddAnnotation {
                image_key: "test.jpg".to_string(),
                annotation: Annotation::new(i, 0, Shape::Point(Point::new(i as f32, i as f32))),
            });
        }

        assert_eq!(stack.undo_count(), 3);
    }

    #[test]
    fn test_command_descriptions() {
        let add = Command::AddAnnotation {
            image_key: "test.jpg".to_string(),
            annotation: Annotation::new(1, 0, Shape::Point(Point::new(10.0, 10.0))),
        };
        assert_eq!(add.description(), "Add annotation");

        let remove = Command::RemoveAnnotation {
            image_key: "test.jpg".to_string(),
            annotation: Annotation::new(1, 0, Shape::Point(Point::new(10.0, 10.0))),
        };
        assert_eq!(remove.description(), "Delete annotation");

        let modify = Command::ModifyShape {
            image_key: "test.jpg".to_string(),
            annotation_id: 1,
            old_shape: Shape::Point(Point::new(10.0, 10.0)),
            new_shape: Shape::Point(Point::new(20.0, 20.0)),
        };
        assert_eq!(modify.description(), "Move/resize annotation");
    }
}
