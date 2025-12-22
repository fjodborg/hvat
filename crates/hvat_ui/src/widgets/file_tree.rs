//! File tree widget for hierarchical file/folder display
//!
//! A VSCode-style file explorer widget that displays folders and files
//! in a tree structure with expand/collapse functionality.

use crate::callback::Callback;
use crate::constants::{char_width, FONT_SIZE_SECONDARY};
use crate::event::{Event, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::state::FileTreeState;
use crate::widget::{EventResult, Widget};

/// Configuration for the file tree widget
#[derive(Debug, Clone)]
pub struct FileTreeConfig {
    /// Indentation per nesting level (pixels)
    pub indent_size: f32,
    /// Height of each row (pixels)
    pub row_height: f32,
    /// Font size for labels
    pub font_size: f32,
    /// Text color for folders
    pub folder_color: Color,
    /// Text color for files
    pub file_color: Color,
    /// Text color for selected item
    pub selected_color: Color,
    /// Background color for hovered row
    pub hover_bg: Color,
    /// Background color for selected row
    pub selected_bg: Color,
    /// Chevron color
    pub chevron_color: Color,
}

impl Default for FileTreeConfig {
    fn default() -> Self {
        Self {
            indent_size: 16.0,
            row_height: 22.0,
            font_size: FONT_SIZE_SECONDARY,
            folder_color: Color::TEXT_PRIMARY,
            file_color: Color::TEXT_SECONDARY,
            selected_color: Color::TEXT_PRIMARY,
            hover_bg: Color::rgba(1.0, 1.0, 1.0, 0.05),
            selected_bg: Color::rgba(0.2, 0.4, 0.8, 0.3),
            chevron_color: Color::TEXT_SECONDARY,
        }
    }
}

/// A node in the file tree (folder or file)
#[derive(Debug, Clone)]
pub enum FileTreeNode {
    /// A folder that can contain other nodes
    Folder {
        /// Full path relative to project root (used as ID)
        path: String,
        /// Display name shown in the tree
        name: String,
        /// Child nodes (folders and files)
        children: Vec<FileTreeNode>,
    },
    /// A file (leaf node)
    File {
        /// Full path relative to project root (used as ID)
        path: String,
        /// Display name shown in the tree
        name: String,
        /// Optional associated data (e.g., index in image list)
        index: Option<usize>,
    },
}

impl FileTreeNode {
    /// Create a new folder node
    pub fn folder(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self::Folder {
            path: path.into(),
            name: name.into(),
            children: Vec::new(),
        }
    }

    /// Create a new file node
    pub fn file(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self::File {
            path: path.into(),
            name: name.into(),
            index: None,
        }
    }

    /// Create a new file node with an associated index
    pub fn file_with_index(path: impl Into<String>, name: impl Into<String>, index: usize) -> Self {
        Self::File {
            path: path.into(),
            name: name.into(),
            index: Some(index),
        }
    }

    /// Get the path of this node
    pub fn path(&self) -> &str {
        match self {
            Self::Folder { path, .. } => path,
            Self::File { path, .. } => path,
        }
    }

    /// Get the display name of this node
    pub fn name(&self) -> &str {
        match self {
            Self::Folder { name, .. } => name,
            Self::File { name, .. } => name,
        }
    }

    /// Check if this node is a folder
    pub fn is_folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }

    /// Add a child to a folder node (no-op if this is a file)
    pub fn add_child(&mut self, child: FileTreeNode) {
        if let Self::Folder { children, .. } = self {
            children.push(child);
        }
    }

    /// Get children of a folder (empty slice for files)
    pub fn children(&self) -> &[FileTreeNode] {
        match self {
            Self::Folder { children, .. } => children,
            Self::File { .. } => &[],
        }
    }

    /// Get mutable children of a folder
    pub fn children_mut(&mut self) -> Option<&mut Vec<FileTreeNode>> {
        match self {
            Self::Folder { children, .. } => Some(children),
            Self::File { .. } => None,
        }
    }
}

