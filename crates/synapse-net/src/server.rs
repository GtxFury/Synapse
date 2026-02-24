use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use synapse_protocol::screen::{Edge, ScreenId, ScreenInfo, ScreenPosition, ScreenRect};
use synapse_protocol::{Message, MessageCodec};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio_util::codec::Framed;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{LocalAction, ServerEvent};

type PeerMap = Arc<RwLock<HashMap<String, PeerInfo>>>;

struct PeerInfo {
    tx: mpsc::UnboundedSender<Message>,
    #[allow(dead_code)]
    screen_w: u32,
    #[allow(dead_code)]
    screen_h: u32,
}

// ── 边缘检测阈值 ──
const EDGE_THRESHOLD: f64 = 2.0;

// ── FocusManager ──

#[derive(Debug, Clone)]
enum FocusState {
    Local,
    Remote {
        device_id: String,
        virtual_x: f64,
        virtual_y: f64,
        remote_w: u32,
        remote_h: u32,
        entered_edge: Edge,
    },
}

struct FocusManager {
    state: FocusState,
    screen_w: u32,
    screen_h: u32,
    center_x: i32,
    center_y: i32,
    /// 边缘方向 → (device_id, 远程屏幕宽, 高)
    edge_devices: HashMap<Edge, (String, u32, u32)>,
}

impl FocusManager {
    fn new(screen_w: u32, screen_h: u32) -> Self {
        Self {
            state: FocusState::Local,
            screen_w,
            screen_h,
            center_x: screen_w as i32 / 2,
            center_y: screen_h as i32 / 2,
            edge_devices: HashMap::new(),
        }
    }

    fn set_edge_device(&mut self, edge: Edge, device_id: String, w: u32, h: u32) {
        self.edge_devices.insert(edge, (device_id, w, h));
    }

    fn remove_device(&mut self, device_id: &str) {
        self.edge_devices.retain(|_, (id, _, _)| id != device_id);
        // 如果焦点在被移除的设备上，切回本地
        if let FocusState::Remote { device_id: ref fid, .. } = self.state {
            if fid == device_id {
                self.state = FocusState::Local;
            }
        }
    }

    /// 反向边缘
    fn opposite_edge(edge: &Edge) -> Edge {
        match edge {
            Edge::Left => Edge::Right,
            Edge::Right => Edge::Left,
            Edge::Top => Edge::Bottom,
            Edge::Bottom => Edge::Top,
        }
    }

    /// 检测绝对坐标是否到达屏幕边缘，返回对应 Edge
    fn check_edge(&self, x: f64, y: f64) -> Option<Edge> {
        if x <= EDGE_THRESHOLD { return Some(Edge::Left); }
        if x >= self.screen_w as f64 - EDGE_THRESHOLD { return Some(Edge::Right); }
        if y <= EDGE_THRESHOLD { return Some(Edge::Top); }
        if y >= self.screen_h as f64 - EDGE_THRESHOLD { return Some(Edge::Bottom); }
        None
    }

    /// 计算进入远程屏幕时的初始虚拟光标位置
    fn entry_position(edge: &Edge, x: f64, y: f64, sw: u32, sh: u32, rw: u32, rh: u32) -> (f64, f64) {
        match edge {
            Edge::Right => (0.0, y * rh as f64 / sh as f64),
            Edge::Left => (rw as f64, y * rh as f64 / sh as f64),
            Edge::Bottom => (x * rw as f64 / sw as f64, 0.0),
            Edge::Top => (x * rw as f64 / sw as f64, rh as f64),
        }
    }

    /// 检测虚拟光标是否到达远程屏幕的反向边缘
    fn check_virtual_edge(vx: f64, vy: f64, rw: u32, rh: u32, entered_edge: &Edge) -> bool {
        let exit_edge = Self::opposite_edge(entered_edge);
        match exit_edge {
            Edge::Left => vx <= 0.0,
            Edge::Right => vx >= rw as f64,
            Edge::Top => vy <= 0.0,
            Edge::Bottom => vy >= rh as f64,
        }
    }
}

