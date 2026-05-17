use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::document::PdfDocument;

pub struct PdfApp {
    config: AppConfig,
    doc: Option<Arc<Mutex<PdfDocument>>>,
    current_page: usize,
    zoom: f32,
    continuous_scroll: bool,
    show_sidebar: bool,
    show_thumbnails: bool,
    dark_mode: bool,
    search_query: String,
    search_results: Vec<(usize, String)>,
    cache: lru::LruCache<usize, TextureHandle>,
    outline: Vec<crate::document::OutlineItem>,
}

impl PdfApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        Self {
            config,
            doc: None,
            current_page: 0,
            zoom: 1.0,
            continuous_scroll: true,
            show_sidebar: true,
            show_thumbnails: false,
            dark_mode: true,
            search_query: String::new(),
            search_results: Vec::new(),
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(50).unwrap()),
            outline: Vec::new(),
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        if let Ok(doc) = PdfDocument::open(&path) {
            self.config.add_recent(path.clone());
            self.doc = Some(Arc::new(Mutex::new(doc)));
            self.current_page = 0;
            self.zoom = 1.0;
            self.cache.clear();
            if let Ok(doc) = self.doc.as_ref().unwrap().lock() {
                self.outline = doc.get_outline().unwrap_or_default();
            }
        }
    }

    fn render_page_to_texture(&mut self, ctx: &egui::Context, page: usize) -> Option<TextureHandle> {
        if let Some(doc) = &self.doc {
            let doc = doc.lock().unwrap();
            if let Ok(img) = doc.render_page(page, self.zoom) {
                let size = [img.width() as usize, img.height() as usize];
                let pixels = img.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                let handle = ctx.load_texture(
                    format!("page_{}", page),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                return Some(handle);
            }
        }
        None
    }
}

impl eframe::App for PdfApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.open_file(path);
                    }
                }

                if let Some(doc) = &self.doc {
                    let page_count = doc.lock().unwrap().page_count;
                    ui.separator();
                    if ui.button("◀").clicked() && self.current_page > 0 {
                        self.current_page -= 1;
                    }
                    
                    ui.add(egui::DragValue::new(&mut self.current_page)
                        .range(0..=page_count.saturating_sub(1))
                        .speed(0.5)
                        .prefix("Page ")
                        .suffix(&format!(" / {}", page_count)));
                    
                    if ui.button("▶").clicked() && self.current_page < page_count - 1 {
                        self.current_page += 1;
                    }

                    ui.separator();
                    if ui.button("-").clicked() { self.zoom = (self.zoom - 0.1).max(0.2); }
                    ui.add(egui::DragValue::new(&mut self.zoom).range(0.2..=3.0).speed(0.05).suffix("x"));
                    if ui.button("+").clicked() { self.zoom = (self.zoom + 0.1).min(3.0); }
                    if ui.button("Fit Width").clicked() { self.zoom = 1.0; }

                    ui.separator();
                    ui.toggle_value(&mut self.show_sidebar, "☰ Outline");
                    ui.toggle_value(&mut self.show_thumbnails, "🖼 Thumbs");
                    ui.toggle_value(&mut self.dark_mode, "🌙 Dark");

                    ui.separator();
                    ui.text_edit_singleline(&mut self.search_query);
                    if ui.button("Search").clicked() {
                        self.search_results.clear();
                        if let Ok(doc) = doc.lock() {
                            for p in 0..page_count {
                                if let Ok(text) = doc.get_text(p) {
                                    if text.to_lowercase().contains(&self.search_query.to_lowercase()) {
                                        self.search_results.push((p, "Found".to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });

        if self.doc.is_some() {
            let doc = self.doc.as_ref().unwrap();
            let page_count = doc.lock().unwrap().page_count;

            egui::SidePanel::left("sidebar").show_animated(ctx, self.show_sidebar, |ui| {
                ui.heading("Outline");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for item in &self.outline {
                        let indent = item.depth as f32 * 10.0;
                        ui.add_space(indent);
                        if ui.button(&item.title).clicked() {
                            self.current_page = item.page.min(page_count - 1);
                        }
                    }
                });
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(texture) = self.render_page_to_texture(ctx, self.current_page) {
                    ui.centered_and_justified(|ui| {
                        ui.image(&texture);
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Rendering...");
                    });
                }
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("PDF Viewer");
                    ui.label("No document open. Click 'Open' to select a PDF.");
                    if !self.config.recent_files.is_empty() {
                        ui.label("Recent files:");
                        for recent in &self.config.recent_files {
                            if ui.button(recent.to_string_lossy()).clicked() {
                                self.open_file(recent.clone());
                            }
                        }
                    }
                });
            });
        }
    }
}
