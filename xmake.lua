set_project("DelayLama")
set_languages("cxx20")
add_repositories("delaylama-packages xmake-repo", {rootdir = os.scriptdir()})
add_requires("juce 8.0.12", {configs = {shared = false}})
add_requires("clap 1.2.7", {configs = {shared = false}})
add_requires("clap-helpers 2026.8.5", {configs = {shared = false}})
add_requires("clap-juce-extensions 2026.8.5", {configs = {shared = false}})
option("tests", {
    default = true,
    showmenu = true,
    description = "Build focused Delay Lama tests"
})
option("formats", {
    default = "vst3,clap,au",
    showmenu = true,
    description = "Plugin formats (vst3,clap,au,auv3,aax,lv2)"
})

local function abort(message)
    -- Delegate failure because xmake removes Lua's error APIs from this sandbox.
    print("xmake configuration error: " .. message)
    option(nil)
end

local function add_linux_position_independent_code()
    if is_plat("linux") then
        add_cflags("-fPIC", {force = true})
        add_cxxflags("-fPIC", {force = true})
    end
end

local juce_modules = {
    core = {
        name = "juce_core",
        extras = {"juce_core/juce_core_CompilationTime.cpp"}
    },
    events = {name = "juce_events", deps = {"juce_core"}},
    data_structures = {name = "juce_data_structures", deps = {"juce_events"}},
    graphics = {
        name = "juce_graphics",
        deps = {"juce_events", "juce_data_structures"},
        extras = {
            "juce_graphics/juce_graphics_Harfbuzz.cpp",
            "juce_graphics/juce_graphics_Sheenbidi.c"
        }
    },
    gui_basics = {name = "juce_gui_basics", deps = {"juce_graphics", "juce_data_structures"}},
    gui_extra = {name = "juce_gui_extra", deps = {"juce_gui_basics"}},
    audio_basics = {name = "juce_audio_basics", deps = {"juce_core"}},
    audio_devices = {name = "juce_audio_devices", deps = {"juce_audio_basics", "juce_events"}},
    audio_formats = {name = "juce_audio_formats", deps = {"juce_audio_basics"}},
    audio_processors_headless = {
        name = "juce_audio_processors_headless",
        deps = {"juce_audio_basics", "juce_events"},
        extras = {
            "juce_audio_processors_headless/juce_audio_processors_headless_ara.cpp",
            "juce_audio_processors_headless/juce_audio_processors_headless_lv2_libs.cpp"
        }
    },
    audio_processors = {
        name = "juce_audio_processors",
        deps = {"juce_gui_extra", "juce_audio_processors_headless"}
    },
    audio_utils = {
        name = "juce_audio_utils",
        deps = {"juce_audio_processors", "juce_audio_formats", "juce_audio_devices"}
    }
}

local function add_juce_common()
    add_linux_position_independent_code()
    add_packages("juce")
    if is_plat("macosx", "iphoneos") then
        add_mxxflags("-fno-objc-arc", {force = true})
    end
    add_defines(
        "JUCE_GLOBAL_MODULE_SETTINGS_INCLUDED=1",
        "JUCE_USE_CURL=0",
        "JUCE_WEB_BROWSER=0",
        "JUCE_USE_CAMERA=0",
        "JUCE_USE_CDBURNER=0",
        "JUCE_USE_CDREADER=0",
        "JUCE_USE_WINRT_MIDI=0",
        "JUCE_WASAPI=0",
        "JUCE_DIRECTSOUND=0",
        "JUCE_ALSA=0",
        "JUCE_JACK=0",
        "JUCE_USE_OPENGL=0",
        "JUCE_DISABLE_JUCE_VERSION_PRINTING=1")
    if is_mode("debug") then
        add_defines("DEBUG=1", "_DEBUG=1")
    else
        add_defines("NDEBUG=1", "_NDEBUG=1")
    end
    if is_plat("linux") then
        add_defines("LINUX=1")
        add_includedirs("/usr/include/freetype2")
        add_links("rt", "dl", "pthread", "freetype", "fontconfig")
    end
    set_languages("cxx20")
end

-- Keep one source callback because later xmake on_load handlers replace earlier ones.
local function add_juce_package_sources(target, relative_paths)
    local package = target:pkg("juce")
    if not package then
        abort("JUCE package is unavailable")
    end
    for _, relative_path in ipairs(relative_paths) do
        local source = path.join(package:installdir(), "modules", relative_path)
        if not os.isfile(source) then
            abort("missing JUCE package source: " .. source)
        end
        target:add("files", source)
    end
end

