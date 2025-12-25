//! Web Worker manager for async image decoding (WASM only)
//!
//! This module provides an `ImageDecoderWorker` that manages communication with
//! a separate WASM binary running in a Web Worker. Image decoding AND RGBA packing
//! is offloaded to the worker thread to keep the main thread responsive during preloading.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

/// Pre-packed RGBA layer data received from the worker, ready for GPU upload
pub struct PackedLayer {
    /// RGBA pixel data (width * height * 4 bytes)
    pub rgba_data: Vec<u8>,
    /// Layer index in the texture array
    pub layer_index: u32,
}

/// Decoded and pre-packed image data received from the worker
pub struct DecodedImage {
    /// Path/name of the decoded image
    pub path: PathBuf,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Number of spectral bands
    pub num_bands: usize,
    /// Number of texture layers
    pub num_layers: u32,
    /// Pre-packed RGBA layers ready for GPU upload
    pub layers: Vec<PackedLayer>,
}

/// Error result from worker decode attempt
pub struct DecodeError {
    /// Path/name of the image that failed
    pub path: PathBuf,
    /// Error message describing the failure
    pub error: String,
}

/// Result from the worker - either decoded data or an error
pub enum WorkerResult {
    /// Successfully decoded image
    Decoded(DecodedImage),
    /// Decode failed with error
    Error(DecodeError),
}

/// Manages a Web Worker for async image decoding.
///
/// The worker runs image decoding in a separate thread, preventing
/// the main thread from blocking during CPU-intensive decode operations.
pub struct ImageDecoderWorker {
    /// The underlying Web Worker
    worker: Worker,
    /// Counter for generating unique request IDs
    next_id: u32,
    /// Map from request ID to image path (for matching responses)
    pending: Rc<RefCell<HashMap<u32, PathBuf>>>,
    /// Completed results waiting to be processed by main thread
    results: Rc<RefCell<Vec<WorkerResult>>>,
    /// Whether the worker has signaled it's ready
    ready: Rc<RefCell<bool>>,
    /// Messages queued before worker was ready
    queued: Vec<(u32, PathBuf, Vec<u8>)>,
    /// Closure stored to prevent deallocation
    _onmessage: Closure<dyn Fn(MessageEvent)>,
}

impl ImageDecoderWorker {
    /// Spawn a new image decoder worker.
    ///
    /// Returns `Err` if the worker fails to create (e.g., worker JS not found).
    pub fn spawn() -> Result<Self, String> {
        // Create worker options for ES module worker
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);

        // Use the wrapper that initializes the WASM module
        let worker = Worker::new_with_options("./image-decoder-worker-wrapper.js", &options)
            .map_err(|e| format!("Failed to create worker: {:?}", e))?;

        let results: Rc<RefCell<Vec<WorkerResult>>> = Rc::new(RefCell::new(Vec::new()));
        let ready = Rc::new(RefCell::new(false));
        let pending: Rc<RefCell<HashMap<u32, PathBuf>>> = Rc::new(RefCell::new(HashMap::new()));

        // Set up message handler for worker responses
        let results_clone = Rc::clone(&results);
        let ready_clone = Rc::clone(&ready);
        let pending_clone = Rc::clone(&pending);

        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();

            // Check for ready signal from worker
            if let Ok(msg_type) = Reflect::get(&data, &"type".into()) {
                if msg_type.as_string().as_deref() == Some("ready") {
                    *ready_clone.borrow_mut() = true;
                    log::info!("Image decoder worker ready");
                    return;
                }
            }

            // Extract request ID
            let id = match Reflect::get(&data, &"id".into()) {
                Ok(v) => v.as_f64().map(|v| v as u32),
                Err(_) => None,
            };

            let Some(id) = id else {
                log::warn!("Worker response missing 'id' field");
                return;
            };

            // Look up the path for this request
            let Some(path) = pending_clone.borrow_mut().remove(&id) else {
                log::warn!("Received response for unknown request ID: {}", id);
                return;
            };

            // Check for error response
            if let Ok(error) = Reflect::get(&data, &"error".into()) {
                if let Some(error_str) = error.as_string() {
                    log::debug!("Worker decode error for {:?}: {}", path, error_str);
                    results_clone
                        .borrow_mut()
                        .push(WorkerResult::Error(DecodeError {
                            path,
                            error: error_str,
                        }));
                    return;
                }
            }

