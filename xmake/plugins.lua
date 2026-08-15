local build = _delaylama_plugin_context
local abort = build.abort
local add_apple_frameworks = build.add_apple_frameworks
local add_juce_common = build.add_juce_common
local add_juce_package_sources = build.add_juce_package_sources

local project_root = os.projectdir()
local identity_manifest = path.join(project_root, "config", "identity.json")
local identity_version_byte_max = 255
local identity_fourcc_width = 4
local macos_auv3_minimum_version = "13.0"

local function validate_identity(identity, fail)
    local function require_field(value, message)
        if not value then
            fail(message)
        end
    end

    require_field(identity.productName, "identity manifest missing productName")
    require_field(identity.version, "identity manifest missing version")
    require_field(identity.bundleIdentifier, "identity manifest missing bundleIdentifier")
    require_field(identity.publisher, "identity manifest missing publisher")
    require_field(identity.publisher.name, "identity manifest missing publisher.name")
    require_field(identity.publisher.website, "identity manifest missing publisher.website")
    require_field(identity.audioUnit, "identity manifest missing audioUnit")
    require_field(identity.audioUnit.type, "identity manifest missing audioUnit.type")
    require_field(identity.audioUnit.subtype, "identity manifest missing audioUnit.subtype")
    require_field(identity.audioUnit.manufacturer, "identity manifest missing audioUnit.manufacturer")
    require_field(
        identity.audioUnit.auV3ExtensionSuffix,
        "identity manifest missing audioUnit.auV3ExtensionSuffix")
    require_field(identity.vst3, "identity manifest missing vst3")
    require_field(identity.vst3.name, "identity manifest missing vst3.name")
    require_field(identity.vst3.vendor, "identity manifest missing vst3.vendor")
    require_field(identity.clap, "identity manifest missing clap")
    require_field(identity.clap.id, "identity manifest missing clap.id")
    require_field(identity.clap.features, "identity manifest missing clap.features")
    if #identity.clap.features == 0 then
        fail("identity manifest clap.features must contain at least one feature")
    end
    require_field(identity.aax, "identity manifest missing aax")
    require_field(identity.aax.identifier, "identity manifest missing aax.identifier")
    require_field(identity.aax.category, "identity manifest missing aax.category")
    require_field(identity.lv2, "identity manifest missing lv2")
    require_field(identity.lv2.uri, "identity manifest missing lv2.uri")
    if not string.match(identity.lv2.uri, "^https?://")
        and not string.match(identity.lv2.uri, "^urn:") then
        fail("identity manifest lv2.uri must begin with http://, https://, or urn:")
    end
end
local function quote_define(value)
    return '"' .. tostring(value):gsub('"', '\\"') .. '"'
end
local function identity_version_code(version, fail)
    local major, minor, patch = string.match(tostring(version), "^(%d+)%.(%d+)%.(%d+)$")
    major, minor, patch = tonumber(major), tonumber(minor), tonumber(patch)
    if not major or not minor or not patch
        or major > identity_version_byte_max
        or minor > identity_version_byte_max
        or patch > identity_version_byte_max then
        fail("identity manifest version must be a numeric major.minor.patch value")
    end
    return string.format("0x%02X%02X%02X", major, minor, patch)
end

local function identity_fourcc(value, field, fail)
    if #value ~= identity_fourcc_width then
        fail("identity manifest " .. field .. " must contain exactly four bytes")
    end
    local first, second, third, fourth = string.byte(value, 1, identity_fourcc_width)
    if not first or not second or not third or not fourth then
        fail("identity manifest " .. field .. " contains an invalid code")
    end
    return string.format("0x%02X%02X%02X%02X", first, second, third, fourth)
end

local function identity_export_prefix(product_name, fail)
    -- Keep the export prefix stable because JUCE appends its AU factory suffix.
    local prefix = tostring(product_name):gsub("[^%a%d]+", "_"):gsub("^_+", ""):gsub("_+$", "")
    if prefix == "" or not string.match(prefix, "^[%a_]") then
        fail("identity manifest productName cannot derive a valid AU export prefix")
    end
    return prefix .. "AU"
end

