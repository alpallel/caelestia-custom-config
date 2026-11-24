use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, glib};
use serde_json::Value;

const APP_ID: &str = "autoclicker-config";
const WINDOW_TITLE: &str = "Auto Clicker";
const MIN_CPS: i32 = 1;
const MAX_CPS: i32 = 100;
const DEFAULT_CPS: i32 = 20;
const MODES: [&str; 2] = ["Click", "Hold"];
const TARGETS: [&str; 2] = ["Mouse", "Keyboard"];
const BUTTONS: [&str; 2] = ["Left", "Right"];
const DEFAULT_KEYCODE: u32 = 57;
const DEFAULT_KEY_NAME: &str = "Space";
const MIN_KEYCODE: i32 = 1;
const MAX_KEYCODE: i32 = 767;

#[derive(Clone)]
struct Config {
    cps: i32,
    button: usize,
    mode: usize,
    target: usize,
    keycode: u32,
    key_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cps: DEFAULT_CPS,
            button: 0,
            mode: 0,
            target: 0,
            keycode: DEFAULT_KEYCODE,
            key_name: DEFAULT_KEY_NAME.to_string(),
        }
    }
}

struct AppState {
    cps: i32,
    button: usize,
    mode: usize,
    target: usize,
    keycode: u32,
    key_name: String,
    running: bool,
    capturing: bool,
    syncing: bool,
}

impl AppState {
    fn from_config(config: &Config) -> Self {
        Self {
            cps: config.cps,
            button: config.button,
            mode: config.mode,
            target: config.target,
            keycode: config.keycode,
            key_name: config.key_name.clone(),
            running: false,
            capturing: false,
            syncing: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpeedSource {
    Scale,
    Entry,
    Button,
}

#[derive(Clone)]
struct Ui {
    window: gtk::Window,
    status_label: gtk::Label,
    toggle_button: gtk::Button,
    mode_buttons: Vec<gtk::ToggleButton>,
    target_buttons: Vec<gtk::ToggleButton>,
    button_buttons: Vec<gtk::ToggleButton>,
    mouse_row: gtk::Box,
    key_row: gtk::Box,
    key_button: gtk::Button,
    speed_section: gtk::Box,
    entry: gtk::Entry,
    scale: gtk::Scale,
    minus_button: gtk::Button,
    plus_button: gtk::Button,
}

fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"))
        .join("caelestia")
}

fn config_file() -> PathBuf {
    config_dir().join("autoclicker.conf")
}

fn script_path() -> PathBuf {
    config_dir().join("scripts").join("autoclicker.sh")
}

fn read_config() -> Config {
    let mut config = Config::default();

    if let Ok(content) = fs::read_to_string(config_file()) {
        for line in content.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();

            match key {
                "cps" => {
                    if let Ok(cps) = value.parse::<i32>() {
                        config.cps = cps.clamp(MIN_CPS, MAX_CPS);
                    }
                }
                "button" => match value {
                    "left" => config.button = 0,
                    "right" => config.button = 1,
                    _ => {}
                },
                "mode" => match value {
                    "click" => config.mode = 0,
                    "hold" => config.mode = 1,
                    _ => {}
                },
                "target" => match value {
                    "mouse" => config.target = 0,
                    "key" => config.target = 1,
                    _ => {}
                },
                "keycode" => {
                    if let Ok(keycode) = value.parse::<u32>() {
                        if (MIN_KEYCODE as u32..=MAX_KEYCODE as u32).contains(&keycode) {
                            config.keycode = keycode;
                        }
                    }
                }
                "key_name" => config.key_name = value.to_string(),
                _ => {}
            }
        }
    }

    if config.key_name.trim().is_empty() {
        config.key_name = if config.keycode == DEFAULT_KEYCODE {
            DEFAULT_KEY_NAME.to_string()
        } else {
            format!("Key {}", config.keycode)
        };
    }

    config
}

fn write_config(config: &Config) -> std::io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;

    let file = config_file();
    let temporary = file.with_file_name("autoclicker.conf.tmp");
    let content = format!(
        "cps={}\nbutton={}\nmode={}\ntarget={}\nkeycode={}\nkey_name={}\n",
        config.cps,
        if config.button == 1 { "right" } else { "left" },
        if config.mode == 1 { "hold" } else { "click" },
        if config.target == 1 { "key" } else { "mouse" },
        config.keycode,
        config.key_name
    );

