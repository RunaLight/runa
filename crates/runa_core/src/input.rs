use core::fmt;
use glam::Vec3;
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock, Weak},
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::{CursorGrabMode, Window};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::components::Camera;

static WINDOW_HANDLE: OnceLock<Mutex<Weak<Window>>> = OnceLock::new();
static WINDOW_STATE: OnceLock<Mutex<WindowState>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct WindowState {
    pub title: String,
    pub fullscreen: bool,
    pub size: (u32, u32),
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            title: "Runa Game".to_string(),
            fullscreen: false,
            size: (1280, 720),
        }
    }
}

/// Represents a physical input that can be bound to an action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputBinding {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl fmt::Display for InputBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputBinding::Key(kc) => write!(f, "{:?}", kc),
            InputBinding::Mouse(mb) => write!(f, "{:?}", mb),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct InputState {
    pub keys_pressed: HashSet<KeyCode>,
    pub keys_just_pressed: HashSet<KeyCode>,
    // pub keys_just_released: HashSet<KeyCode>,
    pub mouse_position: (f32, f32),
    pub mouse_previous_position: (f32, f32),
    pub mouse_delta: (f32, f32), // Relative mouse movement (for locked cursor)
    pub mouse_buttons_pressed: HashSet<MouseButton>,
    pub mouse_buttons_just_pressed: HashSet<MouseButton>,
    pub mouse_buttons_just_released: HashSet<MouseButton>,
    pub mouse_wheel_delta: f32,

    pub camera: Option<Camera>,

    /// Action bindings: action name -> set of bound inputs
    pub actions: HashMap<String, HashSet<InputBinding>>,
    /// Track which default action sets have been registered
    pub default_actions_registered: bool,
}

// ===== Input Actions System =====

impl InputState {
    /// Register an action with a default set of bindings.
    /// If the action already exists, new bindings are added to existing ones.
    pub fn register_action(&mut self, name: &str, default_binds: Vec<InputBinding>) {
        let entry = self.actions.entry(name.to_string()).or_default();
        for bind in default_binds {
            entry.insert(bind);
        }
    }

