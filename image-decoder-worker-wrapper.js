// Web Worker wrapper for image-decoder-worker WASM module
// This wrapper initializes the WASM module when the worker starts.

import init from './image-decoder-worker.js';

// Initialize the WASM module - this calls wasm_bindgen(start) which sets up message handlers
await init();