    fs::write(&temporary, content)?;
    fs::rename(temporary, file)
}

fn save_config(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let config = {
        let state = state.borrow();
        Config {
            cps: state.cps,
            button: state.button,
            mode: state.mode,
            target: state.target,
            keycode: state.keycode,
            key_name: state.key_name.clone(),
        }
    };

    if let Err(error) = write_config(&config) {
        ui.status_label
            .set_text(&format!("Could not save settings: {error}"));
    }
}

fn clicker_status() -> (bool, Option<i32>) {
    let Ok(output) = Command::new(script_path()).arg("status").output() else {
        return (false, None);
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() == 3 && parts[0] == "running" {
        (true, parts[2].parse::<i32>().ok())
    } else {
        (false, None)
    }
}

fn run_clicker_command(command: &str) -> Result<(), String> {
    let output = Command::new(script_path())
        .arg(command)
        .output()
        .map_err(|error| error.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() {
            "Autoclicker command failed".to_string()
        } else {
            error
        })
    }
}

fn focus_existing_window() -> bool {
    let Ok(output) = Command::new("hyprctl").args(["clients", "-j"]).output() else {
        return false;
    };

    let Ok(clients) = serde_json::from_slice::<Value>(&output.stdout) else {
        return false;
    };
    let Some(clients) = clients.as_array() else {
        return false;
    };

    for client in clients {
        let title = client.get("title").and_then(Value::as_str).unwrap_or("");
        let class = client.get("class").and_then(Value::as_str).unwrap_or("");

        if title == WINDOW_TITLE && class.to_lowercase().contains("autoclicker") {
            let Some(address) = client.get("address").and_then(Value::as_str) else {
                continue;
            };

            let dispatch = format!("hl.dsp.focus({{ window = \"address:{address}\" }})");
            let _ = Command::new("hyprctl")
                .args(["dispatch", &dispatch])
                .output();
            return true;
        }
    }

    false
}

fn format_key_name(name: Option<glib::GString>) -> String {
    let Some(name) = name else {
        return DEFAULT_KEY_NAME.to_string();
    };
    let name = name.as_str();

    let special = match name {
        "space" => Some("Space"),
        "Return" => Some("Enter"),
        "KP_Enter" => Some("Numpad Enter"),
        "Escape" => Some("Esc"),
        "BackSpace" => Some("Backspace"),
        "Tab" => Some("Tab"),
        "Shift_L" => Some("Left Shift"),
        "Shift_R" => Some("Right Shift"),
        "Control_L" => Some("Left Ctrl"),
        "Control_R" => Some("Right Ctrl"),
        "Alt_L" => Some("Left Alt"),
        "Alt_R" => Some("Right Alt"),
        "Super_L" => Some("Left Super"),
        "Super_R" => Some("Right Super"),
        "Caps_Lock" => Some("Caps Lock"),
        "Delete" => Some("Delete"),
        "Insert" => Some("Insert"),
        "Home" => Some("Home"),
        "End" => Some("End"),
        "Page_Up" => Some("Page Up"),
        "Page_Down" => Some("Page Down"),
        "Up" => Some("Up"),
        "Down" => Some("Down"),
        "Left" => Some("Left"),
        "Right" => Some("Right"),
        _ => None,
    };

    if let Some(special) = special {
        return special.to_string();
    }

    if name.chars().count() == 1 {
        return name.to_uppercase();
    }

    name.to_string()
}

fn create_segmented(options: &[&str], active: usize) -> (gtk::Box, Vec<gtk::ToggleButton>) {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    container.add_css_class("linked");
    container.set_valign(gtk::Align::Center);

    let buttons: Vec<gtk::ToggleButton> = options
        .iter()
        .map(|label| gtk::ToggleButton::with_label(label))
        .collect();

    for button in buttons.iter().skip(1) {
        button.set_group(Some(&buttons[0]));
    }
    buttons[active].set_active(true);

    for button in &buttons {
        container.append(button);
    }

    (container, buttons)
}

