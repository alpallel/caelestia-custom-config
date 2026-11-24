#!/usr/bin/env python3

import json
import os
import subprocess
import sys
from pathlib import Path

APP_ID = "autoclicker-config"
WINDOW_TITLE = "Auto Clicker"
CONFIG_DIR = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "caelestia"
CONFIG_FILE = CONFIG_DIR / "autoclicker.conf"
SCRIPT = CONFIG_DIR / "scripts" / "autoclicker.sh"

MIN_CPS = 1
MAX_CPS = 100
DEFAULT_CPS = 20
BUTTONS = ("left", "right")
DEFAULT_BUTTON = "left"
MODES = ("click", "hold")
DEFAULT_MODE = "click"
TARGETS = ("mouse", "key")
DEFAULT_TARGET = "mouse"
DEFAULT_KEYCODE = 57
DEFAULT_KEY_NAME = "Space"
MIN_KEYCODE = 1
MAX_KEYCODE = 767

SPECIAL_KEYS = {
    "space": "Space",
    "Return": "Enter",
    "KP_Enter": "Numpad Enter",
    "Escape": "Esc",
    "BackSpace": "Backspace",
    "Tab": "Tab",
    "Shift_L": "Left Shift",
    "Shift_R": "Right Shift",
    "Control_L": "Left Ctrl",
    "Control_R": "Right Ctrl",
    "Alt_L": "Left Alt",
    "Alt_R": "Right Alt",
    "Super_L": "Left Super",
    "Super_R": "Right Super",
    "Caps_Lock": "Caps Lock",
    "Delete": "Delete",
    "Insert": "Insert",
    "Home": "Home",
    "End": "End",
    "Page_Up": "Page Up",
    "Page_Down": "Page Down",
    "Up": "Up",
    "Down": "Down",
    "Left": "Left",
    "Right": "Right",
}


def format_key_name(name):
    if not name:
        return DEFAULT_KEY_NAME
    if name in SPECIAL_KEYS:
        return SPECIAL_KEYS[name]
    if len(name) == 1:
        return name.upper()
    return name


def read_config():
    cps = DEFAULT_CPS
    button = DEFAULT_BUTTON
    mode = DEFAULT_MODE
    target = DEFAULT_TARGET
    keycode = DEFAULT_KEYCODE
    key_name = ""

    try:
        for line in CONFIG_FILE.read_text().splitlines():
            key, separator, value = line.partition("=")
            if not separator:
                continue

            key = key.strip()
            value = value.strip()

            if key == "cps":
                try:
                    cps = max(MIN_CPS, min(MAX_CPS, int(value)))
                except ValueError:
                    pass
            elif key == "button" and value in BUTTONS:
                button = value
            elif key == "mode" and value in MODES:
                mode = value
            elif key == "target" and value in TARGETS:
                target = value
            elif key == "keycode":
                try:
                    code = int(value)
                except ValueError:
                    pass
                else:
                    if MIN_KEYCODE <= code <= MAX_KEYCODE:
                        keycode = code
            elif key == "key_name":
                key_name = value
    except OSError:
        pass

    if not key_name:
        key_name = DEFAULT_KEY_NAME if keycode == DEFAULT_KEYCODE else f"Key {keycode}"

    return cps, button, mode, target, keycode, key_name


def write_config(cps, button, mode, target, keycode, key_name):
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    temporary = CONFIG_FILE.with_name(CONFIG_FILE.name + ".tmp")
    temporary.write_text(
        f"cps={cps}\n"
        f"button={button}\n"
        f"mode={mode}\n"
        f"target={target}\n"
        f"keycode={keycode}\n"
        f"key_name={key_name}\n"
    )
    temporary.replace(CONFIG_FILE)


def clicker_status():
    try:
        result = subprocess.run(
            [str(SCRIPT), "status"],
            capture_output=True,
            text=True,
            timeout=2,
        )
    except (OSError, subprocess.SubprocessError):
        return False, None

    parts = result.stdout.strip().split()
    if len(parts) == 3 and parts[0] == "running":
        try:
            return True, int(parts[2])
        except ValueError:
            return True, None
    return False, None


