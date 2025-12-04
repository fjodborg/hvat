//! Message handlers for HVAT application.
//!
//! Each handler processes a specific category of messages,
//! keeping the main HvatApp update function clean and organized.

use crate::annotation::{AnnotationStore, AnnotationTool, Category, DragHandle, DrawingState, Point, Shape};
use crate::hyperspectral::{BandSelection, HyperspectralImage};
use crate::image_cache::ImageCache;
use crate::message::{
    AnnotationMessage, BandMessage, CounterMessage, ExportFormat, ImageLoadMessage,
    ImageSettingsMessage, ImageViewMessage, NavigationMessage, PersistenceMode, Tab, UIMessage,
};
use crate::theme::Theme;
use crate::ui_constants::{threshold, zoom as zoom_const};
use crate::widget_state::WidgetState;
use hvat_ui::{HyperspectralImageHandle, ImageHandle};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use crate::wasm_file::open_wasm_file_picker;

/// Handle navigation messages.
pub fn handle_navigation(msg: NavigationMessage, current_tab: &mut Tab) {
    match msg {
        NavigationMessage::SwitchTab(tab) => {
            log::debug!("🔄 Switching to tab: {:?}", tab);
            *current_tab = tab;
        }
    }
}

/// Handle counter messages.
pub fn handle_counter(msg: CounterMessage, counter: &mut i32) {
    match msg {
        CounterMessage::Increment => {
            *counter += 1;
            log::debug!("➕ Counter incremented: {}", counter);
        }
        CounterMessage::Decrement => {
            *counter -= 1;
            log::debug!("➖ Counter decremented: {}", counter);
        }
        CounterMessage::Reset => {
            *counter = 0;
            log::debug!("🔄 Counter reset");
        }
    }
}

/// Handle image view messages (pan, zoom, drag).
///
/// Uses change detection for drag moves to avoid redundant pan updates
/// when the mouse hasn't actually moved.
pub fn handle_image_view(
    msg: ImageViewMessage,
    zoom: &mut f32,
    pan_x: &mut f32,
    pan_y: &mut f32,
    widget_state: &mut WidgetState,
) {
    match msg {
        ImageViewMessage::ZoomIn => {
            *zoom = (*zoom * zoom_const::FACTOR).min(zoom_const::MAX);
            log::debug!("🔍 Zoom in: {:.2}x", zoom);
        }
        ImageViewMessage::ZoomOut => {
            *zoom = (*zoom / zoom_const::FACTOR).max(zoom_const::MIN);
            log::debug!("🔍 Zoom out: {:.2}x", zoom);
        }
        ImageViewMessage::ResetView => {
            *zoom = 1.0;
            *pan_x = 0.0;
            *pan_y = 0.0;
            log::debug!("🔄 View reset");
        }
        ImageViewMessage::ResetToOneToOne => {
            // Handled specially in hvat_app.rs since it needs image dimensions
            // This arm exists for completeness but should not be reached
        }
        ImageViewMessage::ReportBounds(width, height) => {
            widget_state.image.set_bounds(width, height);
        }
        ImageViewMessage::PanLeft => {
            *pan_x -= zoom_const::PAN_STEP;
            log::debug!("⬅️  Pan left: ({:.0}, {:.0})", pan_x, pan_y);
        }
        ImageViewMessage::PanRight => {
            *pan_x += zoom_const::PAN_STEP;
            log::debug!("➡️  Pan right: ({:.0}, {:.0})", pan_x, pan_y);
        }
        ImageViewMessage::PanUp => {
            *pan_y -= zoom_const::PAN_STEP;
            log::debug!("⬆️  Pan up: ({:.0}, {:.0})", pan_x, pan_y);
        }
        ImageViewMessage::PanDown => {
            *pan_y += zoom_const::PAN_STEP;
            log::debug!("⬇️  Pan down: ({:.0}, {:.0})", pan_x, pan_y);
        }
        ImageViewMessage::Pan(pan) => {
            // Only update if pan actually changed
            if (*pan_x - pan.0).abs() > threshold::PAN_CHANGE
                || (*pan_y - pan.1).abs() > threshold::PAN_CHANGE
            {
                *pan_x = pan.0;
                *pan_y = pan.1;
            }
        }
        ImageViewMessage::ZoomAtPoint(new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y) => {
            // Only process if zoom actually changed
            if (*zoom - new_zoom).abs() > threshold::ZOOM_CHANGE {
                let old_zoom = *zoom;
                let cursor_rel_x = cursor_x - widget_center_x;
                let cursor_rel_y = cursor_y - widget_center_y;
                let img_x = (cursor_rel_x - *pan_x) / old_zoom;
                let img_y = (cursor_rel_y - *pan_y) / old_zoom;
                *zoom = new_zoom;
                *pan_x = cursor_rel_x - img_x * new_zoom;
                *pan_y = cursor_rel_y - img_y * new_zoom;
                log::debug!(
                    "🔍 Zoom-to-cursor: {:.2}x at ({:.1}, {:.1}), pan: ({:.1}, {:.1})",
                    zoom,
                    cursor_x,
                    cursor_y,
                    pan_x,
                    pan_y
                );
            }
        }
        ImageViewMessage::DragStart(pos) => {
            widget_state.image.start_drag(pos);
            log::debug!("Pan drag started at ({:.1}, {:.1})", pos.0, pos.1);
        }
        ImageViewMessage::DragMove(pos) => {
            if let Some((dx, dy)) = widget_state.image.update_drag(pos) {
                // Only update if there's meaningful movement
                if dx.abs() > threshold::DRAG_MOVEMENT || dy.abs() > threshold::DRAG_MOVEMENT {
                    *pan_x += dx;
                    *pan_y += dy;
                    log::debug!(
                        "🖐️ Panning: delta({:.1}, {:.1}) -> pan({:.1}, {:.1})",
                        dx,
                        dy,
                        pan_x,
                        pan_y
                    );
                }
            }
        }
        ImageViewMessage::DragEnd => {
            widget_state.image.end_drag();
            log::debug!("Pan drag ended");
        }
    }
}