fn create_ui(config: &Config) -> Ui {
    let window = gtk::Window::new();
    window.set_title(Some(WINDOW_TITLE));
    window.set_default_size(400, 400);
    window.set_resizable(false);

    let header = gtk::HeaderBar::new();
    header.set_show_title_buttons(false);
    header.set_title_widget(Some(&gtk::Label::new(Some(WINDOW_TITLE))));
    window.set_titlebar(Some(&header));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let status_label = gtk::Label::new(Some("Stopped"));
    status_label.set_xalign(0.0);
    status_label.add_css_class("dim-label");

    let mode_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mode_title = gtk::Label::new(Some("Mode"));
    mode_title.set_xalign(0.0);
    mode_title.set_hexpand(true);
    let (mode_control, mode_buttons) = create_segmented(&MODES, config.mode);
    mode_row.append(&mode_title);
    mode_row.append(&mode_control);

    let target_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let target_title = gtk::Label::new(Some("Target"));
    target_title.set_xalign(0.0);
    target_title.set_hexpand(true);
    let (target_control, target_buttons) = create_segmented(&TARGETS, config.target);
    target_row.append(&target_title);
    target_row.append(&target_control);

    let speed_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let speed_title = gtk::Label::new(Some("Click speed"));
    speed_title.set_xalign(0.0);
    speed_title.set_hexpand(true);
    speed_title.set_margin_end(16);

    let entry = gtk::Entry::new();
    entry.set_text(&config.cps.to_string());
    entry.set_width_chars(4);
    entry.set_max_width_chars(4);
    entry.set_max_length(3);
    entry.set_input_purpose(gtk::InputPurpose::Number);
    entry.set_valign(gtk::Align::Center);
    entry.set_tooltip_text(Some(&format!("{MIN_CPS}–{MAX_CPS} clicks per second")));

    let minus_button = gtk::Button::with_label("−");
    minus_button.set_tooltip_text(Some("Decrease speed"));
    minus_button.set_valign(gtk::Align::Center);

    let plus_button = gtk::Button::with_label("+");
    plus_button.set_tooltip_text(Some("Increase speed"));
    plus_button.set_valign(gtk::Align::Center);

    let speed_controls = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    speed_controls.add_css_class("linked");
    speed_controls.append(&minus_button);
    speed_controls.append(&entry);
    speed_controls.append(&plus_button);

    let cps_label = gtk::Label::new(Some("CPS"));
    cps_label.set_xalign(1.0);
    cps_label.add_css_class("dim-label");

    speed_row.append(&speed_title);
    speed_row.append(&speed_controls);
    speed_row.append(&cps_label);

    let scale = gtk::Scale::with_range(
        gtk::Orientation::Horizontal,
        MIN_CPS as f64,
        MAX_CPS as f64,
        1.0,
    );
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    scale.set_value(config.cps as f64);

    let speed_section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    speed_section.append(&speed_row);
    speed_section.append(&scale);
    speed_section.set_sensitive(config.mode != 1);

    let mouse_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mouse_title = gtk::Label::new(Some("Mouse button"));
    mouse_title.set_xalign(0.0);
    mouse_title.set_hexpand(true);
    let (button_control, button_buttons) = create_segmented(&BUTTONS, config.button);
    mouse_row.append(&mouse_title);
    mouse_row.append(&button_control);

    let key_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let key_title = gtk::Label::new(Some("Keyboard key"));
    key_title.set_xalign(0.0);
    key_title.set_hexpand(true);

    let key_button = gtk::Button::with_label(&config.key_name);
    key_button.set_valign(gtk::Align::Center);
    key_button.set_tooltip_text(Some("Click, then press a key to capture it"));

    key_row.append(&key_title);
    key_row.append(&key_button);

    let action_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    action_row.set_halign(gtk::Align::End);

    let close_button = gtk::Button::with_label("Close");
    let toggle_button = gtk::Button::with_label(if config.mode == 1 { "Hold" } else { "Start" });
    toggle_button.add_css_class("suggested-action");

    action_row.append(&close_button);
    action_row.append(&toggle_button);

    content.append(&status_label);
    content.append(&mode_row);
    content.append(&target_row);
    content.append(&speed_section);
    content.append(&mouse_row);
    content.append(&key_row);
    content.append(&action_row);

    window.set_child(Some(&content));

    mouse_row.set_visible(config.target == 0);
    key_row.set_visible(config.target == 1);

    let ui = Ui {
        window: window.clone(),
        status_label,
        toggle_button,
        mode_buttons,
        target_buttons,
        button_buttons,
        mouse_row,
        key_row,
        key_button,
        speed_section,
        entry,
        scale,
        minus_button,
        plus_button,
    };

    let loop_ui = ui.clone();
    close_button.connect_clicked(move |_| {
        loop_ui.window.close();
    });

    ui
}