local function split_formats()
    local result = {}
    local configured = get_config("formats") or "vst3,au"
    if configured:gsub("%s", "") == "" then
        abort("formats must contain at least one supported format")
    end
    for token in string.gmatch(configured, "[^,]+") do
        local format = string.lower(token:gsub("^%s+", ""):gsub("%s+$", ""))
        if format ~= "" then
            if format == "vst3" or format == "clap" or format == "au" or format == "auv3"
                or format == "aax" or format == "lv2" then
                result[format] = true
            else
                abort("unknown plugin format: " .. format)
            end
        end
    end
    if not result.vst3 and not result.clap and not result.au and not result.auv3
        and not result.aax and not result.lv2 then
        abort("formats must contain at least one supported format")
    end
    return result
end

local requested_formats = split_formats()

local function supports_vst3()
    return is_plat("macosx", "windows", "linux")
end
local function supports_clap()
    return is_plat("macosx", "windows", "linux")
end
local function supports_au()
    return is_plat("macosx")
end

local function supports_auv3()
    return is_plat("macosx", "iphoneos")
end

local function supports_aax()
    return is_plat("macosx", "windows")
end

local function supports_lv2()
    return is_plat("macosx", "windows", "linux")
end

local plugin_defines = {
    "JucePlugin_Build_VST=0",
    "JucePlugin_Build_Standalone=0",
    "JucePlugin_Build_Unity=0",
    "JucePlugin_IsSynth=1",
    "JucePlugin_ManufacturerEmail=\"\"",
    "JucePlugin_ProducesMidiOutput=1",
    "JucePlugin_IsMidiEffect=0",
    "JucePlugin_WantsMidiInput=1",
    "JucePlugin_EditorRequiresKeyboardFocus=1",
    "JucePlugin_VSTUniqueID=JucePlugin_PluginCode",
    "JucePlugin_VSTCategory=\"Instrument\"",
    "JucePlugin_Vst3Category=\"Instrument|Synth\"",
    "JucePlugin_AUManufacturerCode=JucePlugin_ManufacturerCode",
    "JucePlugin_VSTNumMidiIns=1",
    "JucePlugin_VSTNumMidiOuts=1"
}
local function identity_plugin_defines(identity, fail)
    local manufacturer_code = identity_fourcc(
        identity.audioUnit.manufacturer,
        "audioUnit.manufacturer",
        fail)
    local plugin_code = identity_fourcc(identity.audioUnit.subtype, "audioUnit.subtype", fail)
    local au_main_type = identity_fourcc(identity.audioUnit.type, "audioUnit.type", fail)
    local export_prefix = identity_export_prefix(identity.productName, fail)
    local version_code = identity_version_code(identity.version, fail)
    return {
        "JucePlugin_Manufacturer=" .. quote_define(identity.publisher.name),
        "JucePlugin_ManufacturerWebsite=" .. quote_define(identity.publisher.website),
        "JucePlugin_Name=" .. quote_define(identity.productName),
        "JucePlugin_Desc=" .. quote_define(identity.productName),
        "JucePlugin_CFBundleIdentifier=" .. quote_define(identity.bundleIdentifier),
        "JucePlugin_ManufacturerCode=" .. manufacturer_code,
        "JucePlugin_PluginCode=" .. plugin_code,
        "JucePlugin_AUMainType=" .. au_main_type,
        "JucePlugin_AUSubType=" .. plugin_code,
        "JucePlugin_AUExportPrefix=" .. export_prefix,
        "JucePlugin_AUExportPrefixQuoted=" .. quote_define(export_prefix),
        "JucePlugin_Version=" .. version_code,
        "JucePlugin_VersionString=" .. quote_define(identity.version),
        "JucePlugin_VersionCode=" .. version_code,
        "JucePlugin_AAXIdentifier=" .. quote_define(identity.aax.identifier),
        "JucePlugin_AAXManufacturerCode=" .. manufacturer_code,
        "JucePlugin_AAXProductId=" .. plugin_code,
        "JucePlugin_AAXCategory=AAX_ePlugInCategory_" .. identity.aax.category,
        "JucePlugin_AAXDisableBypass=0",
        "JucePlugin_AAXDisableMultiMono=0"
    }
end
local function add_plugin_common()
    add_juce_common()
    add_defines(table.unpack(plugin_defines))
    add_defines("JUCE_VST3_CAN_REPLACE_VST2=0", "JUCE_WEB_BROWSER=0", "JUCE_USE_CURL=0")
    add_includedirs(path.join(project_root, "src"))
    add_deps("DelayLamaDsp", "DelayLamaHost", "DelayLamaEditorAssets", "juce_audio_utils")
    if is_plat("macosx", "iphoneos") then
        add_apple_frameworks()
    end