/// Handle image settings messages (brightness, contrast, etc.).
///
/// Uses change detection to avoid redundant updates and log spam when
/// the slider value hasn't actually changed (common during continuous drag).
pub fn handle_image_settings(
    msg: ImageSettingsMessage,
    brightness: &mut f32,
    contrast: &mut f32,
    gamma: &mut f32,
    hue_shift: &mut f32,
    widget_state: &mut WidgetState,
) {
    // Helper to check if f32 values are meaningfully different
    fn changed(old: f32, new: f32) -> bool {
        (old - new).abs() > threshold::FLOAT_EPSILON
    }

    match msg {
        ImageSettingsMessage::SliderDragStart(id) => {
            // Safety: if there's already a drag in progress, end it first
            // This handles cases where MouseReleased was missed
            if widget_state.slider.active_slider.is_some() {
                log::warn!("Starting new drag while previous drag still active - forcing end");
                widget_state.slider.end_drag();
            }
            widget_state.slider.start_drag(id);
            log::debug!("Slider drag started: {:?}", id);
        }
        ImageSettingsMessage::SliderDragEnd => {
            widget_state.slider.end_drag();
            log::debug!("Slider drag ended");
        }
        ImageSettingsMessage::SetBrightness(value) => {
            if changed(*brightness, value) {
                *brightness = value;
                log::debug!("☀️  Brightness: {:.2}", brightness);
            }
        }
        ImageSettingsMessage::SetContrast(value) => {
            if changed(*contrast, value) {
                *contrast = value;
                log::debug!("🎛️  Contrast: {:.2}", contrast);
            }
        }
        ImageSettingsMessage::SetGamma(value) => {
            if changed(*gamma, value) {
                *gamma = value;
                log::debug!("📊 Gamma: {:.2}", gamma);
            }
        }
        ImageSettingsMessage::SetHueShift(value) => {
            if changed(*hue_shift, value) {
                *hue_shift = value;
                log::debug!("🎨 Hue shift: {:.2}", hue_shift);
            }
        }
        ImageSettingsMessage::Reset => {
            *brightness = 0.0;
            *contrast = 1.0;
            *gamma = 1.0;
            *hue_shift = 0.0;
            log::debug!("🔄 Image settings reset");
        }
    }
}

/// State needed for image load handler.
pub struct ImageLoadState<'a> {
    pub image_cache: &'a mut ImageCache,
    pub current_image_index: &'a mut usize,
    pub current_image: &'a mut ImageHandle,
    pub hyperspectral_image: &'a mut HyperspectralImage,
    pub hyperspectral_handle: &'a mut HyperspectralImageHandle,
    pub band_selection: &'a mut BandSelection,
    pub status_message: &'a mut Option<String>,
    pub zoom: &'a mut f32,
    pub pan_x: &'a mut f32,
    pub pan_y: &'a mut f32,
}

/// Handle image load messages.
pub fn handle_image_load(msg: ImageLoadMessage, state: &mut ImageLoadState) {
    match msg {
        ImageLoadMessage::LoadFolder => {
            log::info!("📂 Opening folder dialog...");
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    log::info!("📂 Selected folder: {:?}", folder);
                    match state.image_cache.load_from_folder(&folder) {
                        Ok(count) if count > 0 => {
                            *state.current_image_index = 0;
                            *state.status_message = Some(format!("Loaded {} images", count));
                            log::info!("📂 Found {} images", count);
                            load_current_image(state);
                        }
                        Ok(_) => {
                            *state.status_message = Some("No images found in folder".to_string());
                            log::warn!("📂 No images found in folder");
                        }
                        Err(e) => {
                            *state.status_message = Some(format!("Error reading folder: {}", e));
                            log::error!("📂 Error reading folder: {}", e);
                        }
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                *state.status_message = Some("Opening file picker...".to_string());
                log::info!("📂 Opening WASM file picker...");
                open_wasm_file_picker();
            }
        }
        ImageLoadMessage::FolderLoaded(_paths) => {
            // Deprecated - LoadFolder now handles loading directly
        }
        ImageLoadMessage::NextImage => {
            if !state.image_cache.is_empty() {
                *state.current_image_index = state.image_cache.next_index(*state.current_image_index);
                load_current_image(state);
                *state.zoom = 1.0;
                *state.pan_x = 0.0;
                *state.pan_y = 0.0;
            }
        }
        ImageLoadMessage::PreviousImage => {
            if !state.image_cache.is_empty() {
                *state.current_image_index = state.image_cache.prev_index(*state.current_image_index);
                load_current_image(state);
                *state.zoom = 1.0;
                *state.pan_x = 0.0;
                *state.pan_y = 0.0;
            }
        }
        ImageLoadMessage::ImageLoaded(handle) => {
            *state.current_image = handle;
        }
        #[cfg(target_arch = "wasm32")]
        ImageLoadMessage::WasmFilesLoaded(files) => {
            log::info!("📂 WASM: {} files loaded (lazy - not decoded yet)", files.len());
            if files.is_empty() {
                *state.status_message = Some("No files selected".to_string());
                return;
            }
            let count = state.image_cache.load_from_bytes(files);
            *state.current_image_index = 0;
            *state.status_message = Some(format!("Loaded {} images", count));
            load_current_image(state);
        }
    }
}

/// Load the current image using the image cache.
/// Note: Does NOT reset band_selection - that's handled by apply_settings_for_image()
/// in hvat_app.rs based on the persistence mode.
fn load_current_image(state: &mut ImageLoadState) {
    let index = *state.current_image_index;

    // Load the current image and convert to hyperspectral
    if let Some(handle) = state.image_cache.get_or_load(index) {
        // Convert loaded RGBA image to hyperspectral (3 bands: R, G, B)
        let hyper = HyperspectralImage::from_rgba(handle.data(), handle.width(), handle.height());

        // Create GPU handle for hyperspectral rendering
        // Band compositing happens on GPU, no CPU composite needed
        *state.hyperspectral_handle = hyper.to_gpu_handle();

        // Store the hyperspectral data
        *state.hyperspectral_image = hyper;

        // Update status message
        let name = state.image_cache.get_name(index).unwrap_or_default();
        *state.status_message = Some(format!(
            "Image {}/{}: {} (3 bands)",
            index + 1,
            state.image_cache.len(),
            name
        ));
    } else {
        *state.status_message = Some("Failed to load image".to_string());
    }

    // Preload adjacent images
    state.image_cache.preload_adjacent(index);
}