fn cancel_key_capture(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let name = {
        let mut state = state.borrow_mut();
        state.capturing = false;
        state.key_name.clone()
    };

    ui.key_button.remove_css_class("suggested-action");
    ui.key_button.set_label(&name);
}

fn set_speed(state: &Rc<RefCell<AppState>>, ui: &Ui, cps: i32, source: SpeedSource) {
    let cps = cps.clamp(MIN_CPS, MAX_CPS);
    state.borrow_mut().cps = cps;
    state.borrow_mut().syncing = true;

    if source != SpeedSource::Scale {
        ui.scale.set_value(cps as f64);
    }
    if source != SpeedSource::Entry {
        ui.entry.set_text(&cps.to_string());
    }

    state.borrow_mut().syncing = false;
    save_config(state, ui);
}

fn commit_entry(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let current = state.borrow().cps;
    let text = ui.entry.text().to_string();
    let cps = text.trim().parse::<i32>().unwrap_or(current);
    set_speed(state, ui, cps, SpeedSource::Button);
}

fn refresh_status(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let (running, cps) = clicker_status();

    let (mode, target, button, key_name) = {
        let mut state = state.borrow_mut();
        state.running = running;
        (
            state.mode,
            state.target,
            state.button,
            state.key_name.clone(),
        )
    };

    let input_label = if target == 0 {
        BUTTONS[button].to_string()
    } else {
        key_name
    };

    if running {
        if mode == 1 {
            ui.status_label
                .set_text(&format!("Holding · {input_label}"));
        } else {
            let details = match cps {
                Some(cps) => format!("{cps} CPS · {input_label}"),
                None => input_label.clone(),
            };
            ui.status_label.set_text(&format!("Running · {details}"));
        }

        ui.toggle_button
            .set_label(if mode == 1 { "Release" } else { "Stop" });
        ui.toggle_button.remove_css_class("suggested-action");
        ui.toggle_button.add_css_class("destructive-action");
    } else {
        ui.status_label.set_text("Stopped");
        ui.toggle_button
            .set_label(if mode == 1 { "Hold" } else { "Start" });
        ui.toggle_button.remove_css_class("destructive-action");
        ui.toggle_button.add_css_class("suggested-action");
    }

    ui.mouse_row.set_visible(target == 0);
    ui.key_row.set_visible(target == 1);
    ui.speed_section.set_sensitive(mode != 1);

    for button in &ui.mode_buttons {
        button.set_sensitive(!running);
    }
    for button in &ui.target_buttons {
        button.set_sensitive(!running);
    }
    for button in &ui.button_buttons {
        button.set_sensitive(!(running && mode == 1));
    }
    ui.key_button.set_sensitive(!running);
}

