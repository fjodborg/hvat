//! Background thread for async image decoding (native only)
//!
//! This module provides a `NativeDecoderThread` that manages a background thread
//! for image decoding. This mirrors the WASM `ImageDecoderWorker` API for unified
//! handling in the main application.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use super::preload_types::{
    DecodeError, DecodeResult, DecodedImage, calculate_num_layers, pack_bands_to_layers,
};
use crate::data::HyperspectralData;

/// Request to decode an image, sent to the background thread.
struct DecodeRequest {
    /// Unique request ID for tracking (reserved for future use)
    #[allow(dead_code)]
    id: u32,
    /// Path for cache key
    path: PathBuf,
    /// Raw image bytes
    data: Vec<u8>,
}

/// Message sent to the decoder thread.
enum ThreadMessage {
    /// Decode an image
    Decode(DecodeRequest),
    /// Shutdown the thread
    Shutdown,
}

/// Manages a background thread for async image decoding.
///
/// The thread runs image decoding in the background, preventing the main thread
/// from blocking during CPU-intensive decode operations. API mirrors the WASM
/// `ImageDecoderWorker` for unified handling.
pub struct NativeDecoderThread {
    /// Sender for requests to the background thread
    request_tx: Sender<ThreadMessage>,
    /// Receiver for results from the background thread
    result_rx: Receiver<DecodeResult>,
    /// Handle to the background thread (for joining on drop)
    thread_handle: Option<JoinHandle<()>>,
    /// Counter for generating unique request IDs
    next_id: u32,
    /// Set of pending request paths (for is_pending check)
    pending_paths: HashSet<PathBuf>,
}

impl NativeDecoderThread {
    /// Spawn a new decoder thread.
    ///
    /// Returns `Err` if the thread fails to spawn.
    pub fn spawn() -> Result<Self, String> {
        let (request_tx, request_rx) = mpsc::channel::<ThreadMessage>();
        let (result_tx, result_rx) = mpsc::channel::<DecodeResult>();

        let thread_handle = thread::Builder::new()
            .name("image-decoder".to_string())
            .spawn(move || {
                log::info!("Native image decoder thread started");
                Self::thread_loop(request_rx, result_tx);
                log::info!("Native image decoder thread exiting");
            })
            .map_err(|e| format!("Failed to spawn decoder thread: {}", e))?;

        log::info!("Native image decoder thread spawned");

        Ok(Self {
            request_tx,
            result_rx,
            thread_handle: Some(thread_handle),
            next_id: 0,
            pending_paths: HashSet::new(),
        })
    }

    /// Background thread main loop.
    fn thread_loop(request_rx: Receiver<ThreadMessage>, result_tx: Sender<DecodeResult>) {
        loop {
            match request_rx.recv() {
                Ok(ThreadMessage::Decode(request)) => {
                    let result = Self::decode_image(request.path, &request.data);
                    if result_tx.send(result).is_err() {
                        log::warn!("Result channel closed, decoder thread exiting");
                        break;
                    }
                }
                Ok(ThreadMessage::Shutdown) => {
                    log::debug!("Received shutdown signal");
                    break;
                }
                Err(_) => {
                    // Channel closed, exit
                    log::debug!("Request channel closed, decoder thread exiting");
                    break;
                }
            }
        }
    }

    /// Decode an image and pack it into RGBA layers.
    fn decode_image(path: PathBuf, data: &[u8]) -> DecodeResult {
        log::debug!("Decoding image: {:?} ({} bytes)", path, data.len());

        // Try to load as hyperspectral first
        match HyperspectralData::from_bytes(data) {
            Ok(hyper) => {
                let width = hyper.width;
                let height = hyper.height;
                let num_bands = hyper.bands.len();
                let num_layers = calculate_num_layers(num_bands);

                // Pack bands into RGBA layers
                let layers = pack_bands_to_layers(&hyper.bands, width, height, num_layers);

                log::debug!(
                    "Decoded {:?}: {}x{} with {} bands, {} layers",
                    path,
                    width,
                    height,
                    num_bands,
                    num_layers
                );

                DecodeResult::Decoded(DecodedImage {
                    path,
                    width,
                    height,
                    num_bands,
                    num_layers,
                    layers,
                })
            }
            Err(e) => {
                log::debug!("Failed to decode {:?}: {}", path, e);
                DecodeResult::Error(DecodeError {
                    path,
                    error: e.to_string(),
                })
            }
        }
    }

    /// Request decode of an image.
    ///
    /// The request is sent to the background thread asynchronously.
    pub fn request_decode(&mut self, path: PathBuf, data: Vec<u8>) {
        let id = self.next_id;
        self.next_id += 1;

        self.pending_paths.insert(path.clone());

        let request = DecodeRequest { id, path, data };

        if self
            .request_tx
            .send(ThreadMessage::Decode(request))
            .is_err()
        {
            log::error!("Failed to send decode request: channel closed");
        } else {
            log::debug!("Sent decode request {}", id);
        }
    }

    /// Take one completed result from the queue.
    ///
    /// Returns the oldest result, or None if no results are available.
    /// Non-blocking.
    pub fn take_one_result(&mut self) -> Option<DecodeResult> {
        match self.result_rx.try_recv() {
            Ok(result) => {
                // Remove from pending set
                match &result {
                    DecodeResult::Decoded(img) => {
                        self.pending_paths.remove(&img.path);
                    }
                    DecodeResult::Error(err) => {
                        self.pending_paths.remove(&err.path);
                    }
                }
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                log::warn!("Decoder thread disconnected");
                None
            }
        }
    }

    /// Check if there are completed results waiting to be processed.
    #[allow(dead_code)]
    pub fn has_results(&self) -> bool {
        // Note: This is a heuristic - we can't peek without consuming
        // For now, we'll just return whether we have pending requests
        // The actual check happens in take_one_result
        !self.pending_paths.is_empty()
    }

    /// Get the number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending_paths.len()
    }

    /// Check if a specific path has a pending request.
    pub fn is_pending(&self, path: &PathBuf) -> bool {
        self.pending_paths.contains(path)
    }

    /// Flush any queued messages (no-op for native, exists for API parity with WASM).
    #[allow(dead_code)]
    pub fn flush_queue(&mut self) {
        // No-op: native thread doesn't need queue flushing like WASM worker
    }
}

impl Drop for NativeDecoderThread {
    fn drop(&mut self) {
        log::debug!("Shutting down native decoder thread");

        // Send shutdown signal
        let _ = self.request_tx.send(ThreadMessage::Shutdown);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            if let Err(e) = handle.join() {
                log::warn!("Decoder thread panicked: {:?}", e);
            }
        }
    }
}
