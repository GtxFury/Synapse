use anyhow::Result;
use synapse_protocol::input::{ButtonAction, KeyAction, KeyCode, MouseButton};
use synapse_protocol::Message;
use tokio::sync::mpsc;
use tracing::info;

/// 输入捕获器，封装 rdev::listen
pub struct InputCapturer {
    _private: (),
}

impl InputCapturer {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// 启动全局输入监听，将事件发送到 channel
    pub fn start(&self, tx: mpsc::UnboundedSender<rdev::Event>) -> Result<()> {
        info!("starting input capture");

        std::thread::spawn(move || {
            rdev::listen(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to listen for input events");
        });

        Ok(())
    }
}

/// 将 rdev 原始事件转换为协议 Message
pub fn rdev_event_to_message(event: &rdev::Event) -> Option<Message> {
    match &event.event_type {
        rdev::EventType::MouseMove { x, y } => Some(Message::MouseMove { x: *x, y: *y }),
        rdev::EventType::ButtonPress(btn) => Some(Message::MouseButtonEvent {
            button: rdev_button_to_proto(btn),
            action: ButtonAction::Press,
        }),
        rdev::EventType::ButtonRelease(btn) => Some(Message::MouseButtonEvent {
            button: rdev_button_to_proto(btn),
            action: ButtonAction::Release,
        }),
        rdev::EventType::KeyPress(key) => Some(Message::KeyEvent {
            key: rdev_key_to_proto(key),
            action: KeyAction::Press,
        }),
        rdev::EventType::KeyRelease(key) => Some(Message::KeyEvent {
            key: rdev_key_to_proto(key),
            action: KeyAction::Release,
        }),
        rdev::EventType::Wheel { delta_x, delta_y } => Some(Message::MouseScroll {
            dx: *delta_x as f64,
            dy: *delta_y as f64,
        }),
    }
}

fn rdev_button_to_proto(btn: &rdev::Button) -> MouseButton {
    match btn {
        rdev::Button::Left => MouseButton::Left,
        rdev::Button::Right => MouseButton::Right,
        rdev::Button::Middle => MouseButton::Middle,
        rdev::Button::Unknown(4) => MouseButton::Back,
        rdev::Button::Unknown(5) => MouseButton::Forward,
        _ => MouseButton::Left,
    }
}

fn rdev_key_to_proto(key: &rdev::Key) -> KeyCode {
    match key {
        rdev::Key::KeyA => KeyCode::KeyA,
        rdev::Key::KeyB => KeyCode::KeyB,
        rdev::Key::KeyC => KeyCode::KeyC,
        rdev::Key::KeyD => KeyCode::KeyD,
        rdev::Key::KeyE => KeyCode::KeyE,
        rdev::Key::KeyF => KeyCode::KeyF,
        rdev::Key::KeyG => KeyCode::KeyG,
        rdev::Key::KeyH => KeyCode::KeyH,
        rdev::Key::KeyI => KeyCode::KeyI,
        rdev::Key::KeyJ => KeyCode::KeyJ,
        rdev::Key::KeyK => KeyCode::KeyK,
        rdev::Key::KeyL => KeyCode::KeyL,
        rdev::Key::KeyM => KeyCode::KeyM,
        rdev::Key::KeyN => KeyCode::KeyN,
        rdev::Key::KeyO => KeyCode::KeyO,
        rdev::Key::KeyP => KeyCode::KeyP,
        rdev::Key::KeyQ => KeyCode::KeyQ,
        rdev::Key::KeyR => KeyCode::KeyR,
        rdev::Key::KeyS => KeyCode::KeyS,
        rdev::Key::KeyT => KeyCode::KeyT,
        rdev::Key::KeyU => KeyCode::KeyU,
        rdev::Key::KeyV => KeyCode::KeyV,
        rdev::Key::KeyW => KeyCode::KeyW,
        rdev::Key::KeyX => KeyCode::KeyX,
        rdev::Key::KeyY => KeyCode::KeyY,
        rdev::Key::KeyZ => KeyCode::KeyZ,
        rdev::Key::Num0 => KeyCode::Num0,
        rdev::Key::Num1 => KeyCode::Num1,
        rdev::Key::Num2 => KeyCode::Num2,
        rdev::Key::Num3 => KeyCode::Num3,
        rdev::Key::Num4 => KeyCode::Num4,
        rdev::Key::Num5 => KeyCode::Num5,
        rdev::Key::Num6 => KeyCode::Num6,
        rdev::Key::Num7 => KeyCode::Num7,
        rdev::Key::Num8 => KeyCode::Num8,
        rdev::Key::Num9 => KeyCode::Num9,
        rdev::Key::F1 => KeyCode::F1,
        rdev::Key::F2 => KeyCode::F2,
        rdev::Key::F3 => KeyCode::F3,
        rdev::Key::F4 => KeyCode::F4,
        rdev::Key::F5 => KeyCode::F5,
        rdev::Key::F6 => KeyCode::F6,
        rdev::Key::F7 => KeyCode::F7,
        rdev::Key::F8 => KeyCode::F8,
        rdev::Key::F9 => KeyCode::F9,
        rdev::Key::F10 => KeyCode::F10,
        rdev::Key::F11 => KeyCode::F11,
        rdev::Key::F12 => KeyCode::F12,
        rdev::Key::ShiftLeft => KeyCode::LeftShift,
        rdev::Key::ShiftRight => KeyCode::RightShift,
        rdev::Key::ControlLeft => KeyCode::LeftCtrl,
        rdev::Key::ControlRight => KeyCode::RightCtrl,
        rdev::Key::Alt => KeyCode::LeftAlt,
        rdev::Key::AltGr => KeyCode::RightAlt,
        rdev::Key::MetaLeft => KeyCode::LeftMeta,
        rdev::Key::MetaRight => KeyCode::RightMeta,
        rdev::Key::Escape => KeyCode::Escape,
        rdev::Key::Tab => KeyCode::Tab,
        rdev::Key::CapsLock => KeyCode::CapsLock,
        rdev::Key::Space => KeyCode::Space,
        rdev::Key::Return => KeyCode::Enter,
        rdev::Key::Backspace => KeyCode::Backspace,
        rdev::Key::Delete => KeyCode::Delete,
        rdev::Key::Insert => KeyCode::Insert,
        rdev::Key::Home => KeyCode::Home,
        rdev::Key::End => KeyCode::End,
        rdev::Key::PageUp => KeyCode::PageUp,
        rdev::Key::PageDown => KeyCode::PageDown,
        rdev::Key::UpArrow => KeyCode::ArrowUp,
        rdev::Key::DownArrow => KeyCode::ArrowDown,
        rdev::Key::LeftArrow => KeyCode::ArrowLeft,
        rdev::Key::RightArrow => KeyCode::ArrowRight,
        rdev::Key::PrintScreen => KeyCode::PrintScreen,
        rdev::Key::ScrollLock => KeyCode::ScrollLock,
        rdev::Key::Pause => KeyCode::Pause,
        rdev::Key::Unknown(code) => KeyCode::Unknown(*code as u32),
        _ => KeyCode::Unknown(0),
    }
}
