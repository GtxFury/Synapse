use anyhow::Result;
use clap::{Parser, Subcommand};
use synapse_clipboard::{ClipboardContent, ClipboardWatcher};
use synapse_input::capture::{rdev_event_to_message, InputCapturer};
use synapse_input::InputSimulator;
use synapse_net::{ClientEvent, Server, ServerEvent};
use synapse_protocol::Message;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "synapse", version, about = "多设备跨平台协作工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 以服务端模式运行（主控端）
    Server {
        /// 监听地址
        #[arg(short, long, default_value = "0.0.0.0:24800")]
        bind: String,
    },
    /// 以客户端模式运行（被控端）
    Client {
        /// 服务端地址
        #[arg(short, long)]
        server: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("synapse=info".parse()?))
        .init();

    let cli = Cli::parse();
    let cancel = CancellationToken::new();

    // Ctrl+C 处理
    let cancel_ctrlc = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutting down...");
        cancel_ctrlc.cancel();
    });

    match cli.command {
        Command::Server { bind } => {
            tracing::info!(addr = %bind, "starting synapse server");

            // 输入捕获
            let (rdev_tx, mut rdev_rx) = mpsc::unbounded_channel();
            let (input_tx, input_rx) = mpsc::unbounded_channel();
            let capturer = InputCapturer::new();
            capturer.start(rdev_tx)?;

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

            // 剪贴板监控
            let (clip_tx, mut clip_rx) = mpsc::unbounded_channel();
            let (clip_msg_tx, clip_msg_rx) = mpsc::unbounded_channel();
            let watcher = ClipboardWatcher::new(Duration::from_millis(500));
            watcher.watch(clip_tx).await?;

            let cancel_clip = cancel.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel_clip.cancelled() => break,
                        Some(content) = clip_rx.recv() => {
                            let msg = match content {
                                ClipboardContent::Text(text) => Message::ClipboardText { text },
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

            // 服务端事件处理
            let (event_tx, mut event_rx) = mpsc::unbounded_channel();
            tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        ServerEvent::DeviceConnected { device_id, device_name } => {
                            tracing::info!(%device_id, %device_name, "device connected");
                        }
                        ServerEvent::DeviceDisconnected { device_id } => {
                            tracing::info!(%device_id, "device disconnected");
                        }
                        ServerEvent::Log(msg) => {
                            tracing::info!("{msg}");
                        }
                    }
                }
            });

            let server = Server::new(bind);
            server.run(input_rx, clip_msg_rx, event_tx, cancel).await?;
        }
        Command::Client { server } => {
            tracing::info!(addr = %server, "connecting to synapse server");

            let hostname = hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "cli-client".into());

            let (message_tx, mut message_rx) = mpsc::unbounded_channel();
            let (event_tx, mut event_rx) = mpsc::unbounded_channel();

            // 事件处理
            tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        ClientEvent::Connected { server_device_id, server_device_name } => {
                            tracing::info!(%server_device_id, %server_device_name, "connected");
                        }
                        ClientEvent::Disconnected => {
                            tracing::info!("disconnected from server");
                        }
                        ClientEvent::Log(msg) => {
                            tracing::info!("{msg}");
                        }
                    }
                }
            });

            // 消息处理（输入模拟）
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

            let client = synapse_net::Client::new(server);
            client.connect(hostname.clone(), hostname, message_tx, event_tx, cancel).await?;
        }
    }

    Ok(())
}