def focus_existing_window():
    try:
        result = subprocess.run(
            ["hyprctl", "clients", "-j"],
            capture_output=True,
            text=True,
            timeout=2,
        )
        if result.returncode != 0:
            return False
        clients = json.loads(result.stdout)
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError):
        return False

    for client in clients:
        if (
            client.get("title") == WINDOW_TITLE
            and "autoclicker" in (client.get("class") or "").lower()
        ):
            address = client.get("address")
            if not address:
                continue
            subprocess.run(
                [
                    "hyprctl",
                    "dispatch",
                    f'hl.dsp.focus({{ window = "address:{address}" }})',
                ],
                capture_output=True,
                text=True,
                timeout=2,
            )
            return True

    return False


if focus_existing_window():
    sys.exit(0)

import gi  # noqa: E402

gi.require_version("Gdk", "4.0")
gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, GLib, Gtk  # noqa: E402


def create_segmented(options, active_index, callback):
    box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
    box.add_css_class("linked")
    box.set_valign(Gtk.Align.CENTER)

    buttons = [Gtk.ToggleButton(label=label) for label in options]
    for button in buttons[1:]:
        button.set_group(buttons[0])
    buttons[active_index].set_active(True)

    for index, button in enumerate(buttons):
        button.connect("toggled", callback, index)
        box.append(button)

    return box, buttons


