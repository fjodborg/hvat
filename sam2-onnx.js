// SAM2 ONNX Runtime Web Inference Module
// Encoder runs in a Web Worker, decoder on main thread
// Features: IndexedDB caching, WebGL/WASM execution providers

// =============================================================================
// Configuration
// =============================================================================

const CONFIG = {
  // Model URLs (g-ronimo's optimized SAM2-tiny for browser)
  encoderUrl:
    "https://huggingface.co/g-ronimo/sam2-tiny/resolve/main/sam2_hiera_tiny_encoder.with_runtime_opt.ort",
  decoderUrl:
    "https://huggingface.co/g-ronimo/sam2-tiny/resolve/main/sam2_hiera_tiny_decoder_pr1.onnx",

  // Encoder expects 1024x1024 input
  encoderInputSize: 1024,

  // ImageNet normalization
  mean: [0.485, 0.456, 0.406],
  std: [0.229, 0.224, 0.225],

  // IndexedDB cache settings
  dbName: "sam2-model-cache",
  dbVersion: 1,
  encoderStore: "encoder",
  decoderStore: "decoder",
};

// =============================================================================
// Global State
// =============================================================================

let encoderWorker = null;
let decoderSession = null;
let isLoading = false;
let loadProgress = 0;
let isWarmedUp = false;

// Pending encoder requests (id -> {resolve, reject})
const pendingEncoderRequests = new Map();
let encoderRequestId = 0;

// =============================================================================
// IndexedDB Model Cache
// =============================================================================

function openModelCache() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(CONFIG.dbName, CONFIG.dbVersion);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    request.onupgradeneeded = (event) => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains(CONFIG.encoderStore)) {
        db.createObjectStore(CONFIG.encoderStore);
      }
      if (!db.objectStoreNames.contains(CONFIG.decoderStore)) {
        db.createObjectStore(CONFIG.decoderStore);
      }
    };
  });
}

async function getCachedModel(storeName, url) {
  try {
    const db = await openModelCache();
    return new Promise((resolve) => {
      const tx = db.transaction(storeName, "readonly");
      const store = tx.objectStore(storeName);
      const request = store.get(url);
      request.onerror = () => resolve(null);
      request.onsuccess = () => resolve(request.result);
    });
  } catch (e) {
    console.warn("[SAM2] IndexedDB not available:", e);
    return null;
  }
}

async function cacheModel(storeName, url, data) {
  try {
    const db = await openModelCache();
    return new Promise((resolve) => {
      const tx = db.transaction(storeName, "readwrite");
      const store = tx.objectStore(storeName);
      const request = store.put(data, url);
      request.onerror = () => resolve(false);
      request.onsuccess = () => resolve(true);
    });
  } catch (e) {
    console.warn("[SAM2] Failed to cache model:", e);
    return false;
  }
}

// =============================================================================
// ONNX Runtime Initialization
// =============================================================================

async function initOrt() {
  if (window.ort) return;

  const script = document.createElement("script");
  script.src = "https://cdn.jsdelivr.net/npm/onnxruntime-web/dist/ort.min.js";
  script.async = true;

  await new Promise((resolve, reject) => {
    script.onload = resolve;
    script.onerror = () => reject(new Error("Failed to load onnxruntime-web"));
    document.head.appendChild(script);
  });

  // Configure for best performance
  ort.env.wasm.numThreads = navigator.hardwareConcurrency || 4;
  ort.env.wasm.simd = true;

  console.log(
    "[SAM2] onnxruntime-web loaded, version:",
    ort.env.versions.common,
  );
  console.log(
    "[SAM2] WASM config: threads=%d, simd=%s",
    ort.env.wasm.numThreads,
    ort.env.wasm.simd,
  );
}

