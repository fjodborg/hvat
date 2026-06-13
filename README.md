# hvat (Deprecated) 

> **This project is no longer actively maintained.**
> See the successor project below before building anything here.

---

## Superseded

This version of HVAT is deprecated for two reasons:

1. **Fully local annotation in the browser did not gain enough traction** to justify the architectural complexity it imposed.
2. **SAM2 runs too slowly over WebGL** to be practical for real annotation workflows.

It has been replaced by a classical frontend and backend architecture with transparent communication on the frontend side and open, swappable backends. Please go to the new project instead:

**New project (in active development): [<placeholder url>](<placeholder url>)**

---

**Hyperspectral Visualization and Annotation Tool** — A GPU-accelerated application for viewing and annotating hyperspectral and RGB images. Runs in both native and WASM environments.

## Demo

[Live demo](https://fjodborg.github.io/hvat/) (deprecated, kept for reference)

## Why another annotation tool?

I built this out of frustration with existing tools that either only work with RGB images, don't match my workflow, or are so outdated that they lack GPU acceleration and multithreading support.

## Building

```bash
# Always format first
cargo fmt --all

# Native build
cargo build --release
cargo run --release

# WASM build (requires trunk)
trunk build --release
trunk serve --release  # Opens browser at localhost:8080
```

## Building with SAM2 support

SAM2 is an optional feature gated behind `--features sam2`. Both native and
WASM builds require the flag — without it the SAM2 UI and Rust inference code
are compiled out entirely (even though `sam2-onnx.js` is always bundled on WASM).

### Native

1. Format and build with the feature flag:

   ```bash
   cargo fmt --all
   cargo build --features sam2
   ```

2. Download the SAM2 tiny ONNX models and place them in `models/sam2/` relative
   to the project root:

   ```
   models/
   └── sam2/
       ├── sam2_hiera_tiny.encoder.onnx
       └── sam2_hiera_tiny.decoder.onnx
   ```

   You can export these from [Meta's SAM2 repository](https://github.com/facebookresearch/segment-anything-2)
   or download a pre-exported version from Hugging Face (search for `sam2-tiny onnx`).
   The models are licensed under the **Apache 2.0** license by Meta Platforms, Inc.

   ONNX Runtime binaries are downloaded automatically by the `ort` crate on the
   first build. This can take a minute depending on your connection.

3. Run:

   ```bash
   cargo run --features sam2
   ```

   Enable SAM2 in the sidebar under **Annotation Tools > AI Segmentation > Enable SAM2**.

### WASM

The WASM build also requires `--features sam2`. The easiest way is to add
`data-cargo-features="sam2"` to the main rust link tag in `index.html`:

```html
<link data-trunk rel="rust" data-target-name="hvat" data-keep-debug="true" data-wasm-opt="0" data-cargo-features="sam2" />
```

Then build and serve normally:

```bash
cargo fmt --all
trunk build
trunk serve
```

Enable SAM2 in the sidebar under **Annotation Tools > AI Segmentation > Enable SAM2**.
The models are downloaded automatically on first use from Hugging Face
([g-ronimo/sam2-tiny](https://huggingface.co/g-ronimo/sam2-tiny)) and cached in
IndexedDB so subsequent loads are instant. An internet connection is required for
the first load.

> **Note:** SAM2 inference over WebGL is noticeably slow on large images.
> This is one of the primary reasons this project has been deprecated in favour
> of the successor project with a proper backend.

---

## Features

<details>
<summary>Completed</summary>

- Image viewer with pan/zoom and keyboard shortcuts
- GPU-accelerated hyperspectral band rendering
    - RGB band selection sliders
    - Image enhancements (brightness, contrast, gamma, hue)
- Folder browsing with image discovery
    - Standard image formats (PNG, JPEG, etc.)
    - NumPy .npy hyperspectral files
- Drag-and-drop folder and ZIP loading (native and WASM)
- Tool selection UI for annotations
- Annotation system: bounding box, polygon, point
- Annotation editing (resize, move, vertex insertion/removal)
- Label category management with colour swatches
- Per-image tagging
- Undo/redo system (50 levels)
- Customisable hotkeys
- GPU preloading of adjacent images
- PWA support (offline, installable)
- File explorer panel
- Right-click context menu
- Configuration persistence (native: ~/.config/hvat, WASM: localStorage)
- SAM2 AI-assisted segmentation (optional, see above)

</details>

<details>
<summary>Known Limitations</summary>

- **SAM2 WebGL performance** — Inference is slow. Use the native build or the successor project for production use.
- **Native drag-and-drop not supported on Wayland** — Winit doesn't support drag-and-drop on Wayland. Use `env -u WAYLAND_DISPLAY cargo run` to fall back to XWayland, or use the Open Folder button.
- **Only tested on Linux** — Windows and macOS support not verified.
- **TIFF preloading blocks on WASM** — The browser's `createImageBitmap()` does not support TIFF. Use PNG, JPEG, or WebP for smooth WASM performance.

</details>

---

## License

MIT — see [LICENSE](LICENSE).

The SAM2 model weights (downloaded at runtime when SAM2 is enabled) are
licensed under the **Apache 2.0** license by Meta Platforms, Inc.
See [LICENSE](LICENSE) for full third-party license notices.