/// TCP 服务端
pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// 启动服务端完整消息循环（焦点驱动模式）
    pub async fn run(
        &self,
        input_rx: mpsc::UnboundedReceiver<Message>,
        clipboard_rx: mpsc::UnboundedReceiver<Message>,
        local_action_tx: mpsc::UnboundedSender<LocalAction>,
        event_tx: mpsc::UnboundedSender<ServerEvent>,
        screen_size: (u32, u32),
        client_direction: Edge,
        cancel: CancellationToken,
    ) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!(addr = %self.addr, "server listening");
        let _ = event_tx.send(ServerEvent::Log(format!("Listening on {}", self.addr)));

        let peers: PeerMap = Arc::new(RwLock::new(HashMap::new()));
        let focus = Arc::new(tokio::sync::Mutex::new(
            FocusManager::new(screen_size.0, screen_size.1),
        ));
        let client_direction = Arc::new(client_direction);

        // 焦点驱动的输入处理任务
        let peers_input = peers.clone();
        let focus_input = focus.clone();
        let cancel_input = cancel.clone();
        let event_tx_input = event_tx.clone();
        let local_action = local_action_tx.clone();
        tokio::spawn(async move {
            let mut input_rx = input_rx;
            let mut clipboard_rx = clipboard_rx;
            loop {
                let msg = tokio::select! {
                    _ = cancel_input.cancelled() => break,
                    Some(msg) = input_rx.recv() => msg,
                    Some(msg) = clipboard_rx.recv() => msg,
                    else => break,
                };
                // PLACEHOLDER_INPUT_HANDLER
                handle_input_message(
                    msg,
                    &focus_input,
                    &peers_input,
                    &local_action,
                    &event_tx_input,
                ).await;
            }
        });

        // Accept 循环
        let client_dir = client_direction.clone();
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
            let focus = focus.clone();
            let event_tx = event_tx.clone();
            let cancel = cancel.clone();
            let client_dir = client_dir.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_client(
                    stream, peer_addr, peers, focus, event_tx, cancel, &client_dir,
                ).await {
                    warn!(%peer_addr, "client handler error: {e}");
                }
            });
        }

        Ok(())
    }
}

async fn handle_input_message(
    msg: Message,
    focus: &tokio::sync::Mutex<FocusManager>,
    peers: &PeerMap,
    local_action_tx: &mpsc::UnboundedSender<LocalAction>,
    event_tx: &mpsc::UnboundedSender<ServerEvent>,
) {
    let mut fm = focus.lock().await;

    match &fm.state.clone() {
        FocusState::Local => {
            // 焦点在本地：只关心 MouseMove 的边缘检测
            if let Message::MouseMove { x, y } = &msg {
                if let Some(edge) = fm.check_edge(*x, *y) {
                    // 检查该边缘是否有设备
                    if let Some((device_id, rw, rh)) = fm.edge_devices.get(&edge).cloned() {
                        let (vx, vy) = FocusManager::entry_position(
                            &edge, *x, *y, fm.screen_w, fm.screen_h, rw, rh,
                        );
                        info!(
                            %device_id, ?edge, vx, vy,
                            "focus switching to remote device"
                        );
                        fm.state = FocusState::Remote {
                            device_id: device_id.clone(),
                            virtual_x: vx,
                            virtual_y: vy,
                            remote_w: rw,
                            remote_h: rh,
                            entered_edge: edge.clone(),
                        };
                        // 锁定鼠标到屏幕中心
                        let _ = local_action_tx.send(LocalAction::MoveMouse(
                            fm.center_x, fm.center_y,
                        ));
                        // 通知 Client 进入屏幕
                        let peers_r = peers.read().await;
                        if let Some(peer) = peers_r.get(&device_id) {
                            let _ = peer.tx.send(Message::EnterScreen {
                                screen_id: ScreenId(0),
                                position: ScreenPosition { x: vx, y: vy },
                            });
                            // 发送初始绝对定位
                            let _ = peer.tx.send(Message::MouseMove { x: vx, y: vy });
                        }
                        let _ = event_tx.send(ServerEvent::FocusChanged {
                            target: device_id,
                        });
                    }
                }
            }
            // 其他消息在 Local 模式下忽略（不转发）
        }
        FocusState::Remote {
            device_id,
            virtual_x,
            virtual_y,
            remote_w,
            remote_h,
            entered_edge,
        } => {
            let device_id = device_id.clone();
            let remote_w = *remote_w;
            let remote_h = *remote_h;
            let entered_edge = entered_edge.clone();

            match &msg {
                Message::MouseMove { x, y } => {
                    // 计算 delta（相对于屏幕中心）
                    let dx = *x as f64 - fm.center_x as f64;
                    let dy = *y as f64 - fm.center_y as f64;
                    if dx == 0.0 && dy == 0.0 {
                        return; // 忽略锁回中心产生的事件
                    }

                    // 更新虚拟光标
                    let new_vx = (*virtual_x + dx).clamp(0.0, remote_w as f64);
                    let new_vy = (*virtual_y + dy).clamp(0.0, remote_h as f64);

                    // 检测是否到达反向边缘（切回本地）
                    if FocusManager::check_virtual_edge(
                        new_vx, new_vy, remote_w, remote_h, &entered_edge,
                    ) {
                        info!(%device_id, "focus switching back to local");
                        // 发送 LeaveScreen 给 Client
                        let peers_r = peers.read().await;
                        if let Some(peer) = peers_r.get(&device_id) {
                            let _ = peer.tx.send(Message::LeaveScreen {
                                screen_id: ScreenId(0),
                                edge: FocusManager::opposite_edge(&entered_edge),
                                position: ScreenPosition { x: new_vx, y: new_vy },
                            });
                        }
                        fm.state = FocusState::Local;
                        let _ = event_tx.send(ServerEvent::FocusChanged {
                            target: "local".into(),
                        });
                        return;
                    }

                    // 更新虚拟光标位置
                    fm.state = FocusState::Remote {
                        device_id: device_id.clone(),
                        virtual_x: new_vx,
                        virtual_y: new_vy,
                        remote_w,
                        remote_h,
                        entered_edge,
                    };

                    // 发送 MouseDelta 给焦点设备
                    let peers_r = peers.read().await;
                    if let Some(peer) = peers_r.get(&device_id) {
                        let _ = peer.tx.send(Message::MouseDelta { dx, dy });
                    }

                    // 锁回鼠标到屏幕中心
                    let _ = local_action_tx.send(LocalAction::MoveMouse(
                        fm.center_x, fm.center_y,
                    ));
                }
                Message::KeyEvent { .. }
                | Message::MouseButtonEvent { .. }
                | Message::MouseScroll { .. } => {
                    // 转发给焦点设备
                    let peers_r = peers.read().await;
                    if let Some(peer) = peers_r.get(&device_id) {
                        let _ = peer.tx.send(msg);
                    }
                }
                Message::ClipboardText { .. } | Message::ClipboardImage { .. } => {
                    // 剪贴板同步给焦点设备
                    let peers_r = peers.read().await;
                    if let Some(peer) = peers_r.get(&device_id) {
                        let _ = peer.tx.send(msg);
                    }
                }
                _ => {}
            }
        }
    }
}

