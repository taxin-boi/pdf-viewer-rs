use eframe::egui;
use egui::{ColorImage, TextureHandle};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::config::AppConfig;
use crate::document::PdfDocument;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    None,
    Pen,
    Highlight,
    Eraser,
}

#[derive(Clone, Debug)]
pub enum StrokeType {
    Pen(egui::Color32),
    Highlight(egui::Color32),
}

#[derive(Clone, Debug)]
pub struct Stroke {
    pub points: Vec<egui::Pos2>,
    pub stroke_type: StrokeType,
    pub width: f32,
}

pub struct PdfApp {
    config: AppConfig,
    doc: Option<Arc<Mutex<PdfDocument>>>,
    current_page: usize,
    zoom: f32,
    show_sidebar: bool,
    dark_mode: bool,
    cache: lru::LruCache<usize, TextureHandle>,
    outline: Vec<crate::document::OutlineItem>,
    
    pub tool: Tool,
    pub pen_color: egui::Color32,
    pub highlight_color: egui::Color32,
    pub stroke_width: f32,
    pub annotations: HashMap<usize, Vec<Stroke>>,
    pub current_stroke: Option<Stroke>,
    pub is_drawing: bool,
}

impl PdfApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        Self {
            config,
            doc: None,
            current_page: 0,
            zoom: 1.0,
            show_sidebar: true,
            dark_mode: true,
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(50).unwrap()),
            outline: Vec::new(),
            tool: Tool::None,
            pen_color: egui::Color32::BLACK,
            highlight_color: egui::Color32::from_rgba_unmultiplied(255, 255, 0, 80),
            stroke_width: 2.0,
            annotations: HashMap::new(),
            current_stroke: None,
            is_drawing: false,
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        if let Ok(doc) = PdfDocument::open(&path) {
            self.config.add_recent(path.clone());
            self.doc = Some(Arc::new(Mutex::new(doc)));
            self.current_page = 0;
            self.zoom = 1.0;
            self.cache.clear();
            self.annotations.clear();
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

    fn clear_page_annotations(&mut self) {
        self.annotations.remove(&self.current_page);
    }

    fn erase_at_point(&mut self, pos: egui::Pos2, radius: f32) {
        if let Some(strokes) = self.annotations.get_mut(&self.current_page) {
            strokes.retain(|stroke| {
                stroke.points.iter().all(|p| {
                    let dx = p.x - pos.x;
                    let dy = p.y - pos.y;
                    (dx * dx + dy * dy) > radius * radius
                })
            });
        }
    }

    fn draw_annotations(&self, ui: &mut egui::Ui, page: usize, rect: egui::Rect) {
        if let Some(strokes) = self.annotations.get(&page) {
            for stroke in strokes {
                match &stroke.stroke_type {
                    StrokeType::Highlight(color) => {
                        if stroke.points.len() > 1 {
                            let screen_points: Vec<egui::Pos2> = stroke.points.iter().map(|p| {
                                egui::Pos2::new(p.x + rect.min.x, p.y + rect.min.y)
                            }).collect();
                            ui.painter().add(egui::Shape::line(
                                screen_points,
                                egui::Stroke::new(stroke.width, *color),
                            ));
                        }
                    }
                    StrokeType::Pen(color) => {
                        if stroke.points.len() > 1 {
                            let screen_points: Vec<egui::Pos2> = stroke.points.iter().map(|p| {
                                egui::Pos2::new(p.x + rect.min.x, p.y + rect.min.y)
                            }).collect();
                            ui.painter().add(egui::Shape::line(
                                screen_points,
                                egui::Stroke::new(stroke.width, *color),
                            ));
                        }
                    }
                }
            }
        }
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

                    ui.separator();
                    ui.toggle_value(&mut self.show_sidebar, "☰ Outline");
                    ui.toggle_value(&mut self.dark_mode, "🌙 Dark");

                    ui.separator();
                    
                    ui.label("Tool:");
                    if ui.selectable_label(self.tool == Tool::None, "🖱 View").clicked() {
                        self.tool = Tool::None;
                    }
                    if ui.selectable_label(self.tool == Tool::Pen, "✏ Pen").clicked() {
                        self.tool = Tool::Pen;
                    }
                    if ui.selectable_label(self.tool == Tool::Highlight, "🖍 Highlight").clicked() {
                        self.tool = Tool::Highlight;
                    }
                    if ui.selectable_label(self.tool == Tool::Eraser, "🧹 Eraser").clicked() {
                        self.tool = Tool::Eraser;
                    }

                    if !matches!(self.tool, Tool::None) {
                        ui.separator();
                        ui.label("Width:");
                        ui.add(egui::Slider::new(&mut self.stroke_width, 1.0..=30.0));

                        match self.tool {
                            Tool::Pen => {
                                ui.label("Color:");
                                ui.color_edit_button_srgba(&mut self.pen_color);
                            }
                            Tool::Highlight => {
                                ui.label("Color:");
                                ui.color_edit_button_srgba(&mut self.highlight_color);
                            }
                            Tool::Eraser => {}
                            Tool::None => {}
                        }

                        ui.separator();
                        if ui.button("🗑 Clear").clicked() {
                            self.clear_page_annotations();
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
                egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                    if let Some(texture) = self.render_page_to_texture(ctx, self.current_page) {
                        let image_size = texture.size_vec2();
                        
                        ui.horizontal_centered(|ui| {
                            let (rect, _response) = ui.allocate_exact_size(image_size, egui::Sense::click_and_drag());
                            ui.painter().image(texture.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);

                            self.draw_annotations(ui, self.current_page, rect);

                            if !matches!(self.tool, Tool::None) {
                                let pointer = ui.input(|i| i.pointer.interact_pos());
                                let is_pressed = ui.input(|i| i.pointer.primary_down());

                                if let Some(pos) = pointer {
                                    if rect.contains(pos) {
                                        let local_pos = egui::Pos2::new(pos.x - rect.min.x, pos.y - rect.min.y);

                                        if is_pressed {
                                            if !self.is_drawing {
                                                self.is_drawing = true;
                                                let stroke_type = match self.tool {
                                                    Tool::Pen => StrokeType::Pen(self.pen_color),
                                                    Tool::Highlight => StrokeType::Highlight(self.highlight_color),
                                                    Tool::Eraser => StrokeType::Pen(self.pen_color),
                                                    Tool::None => StrokeType::Pen(self.pen_color),
                                                };
                                                self.current_stroke = Some(Stroke {
                                                    points: vec![local_pos],
                                                    stroke_type,
                                                    width: self.stroke_width,
                                                });
                                            } else if let Some(ref mut stroke) = self.current_stroke {
                                                stroke.points.push(local_pos);
                                            }
                                            
                                            if self.tool == Tool::Eraser {
                                                self.erase_at_point(local_pos, self.stroke_width);
                                            }
                                        } else {
                                            if self.is_drawing {
                                                if let Some(stroke) = self.current_stroke.take() {
                                                    if !matches!(self.tool, Tool::Eraser) {
                                                        self.annotations.entry(self.current_page).or_insert_with(Vec::new).push(stroke);
                                                    }
                                                }
                                                self.is_drawing = false;
                                            }
                                        }
                                    }
                                }

                                if let Some(ref stroke) = self.current_stroke {
                                    match &stroke.stroke_type {
                                        StrokeType::Highlight(color) => {
                                            if stroke.points.len() > 1 {
                                                let screen_points: Vec<egui::Pos2> = stroke.points.iter().map(|p| {
                                                    egui::Pos2::new(p.x + rect.min.x, p.y + rect.min.y)
                                                }).collect();
                                                ui.painter().add(egui::Shape::line(
                                                    screen_points,
                                                    egui::Stroke::new(stroke.width, *color),
                                                ));
                                            }
                                        }
                                        StrokeType::Pen(color) => {
                                            if stroke.points.len() > 1 {
                                                let screen_points: Vec<egui::Pos2> = stroke.points.iter().map(|p| {
                                                    egui::Pos2::new(p.x + rect.min.x, p.y + rect.min.y)
                                                }).collect();
                                                ui.painter().add(egui::Shape::line(
                                                    screen_points,
                                                    egui::Stroke::new(stroke.width, *color),
                                                ));
                                            }
                                        }
                                    }
                                }

                                if self.tool == Tool::Eraser {
                                    if let Some(pos) = pointer {
                                        ui.painter().circle_stroke(pos, self.stroke_width, egui::Stroke::new(1.0, egui::Color32::RED));
                                    }
                                }
                            }
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Rendering...");
                        });
                    }
                });
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("PDF Viewer");
                    ui.label("No document open. Click 'Open' to select a PDF.");
                    if !self.config.recent_files.is_empty() {
                        ui.label("Recent files:");
                        let mut clicked_path = None;
                        for recent in &self.config.recent_files {
                            if ui.button(recent.to_string_lossy()).clicked() {
                                clicked_path = Some(recent.clone());
                            }
                        }
                        if let Some(path) = clicked_path {
                            self.open_file(path);
                        }
                    }
                });
            });
        }
    }
}