async function createSession(modelData, name) {
  const startTime = performance.now();

  // Try WebGL first, fall back to WASM
  let session;
  try {
    session = await ort.InferenceSession.create(modelData.buffer, {
      executionProviders: ["webgl"],
      graphOptimizationLevel: "all",
    });
    console.log("[SAM2] %s using WebGL provider", name);
  } catch (e) {
    console.log("[SAM2] %s WebGL failed (%s), using WASM", name, e.message);
    session = await ort.InferenceSession.create(modelData.buffer, {
      executionProviders: ["wasm"],
      graphOptimizationLevel: "all",
    });
  }

  console.log(
    "[SAM2] %s session created in %dms",
    name,
    Math.round(performance.now() - startTime),
  );
  console.log(
    "[SAM2] %s inputs: %s, outputs: %s",
    name,
    session.inputNames,
    session.outputNames,
  );

  return session;
}

// =============================================================================
// Encoder Web Worker
// =============================================================================

function createEncoderWorker() {
  const workerCode = `
// SAM2 Encoder Web Worker - runs inference in background thread

let encoderSession = null;
const INPUT_SIZE = ${CONFIG.encoderInputSize};
const MEAN = ${JSON.stringify(CONFIG.mean)};
const STD = ${JSON.stringify(CONFIG.std)};

// IndexedDB helpers (duplicated because workers can't share code)
const DB_NAME = '${CONFIG.dbName}';
const DB_VERSION = ${CONFIG.dbVersion};
const STORE_NAME = '${CONFIG.encoderStore}';

function openDB() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open(DB_NAME, DB_VERSION);
        req.onerror = () => reject(req.error);
        req.onsuccess = () => resolve(req.result);
        req.onupgradeneeded = (e) => {
            const db = e.target.result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                db.createObjectStore(STORE_NAME);
            }
        };
    });
}

async function getFromCache(url) {
    try {
        const db = await openDB();
        return new Promise((resolve) => {
            const tx = db.transaction(STORE_NAME, 'readonly');
            const req = tx.objectStore(STORE_NAME).get(url);
            req.onerror = () => resolve(null);
            req.onsuccess = () => resolve(req.result);
        });
    } catch { return null; }
}

async function saveToCache(url, data) {
    try {
        const db = await openDB();
        return new Promise((resolve) => {
            const tx = db.transaction(STORE_NAME, 'readwrite');
            const req = tx.objectStore(STORE_NAME).put(data, url);
            req.onerror = () => resolve(false);
            req.onsuccess = () => resolve(true);
        });
    } catch { return false; }
}

// Preprocess image: resize to 1024x1024, normalize with ImageNet stats
function preprocessImage(imageData, width, height) {
    const longestSide = Math.max(width, height);
    const scale = INPUT_SIZE / longestSide;
    const newW = Math.round(width * scale);
    const newH = Math.round(height * scale);

    const output = new Float32Array(3 * INPUT_SIZE * INPUT_SIZE);

    // Fill with padding (normalized zero)
    for (let c = 0; c < 3; c++) {
        const padVal = -MEAN[c] / STD[c];
        const offset = c * INPUT_SIZE * INPUT_SIZE;
        for (let i = 0; i < INPUT_SIZE * INPUT_SIZE; i++) {
            output[offset + i] = padVal;
        }
    }

    // Resize and normalize
    const invScale = 1 / scale;
    for (let y = 0; y < Math.min(newH, INPUT_SIZE); y++) {
        for (let x = 0; x < Math.min(newW, INPUT_SIZE); x++) {
            const srcX = Math.min(Math.floor(x * invScale), width - 1);
            const srcY = Math.min(Math.floor(y * invScale), height - 1);
            const srcIdx = (srcY * width + srcX) * 3;

            for (let c = 0; c < 3; c++) {
                const pixel = imageData[srcIdx + c] / 255;
                output[c * INPUT_SIZE * INPUT_SIZE + y * INPUT_SIZE + x] = (pixel - MEAN[c]) / STD[c];
            }
        }
    }

    return output;
}

self.onmessage = async function(e) {
    const { type, id, data } = e.data;

    if (type === 'load') {
        try {
            // Load ONNX Runtime
            if (typeof ort === 'undefined') {
                importScripts('https://cdn.jsdelivr.net/npm/onnxruntime-web/dist/ort.min.js');
                ort.env.wasm.numThreads = navigator.hardwareConcurrency || 4;
                ort.env.wasm.simd = true;
                console.log('[SAM2 Worker] ORT loaded, threads=%d', ort.env.wasm.numThreads);
            }

            // Try cache first
            let modelData = await getFromCache(data.encoderUrl);
            if (modelData) {
                console.log('[SAM2 Worker] Using cached encoder');
                self.postMessage({ type: 'progress', progress: 0.9 });
            } else {
                console.log('[SAM2 Worker] Downloading encoder...');
                const resp = await fetch(data.encoderUrl);
                if (!resp.ok) throw new Error('Fetch failed: ' + resp.status);

                const total = parseInt(resp.headers.get('content-length') || '0');
                const reader = resp.body.getReader();
                const chunks = [];
                let received = 0;

                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    chunks.push(value);
                    received += value.length;
                    self.postMessage({ type: 'progress', progress: received / total });
                }

                modelData = new Uint8Array(received);
                let offset = 0;
                for (const chunk of chunks) {
                    modelData.set(chunk, offset);
                    offset += chunk.length;
                }

                await saveToCache(data.encoderUrl, modelData);
            }

            // Create session
            console.log('[SAM2 Worker] Creating session (%d bytes)...', modelData.length);
            const t0 = performance.now();

            try {
                encoderSession = await ort.InferenceSession.create(modelData.buffer, {
                    executionProviders: ['webgl'],
                    graphOptimizationLevel: 'all'
                });
                console.log('[SAM2 Worker] Using WebGL');
            } catch (e) {
                console.log('[SAM2 Worker] WebGL failed, using WASM');
                encoderSession = await ort.InferenceSession.create(modelData.buffer, {
                    executionProviders: ['wasm'],
                    graphOptimizationLevel: 'all'
                });
            }

            console.log('[SAM2 Worker] Session created in %dms', Math.round(performance.now() - t0));

            // Warmup
            console.log('[SAM2 Worker] Warming up...');
            const warmupT = performance.now();
            const warmupTensor = new ort.Tensor('float32', new Float32Array(3 * INPUT_SIZE * INPUT_SIZE), [1, 3, INPUT_SIZE, INPUT_SIZE]);
            await encoderSession.run({ image: warmupTensor });
            console.log('[SAM2 Worker] Warmup done in %dms', Math.round(performance.now() - warmupT));

            self.postMessage({ type: 'loaded' });
        } catch (error) {
            console.error('[SAM2 Worker] Load error:', error);
            self.postMessage({ type: 'error', error: error.message });
        }
    }
    else if (type === 'encode') {
        if (!encoderSession) {
            self.postMessage({ type: 'result', id, success: false, error: 'Not loaded' });
            return;
        }

        try {
            const { imageData, width, height } = data;
            console.log('[SAM2 Worker] Encoding %dx%d...', width, height);
            const t0 = performance.now();

            const inputData = preprocessImage(new Uint8Array(imageData), width, height);
            const tensor = new ort.Tensor('float32', inputData, [1, 3, INPUT_SIZE, INPUT_SIZE]);
            const results = await encoderSession.run({ image: tensor });

            console.log('[SAM2 Worker] Encoding took %dms', Math.round(performance.now() - t0));

            const outputs = encoderSession.outputNames;
            const imageEmbed = results.image_embed || results[outputs[0]];
            const highRes0 = results.high_res_feats_0 || results[outputs[1]];
            const highRes1 = results.high_res_feats_1 || results[outputs[2]];

            self.postMessage({
                type: 'result',
                id,
                success: true,
                embeddings: {
                    imageEmbed: { data: Array.from(imageEmbed.data), shape: imageEmbed.dims },
                    highResFeats0: { data: Array.from(highRes0.data), shape: highRes0.dims },
                    highResFeats1: { data: Array.from(highRes1.data), shape: highRes1.dims },
                    originalSize: [width, height]
                }
            });
        } catch (error) {
            console.error('[SAM2 Worker] Encode error:', error);
            self.postMessage({ type: 'result', id, success: false, error: error.message });
        }
    }
};
`;

  const blob = new Blob([workerCode], { type: "application/javascript" });
  const worker = new Worker(URL.createObjectURL(blob));

  worker.onmessage = function (e) {
    const { type, id, progress, success, embeddings, error } = e.data;

    switch (type) {
      case "progress":
        loadProgress = 0.05 + progress * 0.4;
        break;
      case "loaded":
        console.log("[SAM2] Encoder worker ready");
        isWarmedUp = true;
        break;
      case "error":
        console.error("[SAM2] Encoder worker error:", error);
        break;
      case "result":
        const pending = pendingEncoderRequests.get(id);
        if (pending) {
          pendingEncoderRequests.delete(id);
          pending.resolve(
            success ? { success: true, embeddings } : { success: false, error },
          );
        }
        break;
    }
  };

  worker.onerror = (e) => console.error("[SAM2] Worker error:", e);

  return worker;
}

