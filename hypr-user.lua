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
