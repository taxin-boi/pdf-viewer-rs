# PDF Viewer RS

A high-performance, lightweight PDF reader built in Rust with MuPDF and egui.

## Features

- **Blazing Fast Rendering**: Powered by MuPDF for instant page loads
- **Low Memory Footprint**: Efficient caching and texture management
- **Modern UI**: Clean dark/light themes with intuitive controls
- **Navigation**: Page jumping, continuous scroll, zoom in/out
- **Outline/Bookmarks**: Sidebar navigation for structured documents
- **Search**: Full-text search across all pages
- **Recent Files**: Quick access to previously opened documents
- **Cross-Platform**: Runs on Linux, macOS, and Windows

## Requirements

- Rust 1.70+
- `pkg-config`
- `libmupdf-dev` (or equivalent MuPDF development libraries)

## Build & Run

```bash
cargo run --release
```

## Controls

| Action | Shortcut / Control |
|--------|-------------------|
| Open File | 📂 Button |
| Next/Prev Page | ◀ ▶ Buttons |
| Zoom | +/- Buttons or DragValue |
| Toggle Sidebar | ☰ Outline |
| Dark Mode | 🌙 Dark |
| Search | Type query + Search |

## Architecture

- **MuPDF**: Industry-standard PDF rendering engine
- **egui**: Immediate mode GUI for low latency and high performance
- **LruCache**: Smart texture caching to avoid re-rendering pages
- **Async-ready**: Designed for future async loading and background rendering

## License

MIT