/// Handle UI messages (scroll, theme, debug).
///
/// Uses change detection for scroll offset to avoid redundant updates.
pub fn handle_ui(
    msg: UIMessage,
    widget_state: &mut WidgetState,
    show_debug_info: &mut bool,
    theme: &mut Theme,
    band_persistence: &mut PersistenceMode,
    image_settings_persistence: &mut PersistenceMode,
) {
    match msg {
        UIMessage::ScrollY(offset) => {
            // Only update if scroll position actually changed
            let current = widget_state.scroll.offset_y;
            if (current - offset).abs() > threshold::SCROLL_CHANGE {
                widget_state.scroll.set_offset_y(offset);
                log::debug!("📜 Scroll Y offset: {:.1}", offset);
            }
        }
        UIMessage::ScrollbarDragStartY(mouse_y) => {
            widget_state.scroll.start_drag_y(mouse_y);
            log::debug!("📜 Scrollbar Y drag started at y={:.1}", mouse_y);
        }
        UIMessage::ScrollbarDragEndY => {
            widget_state.scroll.end_drag_y();
            log::debug!("📜 Scrollbar Y drag ended");
        }
        UIMessage::ScrollX(offset) => {
            // Only update if scroll position actually changed
            let current = widget_state.scroll.offset_x;
            if (current - offset).abs() > threshold::SCROLL_CHANGE {
                widget_state.scroll.set_offset_x(offset);
                log::debug!("📜 Scroll X offset: {:.1}", offset);
            }
        }
        UIMessage::ScrollbarDragStartX(mouse_x) => {
            widget_state.scroll.start_drag_x(mouse_x);
            log::debug!("📜 Scrollbar X drag started at x={:.1}", mouse_x);
        }
        UIMessage::ScrollbarDragEndX => {
            widget_state.scroll.end_drag_x();
            log::debug!("📜 Scrollbar X drag ended");
        }
        UIMessage::SidebarScrollY(offset) => {
            let current = widget_state.sidebar_scroll.offset_y;
            if (current - offset).abs() > threshold::SCROLL_CHANGE {
                widget_state.sidebar_scroll.set_offset_y(offset);
                log::debug!("📜 Sidebar scroll Y offset: {:.1}", offset);
            }
        }
        UIMessage::SidebarScrollbarDragStartY(mouse_y) => {
            widget_state.sidebar_scroll.start_drag_y(mouse_y);
            log::debug!("📜 Sidebar scrollbar Y drag started at y={:.1}", mouse_y);
        }
        UIMessage::SidebarScrollbarDragEndY => {
            widget_state.sidebar_scroll.end_drag_y();
            log::debug!("📜 Sidebar scrollbar Y drag ended");
        }
        UIMessage::ToggleDebugInfo => {
            *show_debug_info = !*show_debug_info;
            log::debug!(
                "🐛 Debug info: {}",
                if *show_debug_info { "ON" } else { "OFF" }
            );
        }
        UIMessage::SetTheme(new_theme) => {
            *theme = new_theme.clone();
            log::debug!("🎨 Theme changed to: {:?}", theme.choice);
        }
        UIMessage::SetBandPersistence(mode) => {
            *band_persistence = mode;
            // Close dropdown after selection
            widget_state.dropdown.close_band_persistence();
            log::debug!("🎚️ Band persistence mode: {:?}", mode);
        }
        UIMessage::SetImageSettingsPersistence(mode) => {
            *image_settings_persistence = mode;
            // Close dropdown after selection
            widget_state.dropdown.close_image_settings_persistence();
            log::debug!("🎚️ Image settings persistence mode: {:?}", mode);
        }
        UIMessage::OpenBandPersistenceDropdown => {
            widget_state.dropdown.open_band_persistence();
            log::debug!("📋 Band persistence dropdown opened");
        }
        UIMessage::CloseBandPersistenceDropdown => {
            widget_state.dropdown.close_band_persistence();
            log::debug!("📋 Band persistence dropdown closed");
        }
        UIMessage::OpenImageSettingsPersistenceDropdown => {
            widget_state.dropdown.open_image_settings_persistence();
            log::debug!("📋 Image settings persistence dropdown opened");
        }
        UIMessage::CloseImageSettingsPersistenceDropdown => {
            widget_state.dropdown.close_image_settings_persistence();
            log::debug!("📋 Image settings persistence dropdown closed");
        }
        UIMessage::ToggleImageSettingsCollapsed => {
            widget_state.collapsible.toggle_image_settings();
            log::debug!("📦 Image settings collapsed: {}", widget_state.collapsible.image_settings_collapsed);
        }
        UIMessage::ToggleBandSettingsCollapsed => {
            widget_state.collapsible.toggle_band_settings();
            log::debug!("📦 Band settings collapsed: {}", widget_state.collapsible.band_settings_collapsed);
        }
        UIMessage::SetNewCategoryText(text) => {
            widget_state.category_input.set_text(text);
        }
        UIMessage::SetCategoryInputFocused(focused) => {
            widget_state.category_input.set_focused(focused);
        }
        UIMessage::SubmitNewCategory => {
            // This is handled in hvat_app.rs since we need access to annotations
            // The handler there will add the category and clear the input
        }
        UIMessage::SetNewTagText(text) => {
            widget_state.tag_input.set_text(text);
        }
        UIMessage::SetTagInputFocused(focused) => {
            widget_state.tag_input.set_focused(focused);
        }
        UIMessage::SubmitNewTag => {
            // This is handled in hvat_app.rs since we need access to available_tags
            // The handler there will add the tag and clear the input
        }
    }
}

/// State needed for annotation handler.
pub struct AnnotationState<'a> {
    pub annotations_map: &'a mut HashMap<String, AnnotationStore>,
    pub categories: &'a mut HashMap<u32, Category>,
    pub drawing_state: &'a mut DrawingState,
    pub image_key: String,
    pub zoom: f32,
    pub status_message: &'a mut Option<String>,
    pub export_dialog_open: &'a mut bool,
    pub export_format: &'a mut ExportFormat,
    pub widget_state: &'a mut WidgetState,
    pub image_cache: &'a ImageCache,
}

impl<'a> AnnotationState<'a> {
    /// Get annotations for the current image.
    pub fn annotations(&self) -> &AnnotationStore {
        static EMPTY: std::sync::OnceLock<AnnotationStore> = std::sync::OnceLock::new();
        self.annotations_map
            .get(&self.image_key)
            .unwrap_or_else(|| EMPTY.get_or_init(AnnotationStore::new))
    }

    /// Get mutable annotations for the current image.
    /// Uses get_mut first to avoid cloning the key on every call.
    pub fn annotations_mut(&mut self) -> &mut AnnotationStore {
        // Fast path: check if entry exists without cloning key
        if self.annotations_map.contains_key(&self.image_key) {
            // Safe: we just checked it exists
            self.annotations_map.get_mut(&self.image_key).unwrap()
        } else {
            // Slow path: only clone key when inserting new entry
            self.annotations_map
                .entry(self.image_key.clone())
                .or_insert_with(AnnotationStore::new)
        }
    }

    /// Get all global categories.
    pub fn global_categories(&self) -> impl Iterator<Item = &Category> {
        self.categories.values()
    }

    /// Add a category to global categories.
    pub fn add_global_category(&mut self, category: Category) {
        self.categories.insert(category.id, category);
    }

