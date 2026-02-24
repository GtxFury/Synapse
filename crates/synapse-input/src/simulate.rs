use anyhow::Result;
use enigo::{Enigo, Keyboard, Mouse, Settings};
use synapse_protocol::input::{ButtonAction, KeyAction, KeyCode, MouseButton};
use tracing::debug;

/// 输入模拟器，封装 enigo
pub struct InputSimulator {
    enigo: Enigo,
}

impl InputSimulator {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo })
    }

    /// 模拟鼠标移动到绝对坐标
    pub fn move_mouse(&mut self, x: i32, y: i32) -> Result<()> {
        debug!(x, y, "simulating mouse move");
        self.enigo.move_mouse(x, y, enigo::Coordinate::Abs)?;
        Ok(())
    }

    /// 模拟鼠标按键
    pub fn mouse_button(&mut self, button: MouseButton, action: ButtonAction) -> Result<()> {
        debug!(?button, ?action, "simulating mouse button");
        let btn = to_enigo_button(button);
        match action {
            ButtonAction::Press => self.enigo.button(btn, enigo::Direction::Press)?,
            ButtonAction::Release => self.enigo.button(btn, enigo::Direction::Release)?,
        }
        Ok(())
    }

    /// 模拟键盘事件
    pub fn key_event(&mut self, key: KeyCode, action: KeyAction) -> Result<()> {
        debug!(?key, ?action, "simulating key event");
        let enigo_key = to_enigo_key(key);
        match action {
            KeyAction::Press => self.enigo.key(enigo_key, enigo::Direction::Press)?,
            KeyAction::Release => self.enigo.key(enigo_key, enigo::Direction::Release)?,
        }
        Ok(())
    }

    /// 模拟滚轮
    pub fn scroll(&mut self, dx: i32, dy: i32) -> Result<()> {
        debug!(dx, dy, "simulating scroll");
        if dy != 0 {
            self.enigo.scroll(dy, enigo::Axis::Vertical)?;
        }
        if dx != 0 {
            self.enigo.scroll(dx, enigo::Axis::Horizontal)?;
        }
        Ok(())
    }
}

fn to_enigo_button(button: MouseButton) -> enigo::Button {
    match button {
        MouseButton::Left => enigo::Button::Left,
        MouseButton::Right => enigo::Button::Right,
        MouseButton::Middle => enigo::Button::Middle,
        MouseButton::Back => enigo::Button::Back,
        MouseButton::Forward => enigo::Button::Forward,
    }
}

fn to_enigo_key(key: KeyCode) -> enigo::Key {
    match key {
        KeyCode::KeyA => enigo::Key::Unicode('a'),
        KeyCode::KeyB => enigo::Key::Unicode('b'),
        KeyCode::KeyC => enigo::Key::Unicode('c'),
        KeyCode::KeyD => enigo::Key::Unicode('d'),
        KeyCode::KeyE => enigo::Key::Unicode('e'),
        KeyCode::KeyF => enigo::Key::Unicode('f'),
        KeyCode::KeyG => enigo::Key::Unicode('g'),
        KeyCode::KeyH => enigo::Key::Unicode('h'),
        KeyCode::KeyI => enigo::Key::Unicode('i'),
        KeyCode::KeyJ => enigo::Key::Unicode('j'),
        KeyCode::KeyK => enigo::Key::Unicode('k'),
        KeyCode::KeyL => enigo::Key::Unicode('l'),
        KeyCode::KeyM => enigo::Key::Unicode('m'),
        KeyCode::KeyN => enigo::Key::Unicode('n'),
        KeyCode::KeyO => enigo::Key::Unicode('o'),
        KeyCode::KeyP => enigo::Key::Unicode('p'),
        KeyCode::KeyQ => enigo::Key::Unicode('q'),
        KeyCode::KeyR => enigo::Key::Unicode('r'),
        KeyCode::KeyS => enigo::Key::Unicode('s'),
        KeyCode::KeyT => enigo::Key::Unicode('t'),
        KeyCode::KeyU => enigo::Key::Unicode('u'),
        KeyCode::KeyV => enigo::Key::Unicode('v'),
        KeyCode::KeyW => enigo::Key::Unicode('w'),
        KeyCode::KeyX => enigo::Key::Unicode('x'),
        KeyCode::KeyY => enigo::Key::Unicode('y'),
        KeyCode::KeyZ => enigo::Key::Unicode('z'),
        KeyCode::Num0 => enigo::Key::Unicode('0'),
        KeyCode::Num1 => enigo::Key::Unicode('1'),
        KeyCode::Num2 => enigo::Key::Unicode('2'),
        KeyCode::Num3 => enigo::Key::Unicode('3'),
        KeyCode::Num4 => enigo::Key::Unicode('4'),
        KeyCode::Num5 => enigo::Key::Unicode('5'),
        KeyCode::Num6 => enigo::Key::Unicode('6'),
        KeyCode::Num7 => enigo::Key::Unicode('7'),
        KeyCode::Num8 => enigo::Key::Unicode('8'),
        KeyCode::Num9 => enigo::Key::Unicode('9'),
        KeyCode::Escape => enigo::Key::Escape,
        KeyCode::Tab => enigo::Key::Tab,
        KeyCode::CapsLock => enigo::Key::CapsLock,
        KeyCode::Space => enigo::Key::Space,
        KeyCode::Enter => enigo::Key::Return,
        KeyCode::Backspace => enigo::Key::Backspace,
        KeyCode::Delete => enigo::Key::Delete,
        KeyCode::ArrowUp => enigo::Key::UpArrow,
        KeyCode::ArrowDown => enigo::Key::DownArrow,
        KeyCode::ArrowLeft => enigo::Key::LeftArrow,
        KeyCode::ArrowRight => enigo::Key::RightArrow,
        KeyCode::Home => enigo::Key::Home,
        KeyCode::End => enigo::Key::End,
        KeyCode::PageUp => enigo::Key::PageUp,
        KeyCode::PageDown => enigo::Key::PageDown,
        KeyCode::F1 => enigo::Key::F1,
        KeyCode::F2 => enigo::Key::F2,
        KeyCode::F3 => enigo::Key::F3,
        KeyCode::F4 => enigo::Key::F4,
        KeyCode::F5 => enigo::Key::F5,
        KeyCode::F6 => enigo::Key::F6,
        KeyCode::F7 => enigo::Key::F7,
        KeyCode::F8 => enigo::Key::F8,
        KeyCode::F9 => enigo::Key::F9,
        KeyCode::F10 => enigo::Key::F10,
        KeyCode::F11 => enigo::Key::F11,
        KeyCode::F12 => enigo::Key::F12,
        KeyCode::LeftShift | KeyCode::RightShift => enigo::Key::Shift,
        KeyCode::LeftCtrl | KeyCode::RightCtrl => enigo::Key::Control,
        KeyCode::LeftAlt | KeyCode::RightAlt => enigo::Key::Alt,
        KeyCode::LeftMeta | KeyCode::RightMeta => enigo::Key::Meta,
        _ => enigo::Key::Unicode('\0'),
    }
}
