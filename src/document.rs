use mupdf::document::Document;
use mupdf::page::Page;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use image::RgbaImage;

pub struct PdfDocument {
    doc: Arc<Mutex<Document>>,
    pub page_count: usize,
    pub path: String,
}

impl PdfDocument {
    pub fn open(path: &Path) -> Result<Self, String> {
        let doc = Document::open(path).map_err(|e| e.to_string())?;
        let page_count = doc.page_count().map_err(|e| e.to_string())?;
        Ok(Self {
            doc: Arc::new(Mutex::new(doc)),
            page_count,
            path: path.to_string_lossy().to_string(),
        })
    }

    pub fn render_page(&self, page_num: usize, zoom: f32) -> Result<RgbaImage, String> {
        let doc = self.doc.lock().map_err(|e| e.to_string())?;
        let mut page = doc.load_page(page_num as i32).map_err(|e| e.to_string())?;
        
        // Calculate dimensions based on zoom
        let bounds = page.bounds().map_err(|e| e.to_string())?;
        let width = ((bounds.x1 - bounds.x0) * zoom) as u32;
        let height = ((bounds.y1 - bounds.y0) * zoom) as u32;
        
        let matrix = mupdf::Matrix::new_scale(zoom, zoom);
        let pixmap = page.to_pixmap(&matrix, &mupdf::Colorspace::device_rgb(), false, true)
            .map_err(|e| e.to_string())?;
        
        let (w, h) = (pixmap.width(), pixmap.height());
        let samples = pixmap.samples();
        
        let mut img = RgbaImage::new(w as u32, h as u32);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let idx = ((y as usize * w as usize + x as usize) * 3) as usize;
            if idx + 2 < samples.len() {
                *pixel = image::Rgba([samples[idx], samples[idx+1], samples[idx+2], 255]);
            }
        }
        
        Ok(img)
    }

    pub fn get_text(&self, page_num: usize) -> Result<String, String> {
        let doc = self.doc.lock().map_err(|e| e.to_string())?;
        let page = doc.load_page(page_num as i32).map_err(|e| e.to_string())?;
        page.to_text().map_err(|e| e.to_string())
    }

    pub fn get_outline(&self) -> Result<Vec<OutlineItem>, String> {
        let doc = self.doc.lock().map_err(|e| e.to_string())?;
        let outline = doc.outline().map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        self.flatten_outline(&outline, &mut items, 0);
        Ok(items)
    }

    fn flatten_outline(&self, outline: &Vec<mupdf::outline::Outline>, items: &mut Vec<OutlineItem>, depth: usize) {
        for item in outline {
            items.push(OutlineItem {
                title: item.title.clone(),
                page: item.page.unwrap_or(0),
                depth,
            });
            if let Some(ref children) = item.down {
                self.flatten_outline(children, items, depth + 1);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct OutlineItem {
    pub title: String,
    pub page: usize,
    pub depth: usize,
}
