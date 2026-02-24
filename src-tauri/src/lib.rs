use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use synapse_clipboard::{ClipboardContent, ClipboardWatcher};
use synapse_input::capture::{get_screen_size, rdev_event_to_message, InputCapturer};
use synapse_input::InputSimulator;
use synapse_net::{Client, ClientEvent, LocalAction, Server, ServerEvent};
use synapse_protocol::screen::Edge;
use synapse_protocol::Message;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    Idle,
    Server,
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub role: Role,
    pub connected: bool,
    pub devices: Vec<DeviceInfo>,
}

struct AppState {
    role: Role,
    connected: bool,
    devices: Vec<DeviceInfo>,
    cancel: Option<CancellationToken>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            role: Role::Idle,
            connected: false,
            devices: vec![],
            cancel: None,
            handle: None,
        }
    }
}

type SharedState = Arc<Mutex<AppState>>;

fn parse_direction(s: &str) -> Edge {
    match s.to_lowercase().as_str() {
        "left" => Edge::Left,
        "right" => Edge::Right,
        "top" => Edge::Top,
        "bottom" => Edge::Bottom,
        _ => Edge::Right,
    }
}

#[tauri::command]
async fn start_server(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    bind: String,
    client_direction: Option<String>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if s.role != Role::Idle {
        return Err("Already running".into());
    }

    let cancel = CancellationToken::new();
    s.role = Role::Server;
    s.connected = true;
    s.cancel = Some(cancel.clone());

    let _ = app.emit("synapse://status", AppStatus {
        role: Role::Server,
        connected: true,
        devices: vec![],
    });

    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    let direction = parse_direction(&client_direction.unwrap_or_else(|| "right".into()));

    let handle = tokio::spawn(async move {
        // 获取屏幕尺寸
        let screen_size = get_screen_size();

        // 输入捕获 channel
        let (rdev_tx, mut rdev_rx) = mpsc::unbounded_channel();
        let (input_tx, input_rx) = mpsc::unbounded_channel();

        // 剪贴板 channel
        let (clip_content_tx, mut clip_content_rx) = mpsc::unbounded_channel();
        let (clip_msg_tx, clip_msg_rx) = mpsc::unbounded_channel();

        // 服务端事件 channel
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        // 启动输入捕获
        let capturer = InputCapturer::new();
        if let Err(e) = capturer.start(rdev_tx) {
            let _ = app_clone.emit("synapse://log", format!("Input capture error: {e}"));
        }

        // rdev -> protocol 转换任务
        let cancel_input = cancel.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_input.cancelled() => break,
                    Some(event) = rdev_rx.recv() => {
                        if let Some(msg) = rdev_event_to_message(&event) {
                            let _ = input_tx.send(msg);
                        }
                    }
                    else => break,
                }
            }
        });

        // 启动剪贴板监控
        let watcher = ClipboardWatcher::new(Duration::from_millis(500));
        let _ = watcher.watch(clip_content_tx).await;

        // 剪贴板内容 -> protocol 转换
        let cancel_clip = cancel.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_clip.cancelled() => break,
                    Some(content) = clip_content_rx.recv() => {
                        let msg = match content {
                            ClipboardContent::Text(text) => {
                                Message::ClipboardText { text }
                            }
                            ClipboardContent::Image { width, height, data } => {
                                Message::ClipboardImage {
                                    width: width as u32,
                                    height: height as u32,
                                    data,
                                }
                            }
                        };
                        let _ = clip_msg_tx.send(msg);
                    }
                    else => break,
                }
            }
        });

        // 事件桥接到前端
        let state_events = state_clone.clone();
        let app_events = app_clone.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match &event {
                    ServerEvent::DeviceConnected { device_id, device_name } => {
                        let mut s = state_events.lock().await;
                        s.devices.push(DeviceInfo {
                            device_id: device_id.clone(),
                            device_name: device_name.clone(),
                        });
                        let _ = app_events.emit("synapse://device-connected", DeviceInfo {
                            device_id: device_id.clone(),
                            device_name: device_name.clone(),
                        });
                    }
                    ServerEvent::DeviceDisconnected { device_id } => {
                        let mut s = state_events.lock().await;
                        s.devices.retain(|d| d.device_id != *device_id);
                        let _ = app_events.emit("synapse://device-disconnected", device_id.clone());
                    }
                    ServerEvent::FocusChanged { target } => {
                        let _ = app_events.emit("synapse://log", format!("Focus → {target}"));
                    }
                    ServerEvent::Log(msg) => {
                        let _ = app_events.emit("synapse://log", msg.clone());
                    }
                }
            }
        });

        // LocalAction 处理线程（鼠标锁定等）
        let (local_action_tx, mut local_action_rx) = mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let mut simulator = match InputSimulator::new() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create InputSimulator for local actions: {e}");
                    return;
                }
            };
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                while let Some(action) = local_action_rx.recv().await {
                    match action {
                        LocalAction::MoveMouse(x, y) => {
                            let _ = simulator.move_mouse(x, y);
                        }
                    }
                }
            });
        });

        // 启动服务端
        let server = Server::new(bind);
        if let Err(e) = server.run(
            input_rx, clip_msg_rx, local_action_tx, event_tx,
            screen_size, direction, cancel,
        ).await {
            let _ = app_clone.emit("synapse://log", format!("Server error: {e}"));
        }

        // 清理状态
        let mut s = state_clone.lock().await;
        s.role = Role::Idle;
        s.connected = false;
        s.devices.clear();
        let _ = app_clone.emit("synapse://status", AppStatus {
            role: Role::Idle,
            connected: false,
            devices: vec![],
        });
    });

    s.handle = Some(handle);
    Ok(())
}

