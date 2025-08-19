use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipboardContent {
    pub text: Option<String>,
    pub timestamp: u64,
}

pub struct ClipboardManager {
    clipboard: Arc<Mutex<Clipboard>>,
    last_content: Arc<Mutex<Option<String>>>,
}

impl ClipboardManager {
    pub fn new() -> Result<Self, String> {
        match Clipboard::new() {
            Ok(clipboard) => Ok(Self {
                clipboard: Arc::new(Mutex::new(clipboard)),
                last_content: Arc::new(Mutex::new(None)),
            }),
            Err(e) => Err(format!("Failed to initialize clipboard: {}", e)),
        }
    }

    pub fn get_text(&self) -> Result<String, String> {
        match self.clipboard.lock() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => Ok(text),
                Err(e) => Err(format!("Failed to get text from clipboard: {}", e)),
            },
            Err(e) => Err(format!("Failed to lock clipboard: {}", e)),
        }
    }

    pub fn set_text(&self, text: &str) -> Result<(), String> {
        match self.clipboard.lock() {
            Ok(mut clipboard) => match clipboard.set_text(text.to_string()) {
                Ok(_) => {
                    if let Ok(mut last_content) = self.last_content.lock() {
                        *last_content = Some(text.to_string());
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Failed to set text to clipboard: {}", e)),
            },
            Err(e) => Err(format!("Failed to lock clipboard: {}", e)),
        }
    }

    pub fn check_for_changes(&self) -> Result<Option<ClipboardContent>, String> {
        let current_text = self.get_text()?;
        
        let changed = match self.last_content.lock() {
            Ok(mut last_content) => {
                if last_content.as_ref() != Some(&current_text) {
                    *last_content = Some(current_text.clone());
                    true
                } else {
                    false
                }
            },
            Err(e) => return Err(format!("Failed to lock last_content: {}", e)),
        };

        if changed {
            Ok(Some(ClipboardContent {
                text: Some(current_text),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }))
        } else {
            Ok(None)
        }
    }
}