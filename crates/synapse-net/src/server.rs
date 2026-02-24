use anyhow::Result;
use synapse_protocol::{Message, MessageCodec};
use futures::SinkExt;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use tracing::info;

/// TCP 服务端
pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// 启动服务端，监听连接
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!(addr = %self.addr, "server listening");

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            info!(%peer_addr, "new connection");

            tokio::spawn(async move {
                let mut framed = Framed::new(stream, MessageCodec);

                // 骨架：发送 Ping 作为示例
                if let Err(e) = framed.send(Message::Ping(0)).await {
                    tracing::error!(%peer_addr, "send error: {}", e);
                }

                // TODO: 实现完整的消息处理循环
            });
        }
    }
}