#[tauri::command]
async fn start_client(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    server_addr: String,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if s.role != Role::Idle {
        return Err("Already running".into());
    }

    let cancel = CancellationToken::new();
    s.role = Role::Client;
    s.cancel = Some(cancel.clone());

    let _ = app.emit("synapse://status", AppStatus {
        role: Role::Client,
        connected: false,
        devices: vec![],
    });

    let state_clone = state.inner().clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let (message_tx, mut message_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "client".into());

        // 事件桥接到前端
        let state_events = state_clone.clone();
        let app_events = app_clone.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match &event {
                    ClientEvent::Connected { server_device_id, server_device_name } => {
                        let mut s = state_events.lock().await;
                        s.connected = true;
                        let _ = app_events.emit("synapse://status", AppStatus {
                            role: Role::Client,
                            connected: true,
                            devices: vec![],
                        });
                        let _ = app_events.emit("synapse://log", format!(
                            "Connected to {} ({})", server_device_name, server_device_id
                        ));
                    }
                    ClientEvent::Disconnected => {
                        let mut s = state_events.lock().await;
                        s.connected = false;
                        let _ = app_events.emit("synapse://status", AppStatus {
                            role: Role::Client,
                            connected: false,
                            devices: vec![],
                        });
                    }
                    ClientEvent::Log(msg) => {
                        let _ = app_events.emit("synapse://log", msg.clone());
                    }
                }
            }
        });

        // 消息处理线程（InputSimulator 需要在独立线程运行）
        let cancel_sim = cancel.clone();
        std::thread::spawn(move || {
            let mut simulator = match InputSimulator::new() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create InputSimulator: {e}");
                    return;
                }
            };
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                loop {
                    tokio::select! {
                        _ = cancel_sim.cancelled() => break,
                        Some(msg) = message_rx.recv() => {
                            match msg {
                                Message::MouseMove { x, y } => {
                                    let _ = simulator.move_mouse(x as i32, y as i32);
                                }
                                Message::MouseDelta { dx, dy } => {
                                    let _ = simulator.move_mouse_relative(dx as i32, dy as i32);
                                }
                                Message::MouseButtonEvent { button, action } => {
                                    let _ = simulator.mouse_button(button, action);
                                }
                                Message::KeyEvent { key, action } => {
                                    let _ = simulator.key_event(key, action);
                                }
                                Message::MouseScroll { dx, dy } => {
                                    let _ = simulator.scroll(dx as i32, dy as i32);
                                }
                                Message::ClipboardText { text } => {
                                    let _ = ClipboardWatcher::set_text(&text);
                                }
                                _ => {}
                            }
                        }
                        else => break,
                    }
                }
            });
        });

        // 启动客户端连接
        let screen_size = get_screen_size();
        let client = Client::new(server_addr);
        if let Err(e) = client.connect(
            hostname.clone(),
            hostname,
            screen_size,
            message_tx,
            event_tx,
            cancel,
        ).await {
            let _ = app_clone.emit("synapse://log", format!("Client error: {e}"));
        }

        // 清理状态
        let mut s = state_clone.lock().await;
        s.role = Role::Idle;
        s.connected = false;
        let _ = app_clone.emit("synapse://status", AppStatus {
            role: Role::Idle,
            connected: false,
            devices: vec![],
        });
    });

    s.handle = Some(handle);
    Ok(())
}

#[tauri::command]
async fn stop(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if s.role == Role::Idle {
        return Ok(());
    }

    if let Some(cancel) = s.cancel.take() {
        cancel.cancel();
    }

    s.role = Role::Idle;
    s.connected = false;
    s.devices.clear();
    s.handle = None;

    let _ = app.emit("synapse://status", AppStatus {
        role: Role::Idle,
        connected: false,
        devices: vec![],
    });
    let _ = app.emit("synapse://log", "Stopped".to_string());

    Ok(())
}

#[tauri::command]
async fn get_status(
    state: tauri::State<'_, SharedState>,
) -> Result<AppStatus, String> {
    let s = state.lock().await;
    Ok(AppStatus {
        role: s.role.clone(),
        connected: s.connected,
        devices: s.devices.clone(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SharedState::default())
        .invoke_handler(tauri::generate_handler![
            start_server,
            start_client,
            stop,
            get_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}