end
local juce_adapter_root = path.join(project_root, "build", ".gens")
local juce_adapter_dir = path.join(juce_adapter_root, "juce_adapter")
local juce_adapter_generator = path.join(project_root, "scripts", "adapter", "render.py")
local function add_plugin_identity(target, identity)
    validate_identity(identity, function (message)
        abort(message)
    end)
    target:add(
        "defines",
        table.unpack(identity_plugin_defines(identity, function (message)
            abort(message)
        end)))
end
local function add_generated_adapter_sources(target, expose_source_contract)
    target:add(
        "files",
        path.join(juce_adapter_dir, "processor.cpp"),
        path.join(juce_adapter_dir, "editor.cpp"))
    target:add("includedirs", juce_adapter_root)
    if expose_source_contract then
        target:add(
            "defines",
            "DELAYLAMA_GENERATED_EDITOR_SOURCE="
                .. quote_define(path.absolute(path.join(juce_adapter_dir, "editor.cpp"))),
            "DELAYLAMA_GENERATED_EDITOR_HEADER="
                .. quote_define(path.absolute(path.join(juce_adapter_dir, "editor.hpp"))))
    end
    target:add(
        "depfiles",
        juce_adapter_generator,
        path.join(project_root, "scripts", "adapter", "editor_template.py"),
        path.join(project_root, "scripts", "adapter", "editor_surface.py"),
        path.join(project_root, "scripts", "adapter", "processor.py"),
        path.join(project_root, "scripts", "adapter", "processor_runtime.py"),
        identity_manifest)
    for _, relative_path in ipairs({
        "src/host/processor.hpp",
        "src/host/processor.cpp",
        "src/editor/interaction.hpp",
        "src/editor/interaction.cpp",
        "src/editor/state.hpp",
        "src/host/midi.hpp",
        "src/dsp/midi.hpp",
        "src/editor/resources/assets.hpp",
        "src/editor/resources/assets.cpp"
    }) do
        target:add("depfiles", path.join(project_root, relative_path))
    end
end
local function add_plugin_sources(relative_paths)
    -- Keep generation and source registration together because xmake callbacks are single-valued.
    on_load(function (target)
        local json = import("core.base.json")
        local identity = json.loadfile(identity_manifest)
        add_plugin_identity(target, identity)
        os.vrunv("python3", {
            path.absolute(juce_adapter_generator),
            "--output-dir",
            path.absolute(juce_adapter_dir),
            "--source-root",
            path.absolute(project_root),
            "--identity",
            path.absolute(identity_manifest)
        })
        os.mkdir(juce_adapter_root)
        io.writefile(
            path.join(juce_adapter_root, "JuceLV2Defines.h"),
            "#pragma once\n#define JucePlugin_LV2URI "
                .. quote_define(identity.lv2.uri) .. "\n")
        add_generated_adapter_sources(target, false)
        local package = target:pkg("juce")
        if string.find(target:name(), "AAX", 1, true) then
            local sdk = path.join(
                package:installdir(),
                "modules", "juce_audio_plugin_client", "AAX", "SDK")
            target:add("includedirs", sdk, path.join(sdk, "Interfaces"), path.join(sdk, "Interfaces", "ACF"))
        elseif string.find(target:name(), "LV2", 1, true) then
            target:add(
                "includedirs",
                path.join(
                    package:installdir(),
                    "modules", "juce_audio_processors_headless", "format_types", "LV2_SDK"),
                path.join(
                    package:installdir(),
                    "modules", "juce_audio_processors_headless", "format_types", "LV2_SDK", "lv2"))
        end
        add_juce_package_sources(target, relative_paths)
    end)
