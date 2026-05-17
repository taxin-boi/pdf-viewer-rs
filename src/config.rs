use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use directories::ProjectDirs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub recent_files: Vec<PathBuf>,
    pub theme: String,
    pub zoom_level: f32,
    pub continuous_scroll: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            theme: "dark".to_string(),
            zoom_level: 1.0,
            continuous_scroll: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("rs", "pdfviewer", "pdf-viewer-rs") {
            let config_path = proj_dirs.config_dir().join("config.json");
            if config_path.exists() {
                if let Ok(data) = fs::read_to_string(config_path) {
                    if let Ok(config) = serde_json::from_str(&data) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(proj_dirs) = ProjectDirs::from("rs", "pdfviewer", "pdf-viewer-rs") {
            let config_path = proj_dirs.config_dir().join("config.json");
            if let Some(parent) = config_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(data) = serde_json::to_string_pretty(self) {
                let _ = fs::write(config_path, data);
            }
        }
    }

    pub fn add_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        if self.recent_files.len() > 10 {
            self.recent_files.truncate(10);
        }
        self.save();
    }
}
