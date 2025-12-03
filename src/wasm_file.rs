//! WASM file loading utilities.
//!
//! Provides file picker and file loading functionality for the WASM build.
//! Uses web_sys to interact with browser APIs.

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Global storage for loaded files (raw bytes, not decoded) - the app polls this via tick()
    static WASM_PENDING_FILES: RefCell<Option<Vec<(String, Vec<u8>)>>> = const { RefCell::new(None) };
}

/// Check if there are pending files loaded from WASM file picker.
#[cfg(target_arch = "wasm32")]
pub fn take_wasm_pending_files() -> Option<Vec<(String, Vec<u8>)>> {
    WASM_PENDING_FILES.with(|pending| pending.borrow_mut().take())
}

#[cfg(target_arch = "wasm32")]
fn set_wasm_pending_files(files: Vec<(String, Vec<u8>)>) {
    WASM_PENDING_FILES.with(|pending| {
        *pending.borrow_mut() = Some(files);
    });
}

/// Open the WASM file picker for selecting image files/folders.
#[cfg(target_arch = "wasm32")]
pub fn open_wasm_file_picker() {
    use web_sys::{Document, Event, FileReader, HtmlInputElement};

    let window = web_sys::window().expect("no window");
    let document: Document = window.document().expect("no document");

    // Create a hidden file input element
    let input: HtmlInputElement = document
        .create_element("input")
        .expect("failed to create input")
        .dyn_into()
        .expect("not an input element");

    input.set_type("file");
    input.set_accept("image/*");
    input.set_multiple(true);

    // Enable folder selection using webkitdirectory attribute
    // This is widely supported (Chrome, Edge, Firefox, Safari)
    input
        .set_attribute("webkitdirectory", "")
        .expect("failed to set webkitdirectory");
    input
        .set_attribute("directory", "")
        .expect("failed to set directory"); // Firefox fallback

    // Store raw file bytes as they load (lazy loading - no decoding here)
    let results: Rc<RefCell<Vec<(String, Vec<u8>)>>> = Rc::new(RefCell::new(Vec::new()));
    let total_files: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let loaded_files: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    // Clone for closure
    let results_clone = results.clone();
    let total_clone = total_files.clone();
    let loaded_clone = loaded_files.clone();

    // Handle file selection
    let onchange = Closure::wrap(Box::new(move |event: Event| {
        let input: HtmlInputElement = event
            .target()
            .expect("no target")
            .dyn_into()
            .expect("not input");

        if let Some(files) = input.files() {
            let count = files.length();
            if count == 0 {
                log::warn!("📂 No files selected");
                return;
            }

            // Filter to only image files using the centralized check
            let mut image_files = Vec::new();
            for i in 0..count {
                if let Some(file) = files.get(i) {
                    let name = file.name();
                    if crate::image_cache::is_image_file(&name) {
                        image_files.push(file);
                    }
                }
            }

            if image_files.is_empty() {
                log::warn!("📂 No image files found");
                set_wasm_pending_files(Vec::new());
                return;
            }

            let image_count = image_files.len();

            // Show warning for large folders (> 50 images)
            const LARGE_FOLDER_THRESHOLD: usize = 50;
            if image_count > LARGE_FOLDER_THRESHOLD {
                let window = web_sys::window().expect("no window");
                let message = format!(
                    "Warning: You selected {} images.\n\n\
                    Loading many images in the browser can use significant memory.\n\n\
                    For large datasets, the native desktop application is recommended.\n\n\
                    Continue anyway?",
                    image_count
                );

                // Use confirm() dialog - returns true if user clicks OK
                let confirmed = window.confirm_with_message(&message).unwrap_or(false);
                if !confirmed {
                    log::info!("📂 User cancelled loading {} images", image_count);
                    set_wasm_pending_files(Vec::new());
                    return;
                }
                log::info!("📂 User confirmed loading {} images", image_count);
            }

            *total_clone.borrow_mut() = image_count;
            log::info!(
                "📂 Found {} image files (lazy loading - will decode on demand)",
                image_count
            );

            for file in image_files {
                let name = file.name();
                log::info!("📂 Reading file: {}", name);

                let reader = FileReader::new().expect("failed to create FileReader");

                let results_inner = results_clone.clone();
                let loaded_inner = loaded_clone.clone();
                let total_inner = total_clone.clone();
                let name_clone = name.clone();

                // Handle load complete - store raw bytes, no decoding
                let onload = Closure::wrap(Box::new(move |event: Event| {
                    let reader: FileReader = event
                        .target()
                        .expect("no target")
                        .dyn_into()
                        .expect("not FileReader");

                    if let Ok(result) = reader.result() {
                        let array = js_sys::Uint8Array::new(&result);
                        let bytes = array.to_vec();

                        log::info!(
                            "📂 File {} read: {} bytes (not decoded yet)",
                            name_clone,
                            bytes.len()
                        );

                        // Store raw bytes - decoding happens lazily when viewing
                        results_inner
                            .borrow_mut()
                            .push((name_clone.clone(), bytes));
                    }

                    // Check if all files loaded
                    *loaded_inner.borrow_mut() += 1;
                    let loaded = *loaded_inner.borrow();
                    let total = *total_inner.borrow();

                    if loaded >= total {
                        log::info!("📂 All {} files read (will decode on demand)", total);
                        let files = results_inner.borrow().clone();
                        set_wasm_pending_files(files);
                    }
                }) as Box<dyn FnMut(Event)>);

                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget(); // Leak the closure to keep it alive

                // Read as array buffer
                reader.read_as_array_buffer(&file).expect("failed to read");
            }
        }
    }) as Box<dyn FnMut(Event)>);

    input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    onchange.forget(); // Leak the closure to keep it alive

    // Trigger the file picker
    input.click();
}

// Stub implementations for non-WASM builds
#[cfg(not(target_arch = "wasm32"))]
pub fn take_wasm_pending_files() -> Option<Vec<(String, Vec<u8>)>> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_wasm_file_picker() {
    // No-op on native
}
