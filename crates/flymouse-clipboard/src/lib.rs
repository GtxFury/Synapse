use anyhow::Result;
use arboard::Clipboard;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// 剪贴板变更事件
#[derive(Debug, Clone)]
pub enum ClipboardContent {
    Text(String),
    Image { width: usize, height: usize, data: Vec<u8> },
}

/// 剪贴板监控器，通过轮询检测变更
pub struct ClipboardWatcher {
    poll_interval: Duration,
}

impl ClipboardWatcher {
    pub fn new(poll_interval: Duration) -> Self {
        Self { poll_interval }
    }

    /// 启动剪贴板监控，变更时发送到 channel
    pub async fn watch(&self, tx: mpsc::UnboundedSender<ClipboardContent>) -> Result<()> {
        let interval = self.poll_interval;

        tokio::task::spawn_blocking(move || {
            let mut clipboard = Clipboard::new().expect("failed to access clipboard");
            let mut last_text = String::new();

            loop {
                match clipboard.get_text() {
                    Ok(text) if text != last_text && !text.is_empty() => {
                        debug!(len = text.len(), "clipboard text changed");
                        last_text = text.clone();
                        let _ = tx.send(ClipboardContent::Text(text));
                    }
                    Err(e) => {
                        warn!("clipboard read error: {}", e);
                    }
                    _ => {}
                }
                std::thread::sleep(interval);
            }
        });

        Ok(())
    }

    /// 设置剪贴板文本
    pub fn set_text(text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    }
}