// =============================================================================
// Model Loading
// =============================================================================

async function loadModels() {
  if (isLoading) {
    return { success: false, error: "Loading already in progress" };
  }

  if (encoderWorker && decoderSession && isWarmedUp) {
    return { success: true };
  }

  isLoading = true;
  loadProgress = 0;

  try {
    // Start encoder loading in worker
    encoderWorker = createEncoderWorker();
    loadProgress = 0.05;

    const encoderLoadPromise = new Promise((resolve, reject) => {
      const originalHandler = encoderWorker.onmessage;
      encoderWorker.onmessage = function (e) {
        originalHandler.call(encoderWorker, e);
        if (e.data.type === "loaded") resolve();
        else if (e.data.type === "error") reject(new Error(e.data.error));
      };
    });

    encoderWorker.postMessage({
      type: "load",
      data: { encoderUrl: CONFIG.encoderUrl },
    });

    // Load decoder on main thread in parallel
    await initOrt();
    loadProgress = 0.5;

    let decoderData = await getCachedModel(
      CONFIG.decoderStore,
      CONFIG.decoderUrl,
    );

    if (decoderData) {
      console.log("[SAM2] Using cached decoder");
      loadProgress = 0.85;
    } else {
      console.log("[SAM2] Downloading decoder...");
      const resp = await fetch(CONFIG.decoderUrl);
      if (!resp.ok) throw new Error(`Decoder fetch failed: ${resp.status}`);

      const total = parseInt(resp.headers.get("content-length") || "0");
      const reader = resp.body.getReader();
      const chunks = [];
      let received = 0;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        received += value.length;
        loadProgress = 0.5 + (received / total) * 0.35;
      }

      decoderData = new Uint8Array(received);
      let offset = 0;
      for (const chunk of chunks) {
        decoderData.set(chunk, offset);
        offset += chunk.length;
      }

      await cacheModel(CONFIG.decoderStore, CONFIG.decoderUrl, decoderData);
    }

    decoderSession = await createSession(decoderData, "Decoder");
    loadProgress = 0.9;

    // Warmup decoder
    console.log("[SAM2] Warming up decoder...");
    const t0 = performance.now();
    try {
      await decoderSession.run({
        image_embed: new ort.Tensor(
          "float32",
          new Float32Array(256 * 64 * 64),
          [1, 256, 64, 64],
        ),
        high_res_feats_0: new ort.Tensor(
          "float32",
          new Float32Array(32 * 256 * 256),
          [1, 32, 256, 256],
        ),
        high_res_feats_1: new ort.Tensor(
          "float32",
          new Float32Array(64 * 128 * 128),
          [1, 64, 128, 128],
        ),
        point_coords: new ort.Tensor(
          "float32",
          new Float32Array([512, 512, 0, 0]),
          [1, 2, 2],
        ),
        point_labels: new ort.Tensor(
          "float32",
          new Float32Array([1, -1]),
          [1, 2],
        ),
        mask_input: new ort.Tensor(
          "float32",
          new Float32Array(256 * 256),
          [1, 1, 256, 256],
        ),
        has_mask_input: new ort.Tensor("float32", new Float32Array([0]), [1]),
      });
      console.log(
        "[SAM2] Decoder warmup done in %dms",
        Math.round(performance.now() - t0),
      );
    } catch (e) {
      console.warn("[SAM2] Decoder warmup failed (non-critical):", e.message);
    }

    // Wait for encoder
    await encoderLoadPromise;

    loadProgress = 1.0;
    isLoading = false;
    return { success: true };
  } catch (error) {
    console.error("[SAM2] Load failed:", error);
    isLoading = false;
    encoderWorker = null;
    decoderSession = null;
    isWarmedUp = false;
    return { success: false, error: error.message };
  }
}