    /// Get a global category by ID.
    pub fn get_global_category(&self, id: u32) -> Option<&Category> {
        self.categories.get(&id)
    }
}

/// Handle annotation messages.
pub fn handle_annotation(msg: AnnotationMessage, state: &mut AnnotationState) {
    match msg {
        AnnotationMessage::SetTool(tool) => {
            state.drawing_state.tool = tool;
            // Cancel any in-progress drawing when switching tools
            state.drawing_state.cancel();
            log::debug!("🖌️ Annotation tool: {:?}", tool);
        }
        AnnotationMessage::SetCategory(id) => {
            state.drawing_state.current_category = id;
            log::debug!("🏷️ Category: {}", id);
        }
        AnnotationMessage::SelectCategoryByHotkey(num) => {
            // Map hotkey number (1-9) to category ID based on sorted order (use global categories)
            let mut cat_ids: Vec<u32> = state.global_categories().map(|c| c.id).collect();
            cat_ids.sort();

            // Hotkey 1 = index 0, hotkey 2 = index 1, etc.
            let index = (num as usize).saturating_sub(1);
            if let Some(&cat_id) = cat_ids.get(index) {
                // If an annotation is selected, change its category
                if let Some(selected_id) = state.annotations().selected() {
                    state.annotations_mut().set_category(selected_id, cat_id);
                    log::info!("🏷️ Changed annotation {} to category {} (hotkey {})", selected_id, cat_id, num);
                }
                // Always update the drawing category
                state.drawing_state.current_category = cat_id;
                log::debug!("🏷️ Category by hotkey {}: id={}", num, cat_id);
            } else {
                log::debug!("🏷️ Hotkey {} has no category (only {} exist)", num, cat_ids.len());
            }
        }
        AnnotationMessage::AddCategory(name) => {
            // Add to global categories
            let id = state.categories.keys().max().copied().unwrap_or(0) + 1;
            state.add_global_category(Category::new(id, name.clone()));
            log::debug!("🏷️ Added category: {} (id={})", name, id);
        }
        AnnotationMessage::StartDrawing(x, y) => {
            // Clicking on image unfocuses any text input
            state.widget_state.category_input.set_focused(false);
            state.widget_state.tag_input.set_focused(false);

            match state.drawing_state.tool {
                AnnotationTool::Select => {
                    let point = Point::new(x, y);
                    let hit_radius = threshold::POINT_HIT_RADIUS / state.zoom;
                    let currently_selected = state.annotations().selected();

                    // Check if clicking on the body of the currently selected annotation
                    // If so, cycle to next overlapping annotation immediately
                    if let Some(sel_id) = currently_selected {
                        if let Some(ann) = state.annotations().get(sel_id) {
                            if let Some(handle) = ann.shape.hit_test_handle(&point, hit_radius) {
                                if matches!(handle, DragHandle::Body) {
                                    // Clicking on selected annotation's body - cycle to next
                                    if let Some(next_id) = state.annotations().cycle_selection(&point, Some(sel_id)) {
                                        if next_id != sel_id {
                                            state.annotations_mut().select(Some(next_id));
                                            log::debug!("🔄 Cycled to annotation {}", next_id);
                                            // Start drag on the NEW annotation
                                            if let Some(next_ann) = state.annotations().get(next_id) {
                                                let original = next_ann.shape.clone();
                                                state.drawing_state.editing.start_drag(next_id, DragHandle::Body, point, original);
                                            }
                                            return;
                                        }
                                    }
                                }
                                // Clicking on a handle (corner/vertex) of selected - start drag
                                let original = ann.shape.clone();
                                state.drawing_state.editing.start_drag(sel_id, handle, point, original);
                                log::debug!("🔧 Started dragging {:?} of annotation {}", handle, sel_id);
                                return;
                            }
                        }
                    }

                    // Not clicking on selected annotation - find what we hit
                    if let Some((hit_id, handle)) = state.annotations().hit_test_any_handle(&point, hit_radius) {
                        state.annotations_mut().select(Some(hit_id));
                        if let Some(ann) = state.annotations().get(hit_id) {
                            let original = ann.shape.clone();
                            state.drawing_state.editing.start_drag(hit_id, handle, point, original);
                            log::debug!("🔧 Selected annotation {} and started dragging {:?}", hit_id, handle);
                        }
                    } else {
                        // Clicked on empty space - deselect
                        state.annotations_mut().select(None);
                        log::debug!("🔍 Deselected (clicked empty space)");
                    }
                }
                AnnotationTool::Point => {
                    // Point tool: create immediately on click
                    let category = state.drawing_state.current_category;
                    let id = state
                        .annotations_mut()
                        .add(category, Shape::Point(Point::new(x, y)));
                    log::info!(
                        "✅ Created point annotation {} at ({:.1}, {:.1})",
                        id,
                        x,
                        y
                    );
                }
                AnnotationTool::BoundingBox => {
                    // Start drawing for bbox
                    state.drawing_state.start(Point::new(x, y));
                    log::debug!("✏️ Started bbox at ({:.1}, {:.1})", x, y);
                }
                AnnotationTool::Polygon => {
                    if state.drawing_state.is_drawing {
                        // Check if clicking near first point to close polygon
                        if state.drawing_state.points.len() >= 3 {
                            if let Some(first) = state.drawing_state.points.first() {
                                let click_point = Point::new(x, y);
                                // Close threshold in image coordinates (scaled by zoom)
                                if first.distance_to(&click_point) < threshold::POLYGON_CLOSE / state.zoom {
                                    // Close the polygon
                                    let category = state.drawing_state.current_category;
                                    if let Some(shape) = state.drawing_state.finish() {
                                        let id = state.annotations_mut().add(category, shape);
                                        log::info!(
                                            "✅ Closed polygon annotation {} (category={})",
                                            id,
                                            category
                                        );
                                    }
                                    return;
                                }
                            }
                        }
                        // Not closing - add another point
                        state.drawing_state.add_point(Point::new(x, y));
                        log::debug!(
                            "✏️ Added polygon point at ({:.1}, {:.1}), total: {}",
                            x,
                            y,
                            state.drawing_state.points.len()
                        );
                    } else {
                        // Start new polygon
                        state.drawing_state.start(Point::new(x, y));
                        log::debug!("✏️ Started polygon at ({:.1}, {:.1})", x, y);
                    }
                }
            }
        }
        AnnotationMessage::ContinueDrawing(x, y) => {
            // Handle editing/dragging in Select mode
            if state.drawing_state.editing.is_dragging {
                let point = Point::new(x, y);
                if let (Some(ann_id), Some(handle), Some(start)) = (
                    state.drawing_state.editing.annotation_id,
                    state.drawing_state.editing.handle,
                    state.drawing_state.editing.drag_start,
                ) {
                    // Calculate delta from drag start
                    let delta = Point::new(point.x - start.x, point.y - start.y);

                    // Only apply if there's significant movement (avoid jitter)
                    let move_threshold = 1.0 / state.zoom; // 1 pixel in screen space
                    if delta.x.abs() > move_threshold || delta.y.abs() > move_threshold {
                        // Mark that we've actually moved (not just a click)
                        state.drawing_state.editing.mark_moved();

                        // Get original shape and apply delta
                        if let Some(original) = &state.drawing_state.editing.original_shape {
                            let new_shape = original.apply_drag(handle, delta);
                            state.annotations_mut().update_shape(ann_id, new_shape);
                        }
                    }
                }
                return;
            }

            // Handle normal drawing
            if state.drawing_state.is_drawing {
                match state.drawing_state.tool {
                    AnnotationTool::BoundingBox => {
                        // For bbox, we need exactly 2 points - add second if missing, else update
                        if state.drawing_state.points.len() == 1 {
                            state.drawing_state.add_point(Point::new(x, y));
                        } else {
                            state.drawing_state.update_last(Point::new(x, y));
                        }
                    }
                    AnnotationTool::Polygon => {
                        // For polygon, we don't add points on move - only on click
                        // (ContinueDrawing for polygon should just update preview)
                    }
                    _ => {}
                }
            }
        }
        AnnotationMessage::FinishDrawing => {
            // Handle editing/dragging finish in Select mode
            if state.drawing_state.editing.is_dragging {
                log::debug!("🔧 Finished editing annotation {:?}", state.drawing_state.editing.annotation_id);
                state.drawing_state.editing.finish_drag();
                return;
            }

            // This is called on mouse release - only finish bbox, NOT polygon
            // Polygon is finished via ForceFinishPolygon (Space key) or clicking first point
            match state.drawing_state.tool {
                AnnotationTool::BoundingBox => {
                    let category = state.drawing_state.current_category;
                    if let Some(shape) = state.drawing_state.finish() {
                        let id = state.annotations_mut().add(category, shape);
                        log::info!("✅ Created bbox annotation {} (category={})", id, category);
                    }
                }
                AnnotationTool::Polygon => {
                    // Do nothing on mouse release for polygon - keep drawing
                    log::debug!("📝 Polygon continues (use Space or click first point to close)");
                }
                _ => {
                    // Point tool handles creation in StartDrawing, Select doesn't create
                }
            }
        }
        AnnotationMessage::ForceFinishPolygon => {
            // Called via Space key - force close polygon if valid
            if state.drawing_state.tool == AnnotationTool::Polygon
                && state.drawing_state.is_drawing
            {
                if state.drawing_state.points.len() >= 3 {
                    let category = state.drawing_state.current_category;
                    if let Some(shape) = state.drawing_state.finish() {
                        let id = state.annotations_mut().add(category, shape);
                        log::info!("✅ Created polygon annotation {} (category={})", id, category);
                    }
                } else {
                    log::debug!(
                        "📝 Polygon needs at least 3 points, currently has {}",
                        state.drawing_state.points.len()
                    );
                }
            }
        }
        AnnotationMessage::CancelDrawing => {
            state.drawing_state.cancel();
            log::debug!("❌ Drawing cancelled");
        }
        AnnotationMessage::SelectAnnotation(id) => {
            state.annotations_mut().select(id);
            log::debug!("🔍 Selected annotation: {:?}", id);
        }
        AnnotationMessage::DeleteSelected => {
            if let Some(id) = state.annotations().selected() {
                state.annotations_mut().remove(id);
                log::info!("🗑️ Deleted annotation {}", id);
            }
        }
        AnnotationMessage::ToolShortcut(key) => {
            match key {
                'b' => {
                    state.drawing_state.tool = AnnotationTool::BoundingBox;
                    state.drawing_state.cancel();
                    log::debug!("🖌️ Tool: BoundingBox (via 'b' key)");
                }
                'm' => {
                    state.drawing_state.tool = AnnotationTool::Polygon;
                    state.drawing_state.cancel();
                    log::debug!("🖌️ Tool: Mask/Polygon (via 'm' key)");
                }
                'p' => {
                    state.drawing_state.tool = AnnotationTool::Point;
                    state.drawing_state.cancel();
                    log::debug!("🖌️ Tool: Point (via 'p' key)");
                }
                's' => {
                    state.drawing_state.tool = AnnotationTool::Select;
                    state.drawing_state.cancel();
                    log::debug!("🖌️ Tool: Select (via 's' key)");
                }
                '\x1b' => { // ESC
                    // Cancel drawing or deselect
                    if state.drawing_state.is_drawing {
                        state.drawing_state.cancel();
                        log::debug!("❌ Drawing cancelled (ESC)");
                    } else if state.drawing_state.editing.is_dragging {
                        // Cancel drag and restore original
                        if let Some((id, shape)) = state.drawing_state.editing.cancel_drag() {
                            state.annotations_mut().update_shape(id, shape);
                            log::debug!("❌ Drag cancelled, restored original shape");
                        }
                    } else {
                        state.annotations_mut().select(None);
                        log::debug!("🔍 Selection cleared (ESC)");
                    }
                }
                '\x7f' => { // DEL
                    if let Some(id) = state.annotations().selected() {
                        state.annotations_mut().remove(id);
                        log::info!("🗑️ Deleted annotation {} (DEL key)", id);
                    }
                }
                _ => {}
            }
        }
        AnnotationMessage::StartDrag(x, y) => {
            // Only start drag in Select mode with a selected annotation
            if state.drawing_state.tool != AnnotationTool::Select {
                return;
            }
            let point = Point::new(x, y);

            // Check if clicking on a handle of the selected annotation
            if let Some(selected_id) = state.annotations().selected() {
                let hit_radius = threshold::POINT_HIT_RADIUS / state.zoom;
                if let Some(handle) = state.annotations().hit_test_handle(&point, hit_radius) {
                    if let Some(ann) = state.annotations().get(selected_id) {
                        let original = ann.shape.clone();
                        state.drawing_state.editing.start_drag(selected_id, handle, point, original);
                        log::debug!("🔧 Started dragging handle {:?} of annotation {}", handle, selected_id);
                    }
                    return;
                }
            }

            // Otherwise, hit test for selecting a new annotation
            if let Some(hit_id) = state.annotations().hit_test(&point) {
                state.annotations_mut().select(Some(hit_id));
                if let Some(ann) = state.annotations().get(hit_id) {
                    let original = ann.shape.clone();
                    state.drawing_state.editing.start_drag(hit_id, DragHandle::Body, point, original);
                    log::debug!("🔧 Selected and started body drag of annotation {}", hit_id);
                }
            } else {
                state.annotations_mut().select(None);
                log::debug!("🔍 Deselected (clicked empty space)");
            }
        }
        AnnotationMessage::ContinueDrag(x, y) => {
            if !state.drawing_state.editing.is_dragging {
                return;
            }

            let point = Point::new(x, y);
            if let (Some(ann_id), Some(handle), Some(start)) = (
                state.drawing_state.editing.annotation_id,
                state.drawing_state.editing.handle,
                state.drawing_state.editing.drag_start,
            ) {
                // Calculate delta from drag start
                let delta = Point::new(point.x - start.x, point.y - start.y);

                // Get original shape and apply delta
                if let Some(original) = &state.drawing_state.editing.original_shape {
                    let new_shape = original.apply_drag(handle, delta);
                    state.annotations_mut().update_shape(ann_id, new_shape);
                }
            }
        }
        AnnotationMessage::FinishDrag => {
            if state.drawing_state.editing.is_dragging {
                log::debug!("🔧 Finished dragging annotation {:?}", state.drawing_state.editing.annotation_id);
                state.drawing_state.editing.finish_drag();
            }
        }
        AnnotationMessage::OpenExportDialog => {
            *state.export_dialog_open = true;
            log::debug!("📤 Export dialog opened");
        }
        AnnotationMessage::CloseExportDialog => {
            *state.export_dialog_open = false;
            state.widget_state.dropdown.close_export_format();
            log::debug!("📤 Export dialog closed");
        }
        AnnotationMessage::SetExportFormat(format) => {
            *state.export_format = format;
            log::debug!("📤 Export format set to: {}", format.name());
        }
        AnnotationMessage::ToggleExportFormatDropdown => {
            state.widget_state.dropdown.toggle_export_format();
            log::debug!("📤 Export format dropdown toggled");
        }
        AnnotationMessage::PerformExport => {
            log::info!("📤 PerformExport message received, calling perform_export");
            perform_export(state);
            *state.export_dialog_open = false;
        }
        AnnotationMessage::ExportJson => {
            // Legacy: now opens export dialog instead
            *state.export_dialog_open = true;
            log::debug!("📤 Export dialog opened (via legacy ExportJson)");
        }
        AnnotationMessage::ImportJson => {
            // TODO: Implement file picker for importing
            log::info!("📥 Import not yet implemented");
            *state.status_message = Some("Import not yet implemented".to_string());
        }
        AnnotationMessage::ClearAll => {
            let count = state.annotations().len();
            state.annotations_mut().clear();
            log::info!("🗑️ Cleared {} annotations", count);
            *state.status_message = Some(format!("Cleared {} annotations", count));
        }
    }
}