end
local function add_clap_plugin_sources()
    on_load(function (target)
        local json = import("core.base.json")
        local identity = json.loadfile(identity_manifest)
        add_plugin_identity(target, identity)
        os.vrunv("python3", {
            path.absolute(juce_adapter_generator),
            "--output-dir", path.absolute(juce_adapter_dir),
            "--source-root", path.absolute(project_root),
            "--identity", path.absolute(identity_manifest)
        })
        add_generated_adapter_sources(target, false)
        local adapter = target:pkg("clap-juce-extensions")
        local clap = target:pkg("clap")
        local helpers = target:pkg("clap-helpers")
        if not adapter or not clap or not helpers then
            abort("CLAP adapter packages are unavailable")
        end
        target:add("includedirs",
            path.join(adapter:installdir(), "include"),
            path.join(clap:installdir(), "include"),
            path.join(helpers:installdir(), "include"))
        local wrapper = is_plat("macosx") and "clap-juce-mac.mm" or "clap-juce-wrapper.cpp"
        target:add("files",
            path.join(adapter:installdir(), "src", "extensions", "clap-juce-extensions.cpp"),
            path.join(adapter:installdir(), "src", "wrapper", wrapper))
        local features = {}
        for _, feature in ipairs(identity.clap.features) do
            table.insert(features, quote_define(feature))
        end
        target:add("defines",
            "CLAP_ID=" .. quote_define(identity.clap.id),
            "CLAP_FEATURES=" .. table.concat(features, ","),
            "CLAP_MANUAL_URL=" .. quote_define(identity.publisher.website),
            "CLAP_SUPPORT_URL=" .. quote_define(identity.publisher.website))
    end)
end
if has_config("tests") then
    target("DelayLamaEditorInteractionTests")
        set_kind("binary")
        set_languages("cxx20")
        add_files(path.join(project_root, "tests", "platform", "juce", "interaction.cpp"))
        add_plugin_common()
        on_load(function (target)
            local json = import("core.base.json")
            add_plugin_identity(target, json.loadfile(identity_manifest))
            os.vrunv("python3", {
                path.absolute(juce_adapter_generator),
                "--output-dir",
                path.absolute(juce_adapter_dir),
                "--source-root",
                path.absolute(project_root),
                "--identity",
                path.absolute(identity_manifest)
            })
            add_generated_adapter_sources(target, true)
        end)
end
local function add_vst3_manifest_source()
    -- Keep helper setup together because xmake permits one on_load callback per target.
    on_load(function (target)
        local json = import("core.base.json")
        add_plugin_identity(target, json.loadfile(identity_manifest))
        local relative_path = "juce_audio_plugin_client/VST3/juce_VST3ManifestHelper"
            .. (is_plat("macosx") and ".mm" or ".cpp")
        add_juce_package_sources(target, {relative_path})
    end)
end
if requested_formats.vst3 and supports_vst3() then
    target("DelayLamaVST3Manifest")
        set_kind("binary")
        set_default(false)
        add_juce_common()
        add_defines(table.unpack(plugin_defines))
        add_defines(
            "JucePlugin_Build_VST=0",
            "JucePlugin_Build_VST3=1",
            "JucePlugin_Build_AU=0",
            "JucePlugin_Build_AUv3=0",
            "JUCE_VST3_CAN_REPLACE_VST2=0")
        add_deps("juce_core")
        if is_plat("macosx") then
            add_apple_frameworks()
        end
        add_vst3_manifest_source()
end
if requested_formats.vst3 and supports_vst3() then
    target("DelayLama_VST3")
        -- Link as a binary because a shared target remains MH_DYLIB despite -bundle.
        if is_plat("macosx") then
            set_kind("binary")
            add_ldflags("-bundle", {force = true})
        else
            set_kind("shared")
        end
        set_default(false)
        set_extension("vst3")
        add_plugin_common()
        add_plugin_sources({
            "juce_audio_plugin_client/juce_audio_plugin_client_VST3"
                .. (is_plat("macosx") and ".mm" or ".cpp")
        })
        add_defines("JucePlugin_Build_VST3=1", "JucePlugin_Build_AU=0", "JucePlugin_Build_AUv3=0")
