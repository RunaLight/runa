use std::collections::HashMap;
use std::collections::VecDeque;

use crate::components::Camera;
use crate::components::Time;
use runa_asset::Handle;
use runa_asset::TextureAsset;
use runa_ecs::World;
use runa_render_api::RenderCommands;
use runa_render_api::RenderQueue;
use runa_render_api::TextOutline;
use winit::event::ElementState;
use winit::event::KeyEvent;
use winit::keyboard::Key;
use winit::keyboard::KeyCode;

/// Trait for console commands that only need message output.
pub trait ConsoleCommand: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&mut self, args: &[&str], out: &mut dyn FnMut(String));
    fn detailed_help(&self) -> Vec<&str> {
        vec![]
    }
}

struct EchoCommand;
struct ClearCommand;
struct TestCommand;
struct VersionCommand;

impl ConsoleCommand for EchoCommand {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "Print text to the console"
    }
    fn execute(&mut self, args: &[&str], out: &mut dyn FnMut(String)) {
        let text = args.join(" ");
        out(text);
    }
    fn detailed_help(&self) -> Vec<&str> {
        vec![
            "echo <text> - Print text to the console.",
            "Example: echo Hello world",
        ]
    }
}

impl ConsoleCommand for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }
    fn description(&self) -> &str {
        "Clear console output"
    }
    fn execute(&mut self, _args: &[&str], out: &mut dyn FnMut(String)) {
        // Handled specially in try_execute - messages cleared there
        out("Console cleared.".to_string());
    }
    fn detailed_help(&self) -> Vec<&str> {
        vec!["clear / cls - Clear all messages from the console."]
    }
}

impl ConsoleCommand for TestCommand {
    fn name(&self) -> &str {
        "test"
    }
    fn description(&self) -> &str {
        "Run a test to verify the console works"
    }
    fn execute(&mut self, _args: &[&str], out: &mut dyn FnMut(String)) {
        out("Test command executed successfully!".to_string());
    }
    fn detailed_help(&self) -> Vec<&str> {
        vec!["test - Run a test to verify the console works."]
    }
}

impl ConsoleCommand for VersionCommand {
    fn name(&self) -> &str {
        "version"
    }
    fn description(&self) -> &str {
        "Show engine version info"
    }
    fn execute(&mut self, _args: &[&str], out: &mut dyn FnMut(String)) {
        out(format!("Runa Engine v{}", env!("CARGO_PKG_VERSION")));
    }
    fn detailed_help(&self) -> Vec<&str> {
        vec!["version / ver - Show engine version info."]
    }
}

/// Developer console for in-game commands and debugging
pub struct Console {
    messages: VecDeque<String>,
    max_messages: usize,
    is_visible: bool,
    pub input_buffer: String,
    history: VecDeque<String>,
    history_index: Option<usize>,
    #[allow(dead_code)]
    font_texture: Option<Handle<TextureAsset>>,

    /// Whether to show the stats overlay (FPS, frame time, render time, etc.)
    pub show_stats: bool,
    /// Current FPS value, set externally by the app
    pub current_fps: f32,
    /// Current frame time in ms, set externally by the app
    pub current_frame_time_ms: f32,
    /// Current render time in ms, set externally by the app
    pub current_render_time_ms: f32,
    /// Current update time in ms, set externally by the app
    pub current_update_time_ms: f32,
    /// Current draw call count, set externally by the app
    pub draw_call_count: usize,
    /// Current FPS cap (0 = unlimited)
    pub fps_max: f32,
    /// Current time scale (game speed multiplier)
    pub time_scale: f32,

    pub debug_show_ui_bounds: bool,
    pub debug_show_cursor_bounds: bool,
    pub debug_draw_collisions: bool,

    /// Registered simple commands (message-only)
    commands: HashMap<String, Box<dyn ConsoleCommand>>,
    /// Ordered command names for consistent display
    command_order: Vec<String>,
    /// Extra names for suggestions (not executable commands, e.g. editor.*)
    suggestion_names: Vec<String>,
    /// Currently highlighted suggestion index (for tab cycling)
    suggestion_index: Option<usize>,
}

