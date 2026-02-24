use synapse_protocol::screen::{Edge, ScreenId, ScreenInfo, ScreenPosition, ScreenRect};

/// 屏幕布局管理器
///
/// 管理多台设备的屏幕排列关系，处理鼠标跨屏幕边缘切换
pub struct ScreenLayout {
    screens: Vec<ScreenEntry>,
}

/// 屏幕条目：屏幕信息 + 边缘邻居映射
struct ScreenEntry {
    info: ScreenInfo,
    neighbors: Neighbors,
}

/// 四个方向的邻居屏幕
#[derive(Default)]
struct Neighbors {
    top: Option<ScreenId>,
    bottom: Option<ScreenId>,
    left: Option<ScreenId>,
    right: Option<ScreenId>,
}

impl ScreenLayout {
    pub fn new() -> Self {
        Self {
            screens: Vec::new(),
        }
    }

    /// 添加屏幕
    pub fn add_screen(&mut self, info: ScreenInfo) {
        self.screens.push(ScreenEntry {
            info,
            neighbors: Neighbors::default(),
        });
    }

    /// 设置两个屏幕的邻居关系
    pub fn link(&mut self, from: ScreenId, edge: Edge, to: ScreenId) {
        if let Some(entry) = self.screens.iter_mut().find(|e| e.info.id == from) {
            match edge {
                Edge::Top => entry.neighbors.top = Some(to),
                Edge::Bottom => entry.neighbors.bottom = Some(to),
                Edge::Left => entry.neighbors.left = Some(to),
                Edge::Right => entry.neighbors.right = Some(to),
            }
        }
    }

    /// 检测鼠标是否到达屏幕边缘，返回目标屏幕和映射后的坐标
    pub fn check_edge_crossing(
        &self,
        screen_id: ScreenId,
        pos: ScreenPosition,
    ) -> Option<(ScreenId, Edge, ScreenPosition)> {
        let entry = self.screens.iter().find(|e| e.info.id == screen_id)?;
        let rect = &entry.info.rect;

        let edge = if pos.x <= rect.x as f64 {
            Some((Edge::Left, entry.neighbors.left?))
        } else if pos.x >= (rect.x + rect.width as i32) as f64 {
            Some((Edge::Right, entry.neighbors.right?))
        } else if pos.y <= rect.y as f64 {
            Some((Edge::Top, entry.neighbors.top?))
        } else if pos.y >= (rect.y + rect.height as i32) as f64 {
            Some((Edge::Bottom, entry.neighbors.bottom?))
        } else {
            None
        };

        let (edge, target_id) = edge?;
        let target = self.screens.iter().find(|e| e.info.id == target_id)?;
        let mapped = map_position(edge, pos, rect, &target.info.rect);

        Some((target_id, edge, mapped))
    }

    /// 获取所有屏幕信息
    pub fn screens(&self) -> Vec<&ScreenInfo> {
        self.screens.iter().map(|e| &e.info).collect()
    }
}

/// 将坐标从源屏幕边缘映射到目标屏幕
fn map_position(
    edge: Edge,
    pos: ScreenPosition,
    _src: &ScreenRect,
    target: &ScreenRect,
) -> ScreenPosition {
    match edge {
        Edge::Left => ScreenPosition {
            x: (target.x + target.width as i32) as f64 - 1.0,
            y: pos.y.clamp(target.y as f64, (target.y + target.height as i32) as f64),
        },
        Edge::Right => ScreenPosition {
            x: target.x as f64,
            y: pos.y.clamp(target.y as f64, (target.y + target.height as i32) as f64),
        },
        Edge::Top => ScreenPosition {
            x: pos.x.clamp(target.x as f64, (target.x + target.width as i32) as f64),
            y: (target.y + target.height as i32) as f64 - 1.0,
        },
        Edge::Bottom => ScreenPosition {
            x: pos.x.clamp(target.x as f64, (target.x + target.width as i32) as f64),
            y: target.y as f64,
        },
    }
}
