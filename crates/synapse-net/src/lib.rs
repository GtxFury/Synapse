pub mod client;
pub mod discovery;
pub mod server;

pub use client::Client;
pub use discovery::Discovery;
pub use server::Server;

use serde::{Deserialize, Serialize};

/// 服务端产生的事件，用于通知上层（GUI/CLI）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerEvent {
    /// 新客户端已连接
    DeviceConnected {
        device_id: String,
        device_name: String,
    },
    /// 客户端已断开
    DeviceDisconnected {
        device_id: String,
    },
    /// 日志消息
    Log(String),
}

/// 客户端产生的事件，用于通知上层（GUI/CLI）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientEvent {
    /// 已连接到服务端
    Connected {
        server_device_id: String,
        server_device_name: String,
    },
    /// 与服务端断开
    Disconnected,
    /// 日志消息
    Log(String),
}