    /// Check if an action is currently pressed (any bound input is held).
    pub fn is_action_pressed(&self, name: &str) -> bool {
        let state = self;
        let Some(binds) = state.actions.get(name) else {
            return false;
        };
        for bind in binds {
            match bind {
                InputBinding::Key(kc) => {
                    if state.keys_pressed.contains(kc) {
                        return true;
                    }
                }
                InputBinding::Mouse(mb) => {
                    if state.mouse_buttons_pressed.contains(mb) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if an action was just pressed this frame (any bound input).
    pub fn is_action_just_pressed(&self, name: &str) -> bool {
        let state = self;
        let Some(binds) = state.actions.get(name) else {
            return false;
        };
        for bind in binds {
            match bind {
                InputBinding::Key(kc) => {
                    if state.keys_just_pressed.contains(kc) {
                        return true;
                    }
                }
                InputBinding::Mouse(mb) => {
                    if state.mouse_buttons_just_pressed.contains(mb) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Bind an input to an action.
    pub fn bind_action(&mut self, action: &str, bind: InputBinding) {
        let state = self;
        state
            .actions
            .entry(action.to_string())
            .or_default()
            .insert(bind);
    }

    /// Unbind a specific input from an action.
    pub fn unbind_action(&mut self, action: &str, bind: &InputBinding) {
        if let Some(binds) = self.actions.get_mut(action) {
            binds.remove(bind);
        }
    }

    /// Remove all bindings for an action.
    pub fn unbind_action_all(&mut self, action: &str) {
        self.actions.remove(action);
    }

    /// Get all registered action names.
    pub fn list_actions(&mut self) -> Vec<String> {
        let mut names: Vec<String> = self.actions.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get bindings for an action.
    pub fn action_bindings(&mut self, action: &str) -> Option<Vec<InputBinding>> {
        self.actions
            .get(action)
            .map(|binds| binds.iter().cloned().collect())
    }
}

/// Parse an input binding from a string (e.g. "KeyW", "MouseLeft").
pub fn parse_input_binding(s: &str) -> Option<InputBinding> {
    // Mouse buttons
    let lower = s.to_lowercase();
    let btn = match lower.as_str() {
        "mouseleft" | "leftmouse" | "lmb" => Some(MouseButton::Left),
        "mouseright" | "rightmouse" | "rmb" => Some(MouseButton::Right),
        "mousemiddle" | "middlemouse" | "mmb" => Some(MouseButton::Middle),
        "mouseback" | "backmouse" => Some(MouseButton::Back),
        "mouseforward" | "forwardmouse" => Some(MouseButton::Forward),
        _ => None,
    };
    if let Some(b) = btn {
        return Some(InputBinding::Mouse(b));
    }

    // Keyboard keys
    let key = match lower.as_str() {
        "keyw" | "w" => Some(KeyCode::KeyW),
        "keya" | "a" => Some(KeyCode::KeyA),
        "keys" | "s" => Some(KeyCode::KeyS),
        "keyd" | "d" => Some(KeyCode::KeyD),
        "keyq" | "q" => Some(KeyCode::KeyQ),
        "keye" | "e" => Some(KeyCode::KeyE),
        "keyr" | "r" => Some(KeyCode::KeyR),
        "keyt" | "t" => Some(KeyCode::KeyT),
        "keyy" | "y" => Some(KeyCode::KeyY),
        "keyu" | "u" => Some(KeyCode::KeyU),
        "keyi" | "i" => Some(KeyCode::KeyI),
        "keyo" | "o" => Some(KeyCode::KeyO),
        "keyp" | "p" => Some(KeyCode::KeyP),
        "keyf" | "f" => Some(KeyCode::KeyF),
        "keyg" | "g" => Some(KeyCode::KeyG),
        "keyh" | "h" => Some(KeyCode::KeyH),
        "keyj" | "j" => Some(KeyCode::KeyJ),
        "keyk" | "k" => Some(KeyCode::KeyK),
        "keyl" | "l" => Some(KeyCode::KeyL),
        "keyz" | "z" => Some(KeyCode::KeyZ),
        "keyx" | "x" => Some(KeyCode::KeyX),
        "keyc" | "c" => Some(KeyCode::KeyC),
        "keyv" | "v" => Some(KeyCode::KeyV),
        "keyb" | "b" => Some(KeyCode::KeyB),
        "keyn" | "n" => Some(KeyCode::KeyN),
        "keym" | "m" => Some(KeyCode::KeyM),
        "space" => Some(KeyCode::Space),
        "shift" | "shiftleft" => Some(KeyCode::ShiftLeft),
        "shiftright" => Some(KeyCode::ShiftRight),
        "control" | "ctrl" | "controlleft" => Some(KeyCode::ControlLeft),
        "controlright" | "ctrlright" => Some(KeyCode::ControlRight),
        "alt" | "altleft" => Some(KeyCode::AltLeft),
        "altright" => Some(KeyCode::AltRight),
        "escape" | "esc" => Some(KeyCode::Escape),
        "enter" | "return" => Some(KeyCode::Enter),
        "backspace" => Some(KeyCode::Backspace),
        "tab" => Some(KeyCode::Tab),
        "tilde" | "backquote" | "`" => Some(KeyCode::Backquote),
        "up" | "arrowup" => Some(KeyCode::ArrowUp),
        "down" | "arrowdown" => Some(KeyCode::ArrowDown),
        "left" | "arrowleft" => Some(KeyCode::ArrowLeft),
        "right" | "arrowright" => Some(KeyCode::ArrowRight),
        "f1" => Some(KeyCode::F1),
        "f2" => Some(KeyCode::F2),
        "f3" => Some(KeyCode::F3),
        "f4" => Some(KeyCode::F4),
        "f5" => Some(KeyCode::F5),
        "f6" => Some(KeyCode::F6),
        "f7" => Some(KeyCode::F7),
        "f8" => Some(KeyCode::F8),
        "f9" => Some(KeyCode::F9),
        "f10" => Some(KeyCode::F10),
        "f11" => Some(KeyCode::F11),
        "f12" => Some(KeyCode::F12),
        "0" => Some(KeyCode::Digit0),
        "1" => Some(KeyCode::Digit1),
        "2" => Some(KeyCode::Digit2),
        "3" => Some(KeyCode::Digit3),
        "4" => Some(KeyCode::Digit4),
        "5" => Some(KeyCode::Digit5),
        "6" => Some(KeyCode::Digit6),
        "7" => Some(KeyCode::Digit7),
        "8" => Some(KeyCode::Digit8),
        "9" => Some(KeyCode::Digit9),
        _ => None,
    };
    key.map(InputBinding::Key)
}

impl InputState {
    pub fn initialize(&mut self) {
        let _ = WINDOW_STATE.set(Mutex::new(WindowState::default()));
    }

    pub fn update_frame(&mut self) {
        let input_state = self;
        input_state.keys_just_pressed.clear();
        // input_state.keys_just_released.clear();
        input_state.mouse_buttons_just_pressed.clear();
        input_state.mouse_buttons_just_released.clear();
        input_state.mouse_wheel_delta = 0.0;

        // Update mouse previous position
        input_state.mouse_previous_position = input_state.mouse_position;

        // Reset mouse delta (will be set by MouseMoved event)
        input_state.mouse_delta = (0.0, 0.0);
    }

    pub fn is_key_pressed(&mut self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&mut self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_mouse_button_pressed(&mut self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn is_mouse_button_just_pressed(&mut self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    pub fn is_mouse_button_just_released(&mut self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    pub fn get_mouse_world_position(&mut self) -> Option<Vec3> {
        let input_state = self;
        if let Some(camera) = &input_state.camera {
            let pos_2d = camera.screen_to_world(input_state.mouse_position);
            Some(Vec3::new(pos_2d.x, pos_2d.y, 0.0)) // Z = 0 для совместимости с 2D
        } else {
            None
        }
    }
}

// ===== Cursor Control =====

/// Set the window handle for cursor control (call once at startup)
pub fn set_window_handle(window: &std::sync::Arc<Window>) {
    WINDOW_HANDLE
        .set(Mutex::new(std::sync::Arc::downgrade(window)))
        .ok();
}

pub fn initialize_window_state(title: impl Into<String>, fullscreen: bool, size: (u32, u32)) {
    let title = title.into();
    if let Some(state) = WINDOW_STATE.get() {
        if let Ok(mut state) = state.lock() {
            state.title = title;
            state.fullscreen = fullscreen;
            state.size = size;
        }
    }
}

fn with_window<R>(f: impl FnOnce(&Window) -> R) -> Option<R> {
    let window_weak = WINDOW_HANDLE.get()?;
    let guard = window_weak.lock().ok()?;
    let window = guard.upgrade()?;
    Some(f(&window))
}

fn with_window_state<R>(f: impl FnOnce(&mut WindowState) -> R) -> Option<R> {
    let state = WINDOW_STATE.get()?;
    let mut guard = state.lock().ok()?;
    Some(f(&mut guard))
}

pub fn window_title() -> Option<String> {
    let state = WINDOW_STATE.get()?;
    let guard = state.lock().ok()?;
    Some(guard.title.clone())
}

pub fn is_fullscreen() -> Option<bool> {
    let state = WINDOW_STATE.get()?;
    let guard = state.lock().ok()?;
    Some(guard.fullscreen)
}

pub fn window_size() -> Option<(u32, u32)> {
    let state = WINDOW_STATE.get()?;
    let guard = state.lock().ok()?;
    Some(guard.size)
}

pub fn set_window_title(title: impl Into<String>) {
    let title = title.into();
    let _ = with_window_state(|state| {
        state.title = title.clone();
    });
    let _ = with_window(|window| {
        window.set_title(&title);
    });
}

pub fn set_fullscreen(fullscreen: bool) {
    let _ = with_window_state(|state| {
        state.fullscreen = fullscreen;
    });
    let _ = with_window(|window| {
        if fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                window.current_monitor(),
            )));
        } else {
            window.set_fullscreen(None);
        }
    });
}

pub fn toggle_fullscreen() {
    let current = is_fullscreen().unwrap_or(false);
    set_fullscreen(!current);
}

pub fn set_window_size(width: u32, height: u32) {
    let size = (width.max(1), height.max(1));
    let _ = with_window_state(|state| {
        state.size = size;
    });
    let _ = with_window(|window| {
        let _ = window.request_inner_size(PhysicalSize::new(size.0, size.1));
    });
}

pub fn set_window_position(x: i32, y: i32) {
    let _ = with_window(|window| {
        window.set_outer_position(PhysicalPosition::new(x, y));
    });
}

pub fn move_window_by(dx: i32, dy: i32) {
    let _ = with_window(|window| {
        if let Ok(position) = window.outer_position() {
            window.set_outer_position(PhysicalPosition::new(
                position.x.saturating_add(dx),
                position.y.saturating_add(dy),
            ));
        }
    });
}

pub fn screen_center_position() -> Option<(i32, i32)> {
    with_window(|window| {
        let monitor = window.current_monitor()?;
        let position = monitor.position();
        let size = monitor.size();
        let center_x = position.x.saturating_add((size.width / 2) as i32);
        let center_y = position.y.saturating_add((size.height / 2) as i32);
        Some((center_x, center_y))
    })
    .flatten()
}

pub fn centered_window_position() -> Option<(i32, i32)> {
    with_window(|window| {
        let monitor = window.current_monitor()?;
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let window_size = window.outer_size();

        let x = monitor_position
            .x
            .saturating_add(((monitor_size.width as i64 - window_size.width as i64) / 2) as i32);
        let y = monitor_position
            .y
            .saturating_add(((monitor_size.height as i64 - window_size.height as i64) / 2) as i32);

        Some((x, y))
    })
    .flatten()
}

pub fn center_window() {
    if let Some((x, y)) = centered_window_position() {
        set_window_position(x, y);
    }
}

/// Show or hide the cursor
pub fn show_cursor(show: bool) {
    let _ = with_window(|window| {
        window.set_cursor_visible(show);
    });
}

/// Lock or unlock the cursor to/from the window
pub fn lock_cursor(lock: bool) {
    let _ = with_window(|window| {
        let grab_mode = if lock {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        let _ = window.set_cursor_grab(grab_mode);
    });
}

/// Convenience function to set cursor mode (useful for FPS games)
pub fn set_cursor_mode(visible: bool, locked: bool) {
    show_cursor(visible);
    lock_cursor(locked);
}
