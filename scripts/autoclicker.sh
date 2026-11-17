#!/usr/bin/env bash

PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr_autoclicker.pid"
INTERVAL=0.001  # Delay between clicks in seconds (0.05 = ~20 clicks/sec)

# If already running, terminate the loop
if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    kill "$(cat "$PID_FILE")"
    rm -f "$PID_FILE"
    caelestia shell toaster warn "Auto Clicker" "Disabled" "mouse"
else
    # Run the click loop in a detached subshell
    (
        while true; do
            ydotool click 0xC0
            sleep "$INTERVAL"
        done
    ) &
    echo $! > "$PID_FILE"
    caelestia shell toaster success "Auto Clicker" "Enabled (~20 CPS)" "mouse"
fi
