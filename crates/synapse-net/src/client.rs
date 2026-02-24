use anyhow::Result;
use futures::{SinkExt, StreamExt};
use synapse_protocol::screen::{ScreenId, ScreenInfo, ScreenRect};
use synapse_protocol::{DeviceId, Message, MessageCodec};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::ClientEvent;

/// TCP 客户端
pub struct Client {
    addr: String,
}

impl Client {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// 连接到服务端，进入完整消息循环
    pub async fn connect(
        &self,
        device_id: String,
        device_name: String,
        screen_size: (u32, u32),
        message_tx: mpsc::UnboundedSender<Message>,
        event_tx: mpsc::UnboundedSender<ClientEvent>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let _ = event_tx.send(ClientEvent::Log(format!(
            "Connecting to {}...", self.addr
        )));

        let stream = TcpStream::connect(&self.addr).await?;
        info!(addr = %self.addr, "connected to server");

        let mut framed = Framed::new(stream, MessageCodec);

        // 发送 Hello 握手（携带屏幕信息）
        framed.send(Message::Hello {
            device_id: DeviceId(device_id.clone()),
            device_name: device_name.clone(),
            screens: vec![ScreenInfo {
                id: ScreenId(0),
                name: "primary".into(),
                rect: ScreenRect {
                    x: 0, y: 0,
                    width: screen_size.0,
                    height: screen_size.1,
                },
                is_primary: true,
            }],
        }).await?;

        // 等待 Welcome
        let welcome = loop {
            let msg = tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                result = framed.next() => match result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => return Err(e.into()),
                    None => return Err(anyhow::anyhow!("connection closed before Welcome")),
                },
            };
            match msg {
                Message::Welcome { device_id, device_name, .. } => {
                    break (device_id.0, device_name);
                }
                _ => {
                    warn!("expected Welcome, got {:?}", msg);
                }
            }
        };

        info!(server_id = %welcome.0, server_name = %welcome.1, "handshake complete");
        let _ = event_tx.send(ClientEvent::Connected {
            server_device_id: welcome.0,
            server_device_name: welcome.1,
        });
        let _ = event_tx.send(ClientEvent::Log("Connected to server".into()));

        // 消息接收循环
        loop {
            let msg = tokio::select! {
                _ = cancel.cancelled() => {
                    // 发送 Bye
                    let _ = framed.send(Message::Bye {
                        device_id: DeviceId(device_id.clone()),
                    }).await;
                    break;
                }
                result = framed.next() => match result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        error!("receive error: {e}");
                        break;
                    }
                    None => {
                        info!("server closed connection");
                        break;
                    }
                },
            };

            match &msg {
                Message::Ping(seq) => {
                    let _ = framed.send(Message::Pong(*seq)).await;
                }
                Message::Pong(_) => {}
                _ => {
                    // 转发给上层处理（输入模拟、剪贴板等）
                    let _ = message_tx.send(msg);
                }
            }
        }

        let _ = event_tx.send(ClientEvent::Disconnected);
        let _ = event_tx.send(ClientEvent::Log("Disconnected from server".into()));
        Ok(())
    }
}
