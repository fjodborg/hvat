//! Message handlers for HVAT application.
//!
//! Each handler processes a specific category of messages,
//! keeping the main HvatApp update function clean and organized.

use crate::annotation::{AnnotationStore, AnnotationTool, Category, DrawingState, Point, Shape};
use crate::hyperspectral::{BandSelection, HyperspectralImage};
use crate::image_cache::ImageCache;
use crate::message::{
    AnnotationMessage, BandMessage, CounterMessage, ImageLoadMessage, ImageSettingsMessage,
    ImageViewMessage, NavigationMessage, PersistenceMode, Tab, UIMessage,
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
        UIMessage::ScrollbarDragStartY => {
            widget_state.scroll.start_drag_y();
            log::debug!("📜 Scrollbar Y drag started");
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
        UIMessage::ScrollbarDragStartX => {
            widget_state.scroll.start_drag_x();
            log::debug!("📜 Scrollbar X drag started");
        }
        UIMessage::ScrollbarDragEndX => {
            widget_state.scroll.end_drag_x();
            log::debug!("📜 Scrollbar X drag ended");
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
            log::debug!("🎚️ Band persistence mode: {:?}", mode);
        }
        UIMessage::SetImageSettingsPersistence(mode) => {
            *image_settings_persistence = mode;
            log::debug!("🎚️ Image settings persistence mode: {:?}", mode);
        }
    }
}

/// State needed for annotation handler.
pub struct AnnotationState<'a> {
    pub annotations_map: &'a mut HashMap<String, AnnotationStore>,
    pub drawing_state: &'a mut DrawingState,
    pub image_key: String,
    pub zoom: f32,
    pub status_message: &'a mut Option<String>,
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
        AnnotationMessage::AddCategory(name) => {
            let id = state.annotations().categories().count() as u32;
            state
                .annotations_mut()
                .add_category(Category::new(id, name.clone()));
            log::debug!("🏷️ Added category: {} (id={})", name, id);
        }
        AnnotationMessage::StartDrawing(x, y) => {
            match state.drawing_state.tool {
                AnnotationTool::Select => {
                    // Hit test for selection
                    let hit = state.annotations().hit_test(&Point::new(x, y));
                    state.annotations_mut().select(hit);
                    if let Some(id) = hit {
                        log::debug!("🔍 Selected annotation {}", id);
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
        AnnotationMessage::ExportJson => {
            match state.annotations().to_json() {
                Ok(json) => {
                    log::info!(
                        "📤 Exported {} annotations to JSON",
                        state.annotations().len()
                    );
                    // In a real app, we'd save to file or clipboard
                    // For now, just log a preview
                    if json.len() > 200 {
                        log::debug!("JSON preview: {}...", &json[..200]);
                    } else {
                        log::debug!("JSON: {}", json);
                    }
                    *state.status_message =
                        Some(format!("Exported {} annotations", state.annotations().len()));
                }
                Err(e) => {
                    log::error!("Failed to export JSON: {}", e);
                    *state.status_message = Some(format!("Export failed: {}", e));
                }
            }
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
