use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

/// 输入捕获器，封装 rdev::listen
pub struct InputCapturer {
    _private: (),
}

impl InputCapturer {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// 启动全局输入监听，将事件发送到 channel
    pub fn start(&self, tx: mpsc::UnboundedSender<rdev::Event>) -> Result<()> {
        info!("starting input capture");

        std::thread::spawn(move || {
            rdev::listen(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to listen for input events");
        });

        Ok(())
    }
}