/// A file tree widget for displaying hierarchical file structures
///
/// # Example
/// ```ignore
/// let tree = FileTree::new()
///     .nodes(vec![
///         FileTreeNode::folder("src", "src"),
///         FileTreeNode::file("README.md", "README.md"),
///     ])
///     .state(&file_tree_state)
///     .selected(Some("src/main.rs".to_string()))
///     .on_select(|path| Message::FileSelected(path))
///     .on_state_change(|state| Message::FileTreeStateChanged(state));
/// ```
pub struct FileTree<M> {
    /// Root-level nodes in the tree
    nodes: Vec<FileTreeNode>,
    /// Expansion state (which folders are open)
    state: FileTreeState,
    /// Currently selected item path
    selected: Option<String>,
    /// Width constraint
    width: Length,
    /// Configuration
    config: FileTreeConfig,
    /// Callback when a file is selected (receives file path)
    on_select: Callback<String, M>,
    /// Callback when a folder is toggled (receives folder path)
    on_toggle: Callback<String, M>,
    /// Callback when expansion state changes
    on_state_change: Callback<FileTreeState, M>,
    /// Internal: currently hovered row index (for drawing)
    hovered_row: Option<usize>,
    /// Internal: cached row count for layout
    visible_row_count: usize,
}

impl<M> Default for FileTree<M> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            state: FileTreeState::default(),
            selected: None,
            width: Length::Fill(1.0),
            config: FileTreeConfig::default(),
            on_select: Callback::none(),
            on_toggle: Callback::none(),
            on_state_change: Callback::none(),
            hovered_row: None,
            visible_row_count: 0,
        }
    }
}

impl<M: 'static> FileTree<M> {
    /// Create a new file tree widget
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root nodes of the tree
    pub fn nodes(mut self, nodes: Vec<FileTreeNode>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Set the expansion state (clones the state)
    pub fn state(mut self, state: &FileTreeState) -> Self {
        self.state = state.clone();
        self
    }

    /// Set the currently selected item
    pub fn selected(mut self, path: Option<String>) -> Self {
        self.selected = path;
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the configuration
    pub fn config(mut self, config: FileTreeConfig) -> Self {
        self.config = config;
        self
    }

    /// Set callback for file selection
    pub fn on_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_select = Callback::new(callback);
        self
    }

    /// Set callback for folder toggle
    pub fn on_toggle<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_toggle = Callback::new(callback);
        self
    }

    /// Set callback for state changes
    pub fn on_state_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(FileTreeState) -> M + 'static,
    {
        self.on_state_change = Callback::new(callback);
        self
    }

    /// Count visible rows (for layout calculation)
    fn count_visible_rows(&self) -> usize {
        self.count_visible_rows_recursive(&self.nodes)
    }

    fn count_visible_rows_recursive(&self, nodes: &[FileTreeNode]) -> usize {
        let mut count = 0;
        for node in nodes {
            count += 1; // This node
            if let FileTreeNode::Folder { path, children, .. } = node {
                if self.state.is_expanded(path) {
                    count += self.count_visible_rows_recursive(children);
                }
            }
        }
        count
    }

    /// Get row info at a given y position
    fn row_at_y(&self, y: f32, bounds: Bounds) -> Option<(usize, FileTreeNode, usize)> {
        let relative_y = y - bounds.y;
        if relative_y < 0.0 {
            return None;
        }
        let row_index = (relative_y / self.config.row_height) as usize;
        self.get_row_info(row_index)
    }

    /// Get (row_index, node, depth) for a given row index
    fn get_row_info(&self, target_row: usize) -> Option<(usize, FileTreeNode, usize)> {
        let mut current_row = 0;
        self.get_row_info_recursive(&self.nodes, target_row, &mut current_row, 0)
    }

    fn get_row_info_recursive(
        &self,
        nodes: &[FileTreeNode],
        target_row: usize,
        current_row: &mut usize,
        depth: usize,
    ) -> Option<(usize, FileTreeNode, usize)> {
        for node in nodes {
            if *current_row == target_row {
                return Some((*current_row, node.clone(), depth));
            }
            *current_row += 1;

            if let FileTreeNode::Folder { path, children, .. } = node {
                if self.state.is_expanded(path) {
                    if let Some(result) =
                        self.get_row_info_recursive(children, target_row, current_row, depth + 1)
                    {
                        return Some(result);
                    }
                }
            }
        }
        None
    }

    /// Draw the tree recursively
    fn draw_nodes(
        &self,
        renderer: &mut Renderer,
        bounds: Bounds,
        nodes: &[FileTreeNode],
        depth: usize,
        row: &mut usize,
    ) {
        for node in nodes {
            let y = bounds.y + (*row as f32) * self.config.row_height;
            let x = bounds.x + (depth as f32) * self.config.indent_size;

            // Row bounds for hit testing and backgrounds
            let row_bounds = Bounds::new(bounds.x, y, bounds.width, self.config.row_height);

            // Draw hover background
            if self.hovered_row == Some(*row) {
                renderer.fill_rect(row_bounds, self.config.hover_bg);
            }

            // Draw selection background
            if self.selected.as_ref() == Some(&node.path().to_string()) {
                renderer.fill_rect(row_bounds, self.config.selected_bg);
            }

            // Draw node content
            match node {
                FileTreeNode::Folder {
                    path,
                    name,
                    children,
                } => {
                    // Draw chevron
                    let chevron = if self.state.is_expanded(path) {
                        "▼"
                    } else {
                        "▶"
                    };
                    let chevron_y = y + (self.config.row_height - self.config.font_size) / 2.0;
                    renderer.text(
                        chevron,
                        x,
                        chevron_y,
                        self.config.font_size,
                        self.config.chevron_color,
                    );

                    // Draw folder name
                    let text_x = x + char_width(self.config.font_size) * 2.0;
                    let text_color = if self.selected.as_ref() == Some(&path.to_string()) {
                        self.config.selected_color
                    } else {
                        self.config.folder_color
                    };
                    renderer.text(name, text_x, chevron_y, self.config.font_size, text_color);

                    *row += 1;

                    // Draw children if expanded
                    if self.state.is_expanded(path) {
                        self.draw_nodes(renderer, bounds, children, depth + 1, row);
                    }
                }
                FileTreeNode::File { path, name, .. } => {
                    let text_y = y + (self.config.row_height - self.config.font_size) / 2.0;
                    let text_color = if self.selected.as_ref() == Some(&path.to_string()) {
                        self.config.selected_color
                    } else {
                        self.config.file_color
                    };

                    // Indent files to align with folder names (after chevron space)
                    let text_x = x + char_width(self.config.font_size) * 2.0;
                    renderer.text(name, text_x, text_y, self.config.font_size, text_color);

                    *row += 1;
                }
            }
        }
    }
}