/// Perform the actual export with the selected format.
fn perform_export(state: &mut AnnotationState) {
    use crate::formats::{format_by_name, ImageInfo};

    log::info!("📤 perform_export called with format: {:?}", state.export_format);

    let format_name = match state.export_format {
        ExportFormat::Coco => "COCO",
        ExportFormat::Yolo => "YOLO",
        ExportFormat::YoloSegmentation => "YOLO Segmentation",
        ExportFormat::Datumaro => "Datumaro",
        ExportFormat::PascalVoc => "Pascal VOC",
    };

    let Some(format) = format_by_name(format_name) else {
        log::error!("Unknown export format: {}", format_name);
        *state.status_message = Some(format!("Unknown format: {}", format_name));
        return;
    };

    // Collect all images and their annotations
    let mut stores: Vec<(ImageInfo, &AnnotationStore)> = Vec::new();

    for i in 0..state.image_cache.len() {
        if let Some(name) = state.image_cache.get_name(i) {
            // Get image dimensions from cache
            let (width, height) = state
                .image_cache
                .get_dimensions(i)
                .unwrap_or((1920, 1080)); // Default if not available

            let image_info = ImageInfo::new(&name, width, height);

            // Get annotations for this image (empty store if none)
            static EMPTY: std::sync::OnceLock<AnnotationStore> = std::sync::OnceLock::new();
            let annotations = state
                .annotations_map
                .get(&name)
                .unwrap_or_else(|| EMPTY.get_or_init(AnnotationStore::new));

            stores.push((image_info, annotations));
        }
    }

    log::info!("📤 Collected {} image stores for export", stores.len());

    if stores.is_empty() {
        log::warn!("📤 No images to export - image_cache.len() = {}", state.image_cache.len());
        *state.status_message = Some("No images to export".to_string());
        return;
    }

    // Perform export
    match format.export_dataset(&stores) {
        Ok(result) => {
            if result.is_empty() {
                *state.status_message = Some("Export produced no files".to_string());
                return;
            }

            // Log warnings if any
            for warning in &result.warnings {
                log::warn!("Export warning: {}", warning);
            }

            // Save files
            #[cfg(not(target_arch = "wasm32"))]
            {
                save_export_files_native(&result, format_name, state.status_message);
            }
            #[cfg(target_arch = "wasm32")]
            {
                save_export_files_wasm(&result, format_name, state.status_message);
            }
        }
        Err(e) => {
            log::error!("Export failed: {}", e);
            *state.status_message = Some(format!("Export failed: {}", e));
        }
    }
}

