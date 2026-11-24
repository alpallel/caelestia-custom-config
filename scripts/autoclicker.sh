#!/usr/bin/env bash

PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr_autoclicker.pid"
CONF_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/caelestia"
CONF="$CONF_DIR/autoclicker.conf"
DEFAULT_CPS=20
MIN_CPS=1
MAX_CPS=100
DEFAULT_BUTTON=left
DEFAULT_MODE=click

# Load and validate the configured speed, mouse button, and mode.
load_config() {
    CPS=$DEFAULT_CPS
    BUTTON=$DEFAULT_BUTTON
    MODE=$DEFAULT_MODE

    if [[ -r $CONF ]]; then
        local key value
        while IFS='=' read -r key value; do
            case $key in
                cps) CPS=$value ;;
                button) BUTTON=$value ;;
                mode) MODE=$value ;;
            esac
        done < "$CONF"
    fi

    [[ $CPS =~ ^[0-9]+$ ]] || CPS=$DEFAULT_CPS
    ((${#CPS} > 4)) && CPS=$DEFAULT_CPS
    CPS=$((10#$CPS))
    ((CPS < MIN_CPS)) && CPS=$MIN_CPS
    ((CPS > MAX_CPS)) && CPS=$MAX_CPS

    if [[ $BUTTON == right ]]; then
        CLICK_CODE=0xC1
        BUTTON_DOWN=0x41
        BUTTON_LABEL=Right
    else
        CLICK_CODE=0xC0
        BUTTON_DOWN=0x40
        BUTTON_LABEL=Left
    fi

    [[ $MODE == hold ]] || MODE=click
}

# Release both mouse buttons. Up events for an unpressed button are harmless.
release_buttons() {
    ydotool click 0x80 >/dev/null 2>&1
    ydotool click 0x81 >/dev/null 2>&1
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
            caelestia shell toaster info "Auto Clicker" "Already holding ($BUTTON_LABEL)" "mouse" 2>/dev/null
        else
            caelestia shell toaster info "Auto Clicker" "Already running ($CPS CPS, $BUTTON_LABEL)" "mouse" 2>/dev/null
        fi
        return 0
    fi

    load_config
    if [[ $MODE == hold ]]; then
        release_buttons
        (
            trap 'release_buttons; exit 0' TERM INT
            trap 'release_buttons' EXIT
            ydotool click "$BUTTON_DOWN" >/dev/null 2>&1
            while true; do sleep 0.2; done
        ) >/dev/null 2>&1 &
        echo $! > "$PID_FILE"
        caelestia shell toaster success "Auto Clicker" "Holding ($BUTTON_LABEL)" "mouse" 2>/dev/null
        return 0
    fi

    (
        while true; do
            load_config
            ydotool click "$CLICK_CODE" >/dev/null 2>&1

            # Interval derived from CPS, in microseconds
            local us=$((1000000 / CPS))
            local delay
            printf -v delay '%d.%06d' $((us / 1000000)) $((us % 1000000))
            sleep "$delay"
        done
    ) >/dev/null 2>&1 &

    echo $! > "$PID_FILE"
    caelestia shell toaster success "Auto Clicker" "Enabled ($CPS CPS, $BUTTON_LABEL)" "mouse" 2>/dev/null
}

stop_clicker() {
    local pid
    load_config
    if pid=$(running_pid); then
        release_buttons
        kill "$pid" 2>/dev/null
        rm -f "$PID_FILE"
        if [[ $MODE == hold ]]; then
            caelestia shell toaster warn "Auto Clicker" "Released" "mouse" 2>/dev/null
        else
            caelestia shell toaster warn "Auto Clicker" "Disabled" "mouse" 2>/dev/null
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