impl Console {
    pub fn new() -> Self {
        let mut console = Self {
            messages: VecDeque::new(),
            max_messages: 500,
            is_visible: false,
            input_buffer: String::new(),
            history: VecDeque::new(),
            history_index: None,
            font_texture: None,
            show_stats: false,
            current_fps: 0.0,
            current_frame_time_ms: 0.0,
            current_render_time_ms: 0.0,
            current_update_time_ms: 0.0,
            draw_call_count: 0,
            fps_max: 0.0,
            time_scale: 1.0,
            debug_show_ui_bounds: false,
            debug_show_cursor_bounds: false,
            debug_draw_collisions: false,
            commands: HashMap::new(),
            command_order: Vec::new(),
            suggestion_names: Vec::new(),
            suggestion_index: None,
        };

        let builtin: Vec<Box<dyn ConsoleCommand>> = vec![
            Box::new(EchoCommand),
            Box::new(ClearCommand),
            Box::new(TestCommand),
            Box::new(VersionCommand),
        ];
        for cmd in builtin {
            console.register_command(cmd);
        }

        console.add_message("Runa Console initialized. Press ` to open.");
        console
    }

    /// Register a custom command
    pub fn register_command(&mut self, command: Box<dyn ConsoleCommand>) {
        let name = command.name().to_string();
        if !self.commands.contains_key(&name) {
            self.command_order.push(name.clone());
        }
        self.commands.insert(name, command);
    }

    /// Register multiple commands at once
    pub fn register_commands(&mut self, commands: Vec<Box<dyn ConsoleCommand>>) {
        for cmd in commands {
            self.register_command(cmd);
        }
    }

    /// Unregister a previously registered command
    pub fn unregister_command(&mut self, name: &str) {
        self.commands.remove(name);
        self.command_order.retain(|n| n != name);
    }

