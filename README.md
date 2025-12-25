# hvat

> **Note:** This is a working title. The repository may be renamed in the future.

**Hyperspectral Visualization and Annotation Tool** — A GPU-accelerated application for viewing and annotating hyperspectral and RGB images. Runs in both native and WASM environments.

## Why another annotation tool?

I built this out of frustration with existing tools that either only work with RGB images, don't match my workflow, or are so outdated that they lack GPU acceleration and multithreading support.

The direction and scope are still evolving, but it currently serves my use cases well. The focus will likely remain on hyperspectral images and CV-assisted annotation — both classical algorithms and modern AI-based approaches. I'll continue extending it as needed.

## Building

```bash
# Native build
cargo build --release
cargo run --release

# WASM build (requires trunk)
trunk build --release
trunk serve --release  # Open the app under localhost:8080
```

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
- Tool selection UI for annotations
- Basic label category management
- Basic per-image tagging
- Undo/redo system

</details>

<details>
<summary>In Progress</summary>

- Annotation system
    - [x] Bounding box drawing
    - [x] Polygon drawing
    - [x] Point annotation
    - [x] Annotation overlay rendering
    - [ ] Selection/editing of annotations
    - [ ] Store annotations
    - [ ] Export annotations

</details>

<details>
<summary>Roadmap</summary>

- Drag-and-drop file loading
- Histogram display and auto-stretch
- Basic CV algorithms for aided annotation
    - e.g. watershed, edge detection etc
- Spectral signature plot
- ENVI format (.hdr/.raw)
- Extend Annotation system 
    - Export formats (COCO, YOLO, Pascal VOC)
- SAM2 tiny integration
- Image normalizations options: 
    - total brightness
    - percentiles 3-97%
    - channel based 

</details>


<details>
<summary>Spaceprogram (Maybe in the future features)</summary>

- Wasm plugin modules (Maybe community made plugins?)
- Tiling/chunk loading
    - Dynamic resolution
    - Data formats
- Stitching
    - Line scan
    - Pictures
- Machine learning Integration / Python
- Data Visualization
- Machine learning training visualization (e.g. like tensorboard)
    - e.g. show poorest performance picture
- Video Annotation
- 3D Annotation

</details>

<details>
<summary>Known Limitations</summary>

- **Wasm gpu rendering** — There will sometimes be lag spikes when preloading many images at once, but it is worth.
- **Native drag-and-drop not supported on Wayland** — Winit doesn't support drag-and-drop on Wayland. I use Wayland so i couldn't test it properly. 
- **Only tested on Linux** — Windows and macOS support not verified.

</details>

<details>
<summary>Known Bugs (Yes, bugs are a subset of features)</summary>

</details>