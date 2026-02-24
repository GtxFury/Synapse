use serde::{Deserialize, Serialize};

/// 屏幕标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenId(pub u32);

/// 屏幕上的坐标位置
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenPosition {
    pub x: f64,
    pub y: f64,
}

/// 屏幕边缘方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// 屏幕矩形区域
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 屏幕信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub id: ScreenId,
    pub name: String,
    pub rect: ScreenRect,
    pub is_primary: bool,
}