/// Save export files on native platform using file dialog.
#[cfg(not(target_arch = "wasm32"))]
fn save_export_files_native(
    result: &crate::formats::ExportResult,
    format_name: &str,
    status_message: &mut Option<String>,
) {
    use std::fs;

    // If single file, use save dialog
    if result.files.len() == 1 {
        let (filename, content) = result.files.iter().next().unwrap();
        let extension = filename.rsplit('.').next().unwrap_or("json");

        if let Some(path) = rfd::FileDialog::new()
            .add_filter(format_name, &[extension])
            .set_file_name(filename)
            .save_file()
        {
            match fs::write(&path, content) {
                Ok(()) => {
                    log::info!("📤 Exported to: {:?}", path);
                    *status_message = Some(format!("Exported to {}", path.display()));
                }
                Err(e) => {
                    log::error!("Failed to write file: {}", e);
                    *status_message = Some(format!("Failed to save: {}", e));
                }
            }
        }
    } else {
        // Multiple files - ask for directory
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            let mut saved = 0;
            for (filename, content) in &result.files {
                let path = dir.join(filename);
                match fs::write(&path, content) {
                    Ok(()) => {
                        saved += 1;
                        log::debug!("📤 Saved: {:?}", path);
                    }
                    Err(e) => {
                        log::error!("Failed to write {}: {}", filename, e);
                    }
                }
            }
            log::info!("📤 Exported {} files to: {:?}", saved, dir);
            *status_message = Some(format!("Exported {} files to {}", saved, dir.display()));
        }
    }
}