// =============================================================================
// Inference
// =============================================================================

async function runEncoder(imageData, width, height) {
  if (!encoderWorker || !isWarmedUp) {
    return { success: false, error: "Encoder not loaded" };
  }

  const id = ++encoderRequestId;
  console.log(
    "[SAM2] Encoding %dx%d image (request #%d)...",
    width,
    height,
    id,
  );

  return new Promise((resolve) => {
    pendingEncoderRequests.set(id, { resolve });

    const buffer = imageData.buffer.slice(
      imageData.byteOffset,
      imageData.byteOffset + imageData.byteLength,
    );
    encoderWorker.postMessage(
      { type: "encode", id, data: { imageData: buffer, width, height } },
      [buffer],
    );
  });
}

async function runDecoder(
  embeddings,
  positivePoints,
  negativePoints,
  boundingBox,
) {
  if (!decoderSession) {
    return { success: false, error: "Decoder not loaded" };
  }

  try {
    const [imgW, imgH] = embeddings.originalSize;
    console.log(
      "[SAM2] Decoding: %d positive, %d negative points",
      positivePoints.length,
      negativePoints.length,
    );
    const t0 = performance.now();

    // Prepare prompts
    const boxPoints = boundingBox ? 2 : 0;
    const actualPoints =
      positivePoints.length + negativePoints.length + boxPoints;
    const totalPoints = Math.max(actualPoints, 1) + 1; // +1 for padding

    const coords = new Float32Array(totalPoints * 2);
    const labels = new Float32Array(totalPoints);

    const longestSide = Math.max(imgW, imgH);
    const scale = CONFIG.encoderInputSize / longestSide;

    let idx = 0;

    // Positive points (label = 1)
    for (const [x, y] of positivePoints) {
      coords[idx * 2] = x * scale;
      coords[idx * 2 + 1] = y * scale;
      labels[idx++] = 1;
    }

    // Negative points (label = 0)
    for (const [x, y] of negativePoints) {
      coords[idx * 2] = x * scale;
      coords[idx * 2 + 1] = y * scale;
      labels[idx++] = 0;
    }

    // Bounding box (labels 2 and 3)
    if (boundingBox) {
      const [bx, by, bw, bh] = boundingBox;
      coords[idx * 2] = bx * scale;
      coords[idx * 2 + 1] = by * scale;
      labels[idx++] = 2;
      coords[idx * 2] = (bx + bw) * scale;
      coords[idx * 2 + 1] = (by + bh) * scale;
      labels[idx++] = 3;
    }

    // Padding if no points
    if (actualPoints === 0) {
      coords[0] = 512;
      coords[1] = 512;
      labels[0] = -1;
      idx = 1;
    }

    // Final padding point
    coords[idx * 2] = 0;
    coords[idx * 2 + 1] = 0;
    labels[idx] = -1;

    // Run inference
    const results = await decoderSession.run({
      image_embed: new ort.Tensor(
        "float32",
        new Float32Array(embeddings.imageEmbed.data),
        embeddings.imageEmbed.shape,
      ),
      high_res_feats_0: new ort.Tensor(
        "float32",
        new Float32Array(embeddings.highResFeats0.data),
        embeddings.highResFeats0.shape,
      ),
      high_res_feats_1: new ort.Tensor(
        "float32",
        new Float32Array(embeddings.highResFeats1.data),
        embeddings.highResFeats1.shape,
      ),
      point_coords: new ort.Tensor("float32", coords, [1, totalPoints, 2]),
      point_labels: new ort.Tensor("float32", labels, [1, totalPoints]),
      mask_input: new ort.Tensor(
        "float32",
        new Float32Array(256 * 256),
        [1, 1, 256, 256],
      ),
      has_mask_input: new ort.Tensor("float32", new Float32Array([0]), [1]),
    });

    console.log(
      "[SAM2] Decoder inference took %dms",
      Math.round(performance.now() - t0),
    );

    // Extract best mask
    const masks = results.masks || results[decoderSession.outputNames[0]];
    const iouPreds =
      results.iou_predictions || results[decoderSession.outputNames[1]];

    const iouData = Array.from(iouPreds.data);
    let bestIdx = 0;
    for (let i = 1; i < iouData.length; i++) {
      if (iouData[i] > iouData[bestIdx]) bestIdx = i;
    }

    // Resize mask to original image size
    const [maskH, maskW] = [masks.dims[2], masks.dims[3]];
    const masksData = masks.data;
    const scaledW = Math.round(imgW * scale);
    const scaledH = Math.round(imgH * scale);
    const maskScale = maskW / CONFIG.encoderInputSize;
    const validW = Math.round(scaledW * maskScale);
    const validH = Math.round(scaledH * maskScale);

    const output = new Uint8Array(imgW * imgH);
    const scaleX = Math.max(0, validW - 1) / Math.max(1, imgW - 1);
    const scaleY = Math.max(0, validH - 1) / Math.max(1, imgH - 1);
    const maxX = Math.min(validW - 1, maskW - 1);
    const maxY = Math.min(validH - 1, maskH - 1);
    const maskOffset = bestIdx * maskH * maskW;

    // Bilinear interpolation
    for (let y = 0; y < imgH; y++) {
      for (let x = 0; x < imgW; x++) {
        const srcX = x * scaleX;
        const srcY = y * scaleY;

        const x0 = Math.min(Math.floor(srcX), maxX);
        const y0 = Math.min(Math.floor(srcY), maxY);
        const x1 = Math.min(x0 + 1, maxX);
        const y1 = Math.min(y0 + 1, maxY);

        const fx = srcX - x0;
        const fy = srcY - y0;

        const v00 = masksData[maskOffset + y0 * maskW + x0];
        const v01 = masksData[maskOffset + y0 * maskW + x1];
        const v10 = masksData[maskOffset + y1 * maskW + x0];
        const v11 = masksData[maskOffset + y1 * maskW + x1];

        const val =
          v00 * (1 - fx) * (1 - fy) +
          v01 * fx * (1 - fy) +
          v10 * (1 - fx) * fy +
          v11 * fx * fy;

        output[y * imgW + x] = val > 0 ? 255 : 0;
      }
    }

    return {
      success: true,
      mask: {
        data: Array.from(output),
        width: imgW,
        height: imgH,
        score: iouData[bestIdx],
      },
    };
  } catch (error) {
    console.error("[SAM2] Decoder failed:", error);
    return { success: false, error: error.message };
  }
}

// =============================================================================
// Public API
// =============================================================================

window.sam2 = {
  loadModels,
  getLoadProgress: () => loadProgress,
  isReady: () =>
    encoderWorker !== null && decoderSession !== null && isWarmedUp,
  runEncoder,
  runDecoder,
};

console.log("[SAM2] Module loaded");
