#!/usr/bin/env bash

# Helper to run an app only if it is not already running
run_once() {
    if ! pgrep -x "$1" > /dev/null 2>&1; then
        "$@" &
    fi
}

# 1. Launch apps directly onto specific workspaces using Hyprland's Lua dispatcher
hyprctl dispatch 'hl.dsp.exec_cmd("foot", { workspace = "1 silent" })' &&

hyprctl dispatch 'hl.dsp.exec_cmd("nautilus", { workspace = "1 silent" })' &&

hyprctl dispatch 'hl.dsp.exec_cmd("kate", { workspace = "1 silent" })'

run_once brave