async fn handle_client(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    peers: PeerMap,
    focus: Arc<tokio::sync::Mutex<FocusManager>>,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
    cancel: CancellationToken,
    client_direction: &Edge,
) -> Result<()> {
    let mut framed = Framed::new(stream, MessageCodec);

    // 等待 Hello 握手
    let (device_id, device_name, screens) = loop {
        let msg = tokio::select! {
            _ = cancel.cancelled() => return Ok(()),
            result = framed.next() => match result {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => return Err(e.into()),
                None => return Ok(()),
            },
        };
        match msg {
            Message::Hello { device_id, device_name, screens } => {
                break (device_id.0.clone(), device_name.clone(), screens);
            }
            _ => {
                warn!(%peer_addr, "expected Hello, got {:?}", msg);
            }
        }
    };

    // 回复 Welcome（携带 Server 屏幕信息）
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "server".into());
    let fm = focus.lock().await;
    let server_screen = ScreenInfo {
        id: ScreenId(0),
        name: "primary".into(),
        rect: ScreenRect {
            x: 0, y: 0,
            width: fm.screen_w,
            height: fm.screen_h,
        },
        is_primary: true,
    };
    drop(fm);

    framed.send(Message::Welcome {
        device_id: synapse_protocol::DeviceId(hostname.clone()),
        device_name: hostname,
        screens: vec![server_screen],
    }).await?;

    info!(%peer_addr, %device_id, %device_name, "client handshake complete");
    let _ = event_tx.send(ServerEvent::DeviceConnected {
        device_id: device_id.clone(),
        device_name: device_name.clone(),
    });

    // 从 Client 的 Hello.screens 获取屏幕尺寸
    let (client_w, client_h) = if let Some(s) = screens.first() {
        (s.rect.width, s.rect.height)
    } else {
        (1920, 1080) // 默认值
    };

    // 注册到 peer map 并设置边缘设备
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Message>();
    {
        let mut peers_w = peers.write().await;
        peers_w.insert(device_id.clone(), PeerInfo {
            tx: outgoing_tx,
            screen_w: client_w,
            screen_h: client_h,
        });
    }
    {
        let mut fm = focus.lock().await;
        fm.set_edge_device(client_direction.clone(), device_id.clone(), client_w, client_h);
        info!(
            %device_id, ?client_direction, client_w, client_h,
            "registered edge device"
        );
    }

    // 消息循环
    let result: Result<()> = async {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
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
                Some(msg) = outgoing_rx.recv() => {
                    framed.send(msg).await?;
                }
            }
        }
        Ok(())
    }.await;

    // 清理
    peers.write().await.remove(&device_id);
    {
        let mut fm = focus.lock().await;
        fm.remove_device(&device_id);
    }
    let _ = event_tx.send(ServerEvent::DeviceDisconnected {
        device_id: device_id.clone(),
    });
    let _ = event_tx.send(ServerEvent::Log(format!("Device {device_name} disconnected")));
    info!(%peer_addr, %device_id, "client disconnected");

    result
}