local function add_juce_files(relative_paths)
    on_load(function (target)
        add_juce_package_sources(target, relative_paths)
    end)
end

local function juce_module_sources(module_name, extras)
    local suffix = is_plat("macosx", "iphoneos") and ".mm" or ".cpp"
    local sources = {module_name .. "/" .. module_name .. suffix}
    if extras then
        for _, extra in ipairs(extras) do
            table.insert(sources, extra)
        end
    end
    return sources
end

for _, module in pairs(juce_modules) do
    target(module.name)
        set_kind("static")
        add_juce_common()
        add_juce_files(juce_module_sources(module.name, module.extras))
        if module.deps then
            add_deps(table.unpack(module.deps))
        end
end

local function add_apple_frameworks()
    if is_plat("macosx") then
        add_frameworks(
            "Cocoa",
            "Foundation",
            "IOKit",
            "Security",
            "QuartzCore",
            "CoreAudio",
            "CoreMIDI",
            "AudioToolbox",
            "CoreAudioKit",
            "Accelerate",
            "AVFoundation")
    elseif is_plat("iphoneos") then
        add_frameworks(
            "Foundation",
            "CoreServices",
            "CoreGraphics",
            "CoreText",
            "CoreImage",
            "ImageIO",
            "UIKit",
            "Security",
            "QuartzCore",
            "CoreAudio",
            "CoreMIDI",
            "AudioToolbox",
            "CoreAudioKit",
            "Accelerate",
            "AVFoundation",
            "UserNotifications",
            "UniformTypeIdentifiers")
    end
end

target("DelayLamaDsp")
    set_kind("static")
    add_linux_position_independent_code()
    set_languages("cxx20")
    add_files("src/dsp/control.cpp", "src/dsp/render.cpp")
    add_includedirs("src", {public = true})

target("DelayLamaEditor")
    set_kind("static")
    add_linux_position_independent_code()
    set_languages("cxx20")
    add_files("src/editor/interaction.cpp")
    add_includedirs("src", {public = true})

target("DelayLamaHost")
    set_kind("static")
    add_linux_position_independent_code()
    set_languages("cxx20")
    add_files("src/host/processor.cpp")
    add_includedirs("src", {public = true})
    add_deps("DelayLamaDsp", "DelayLamaEditor")

if is_mode("debug") then
    add_defines("DEBUG=1", "_DEBUG=1")
else
    add_defines("NDEBUG=1", "_NDEBUG=1")
end

target("DelayLamaEditorAssets")
    set_kind("static")
    add_linux_position_independent_code()
    set_languages("cxx20")
    -- Wrap generated initializers so adapters depend on stable asset symbols.
    add_rules("utils.bin2c", {extensions = ".png"})
    add_files(
        "assets/control_panel.png",
        "assets/help_panel.png",
        "assets/monk_sprite_sheet.png",
        "assets/knob_strip_a.png",
        "assets/knob_strip_b.png",
        "assets/scene_background.png",
        "assets/ui_tile_a.png",
        "assets/ui_tile_b.png",
        "assets/ui_arrow.png",
        "src/editor/resources/assets.cpp")

if has_config("tests") then
    target("DelayLamaDspTests")
        set_kind("binary")
        set_languages("cxx20")
        add_files(
            "tests/dsp/main.cpp",
            "tests/dsp/control.cpp",
            "tests/dsp/render.cpp")
        add_deps("DelayLamaDsp")
        add_includedirs("src")
    target("DelayLamaHostTests")
        set_kind("binary")
        set_languages("cxx20")
        add_files(
            "tests/host/main.cpp",
            "tests/host/midi.cpp",
            "tests/host/processor.cpp")
        add_includedirs("src")
        add_deps("juce_audio_basics", "DelayLamaHost")
        add_juce_common()
        if is_plat("macosx", "iphoneos") then
            add_apple_frameworks()
        end
    target("DelayLamaEditorTests")
        set_kind("binary")
        set_languages("cxx20")
        add_files("tests/editor/interaction.cpp")
        add_deps("DelayLamaEditor")
        add_includedirs("src")
    target("DelayLamaPlatformTests")
        set_kind("phony")
        add_deps("DelayLamaEditorInteractionTests")
        on_run(function (target)
            os.execv(target:dep("DelayLamaEditorInteractionTests"):targetfile())
        end)
end
_delaylama_plugin_context = {
    abort = abort,
    add_apple_frameworks = add_apple_frameworks,
    add_juce_common = add_juce_common,
    add_juce_package_sources = add_juce_package_sources
}
includes("xmake/plugins.lua")
_delaylama_plugin_context = nil