    /// Get all registered command names
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.command_order.iter().map(|s| s.as_str())
    }

    /// Check if a command is registered
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn toggle(&mut self) {
        self.is_visible = !self.is_visible;
        if self.is_visible {
            self.input_buffer.clear();
            self.history_index = None;
            self.suggestion_index = None;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible;
        if !visible {
            self.input_buffer.clear();
            self.history_index = None;
            self.suggestion_index = None;
        }
    }

    pub fn add_message<S: Into<String>>(&mut self, message: S) {
        let msg = message.into();
        self.messages.push_back(msg);
        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Get current messages (for custom rendering like in editor)
    pub fn messages(&self) -> impl Iterator<Item = &str> {
        self.messages.iter().map(|s| s.as_str())
    }

    /// Add extra names that appear in suggestions but aren't executable commands
    pub fn add_suggestion_names(&mut self, names: &[&str]) {
        for name in names {
            let n = name.to_string();
            if !self.command_order.contains(&n) && !self.suggestion_names.contains(&n) {
                self.suggestion_names.push(n);
            }
        }
    }

    /// Get matching command names for the current input buffer prefix
    pub fn matching_commands(&self) -> Vec<String> {
        let prefix = self.input_buffer.trim().to_lowercase();
        if prefix.is_empty() {
            return vec![];
        }
        let mut results: Vec<String> = self
            .command_order
            .iter()
            .chain(self.suggestion_names.iter())
            .filter(|name| name.starts_with(&prefix))
            .cloned()
            .collect();

        // Add built-in special commands
        let builtin_commands = [
            "help",
            "fps_max",
            "show_stats",
            "stats",
            "bind",
            "unbind",
            "binds",
            "timescale",
            "quit",
            "cls",
            "show_ui_bounds",
            "ui_bounds",
            "show_cursor",
            "cursor_bounds",
            "show_collision",
            "collision_bounds",
        ];
        for name in &builtin_commands {
            if name.starts_with(&prefix) && !results.contains(&name.to_string()) {
                results.push(name.to_string());
            }
        }
        // Add aliases
        if "editor".starts_with(&prefix) && !results.contains(&"editor".to_string()) {
            results.push("editor".to_string());
        }
        results.sort();
        results
    }

    /// Current selected suggestion index
    pub fn selected_suggestion(&self) -> Option<usize> {
        self.suggestion_index
    }

    /// Collect all command descriptions for help display
    pub fn all_command_descriptions(&self) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = self
            .command_order
            .iter()
            .filter_map(|name| {
                self.commands
                    .get(name)
                    .map(|cmd| (cmd.name().to_string(), cmd.description().to_string()))
            })
            .collect();
        result.push(("help".to_string(), "Show help for commands".to_string()));
        result.push((
            "fps_max [value]".to_string(),
            "Set FPS cap (0 = unlimited)".to_string(),
        ));
        result.push((
            "show_stats".to_string(),
            "Toggle stats overlay (FPS, frame time, etc.)".to_string(),
        ));
        result.push(("stats".to_string(), "Alias for show_stats".to_string()));
        result.push((
            "bind <key> <action>".to_string(),
            "Bind a key to an action".to_string(),
        ));
        result.push((
            "unbind <action> [key]".to_string(),
            "Unbind a key from an action".to_string(),
        ));
        result.push((
            "binds".to_string(),
            "List all action-key bindings".to_string(),
        ));
        result.push((
            "timescale [value]".to_string(),
            "Set/get game speed multiplier".to_string(),
        ));
        result.push((
            "show_ui_bounds".to_string(),
            "Toggle UI boundary overlay".to_string(),
        ));
        result.push((
            "show_cursor".to_string(),
            "Toggle CursorInteractable bounds overlay".to_string(),
        ));
        result.push((
            "show_collision".to_string(),
            "Toggle collision bounds overlay".to_string(),
        ));
        result.push(("quit".to_string(), "Close the console".to_string()));
        // Add "cls" alias for "clear"
        if let Some(pos) = result.iter().position(|(n, _)| n == "clear") {
            let desc = result[pos].1.clone();
            result.push(("cls".to_string(), format!("{} (alias)", desc)));
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Try to execute the current input buffer as a command.
    /// Returns true if the command was found and executed, false if not found.
    pub fn try_execute(&mut self, command: &str, world: &mut World) -> bool {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return true;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_lowercase();
        let args: Vec<&str> = parts.collect();

        match cmd.as_str() {
            "help" => {
                if args.is_empty() {
                    let entries = self.all_command_descriptions();
                    self.add_message("Available commands:");
                    for (name, desc) in entries {
                        self.add_message(format!("  {:<12} {}", name, desc));
                    }
                } else {
                    let topic = args[0];
                    match topic {
                        "help" => {
                            self.add_message("help [cmd] - Show help for a specific command.");
                        }
                        "fps_max" => {
                            self.add_message("fps_max [value] - Set FPS cap.");
                            self.add_message("  Without argument, shows current cap.");
                            self.add_message("  With a number, caps FPS to that value.");
                            self.add_message("  Use 0 for unlimited FPS.");
                        }
                        "show_stats" | "stats" => {
                            self.add_message("show_stats / stats - Toggle stats overlay.");
                            self.add_message("  Shows FPS, frame time, render time, update time, and draw calls.");
                        }
                        "bind" => {
                            self.add_message("bind <key> <action> - Bind a key to an action.");
                            self.add_message("  Example: bind w move_forward");
                            self.add_message("  Example: bind mouseleft attack");
                        }
                        "unbind" => {
                            self.add_message(
                                "unbind <action> [key] - Unbind a key from an action.",
                            );
                            self.add_message("  Without key, unbinds ALL keys from the action.");
                            self.add_message("  Example: unbind move_forward w");
                            self.add_message("  Example: unbind move_forward");
                        }
                        "binds" => {
                            self.add_message("binds - List all action-key bindings.");
                            self.add_message(
                                "  Shows all registered actions and their bound keys.",
                            );
                        }
                        "timescale" => {
                            self.add_message("timescale [value] - Set/get game speed multiplier.");
                            self.add_message(
                                "  Default: 1.0. Use 0.5 for half speed, 2.0 for double speed.",
                            );
                        }
                        "show_ui_bounds" | "ui_bounds" => {
                            self.add_message(
                                "show_ui_bounds / ui_bounds - Toggle UI node boundary overlay.",
                            );
                            self.add_message(
                                "  Draws green outlines around all visible UI elements.",
                            );
                        }
                        "show_cursor" | "cursor_bounds" => {
                            self.add_message("show_cursor / cursor_bounds - Toggle CursorInteractable bounds overlay.");
                            self.add_message("  Draws cyan outlines around interactive objects.");
                        }
                        "show_collision" | "collision_bounds" => {
                            self.add_message("show_collision / collision_bounds - Toggle collision bounds overlay.");
                            self.add_message(
                                "  Draws orange outlines around PhysicsCollision objects.",
                            );
                        }
                        "quit" | "exit" => {
                            self.add_message("quit / exit - Close the console.");
                        }
                        _ => {
                            let help_info = self.commands.get(topic).map(|c| {
                                (
                                    c.name().to_string(),
                                    c.description().to_string(),
                                    c.detailed_help()
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect::<Vec<_>>(),
                                )
                            });
                            if let Some((name, desc, help_lines)) = help_info {
                                if help_lines.is_empty() {
                                    self.add_message(format!("{}: {}", name, desc));
                                } else {
                                    for line in help_lines {
                                        self.add_message(line);
                                    }
                                }
                            } else {
                                self.add_message(format!("No help for '{}'.", topic));
                            }
                        }
                    }
                }
                true
            }
            "fps_max" => {
                if args.is_empty() {
                    if self.fps_max > 0.0 {
                        self.add_message(format!(
                            "FPS cap: {:.0} ({} ms per frame)",
                            self.fps_max,
                            1000.0 / self.fps_max
                        ));
                    } else {
                        self.add_message("FPS cap: unlimited");
                    }
                } else if let Ok(value) = args[0].parse::<f32>() {
                    self.fps_max = if value.is_finite() {
                        value.max(0.0)
                    } else {
                        0.0
                    };
                    if self.fps_max > 0.0 {
                        self.add_message(format!(
                            "FPS cap set to {:.0} ({} ms per frame)",
                            self.fps_max,
                            1000.0 / self.fps_max
                        ));
                    } else {
                        self.add_message("FPS cap: unlimited");
                    }
                } else {
                    self.add_message(format!(
                        "Invalid value: '{}'. Use a number or 0 for unlimited.",
                        args[0]
                    ));
                }
                true
            }
            "show_stats" | "stats" => {
                self.show_stats = !self.show_stats;
                if self.show_stats {
                    self.add_message("Stats overlay: ON");
                } else {
                    self.add_message("Stats overlay: OFF");
                }
                true
            }
            "bind" => {
                if args.len() < 2 {
                    self.add_message("Usage: bind <key> <action>");
                    self.add_message("Examples: bind w move_forward, bind mouseleft attack");
                } else {
                    let key_str = args[0];
                    let action = args[1..].join("_");
                    if let Some(binding) = crate::input::parse_input_binding(key_str) {
                        crate::input::bind_action(&action, binding);
                        self.add_message(format!("Bound '{}' to '{}'", key_str, action));
                    } else {
                        self.add_message(format!("Unknown key: '{}'", key_str));
                    }
                }
                true
            }
            "unbind" => {
                if args.is_empty() {
                    self.add_message("Usage: unbind <action> [key]");
                } else if args.len() == 1 {
                    let action = args[0];
                    crate::input::unbind_action_all(action);
                    self.add_message(format!("Unbound all keys from '{}'", action));
                } else {
                    let action = args[0];
                    let key_str = args[1];
                    if let Some(binding) = crate::input::parse_input_binding(key_str) {
                        crate::input::unbind_action(action, &binding);
                        self.add_message(format!("Unbound '{}' from '{}'", key_str, action));
                    } else {
                        self.add_message(format!("Unknown key: '{}'", key_str));
                    }
                }
                true
            }
            "binds" => {
                let bindings = crate::input::list_action_bindings();
                if bindings.is_empty() {
                    self.add_message("No actions registered.");
                } else {
                    self.add_message("Action bindings:");
                    for (action, binds) in &bindings {
                        let keys: Vec<String> = binds.iter().map(|b| format!("{}", b)).collect();
                        self.add_message(format!("  {:<20} {}", action, keys.join(", ")));
                    }
                }
                true
            }
            "timescale" => {
                if args.is_empty() {
                    self.add_message(format!("Current timescale: {:.2}", self.time_scale));
                } else if let Ok(value) = args[0].parse::<f32>() {
                    let time_scale = value.clamp(0.01, 100.0);
                    world.get_resource_mut::<Time>().unwrap().time_scale = time_scale;
                    self.add_message(format!("Timescale set to {:.2}", time_scale));
                } else {
                    self.add_message(format!(
                        "Invalid value: '{}'. Use a number (0.01-100.0).",
                        args[0]
                    ));
                }
                true
            }
            "show_ui_bounds" | "ui_bounds" => {
                self.debug_show_ui_bounds = !self.debug_show_ui_bounds;
                self.add_message(format!(
                    "UI bounds: {}",
                    if self.debug_show_ui_bounds {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                true
            }
            "show_cursor" | "cursor_bounds" => {
                self.debug_show_cursor_bounds = !self.debug_show_cursor_bounds;
                self.add_message(format!(
                    "Cursor bounds: {}",
                    if self.debug_show_cursor_bounds {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                true
            }
            "show_collision" | "collision_bounds" => {
                self.debug_draw_collisions = !self.debug_draw_collisions;
                self.add_message(format!(
                    "Collision bounds: {}",
                    if self.debug_draw_collisions {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                true
            }
            "quit" | "exit" => {
                self.is_visible = false;
                self.add_message("Console closed.");
                true
            }
            "cls" => {
                self.messages.clear();
                self.add_message("Console cleared.");
                true
            }
            _ => {
                if let Some(mut cmd_obj) = self.commands.remove(&cmd) {
                    let out = &mut self.messages;
                    // For "clear", clear messages first
                    if cmd == "clear" {
                        out.clear();
                    }
                    cmd_obj.execute(&args, &mut |msg| {
                        out.push_back(msg);
                        if out.len() > self.max_messages {
                            out.pop_front();
                        }
                    });
                    self.commands.insert(cmd, cmd_obj);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Execute command and add error message if not found
    pub fn execute_command(&mut self, command: &str, world: &mut World) {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return;
        }
        if !self.try_execute(trimmed, world) {
            self.add_message(format!(
                "Unknown command: '{}'. Type 'help' for commands.",
                trimmed
            ));
        }
    }

    /// Submit the current input buffer as a command
    pub fn submit_input(&mut self, world: &mut World) {
        if self.input_buffer.is_empty() {
            return;
        }
        let cmd = self.input_buffer.clone();
        self.history.push_back(cmd.clone());
        if self.history.len() > 50 {
            self.history.pop_front();
        }
        self.history_index = None;
        self.suggestion_index = None;
        self.add_message(format!("> {}", cmd));
        self.execute_command(&cmd, world);
        self.input_buffer.clear();
    }

    /// Push an entry to command history without executing
    pub fn push_history(&mut self, entry: &str) {
        self.history.push_back(entry.to_string());
        if self.history.len() > 50 {
            self.history.pop_front();
        }
        self.history_index = None;
        self.suggestion_index = None;
    }

    /// Advance the suggestion index forward and insert into buffer (called by Tab).
    pub fn advance_suggestion(&mut self) -> Option<String> {
        let matches = self.matching_commands();
        if matches.is_empty() {
            self.suggestion_index = None;
            return None;
        }
        let new_index = match self.suggestion_index {
            Some(i) => (i + 1) % matches.len(),
            None => 0,
        };
        self.suggestion_index = Some(new_index);
        let selected = matches[new_index].clone();
        self.input_buffer = format!("{} ", selected);
        Some(selected)
    }

    /// Cycle selection to the next suggestion (called by Right arrow), does not modify input buffer.
    pub fn select_next_suggestion(&mut self) {
        let matches = self.matching_commands();
        if matches.is_empty() {
            self.suggestion_index = None;
            return;
        }
        self.suggestion_index = Some(match self.suggestion_index {
            Some(i) => (i + 1) % matches.len(),
            None => 0,
        });
    }

    /// Cycle selection to the previous suggestion (called by Left arrow), does not modify input buffer.
    pub fn select_previous_suggestion(&mut self) {
        let matches = self.matching_commands();
        if matches.is_empty() {
            self.suggestion_index = None;
            return;
        }
        self.suggestion_index = Some(match self.suggestion_index {
            Some(i) => (i + matches.len() - 1) % matches.len(),
            None => matches.len() - 1,
        });
    }

    /// Reset the suggestion index (called on any non-Tab input)
    pub fn reset_suggestion(&mut self) {
        self.suggestion_index = None;
    }

    /// Get current command history
    pub fn history(&self) -> impl Iterator<Item = &str> {
        self.history.iter().map(|s| s.as_str())
    }

    pub fn navigate_history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            Some(i) => i.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.history_index = Some(new_index);
        if let Some(cmd) = self.history.get(new_index) {
            self.input_buffer = cmd.clone();
        }
    }

    pub fn navigate_history_down(&mut self) {
        if let Some(current_index) = self.history_index {
            if current_index < self.history.len() - 1 {
                let new_index = current_index + 1;
                self.history_index = Some(new_index);
                if let Some(cmd) = self.history.get(new_index) {
                    self.input_buffer = cmd.clone();
                }
            } else {
                self.history_index = None;
                self.input_buffer.clear();
            }
        }
    }

    /// Process a keyboard event (runtime path, uses winit)
    pub fn handle_keyboard(&mut self, event: &KeyEvent, state: ElementState, world: &mut World) {
        if state == ElementState::Pressed
            && event.physical_key == winit::keyboard::PhysicalKey::Code(KeyCode::Backquote)
        {
            if !event.repeat {
                self.toggle();
                if self.is_visible {
                    self.add_message("Console opened. Type 'help' for available commands.");
                }
            }
            return;
        }

        if !self.is_visible {
            return;
        }

        if state != ElementState::Pressed {
            return;
        }

        match event.physical_key {
            winit::keyboard::PhysicalKey::Code(KeyCode::Enter) => {
                self.submit_input(world);
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::Backspace) => {
                self.input_buffer.pop();
                self.reset_suggestion();
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::Escape) => {
                self.is_visible = false;
                self.input_buffer.clear();
                self.suggestion_index = None;
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowUp) => {
                self.navigate_history_up();
                self.reset_suggestion();
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowDown) => {
                self.navigate_history_down();
                self.reset_suggestion();
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::Tab) => {
                self.advance_suggestion();
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.select_next_suggestion();
                return;
            }
            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.select_previous_suggestion();
                return;
            }
            _ => {}
        }

        if let Key::Character(c) = &event.logical_key {
            for ch in c.chars() {
                if (' '..='~').contains(&ch) {
                    self.input_buffer.push(ch);
                }
            }
            self.reset_suggestion();
        }
    }

    /// Renders the console overlay and optional stats overlay.
    /// The stats overlay is shown even when the console is hidden.
    pub fn render(&self, queue: &mut RenderQueue, camera: &Camera) {
        // Stats overlay (always visible if enabled)
        if self.show_stats {
            let color = [0.0, 1.0, 0.0, 1.0];
            let outline = Some(TextOutline {
                color: [0.0, 0.0, 0.0, 1.0],
                width: 1.0,
            });

            let fps_text = format!("FPS: {:.1}", self.current_fps);
            let frame_text = format!("Frame: {:.1} ms", self.current_frame_time_ms);
            let render_text = format!("Render: {:.1} ms", self.current_render_time_ms);
            let update_text = format!("Update: {:.1} ms", self.current_update_time_ms);
            let fps_cap = if self.fps_max > 0.0 {
                format!("FPS Cap: {:.0}", self.fps_max)
            } else {
                "FPS Cap: unlimited".to_string()
            };
            let draw_text = format!("Draw calls: {}", self.draw_call_count);

            let mut y = 8.0;
            let line_h = 19.0;
            let stats_lines = [
                fps_text,
                frame_text,
                render_text,
                update_text,
                fps_cap,
                draw_text,
            ];
            for line in &stats_lines {
                queue.commands.push(RenderCommands::Text {
                    text: line.clone(),
                    position: glam::Vec2::new(8.0, y),
                    color,
                    size: 16.0,
                    outline: outline.clone(),
                });
                y += line_h;
            }
        }

        if !self.is_visible {
            return;
        }

        let screen_w = camera.viewport_size.0 as f32;
        let screen_h = camera.viewport_size.1 as f32;

        let text_size = 16.0;
        let line_h = text_size * 1.2;
        let padding = text_size;

        let cx = padding;
        let cy = padding;
        let cw = screen_w - padding * 2.0;
        let ch = screen_h * 0.5;

        // Semi-transparent background
        queue.commands.push(RenderCommands::DebugRect {
            position: glam::Vec3::new(cx + cw / 2.0, cy + ch / 2.0, 0.0),
            size: glam::Vec2::new(cw, ch),
            color: [0.0, 0.0, 0.0, 0.75],
        });

        // Text area inside the console
        let text_x = cx + padding;
        let input_y = cy + ch - line_h - padding;

        // Suggestions panel height
        let suggestions = self.matching_commands();
        let has_suggestions = !suggestions.is_empty();

        // Available height for messages
        let messages_bottom = input_y - padding;
        let messages_top = cy + padding;
        let max_lines = ((messages_bottom - messages_top) / line_h) as usize;

        // Messages (newest at bottom, above input)
        let mut line_y = input_y - line_h;
        for msg in self.messages.iter().rev().take(max_lines) {
            queue.commands.push(RenderCommands::Text {
                text: msg.clone(),
                position: glam::Vec2::new(text_x, line_y),
                color: [1.0, 1.0, 1.0, 1.0],
                size: text_size,
                outline: None,
            });
            line_y -= line_h;
        }

        // Input line with blinking cursor
        let blink = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            / 500)
            & 1
            == 0;
        let cursor = if blink { "█" } else { " " };

        queue.commands.push(RenderCommands::Text {
            text: format!("> {}{}", self.input_buffer, cursor),
            position: glam::Vec2::new(text_x, input_y),
            color: [0.0, 1.0, 0.0, 1.0],
            size: text_size,
            outline: None,
        });

        // Suggestions as an opaque panel BELOW the input line
        if has_suggestions {
            let panel_y = input_y + line_h;
            let panel_h = line_h + padding * 0.5;

            // Opaque background panel
            queue.commands.push(RenderCommands::DebugRect {
                position: glam::Vec3::new(cx + cw / 2.0, panel_y + panel_h / 2.0, 0.0),
                size: glam::Vec2::new(cw, panel_h),
                color: [0.05, 0.05, 0.05, 0.92],
            });

            // Border line at top of suggestion panel
            queue.commands.push(RenderCommands::DebugRect {
                position: glam::Vec3::new(cx + cw / 2.0, panel_y + 0.5, 0.0),
                size: glam::Vec2::new(cw - 2.0, 1.0),
                color: [0.3, 0.5, 0.8, 0.8],
            });

            // Suggestion text with highlight on selected
            let suggestion_text: String = suggestions
                .iter()
                .enumerate()
                .take(16)
                .map(|(i, s)| {
                    if self.suggestion_index == Some(i) {
                        format!("[{}]", s)
                    } else {
                        s.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("  ");

            queue.commands.push(RenderCommands::Text {
                text: suggestion_text,
                position: glam::Vec2::new(text_x, panel_y + padding * 0.25),
                color: [0.6, 0.8, 1.0, 1.0],
                size: text_size,
                outline: Some(TextOutline {
                    color: [0.0, 0.0, 0.0, 0.8],
                    width: 0.5,
                }),
            });
        }
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}
