local vars = require("hypr-vars")
local fn = require("utils.functions")

local function valid_keybind(key)
return type(key) == "string" and key:match("%S") ~= nil
end

local function flatten_keybinds(keybinds, keys)
keys = keys or {}
if type(keybinds) == "table" then
    for _, keybind in pairs(keybinds) do
        flatten_keybinds(keybind, keys)
        end
        elseif valid_keybind(keybinds) then
            keys[#keys + 1] = keybinds
            end
            return keys
            end

            local function create_bind(keybinds, action, flags)
            local get_flags = type(flags) == "function" and flags or function()
            return flags
            end
            for _, key in ipairs(flatten_keybinds(keybinds)) do
                hl.bind(key, action, get_flags(key))
                end
                end

                create_bind(vars.kbAudioWs, fn.toggle("audio"))
                create_bind(vars.kbPower, hl.dsp.global("caelestia:session"))

                -- Trackpad toggle
                create_bind(
                    vars.kbTouchpadToggle,
                    hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/toggle-touchpad.sh"),
                            { locked = true }
                )


-- Autostart scripts
hl.on("hyprland.start", function()
    hl.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/smart-start.sh")
end)



-- One-time placement for Brave on boot
local brave_placed = false

-- Prevent re-triggering if Hyprland reloads while Brave is already running
for _, win in ipairs(hl.get_windows() or {}) do
    if win.class and win.class:lower():find("brave") then
        brave_placed = true
        break
        end
        end

        -- Intercept the first Brave window on boot, move it silently, and disable itself
        hl.on("window.open", function(win)
        if not brave_placed and win.class and win.class:lower():find("brave") then
            brave_placed = true
            hl.dispatch(hl.dsp.window.move({ window = win, workspace = "2", follow = false }))
            end
            end)


hl.exec_cmd("gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark'")
