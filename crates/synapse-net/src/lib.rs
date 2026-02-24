pub mod client;
pub mod discovery;
pub mod server;

pub use client::Client;
pub use discovery::Discovery;
pub use server::Server;

use serde::{Deserialize, Serialize};

/// Server 端需要在本地执行的动作
#[derive(Debug, Clone)]
pub enum LocalAction {
    /// 将鼠标移动到指定绝对坐标（用于焦点在远程时锁定鼠标到屏幕中心）
    MoveMouse(i32, i32),
}

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
    /// 焦点切换
    FocusChanged {
        target: String,
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