fn wire_ui(state: Rc<RefCell<AppState>>, ui: &Ui) {
    for (index, button) in ui.mode_buttons.iter().enumerate() {
        let state = state.clone();
        let ui = ui.clone();
        button.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            state.borrow_mut().mode = index;
            ui.speed_section.set_sensitive(index != 1);
            save_config(&state, &ui);
            refresh_status(&state, &ui);
        });
    }

    for (index, button) in ui.target_buttons.iter().enumerate() {
        let state = state.clone();
        let ui = ui.clone();
        button.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            state.borrow_mut().target = index;
            ui.mouse_row.set_visible(index == 0);
            ui.key_row.set_visible(index == 1);
            save_config(&state, &ui);
            refresh_status(&state, &ui);
        });
    }

    for (index, button) in ui.button_buttons.iter().enumerate() {
        let state = state.clone();
        let ui = ui.clone();
        button.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            state.borrow_mut().button = index;
            save_config(&state, &ui);
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.scale.connect_value_changed(move |scale| {
            if state.borrow().syncing {
                return;
            }
            set_speed(
                &state,
                &ui_c,
                scale.value().round() as i32,
                SpeedSource::Scale,
            );
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.entry.connect_changed(move |entry| {
            if state.borrow().syncing {
                return;
            }
            let text = entry.text().to_string();
            let text = text.trim();
            if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
                return;
            }
            if let Ok(cps) = text.parse::<i32>() {
                set_speed(&state, &ui_c, cps, SpeedSource::Entry);
            }
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.entry.connect_activate(move |_| {
            commit_entry(&state, &ui_c);
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            commit_entry(&state, &ui_c);
        });
        ui.entry.add_controller(focus);
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.minus_button.connect_clicked(move |_| {
            let cps = state.borrow().cps;
            set_speed(&state, &ui_c, cps - 1, SpeedSource::Button);
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.plus_button.connect_clicked(move |_| {
            let cps = state.borrow().cps;
            set_speed(&state, &ui_c, cps + 1, SpeedSource::Button);
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.key_button.connect_clicked(move |button| {
            if state.borrow().running {
                return;
            }
            if state.borrow().capturing {
                cancel_key_capture(&state, &ui_c);
                return;
            }

            state.borrow_mut().capturing = true;
            button.set_label("Press a key…");
            button.add_css_class("suggested-action");
            button.grab_focus();
        });
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            if state.borrow().capturing {
                cancel_key_capture(&state, &ui_c);
            }
        });
        ui.key_button.add_controller(focus);
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        let controller = gtk::EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        controller.connect_key_pressed(move |_controller, keyval, keycode, _modifiers| {
            let capturing = state.borrow().capturing;

            if capturing {
                if keyval == gdk::Key::Escape {
                    cancel_key_capture(&state, &ui_c);
                    return glib::Propagation::Stop;
                }

                let evdev = keycode as i32 - 8;
                if !(MIN_KEYCODE..=MAX_KEYCODE).contains(&evdev) {
                    ui_c.status_label.set_text("Unsupported key");
                    cancel_key_capture(&state, &ui_c);
                    return glib::Propagation::Stop;
                }

                let name = format_key_name(keyval.name());
                {
                    let mut state = state.borrow_mut();
                    state.keycode = evdev as u32;
                    state.key_name = name.clone();
                    state.capturing = false;
                }

                ui_c.key_button.remove_css_class("suggested-action");
                ui_c.key_button.set_label(&name);
                save_config(&state, &ui_c);
                refresh_status(&state, &ui_c);
                return glib::Propagation::Stop;
            }

            if keyval == gdk::Key::Escape {
                ui_c.window.close();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
        ui.window.add_controller(controller);
    }

    {
        let state = state.clone();
        let ui_c = ui.clone();
        ui.toggle_button.connect_clicked(move |_| {
            let running = state.borrow().running;
            let command = if running { "stop" } else { "start" };

            match run_clicker_command(command) {
                Ok(()) => {
                    if command == "start" {
                        ui_c.window.close();
                    } else {
                        refresh_status(&state, &ui_c);
                    }
                }
                Err(error) => {
                    ui_c.status_label
                        .set_text(&format!("Could not update clicker: {error}"));
                }
            }
        });
    }
}

fn main() {
    if focus_existing_window() {
        return;
    }

    glib::set_prgname(Some(APP_ID));

    if let Err(error) = gtk::init() {
        eprintln!("Failed to initialize GTK: {error}");
        std::process::exit(1);
    }

    let config = read_config();
    let ui = create_ui(&config);
    let state = Rc::new(RefCell::new(AppState::from_config(&config)));

    wire_ui(state.clone(), &ui);

    let main_loop = glib::MainLoop::new(None, false);
    let loop_clone = main_loop.clone();
    let window = ui.window.clone();
    window.connect_close_request(move |_| {
        loop_clone.quit();
        glib::Propagation::Proceed
    });

    window.present();

    let state_clone = state.clone();
    let ui_clone = ui.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        refresh_status(&state_clone, &ui_clone);
    });

    let state_clone = state.clone();
    let ui_clone = ui.clone();
    glib::timeout_add_local(Duration::from_millis(500), move || {
        refresh_status(&state_clone, &ui_clone);
        glib::ControlFlow::Continue
    });

    main_loop.run();
}
