use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use synapse_protocol::{Message, MessageCodec};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio_util::codec::Framed;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::ServerEvent;

type PeerMap = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>;

/// TCP 服务端
pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// 启动服务端完整消息循环
    pub async fn run(
        &self,
        input_rx: mpsc::UnboundedReceiver<Message>,
        clipboard_rx: mpsc::UnboundedReceiver<Message>,
        event_tx: mpsc::UnboundedSender<ServerEvent>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!(addr = %self.addr, "server listening");
        let _ = event_tx.send(ServerEvent::Log(format!("Listening on {}", self.addr)));

        let peers: PeerMap = Arc::new(RwLock::new(HashMap::new()));

        // 广播转发任务：将输入和剪贴板事件广播给所有客户端
        let peers_broadcast = peers.clone();
        let cancel_broadcast = cancel.clone();
        tokio::spawn(async move {
            let mut input_rx = input_rx;
            let mut clipboard_rx = clipboard_rx;
            loop {
                let msg = tokio::select! {
                    _ = cancel_broadcast.cancelled() => break,
                    Some(msg) = input_rx.recv() => msg,
                    Some(msg) = clipboard_rx.recv() => msg,
                    else => break,
                };
                let peers = peers_broadcast.read().await;
                for (_, tx) in peers.iter() {
                    let _ = tx.send(msg.clone());
                }
            }
        });

        // Accept 循环
        loop {
            let (stream, peer_addr) = tokio::select! {
                _ = cancel.cancelled() => {
                    info!("server shutting down");
                    break;
                }
                result = listener.accept() => result?,
            };

            info!(%peer_addr, "new connection");
            let _ = event_tx.send(ServerEvent::Log(format!("New connection from {peer_addr}")));

            let peers = peers.clone();
            let event_tx = event_tx.clone();
            let cancel = cancel.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, peer_addr, peers, event_tx, cancel).await {
                    warn!(%peer_addr, "client handler error: {e}");
                }
            });
        }

        Ok(())
    }
}

async fn handle_client(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    peers: PeerMap,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut framed = Framed::new(stream, MessageCodec);

    // 等待 Hello 握手
    let (device_id, device_name) = loop {
        let msg = tokio::select! {
            _ = cancel.cancelled() => return Ok(()),
            result = framed.next() => match result {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => return Err(e.into()),
                None => return Ok(()),
            },
        };
        match msg {
            Message::Hello { device_id, device_name, .. } => {
                break (device_id.0.clone(), device_name.clone());
            }
            _ => {
                warn!(%peer_addr, "expected Hello, got {:?}", msg);
            }
        }
    };

    // 回复 Welcome
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "server".into());
    framed.send(Message::Welcome {
        device_id: synapse_protocol::DeviceId(hostname.clone()),
        device_name: hostname,
        screens: vec![],
    }).await?;

    info!(%peer_addr, %device_id, %device_name, "client handshake complete");
    let _ = event_tx.send(ServerEvent::DeviceConnected {
        device_id: device_id.clone(),
        device_name: device_name.clone(),
    });

    // 注册到 peer map，创建发送 channel
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Message>();
    peers.write().await.insert(device_id.clone(), outgoing_tx);

    // 消息循环
    let result: Result<()> = async {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                // 从客户端接收消息
                incoming = framed.next() => {
                    match incoming {
                        Some(Ok(Message::Ping(seq))) => {
                            framed.send(Message::Pong(seq)).await?;
                        }
                        Some(Ok(Message::Bye { .. })) => break,
                        Some(Ok(msg)) => {
                            info!(%peer_addr, ?msg, "received from client");
                        }
                        Some(Err(e)) => {
                            error!(%peer_addr, "receive error: {e}");
                            break;
                        }
                        None => break,
                    }
                }
                // 向客户端发送消息（来自广播）
                Some(msg) = outgoing_rx.recv() => {
                    framed.send(msg).await?;
                }
            }
        }
        Ok(())
    }.await;

    // 清理
    peers.write().await.remove(&device_id);
    let _ = event_tx.send(ServerEvent::DeviceDisconnected {
        device_id: device_id.clone(),
    });
    let _ = event_tx.send(ServerEvent::Log(format!("Device {device_name} disconnected")));
    info!(%peer_addr, %device_id, "client disconnected");

    result
}