            // Extract decoded data with pre-packed layers
            let width = Reflect::get(&data, &"width".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as u32;
            let height = Reflect::get(&data, &"height".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as u32;
            let num_bands = Reflect::get(&data, &"num_bands".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(3.0) as usize;
            let num_layers = Reflect::get(&data, &"num_layers".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as u32;

            // Extract pre-packed RGBA layers
            let layers = match Reflect::get(&data, &"layers".into()) {
                Ok(layers_js) => {
                    let arr: js_sys::Array = layers_js.unchecked_into();
                    (0..arr.length())
                        .map(|i| {
                            let u8_arr: Uint8Array = arr.get(i).unchecked_into();
                            PackedLayer {
                                rgba_data: u8_arr.to_vec(),
                                layer_index: i,
                            }
                        })
                        .collect::<Vec<_>>()
                }
                Err(_) => {
                    log::warn!("Worker response missing 'layers' field");
                    Vec::new()
                }
            };

            log::debug!(
                "Worker decoded {:?}: {}x{} with {} bands, {} layers (pre-packed)",
                path,
                width,
                height,
                num_bands,
                layers.len()
            );

            results_clone
                .borrow_mut()
                .push(WorkerResult::Decoded(DecodedImage {
                    path,
                    width,
                    height,
                    num_bands,
                    num_layers,
                    layers,
                }));
        }) as Box<dyn Fn(MessageEvent)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        log::info!("Image decoder worker spawned, waiting for ready signal...");

        Ok(Self {
            worker,
            next_id: 0,
            pending,
            results,
            ready,
            queued: Vec::new(),
            _onmessage: onmessage,
        })
    }

    /// Request decode of an image.
    ///
    /// If the worker isn't ready yet, the request is queued and will be
    /// sent when `flush_queue()` is called after the worker signals ready.
    pub fn request_decode(&mut self, path: PathBuf, data: Vec<u8>) {
        let id = self.next_id;
        self.next_id += 1;

        if !*self.ready.borrow() {
            log::debug!("Worker not ready, queueing decode request for {:?}", path);
            self.queued.push((id, path, data));
            return;
        }

        self.send_request(id, path, data);
    }

    /// Send a decode request to the worker.
    fn send_request(&mut self, id: u32, path: PathBuf, data: Vec<u8>) {
        self.pending.borrow_mut().insert(id, path.clone());

        let msg = Object::new();
        Reflect::set(&msg, &"id".into(), &id.into()).unwrap();
        Reflect::set(
            &msg,
            &"name".into(),
            &path.to_string_lossy().into_owned().into(),
        )
        .unwrap();

        let bytes = Uint8Array::from(data.as_slice());
        Reflect::set(&msg, &"bytes".into(), &bytes).unwrap();

        if let Err(e) = self.worker.post_message(&msg) {
            log::error!("Failed to send decode request to worker: {:?}", e);
            self.pending.borrow_mut().remove(&id);
        } else {
            log::debug!("Sent decode request {} for {:?}", id, path);
        }
    }

    /// Process any queued messages if worker is now ready.
    ///
    /// Call this periodically (e.g., each tick) to flush queued requests
    /// once the worker signals it's ready.
    pub fn flush_queue(&mut self) {
        if !*self.ready.borrow() {
            return;
        }

        if !self.queued.is_empty() {
            log::debug!("Flushing {} queued decode requests", self.queued.len());
        }

        let queued = std::mem::take(&mut self.queued);
        for (id, path, data) in queued {
            self.send_request(id, path, data);
        }
    }

    /// Take all completed results from the queue.
    ///
    /// Returns decoded images and errors that have been received from the worker
    /// since the last call. The caller is responsible for processing these
    /// (e.g., uploading decoded data to GPU).
    #[allow(dead_code)]
    pub fn take_results(&mut self) -> Vec<WorkerResult> {
        std::mem::take(&mut *self.results.borrow_mut())
    }

    /// Take one completed result from the queue.
    ///
    /// Returns the oldest result, or None if no results are available.
    /// Use this to process one result per frame to avoid blocking.
    pub fn take_one_result(&mut self) -> Option<WorkerResult> {
        let mut results = self.results.borrow_mut();
        if results.is_empty() {
            None
        } else {
            Some(results.remove(0))
        }
    }

    /// Check if there are completed results waiting to be processed.
    pub fn has_results(&self) -> bool {
        !self.results.borrow().is_empty()
    }

    /// Check if the worker is ready to receive messages.
    #[allow(dead_code)]
    pub fn is_ready(&self) -> bool {
        *self.ready.borrow()
    }

    /// Get the number of pending requests (in-flight + queued).
    pub fn pending_count(&self) -> usize {
        self.pending.borrow().len() + self.queued.len()
    }

    /// Check if a specific path has a pending request.
    pub fn is_pending(&self, path: &PathBuf) -> bool {
        // Check queued requests
        if self.queued.iter().any(|(_, p, _)| p == path) {
            return true;
        }
        // Check in-flight requests
        self.pending.borrow().values().any(|p| p == path)
    }
}
