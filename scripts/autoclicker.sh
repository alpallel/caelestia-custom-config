#!/usr/bin/env bash

PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr_autoclicker.pid"
CONF_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/caelestia"
CONF="$CONF_DIR/autoclicker.conf"
DEFAULT_CPS=20
MIN_CPS=1
MAX_CPS=100
DEFAULT_BUTTON=left
DEFAULT_MODE=click
DEFAULT_TARGET=mouse
DEFAULT_KEYCODE=57
DEFAULT_KEY_NAME=Space

# Load and validate the configured speed, target, and mode.
load_config() {
    CPS=$DEFAULT_CPS
    BUTTON=$DEFAULT_BUTTON
    MODE=$DEFAULT_MODE
    TARGET=$DEFAULT_TARGET
    KEYCODE=$DEFAULT_KEYCODE
    KEY_NAME=$DEFAULT_KEY_NAME

    if [[ -r $CONF ]]; then
        local key value
        while IFS='=' read -r key value; do
            case $key in
                cps) CPS=$value ;;
                button) BUTTON=$value ;;
                mode) MODE=$value ;;
                target) TARGET=$value ;;
                keycode) KEYCODE=$value ;;
                key_name) KEY_NAME=$value ;;
            esac
        done < "$CONF"
    fi

    [[ $CPS =~ ^[0-9]+$ ]] || CPS=$DEFAULT_CPS
    ((${#CPS} > 4)) && CPS=$DEFAULT_CPS
    CPS=$((10#$CPS))
    ((CPS < MIN_CPS)) && CPS=$MIN_CPS
    ((CPS > MAX_CPS)) && CPS=$MAX_CPS

    [[ $TARGET == key ]] || TARGET=mouse

    # ydotool uses evdev keycodes; valid keyboard codes fit in 1-767.
    if [[ ! $KEYCODE =~ ^[0-9]+$ ]] || ((${#KEYCODE} > 3)); then
        KEYCODE=$DEFAULT_KEYCODE
        KEY_NAME=$DEFAULT_KEY_NAME
    else
        KEYCODE=$((10#$KEYCODE))
        if ((KEYCODE < 1 || KEYCODE > 767)); then
            KEYCODE=$DEFAULT_KEYCODE
            KEY_NAME=$DEFAULT_KEY_NAME
        fi
    fi

    [[ -n $KEY_NAME ]] || KEY_NAME="Key $KEYCODE"

    if [[ $BUTTON == right ]]; then
        CLICK_CODE=0xC1
        BUTTON_DOWN=0x41
        BUTTON_UP=0x81
        BUTTON_LABEL=Right
    else
        CLICK_CODE=0xC0
        BUTTON_DOWN=0x40
        BUTTON_UP=0x80
        BUTTON_LABEL=Left
    fi

    [[ $MODE == hold ]] || MODE=click

    if [[ $TARGET == key ]]; then
        INPUT_LABEL=$KEY_NAME
        INPUT_ICON=keyboard
    else
        INPUT_LABEL=$BUTTON_LABEL
        INPUT_ICON=mouse
    fi
}

# Release whatever the current hold mode is holding.
release_inputs() {
    [[ $MODE == hold ]] || return 0
    if [[ $TARGET == key ]]; then
        ydotool key "${KEYCODE}:0" >/dev/null 2>&1
    else
        ydotool click "$BUTTON_UP" >/dev/null 2>&1
    fi
}

# Print the running pid, or return 1 when the clicker is not running.
running_pid() {
    local pid
    [[ -r $PID_FILE ]] || return 1
    IFS= read -r pid < "$PID_FILE" || return 1
    [[ $pid =~ ^[0-9]+$ ]] || return 1
    kill -0 "$pid" 2>/dev/null || return 1
    printf '%s' "$pid"
}

start_clicker() {
    local pid
    if pid=$(running_pid); then
        load_config
        if [[ $MODE == hold ]]; then
            caelestia shell toaster info "Auto Clicker" "Already holding ($INPUT_LABEL)" "$INPUT_ICON" 2>/dev/null
        else
            caelestia shell toaster info "Auto Clicker" "Already running ($CPS CPS, $INPUT_LABEL)" "$INPUT_ICON" 2>/dev/null
        fi
        return 0
    fi

    load_config
    if [[ $MODE == hold ]]; then
        release_inputs
        (
            trap 'release_inputs; exit 0' TERM INT
            trap 'release_inputs' EXIT
            if [[ $TARGET == key ]]; then
                ydotool key "${KEYCODE}:1" >/dev/null 2>&1
            else
                ydotool click "$BUTTON_DOWN" >/dev/null 2>&1
            fi
            while true; do sleep 0.2; done
        ) >/dev/null 2>&1 &
        echo $! > "$PID_FILE"
        caelestia shell toaster success "Auto Clicker" "Holding ($INPUT_LABEL)" "$INPUT_ICON" 2>/dev/null
        return 0
    fi

    (
        while true; do
            load_config
            if [[ $TARGET == key ]]; then
                ydotool key "${KEYCODE}:1" "${KEYCODE}:0" >/dev/null 2>&1
            else
                ydotool click "$CLICK_CODE" >/dev/null 2>&1
            fi

            # Interval derived from CPS, in microseconds
            local us=$((1000000 / CPS))
            local delay
            printf -v delay '%d.%06d' $((us / 1000000)) $((us % 1000000))
            sleep "$delay"
        done
    ) >/dev/null 2>&1 &

    echo $! > "$PID_FILE"
    caelestia shell toaster success "Auto Clicker" "Enabled ($CPS CPS, $INPUT_LABEL)" "$INPUT_ICON" 2>/dev/null
}

stop_clicker() {
    local pid
    load_config
    if pid=$(running_pid); then
        release_inputs
        kill "$pid" 2>/dev/null
        rm -f "$PID_FILE"
        if [[ $MODE == hold ]]; then
            caelestia shell toaster warn "Auto Clicker" "Released" "$INPUT_ICON" 2>/dev/null
        else
            caelestia shell toaster warn "Auto Clicker" "Disabled" "$INPUT_ICON" 2>/dev/null
        fi
    else
        rm -f "$PID_FILE"
    fi
}

toggle_clicker() {
    if running_pid >/dev/null; then
        stop_clicker
    else
        start_clicker
    fi
}

status_clicker() {
    local pid
    if pid=$(running_pid); then
        load_config
        printf 'running %s %s\n' "$pid" "$CPS"
    else
        printf 'stopped\n'
    fi
}

case "${1:-toggle}" in
    start) start_clicker ;;
    stop) stop_clicker ;;
    toggle) toggle_clicker ;;
    status) status_clicker ;;
    *)
        echo "usage: $(basename "$0") [start|stop|toggle|status]" >&2
        exit 2
        ;;
esac