/// Save export files on WASM platform using download.
#[cfg(target_arch = "wasm32")]
fn save_export_files_wasm(
    result: &crate::formats::ExportResult,
    format_name: &str,
    status_message: &mut Option<String>,
) {
    // For WASM, download files
    if result.files.len() == 1 {
        let (filename, content) = result.files.iter().next().unwrap();
        download_json(content, filename);
        *status_message = Some(format!("Downloaded {}", filename));
    } else {
        // Multiple files - create a zip or download each
        // For simplicity, download the main file (usually the annotation file)
        // In a full implementation, we'd create a zip
        for (filename, content) in &result.files {
            if filename.ends_with(".json") || filename.ends_with(".txt") && filename == "classes.txt"
            {
                download_json(content, filename);
            }
        }
        *status_message = Some(format!("Downloaded {} files", result.files.len()));
    }
}

/// Handle band selection messages for hyperspectral images.
/// Returns true if the composite should be regenerated.
pub fn handle_band(
    msg: BandMessage,
    band_selection: &mut BandSelection,
    num_bands: usize,
    widget_state: &mut WidgetState,
) -> bool {
    use hvat_ui::widgets::SliderId;
    let max_band = num_bands.saturating_sub(1);
    match msg {
        BandMessage::SetRedBand(band) => {
            let new_value = band.min(max_band);
            if band_selection.red != new_value {
                band_selection.red = new_value;
                log::debug!("🔴 Red band changed to: {}", band_selection.red);
                true // Only regenerate when value actually changed
            } else {
                false // No change, skip regeneration
            }
        }
        BandMessage::SetGreenBand(band) => {
            let new_value = band.min(max_band);
            if band_selection.green != new_value {
                band_selection.green = new_value;
                log::debug!("🟢 Green band changed to: {}", band_selection.green);
                true
            } else {
                false
            }
        }
        BandMessage::SetBlueBand(band) => {
            let new_value = band.min(max_band);
            if band_selection.blue != new_value {
                band_selection.blue = new_value;
                log::debug!("🔵 Blue band changed to: {}", band_selection.blue);
                true
            } else {
                false
            }
        }
        BandMessage::StartRedBand(band) => {
            // Start drag AND set initial value (called on mouse press)
            widget_state.slider.start_drag(SliderId::BandRed);
            let new_value = band.min(max_band);
            if band_selection.red != new_value {
                band_selection.red = new_value;
                log::debug!("🔴 Red band drag started at: {}", band_selection.red);
                true
            } else {
                log::debug!("🔴 Red band drag started (no change)");
                false
            }
        }
        BandMessage::StartGreenBand(band) => {
            widget_state.slider.start_drag(SliderId::BandGreen);
            let new_value = band.min(max_band);
            if band_selection.green != new_value {
                band_selection.green = new_value;
                log::debug!("🟢 Green band drag started at: {}", band_selection.green);
                true
            } else {
                log::debug!("🟢 Green band drag started (no change)");
                false
            }
        }
        BandMessage::StartBlueBand(band) => {
            widget_state.slider.start_drag(SliderId::BandBlue);
            let new_value = band.min(max_band);
            if band_selection.blue != new_value {
                band_selection.blue = new_value;
                log::debug!("🔵 Blue band drag started at: {}", band_selection.blue);
                true
            } else {
                log::debug!("🔵 Blue band drag started (no change)");
                false
            }
        }
        BandMessage::ApplyBands => {
            // Clear slider drag state since this is called on drag end
            widget_state.slider.end_drag();
            log::debug!("📊 ApplyBands: drag ended (R={}, G={}, B={})",
                band_selection.red, band_selection.green, band_selection.blue);
            false // Don't regenerate - we already regenerated on value change
        }
        BandMessage::ResetBands => {
            *band_selection = BandSelection::default_rgb().clamp(num_bands);
            widget_state.slider.end_drag(); // Also clear any drag state
            log::info!("🔄 Bands reset to default: R={}, G={}, B={}",
                band_selection.red, band_selection.green, band_selection.blue);
            true // Regenerate on reset
        }
    }
}

// ============================================================================
// Project State for handler
// ============================================================================

use crate::hvat_app::ImageSettings;
use crate::message::ProjectMessage;
use crate::project::{
    BandSelectionData, ImageSettingsData, PerImageSettings, PersistenceModeData, Project,
    ProjectSettings,
};

/// State needed for project operations.
pub struct ProjectState<'a> {
    pub annotations_map: &'a HashMap<String, AnnotationStore>,
    pub image_cache: &'a ImageCache,
    pub band_selection: BandSelection,
    pub image_settings: ImageSettings,
    pub band_persistence: PersistenceMode,
    pub image_settings_persistence: PersistenceMode,
    pub stored_band_selections: &'a HashMap<String, BandSelection>,
    pub stored_image_settings: &'a HashMap<String, ImageSettings>,
    pub status_message: &'a mut Option<String>,
}

/// Handle project messages.
///
/// Returns an optional Project that was loaded (caller should apply it to app state).
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_project(msg: ProjectMessage, state: &mut ProjectState) -> Option<Project> {
    match msg {
        ProjectMessage::SaveProject => {
            // Build project from current state
            let project = build_project_from_state(state);

            // Open file dialog for saving
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("HVAT Project", &["hvat"])
                .set_file_name("project.hvat")
                .save_file()
            {
                match project.save_to_file(&path) {
                    Ok(()) => {
                        log::info!("📁 Project saved to: {:?}", path);
                        *state.status_message = Some(format!(
                            "Project saved: {} images, {} annotations",
                            project.image_count(),
                            project.total_annotation_count()
                        ));
                    }
                    Err(e) => {
                        log::error!("Failed to save project: {}", e);
                        *state.status_message = Some(format!("Failed to save: {}", e));
                    }
                }
            }
            None
        }
        ProjectMessage::LoadProject => {
            // Open file dialog for loading
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("HVAT Project", &["hvat"])
                .pick_file()
            {
                match Project::load_from_file(&path) {
                    Ok(project) => {
                        log::info!(
                            "📁 Project loaded from: {:?} ({} images, {} annotations)",
                            path,
                            project.image_count(),
                            project.total_annotation_count()
                        );
                        *state.status_message = Some(format!(
                            "Project loaded: {} images, {} annotations",
                            project.image_count(),
                            project.total_annotation_count()
                        ));
                        return Some(project);
                    }
                    Err(e) => {
                        log::error!("Failed to load project: {}", e);
                        *state.status_message = Some(format!("Failed to load: {}", e));
                    }
                }
            }
            None
        }
        ProjectMessage::ProjectSaved(result) => {
            match result {
                Ok(path) => {
                    log::info!("📁 Project saved to: {:?}", path);
                    *state.status_message = Some("Project saved successfully".to_string());
                }
                Err(e) => {
                    log::error!("Failed to save project: {}", e);
                    *state.status_message = Some(format!("Failed to save: {}", e));
                }
            }
            None
        }
        ProjectMessage::ProjectLoaded(result) => {
            match result {
                Ok((path, project)) => {
                    log::info!("📁 Project loaded from: {:?}", path);
                    *state.status_message = Some(format!(
                        "Project loaded: {} images, {} annotations",
                        project.image_count(),
                        project.total_annotation_count()
                    ));
                    return Some(project);
                }
                Err(e) => {
                    log::error!("Failed to load project: {}", e);
                    *state.status_message = Some(format!("Failed to load: {}", e));
                }
            }
            None
        }
    }
}

