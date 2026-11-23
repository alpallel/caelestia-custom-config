#!/usr/bin/env bash

PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr_autoclicker.pid"
CONF_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/caelestia"
CONF="$CONF_DIR/autoclicker.conf"
DEFAULT_CPS=20
MIN_CPS=1
MAX_CPS=100

# Load the configured click speed into CPS. Clamps bad values to the valid range.
load_cps() {
    CPS=$DEFAULT_CPS

    if [[ -r $CONF ]]; then
        local key value
        while IFS='=' read -r key value; do
            [[ $key == cps ]] && CPS=$value
        done < "$CONF"
    fi

    [[ $CPS =~ ^[0-9]+$ ]] || CPS=$DEFAULT_CPS
    ((${#CPS} > 4)) && CPS=$DEFAULT_CPS
    CPS=$((10#$CPS))
    ((CPS < MIN_CPS)) && CPS=$MIN_CPS
    ((CPS > MAX_CPS)) && CPS=$MAX_CPS
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
        load_cps
        caelestia shell toaster info "Auto Clicker" "Already running ($CPS CPS)" "mouse" 2>/dev/null
        return 0
    fi

    load_cps
    (
        while true; do
            load_cps
            ydotool click 0xC0 >/dev/null 2>&1

            # Interval derived from CPS, in microseconds
            local us=$((1000000 / CPS))
            local delay
            printf -v delay '%d.%06d' $((us / 1000000)) $((us % 1000000))
            sleep "$delay"
        done
    ) >/dev/null 2>&1 &

    echo $! > "$PID_FILE"
    caelestia shell toaster success "Auto Clicker" "Enabled ($CPS CPS)" "mouse" 2>/dev/null
}

stop_clicker() {
    local pid
    if pid=$(running_pid); then
        kill "$pid" 2>/dev/null
        rm -f "$PID_FILE"
        caelestia shell toaster warn "Auto Clicker" "Disabled" "mouse" 2>/dev/null
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
        load_cps
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