end
if requested_formats.clap and supports_clap() then
    target("DelayLama_CLAP")
            if is_plat("macosx") then
            set_kind("binary")
            add_ldflags("-bundle", {force = true})
        else
            set_kind("shared")
        end
        set_default(false)
        set_extension("clap")
        add_plugin_common()
        add_packages("clap", "clap-helpers", "clap-juce-extensions")
        add_clap_plugin_sources()
        add_defines(
            "JucePlugin_Build_VST3=0",
            "JucePlugin_Build_AU=0",
            "JucePlugin_Build_AUv3=0",
            "JucePlugin_Build_AAX=0",
            "JucePlugin_Build_LV2=0",
            "CLAP_MISBEHAVIOUR_HANDLER_LEVEL=Ignore",
            "CLAP_CHECKING_LEVEL=Minimal",
            "CLAP_PROCESS_EVENTS_RESOLUTION_SAMPLES=0",
            "CLAP_ALWAYS_SPLIT_BLOCK=0",
            "CLAP_USE_JUCE_PARAMETER_RANGES=CLAP_USE_JUCE_PARAMETER_RANGES_OFF",
            "CLAP_SUPPORTS_CUSTOM_FACTORY=0")
end
if requested_formats.au and supports_au() then
    target("DelayLama_AU")
        -- AU v2 must link as MH_BUNDLE rather than MH_DYLIB.
        set_kind("binary")
        add_ldflags("-bundle", {force = true})
        set_default(false)
        set_extension("component")
        add_plugin_common()
        add_plugin_sources({
            "juce_audio_plugin_client/juce_audio_plugin_client_AU_1.mm",
            "juce_audio_plugin_client/juce_audio_plugin_client_AU_2.mm"
        })
        add_defines("JucePlugin_Build_VST3=0", "JucePlugin_Build_AU=1", "JucePlugin_Build_AUv3=0")
end
if requested_formats.auv3 and supports_auv3() then
    target("DelayLama_AUv3")
        -- Do not pass -bundle because AUv3 extensions are MH_EXECUTE images.
        set_kind("binary")
        -- Suppress main because Foundation supplies the extension entry point.
        add_cxxflags("-fapplication-extension", {force = true})
        add_mxxflags("-fapplication-extension", {force = true})
        add_ldflags("-e", "_NSExtensionMain", {force = true})
        if is_plat("macosx") then
            local deployment_flag = "-mmacosx-version-min=" .. macos_auv3_minimum_version
            add_cxxflags(deployment_flag, {force = true})
            add_mxxflags(deployment_flag, {force = true})
            add_ldflags(deployment_flag, {force = true})
        end
        set_default(false)
        set_extension("appex")
        add_plugin_common()
        add_plugin_sources({"juce_audio_plugin_client/juce_audio_plugin_client_AUv3.mm"})
        add_defines(
            "JucePlugin_Build_VST3=0",
            "JucePlugin_Build_AU=0",
            "JucePlugin_Build_AUv3=1")
end

if requested_formats.aax and supports_aax() then
    target("DelayLama_AAX")
        if is_plat("macosx") then
            set_kind("binary")
            add_ldflags("-bundle", {force = true})
        else
            set_kind("shared")
        end
        set_default(false)
        set_extension(is_plat("macosx") and "aaxplugin" or "dll")
        add_plugin_common()
        add_plugin_sources({
            "juce_audio_plugin_client/juce_audio_plugin_client_AAX"
                .. (is_plat("macosx") and ".mm" or ".cpp"),
            "juce_audio_plugin_client/juce_audio_plugin_client_AAX_utils.cpp"
        })
        add_defines(
            "JucePlugin_Build_VST3=0",
            "JucePlugin_Build_AU=0",
            "JucePlugin_Build_AUv3=0",
            "JucePlugin_Build_AAX=1",
            "JucePlugin_Build_LV2=0")
end

if requested_formats.lv2 and supports_lv2() then
    target("DelayLamaLV2Manifest")
        set_kind("binary")
        set_default(false)
        set_languages("cxx20")
        add_packages("juce")
        if is_plat("linux") then
            add_links("dl", "pthread")
        end
        on_load(function (target)
            add_juce_package_sources(target, {
                "juce_audio_plugin_client/LV2/juce_LV2ManifestHelper.cpp"
            })
        end)

    target("DelayLama_LV2")
        -- Keep the .so name on Unix because generated Turtle embeds it.
        if is_plat("macosx") then
            set_kind("binary")
            add_ldflags("-bundle", {force = true})
        else
            set_kind("shared")
        end
        set_default(false)
        set_extension(is_plat("windows") and "dll" or "so")
        add_plugin_common()
        add_plugin_sources({
            "juce_audio_plugin_client/juce_audio_plugin_client_LV2"
                .. (is_plat("macosx") and ".mm" or ".cpp")
        })
        add_defines(
            "JucePlugin_Build_VST3=0",
            "JucePlugin_Build_AU=0",
            "JucePlugin_Build_AUv3=0",
            "JucePlugin_Build_AAX=0",
            "JucePlugin_Build_LV2=1")
