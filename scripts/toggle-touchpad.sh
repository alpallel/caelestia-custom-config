#!/usr/bin/env bash

DEVICE="ascp1205:00-093a:3020-touchpad"
MOUSE_DEV="ascp1205:00-093a:3020-mouse"
STATE_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr_touchpad_disabled"

set_state() {
    local enabled="$1"
    local lua_code="
        local dev = '$DEVICE'
        local mdev = '$MOUSE_DEV'
        if type(hl.device) == 'function' then
            hl.device({ name = dev, enabled = $enabled })
            hl.device({ name = mdev, enabled = $enabled })
        else
            hl.config({ device = { name = dev, enabled = $enabled } })
            hl.config({ device = { name = mdev, enabled = $enabled } })
        end
    "
    hyprctl eval "$lua_code"
}

if [ -f "$STATE_FILE" ]; then
    rm -f "$STATE_FILE"
    set_state "true"
    caelestia shell toaster success "Touchpad" "Touchpad enabled" "touchpad_mouse"
else
    touch "$STATE_FILE"
    set_state "false"
    caelestia shell toaster warn "Touchpad" "Touchpad disabled" "touchpad_mouse_off"
fi