/// Handle project messages for WASM.
#[cfg(target_arch = "wasm32")]
pub fn handle_project(msg: ProjectMessage, state: &mut ProjectState) -> Option<Project> {
    match msg {
        ProjectMessage::SaveProject | ProjectMessage::DownloadProject => {
            // Build project and trigger download
            let project = build_project_from_state(state);
            match project.to_json() {
                Ok(json) => {
                    // Trigger file download via JavaScript
                    download_json(&json, "project.hvat");
                    log::info!("📁 Project download triggered");
                    *state.status_message = Some(format!(
                        "Project downloaded: {} images, {} annotations",
                        project.image_count(),
                        project.total_annotation_count()
                    ));
                }
                Err(e) => {
                    log::error!("Failed to serialize project: {}", e);
                    *state.status_message = Some(format!("Failed to save: {}", e));
                }
            }
            None
        }
        ProjectMessage::LoadProject => {
            // Trigger file upload dialog via JavaScript
            trigger_project_upload();
            log::info!("📁 Project upload dialog opened");
            None
        }
        ProjectMessage::ProjectUploaded(filename, json_content) => {
            match Project::from_json(&json_content) {
                Ok(project) => {
                    log::info!(
                        "📁 Project loaded from: {} ({} images, {} annotations)",
                        filename,
                        project.image_count(),
                        project.total_annotation_count()
                    );
                    *state.status_message = Some(format!(
                        "Project loaded: {} images, {} annotations",
                        project.image_count(),
                        project.total_annotation_count()
                    ));
                    return Some(project);
                }
                Err(e) => {
                    log::error!("Failed to parse project file: {}", e);
                    *state.status_message = Some(format!("Failed to load: {}", e));
                }
            }
            None
        }
    }
}

/// Build a Project struct from the current application state.
fn build_project_from_state(state: &ProjectState) -> Project {
    let mut project = Project::new();

    // Add images from cache
    for i in 0..state.image_cache.len() {
        if let Some(name) = state.image_cache.get_name(i) {
            project.add_image(name);
        }
    }

    // Add annotations
    for (image_name, store) in state.annotations_map {
        project.set_annotations(image_name.clone(), store.clone());
    }

    // Set global settings
    project.settings = ProjectSettings {
        band_selection: state.band_selection.into(),
        image_settings: state.image_settings.into(),
        band_persistence: state.band_persistence.into(),
        image_settings_persistence: state.image_settings_persistence.into(),
    };

    // Add per-image settings
    for (image_name, band_sel) in state.stored_band_selections {
        project.set_per_image_band_selection(image_name.clone(), *band_sel);
    }
    for (image_name, img_settings) in state.stored_image_settings {
        project.set_per_image_settings(image_name.clone(), *img_settings);
    }

    project
}

/// WASM helper: Trigger file download via JavaScript.
#[cfg(target_arch = "wasm32")]
fn download_json(json: &str, filename: &str) {
    use wasm_bindgen::JsCast;
    use web_sys::{window, Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let window = window().expect("no window");
    let document = window.document().expect("no document");

    // Create blob from JSON using str sequence
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(json));

    let mut options = BlobPropertyBag::new();
    options.type_("application/json");

    let blob = Blob::new_with_str_sequence(&array).expect("create blob");
    let url = Url::create_object_url_with_blob(&blob).expect("create url");

    // Create download link
    let a: HtmlAnchorElement = document
        .create_element("a")
        .expect("create element")
        .dyn_into()
        .expect("cast to anchor");
    a.set_href(&url);
    a.set_download(filename);
    a.click();

    // Clean up
    let _ = Url::revoke_object_url(&url);
}

/// WASM helper: Trigger project file upload dialog.
#[cfg(target_arch = "wasm32")]
fn trigger_project_upload() {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{window, HtmlInputElement};

    let window = window().expect("no window");
    let document = window.document().expect("no document");

    // Create file input
    let input: HtmlInputElement = document
        .create_element("input")
        .expect("create element")
        .dyn_into()
        .expect("cast to input");
    input.set_type("file");
    input.set_accept(".hvat,application/json");

    // Set up change handler
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let filename = file.name();
                let reader = web_sys::FileReader::new().expect("create reader");

                let filename_clone = filename.clone();
                let reader_clone = reader.clone();
                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(text) = result.as_string() {
                            // Store in pending for next update cycle
                            store_pending_project(filename_clone.clone(), text);
                        }
                    }
                }) as Box<dyn FnMut(_)>);

                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();

                let _ = reader.read_as_text(&file);
            }
        }
    }) as Box<dyn FnMut(_)>);

    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
    closure.forget();

    input.click();
}

/// WASM: Store pending project for next update cycle.
#[cfg(target_arch = "wasm32")]
fn store_pending_project(filename: String, content: String) {
    use std::sync::Mutex;
    static PENDING_PROJECT: Mutex<Option<(String, String)>> = Mutex::new(None);
    if let Ok(mut guard) = PENDING_PROJECT.lock() {
        *guard = Some((filename, content));
    }
}

/// WASM: Take pending project if any.
#[cfg(target_arch = "wasm32")]
pub fn take_pending_project() -> Option<(String, String)> {
    use std::sync::Mutex;
    static PENDING_PROJECT: Mutex<Option<(String, String)>> = Mutex::new(None);
    PENDING_PROJECT.lock().ok().and_then(|mut guard| guard.take())
}