end

if requested_formats.auv3 and is_plat("macosx") then
    target("DelayLama_AUv3Host")
        set_kind("binary")
        set_default(false)
        set_languages("cxx20")
        add_cxxflags("-mmacosx-version-min=" .. macos_auv3_minimum_version, {force = true})
        add_ldflags("-mmacosx-version-min=" .. macos_auv3_minimum_version, {force = true})
        add_files(path.join(project_root, "src", "platform", "auv3", "main.cpp"))
end

target("DelayLamaPlugins")
    set_kind("phony")
    set_default(true)
    if requested_formats.vst3 and supports_vst3() then
        add_deps("DelayLamaVST3Manifest")
        add_deps("DelayLama_VST3")
    end
    if requested_formats.clap and supports_clap() then
        add_deps("DelayLama_CLAP")
    end
    if requested_formats.au and supports_au() then
        add_deps("DelayLama_AU")
    end
    if requested_formats.auv3 and supports_auv3() then
        add_deps("DelayLama_AUv3")
        if is_plat("macosx") then
            add_deps("DelayLama_AUv3Host")
        end
    end
    if requested_formats.aax and supports_aax() then
        add_deps("DelayLama_AAX")
    end
    if requested_formats.lv2 and supports_lv2() then
        add_deps("DelayLamaLV2Manifest")
        add_deps("DelayLama_LV2")
    end
    on_buildcmd(function (target, batchcmds)
        local script = path.join(project_root, "scripts", "package_bundles.py")
        local output_root = path.join(project_root, "build", "bundles")
        local manifest = path.join(project_root, "config", "identity.json")
        local platform_name = is_plat("macosx") and "macosx"
            or (is_plat("iphoneos") and "iphoneos"
                or (is_plat("windows") and "windows" or "linux"))
        local architecture = get_config("arch") or ""
        local function package(format, dependency_name)
            local dependency = target:dep(dependency_name)
            if dependency == nil then
                return
            end
            local binary = dependency:targetfile()
            local arguments = {
                script,
                format,
                path.absolute(binary),
                path.absolute(output_root),
                path.absolute(manifest),
                "--platform",
                platform_name,
                "--arch",
                architecture
            }
            local manifest_tool
            if format == "vst3" then
                manifest_tool = target:dep("DelayLamaVST3Manifest")
                if manifest_tool == nil then
                    abort("VST3 manifest helper dependency is unavailable")
                end
                table.insert(arguments, "--module-info-tool")
                table.insert(arguments, path.absolute(manifest_tool:targetfile()))
            elseif format == "lv2" then
                manifest_tool = target:dep("DelayLamaLV2Manifest")
                if manifest_tool == nil then
                    abort("LV2 manifest helper dependency is unavailable")
                end
                table.insert(arguments, "--lv2-manifest-tool")
                table.insert(arguments, path.absolute(manifest_tool:targetfile()))
            end
            local host_binary
            if format == "auv3" and platform_name == "macosx" then
                local host = target:dep("DelayLama_AUv3Host")
                if host == nil then
                    abort("macOS AUv3 host dependency is unavailable")
                end
                host_binary = host:targetfile()
                table.insert(arguments, "--auv3-host-binary")
                table.insert(arguments, path.absolute(host_binary))
            end
            -- Keep packaging phony so deleted or locally signed bundles are rebuilt.
            batchcmds:vrunv("python3", arguments)
        end
        if requested_formats.vst3 and supports_vst3() then
            package("vst3", "DelayLama_VST3")
        end
        if requested_formats.clap and supports_clap() then
            package("clap", "DelayLama_CLAP")
        end
        if requested_formats.au and supports_au() then
            package("au", "DelayLama_AU")
        end
        if requested_formats.auv3 and supports_auv3() then
            package("auv3", "DelayLama_AUv3")
        end
        if requested_formats.aax and supports_aax() then
            package("aax", "DelayLama_AAX")
        end
        if requested_formats.lv2 and supports_lv2() then
            package("lv2", "DelayLama_LV2")
        end
    end)
