use serde::{Deserialize, Serialize};

use crate::input::{ButtonAction, KeyAction, KeyCode, MouseButton};
use crate::screen::{Edge, ScreenId, ScreenInfo, ScreenPosition};

/// 设备标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

/// 协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // ── 握手 ──
    Hello {
        device_id: DeviceId,
        device_name: String,
        screens: Vec<ScreenInfo>,
    },
    Welcome {
        device_id: DeviceId,
        device_name: String,
        screens: Vec<ScreenInfo>,
    },
    Bye {
        device_id: DeviceId,
    },

    // ── 输入转发 ──
    MouseMove {
        x: f64,
        y: f64,
    },
    MouseButtonEvent {
        button: MouseButton,
        action: ButtonAction,
    },
    MouseScroll {
        dx: f64,
        dy: f64,
    },
    KeyEvent {
        key: KeyCode,
        action: KeyAction,
    },

    // ── 焦点切换 ──
    EnterScreen {
        screen_id: ScreenId,
        position: ScreenPosition,
    },
    LeaveScreen {
        screen_id: ScreenId,
        edge: Edge,
        position: ScreenPosition,
    },

    // ── 剪贴板同步 ──
    ClipboardText {
        text: String,
    },
    ClipboardImage {
        width: u32,
        height: u32,
        data: Vec<u8>,
    },

    // ── 心跳 ──
    Ping(u64),
    Pong(u64),
}
