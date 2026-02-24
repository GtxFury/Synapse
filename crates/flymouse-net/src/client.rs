use anyhow::Result;
use flymouse_protocol::MessageCodec;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::info;

/// TCP 客户端
pub struct Client {
    addr: String,
}

impl Client {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// 连接到服务端
    pub async fn connect(&self) -> Result<()> {
        let stream = TcpStream::connect(&self.addr).await?;
        info!(addr = %self.addr, "connected to server");

        let mut framed = Framed::new(stream, MessageCodec);

        // 骨架：读取消息
        while let Some(result) = framed.next().await {
            match result {
                Ok(msg) => {
                    info!(?msg, "received message");
                    // TODO: 实现完整的消息处理
                }
                Err(e) => {
                    tracing::error!("receive error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