impl<M: Clone + 'static> Widget<M> for FileTree<M> {
    fn layout(&mut self, available: Size) -> Size {
        let width = self.width.resolve(available.width, available.width);
        self.visible_row_count = self.count_visible_rows();
        let height = self.visible_row_count as f32 * self.config.row_height;
        Size::new(width, height.max(self.config.row_height)) // At least one row height
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::trace!(
            "FileTree draw: bounds={:?}, visible_rows={}",
            bounds,
            self.visible_row_count
        );

        let mut row = 0;
        self.draw_nodes(renderer, bounds, &self.nodes, 0, &mut row);
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        match event {
            Event::MouseMove { position, .. } => {
                if bounds.contains(position.0, position.1) {
                    let relative_y = position.1 - bounds.y;
                    let row_index = (relative_y / self.config.row_height) as usize;
                    if row_index < self.visible_row_count {
                        self.hovered_row = Some(row_index);
                    } else {
                        self.hovered_row = None;
                    }
                } else {
                    self.hovered_row = None;
                }
                EventResult::None
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if !bounds.contains(position.0, position.1) {
                    return EventResult::None;
                }

                if let Some((_, node, depth)) = self.row_at_y(position.1, bounds) {
                    match &node {
                        FileTreeNode::Folder { path, .. } => {
                            // Toggle folder on any click on the folder row
                            let _depth = depth; // Unused for now, kept for potential chevron-specific toggle
                            self.state.toggle(path);
                            log::debug!("FileTree: toggled folder '{}'", path);

                            // Emit toggle callback
                            if let Some(msg) = self.on_toggle.call(path.clone()) {
                                return EventResult::Message(msg);
                            }

                            // Emit state change
                            return self.on_state_change.call(self.state.clone()).into();
                        }
                        FileTreeNode::File { path, index, .. } => {
                            log::debug!("FileTree: selected file '{}' (index: {:?})", path, index);
                            return self.on_select.call(path.clone()).into();
                        }
                    }
                }
                EventResult::None
            }

            _ => EventResult::None,
        }
    }
}