class AutoclickerWindow(Gtk.Window):
    def __init__(self):
        super().__init__(title=WINDOW_TITLE)
        self.set_default_size(400, 400)
        self.set_resizable(False)

        self._running = False
        self._syncing = False
        self._capturing = False
        self._timer_id = None

        cps, button, mode, target, keycode, key_name = read_config()
        self._cps = cps
        self._button = button
        self._mode = mode
        self._target = target
        self._keycode = keycode
        self._key_name = key_name

        header = Gtk.HeaderBar()
        header.set_show_title_buttons(False)
        header.set_title_widget(Gtk.Label(label=WINDOW_TITLE))
        self.set_titlebar(header)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        content.set_margin_top(18)
        content.set_margin_bottom(18)
        content.set_margin_start(18)
        content.set_margin_end(18)

        self.status_label = Gtk.Label(label="Stopped", xalign=0)
        self.status_label.add_css_class("dim-label")

        mode_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        mode_title = Gtk.Label(label="Mode", xalign=0)
        mode_title.set_hexpand(True)

        mode_control, self.mode_buttons = create_segmented(
            ["Click", "Hold"], MODES.index(mode), self.on_mode_toggled
        )

        mode_row.append(mode_title)
        mode_row.append(mode_control)

        target_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        target_title = Gtk.Label(label="Target", xalign=0)
        target_title.set_hexpand(True)

        target_control, self.target_buttons = create_segmented(
            ["Mouse", "Keyboard"], TARGETS.index(target), self.on_target_toggled
        )

        target_row.append(target_title)
        target_row.append(target_control)

        speed_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        speed_title = Gtk.Label(label="Click speed", xalign=0)
        speed_title.set_hexpand(True)
        speed_title.set_margin_end(16)

        self.entry = Gtk.Entry()
        self.entry.set_text(str(cps))
        self.entry.set_width_chars(4)
        self.entry.set_max_width_chars(4)
        self.entry.set_max_length(3)
        self.entry.set_input_purpose(Gtk.InputPurpose.NUMBER)
        self.entry.set_valign(Gtk.Align.CENTER)
        self.entry.set_tooltip_text(f"{MIN_CPS}–{MAX_CPS} clicks per second")

        minus_button = Gtk.Button(label="−")
        minus_button.set_tooltip_text("Decrease speed")
        minus_button.set_valign(Gtk.Align.CENTER)
        minus_button.connect("clicked", lambda *_: self.adjust_speed(-1))

        plus_button = Gtk.Button(label="+")
        plus_button.set_tooltip_text("Increase speed")
        plus_button.set_valign(Gtk.Align.CENTER)
        plus_button.connect("clicked", lambda *_: self.adjust_speed(1))

        speed_controls = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        speed_controls.add_css_class("linked")
        speed_controls.append(minus_button)
        speed_controls.append(self.entry)
        speed_controls.append(plus_button)

        cps_label = Gtk.Label(label="CPS", xalign=1)
        cps_label.add_css_class("dim-label")

        speed_row.append(speed_title)
        speed_row.append(speed_controls)
        speed_row.append(cps_label)

        self.scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, MIN_CPS, MAX_CPS, 1)
        self.scale.set_draw_value(False)
        self.scale.set_hexpand(True)
        self.scale.set_value(cps)
        self.scale.connect("value-changed", self.on_scale_changed)
        self.entry.connect("changed", self.on_entry_changed)
        self.entry.connect("activate", lambda *_: self.commit_entry())

        entry_focus = Gtk.EventControllerFocus()
        entry_focus.connect("leave", lambda *_: self.commit_entry())
        self.entry.add_controller(entry_focus)

        self.speed_section = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.speed_section.append(speed_row)
        self.speed_section.append(self.scale)
        self.speed_section.set_sensitive(mode != "hold")

        self.mouse_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        mouse_title = Gtk.Label(label="Mouse button", xalign=0)
        mouse_title.set_hexpand(True)

        button_control, self.button_buttons = create_segmented(
            ["Left", "Right"], BUTTONS.index(button), self.on_button_toggled
        )

        self.mouse_row.append(mouse_title)
        self.mouse_row.append(button_control)

        self.key_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        key_title = Gtk.Label(label="Keyboard key", xalign=0)
        key_title.set_hexpand(True)

        self.key_button = Gtk.Button(label=self._key_name)
        self.key_button.set_valign(Gtk.Align.CENTER)
        self.key_button.set_tooltip_text("Click, then press a key to capture it")
        self.key_button.connect("clicked", self.on_key_button_clicked)
        self.key_button.connect("notify::has-focus", self.on_key_focus_changed)

        self.key_row.append(key_title)
        self.key_row.append(self.key_button)

        action_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        action_row.set_halign(Gtk.Align.END)

        close_button = Gtk.Button(label="Close")
        close_button.connect("clicked", lambda *_: self.close())

        self.toggle_button = Gtk.Button(label="Start")
        self.toggle_button.connect("clicked", self.on_toggle)

        action_row.append(close_button)
        action_row.append(self.toggle_button)

        content.append(self.status_label)
        content.append(mode_row)
        content.append(target_row)
        content.append(self.speed_section)
        content.append(self.mouse_row)
        content.append(self.key_row)
        content.append(action_row)

        self.set_child(content)

        key_controller = Gtk.EventControllerKey()
        key_controller.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
        key_controller.connect("key-pressed", self.on_key_pressed)
        self.add_controller(key_controller)

        self.update_target_rows()

    def start_status_updates(self):
        self.refresh_status()
        self._timer_id = GLib.timeout_add(500, self.refresh_status)
        return False

    def cleanup(self):
        if self._timer_id is not None:
            GLib.source_remove(self._timer_id)
            self._timer_id = None

    def update_target_rows(self):
        self.mouse_row.set_visible(self._target == "mouse")
        self.key_row.set_visible(self._target == "key")

    def save_config(self):
        try:
            write_config(
                self._cps,
                self._button,
                self._mode,
                self._target,
                self._keycode,
                self._key_name,
            )
        except OSError as error:
            self.status_label.set_text(f"Could not save settings: {error}")

    def set_speed(self, cps, source):
        cps = max(MIN_CPS, min(MAX_CPS, int(cps)))
        self._cps = cps

        self._syncing = True
        if source != "scale":
            self.scale.set_value(cps)
        if source != "entry":
            self.entry.set_text(str(cps))
        self._syncing = False

        self.save_config()

    def on_scale_changed(self, scale):
        if not self._syncing:
            self.set_speed(round(scale.get_value()), source="scale")

    def on_entry_changed(self, entry):
        if self._syncing:
            return
        text = entry.get_text().strip()
        if not text.isdigit():
            return
        self.set_speed(int(text), source="entry")

    def commit_entry(self):
        text = self.entry.get_text().strip()
        try:
            cps = int(text)
        except ValueError:
            cps = self._cps
        self.set_speed(cps, source="button")

    def adjust_speed(self, delta):
        self.set_speed(self._cps + delta, source="button")

    def on_button_toggled(self, button, index):
        if not button.get_active() or index >= len(BUTTONS):
            return
        self._button = BUTTONS[index]
        self.save_config()

    def on_mode_toggled(self, button, index):
        if not button.get_active() or index >= len(MODES):
            return
        self._mode = MODES[index]
        self.speed_section.set_sensitive(self._mode != "hold")
        self.save_config()
        self.refresh_status()

    def on_target_toggled(self, button, index):
        if not button.get_active() or index >= len(TARGETS):
            return
        self._target = TARGETS[index]
        self.update_target_rows()
        self.save_config()
        self.refresh_status()

    def on_key_button_clicked(self, _button):
        if self._running:
            return
        if self._capturing:
            self.cancel_key_capture()
            return

        self._capturing = True
        self.key_button.set_label("Press a key…")
        self.key_button.add_css_class("suggested-action")
        self.key_button.grab_focus()

    def on_key_focus_changed(self, button, _pspec):
        if self._capturing and not button.has_focus():
            self.cancel_key_capture()

    def cancel_key_capture(self):
        self._capturing = False
        self.key_button.remove_css_class("suggested-action")
        self.key_button.set_label(self._key_name)

    def on_key_pressed(self, _controller, keyval, keycode, _state):
        if self._capturing:
            if keyval == Gdk.KEY_Escape:
                self.cancel_key_capture()
                return True

            evdev = keycode - 8
            if evdev < MIN_KEYCODE or evdev > MAX_KEYCODE:
                self.status_label.set_text("Unsupported key")
                self.cancel_key_capture()
                return True

            self._keycode = evdev
            self._key_name = format_key_name(Gdk.keyval_name(keyval) or f"Key {evdev}")
            self._capturing = False
            self.key_button.remove_css_class("suggested-action")
            self.key_button.set_label(self._key_name)
            self.save_config()
            self.refresh_status()
            return True

        if keyval == Gdk.KEY_Escape:
            self.close()
            return True
        return False

    def on_toggle(self, _button):
        command = "stop" if self._running else "start"
        try:
            result = subprocess.run(
                [str(SCRIPT), command],
                capture_output=True,
                text=True,
                timeout=2,
            )
        except (OSError, subprocess.SubprocessError) as error:
            self.status_label.set_text(f"Could not update clicker: {error}")
            return

        if result.returncode != 0:
            message = result.stderr.strip() or "Autoclicker command failed"
            self.status_label.set_text(message)
            return

        if command == "start":
            self.close()
            return

        self.refresh_status()

    def refresh_status(self):
        running, cps = clicker_status()
        self._running = running

        input_label = self._button.title() if self._target == "mouse" else self._key_name

        if running:
            if self._mode == "hold":
                self.status_label.set_text(f"Holding · {input_label}")
            else:
                details = f"{cps} CPS · {input_label}" if cps else input_label
                self.status_label.set_text(f"Running · {details}")
            self.toggle_button.set_label("Release" if self._mode == "hold" else "Stop")
            self.toggle_button.remove_css_class("suggested-action")
            self.toggle_button.add_css_class("destructive-action")
        else:
            self.status_label.set_text("Stopped")
            self.toggle_button.set_label("Hold" if self._mode == "hold" else "Start")
            self.toggle_button.remove_css_class("destructive-action")
            self.toggle_button.add_css_class("suggested-action")

        self.update_target_rows()
        self.speed_section.set_sensitive(self._mode != "hold")
        for button in self.mode_buttons:
            button.set_sensitive(not running)
        for button in self.target_buttons:
            button.set_sensitive(not running)
        for button in self.button_buttons:
            button.set_sensitive(not (running and self._mode == "hold"))
        self.key_button.set_sensitive(not running)

        return True


def main():
    GLib.set_prgname(APP_ID)
    GLib.set_application_name(WINDOW_TITLE)
    Gtk.init()

    window = AutoclickerWindow()
    loop = GLib.MainLoop()

    def on_close_request(_window):
        window.cleanup()
        loop.quit()
        return False

    window.connect("close-request", on_close_request)
    window.present()
    GLib.timeout_add(50, window.start_status_updates)

    try:
        loop.run()
    except KeyboardInterrupt:
        pass

    return 0


if __name__ == "__main__":
    sys.exit(main())
