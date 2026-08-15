set shell := ["sh", "-cu"]

mode := env("XMAKE_MODE", "release")
formats := env("XMAKE_FORMATS", "vst3,clap,au")
auv3_macos_minimum_version := "13.0"
auv3_host_bundle := "build/bundles/auv3/Delay Lama AUv3 Host.app"
auv3_extension_bundle := "build/bundles/auv3/Delay Lama AUv3 Host.app/Contents/PlugIns/Delay Lama.appex"
auv3_extension_identifier := "com.xsyetopz.delaylama.delaylamaAUv3"
auv3_entitlements := "config/entitlements/auv3.plist"
clap_validator := env("CLAP_VALIDATOR", "clap-validator")

default:
    @just --list

deps:
    xmake require -y juce clap clap-helpers clap-juce-extensions

configure:
    xmake f -m {{mode}} --formats={{formats}} -y

project: configure
    xmake project -k compile_commands --lsp=clangd -y

build: configure
    xmake build DelayLamaPlugins

build-au-vst3:
    xmake f -m {{mode}} --formats=vst3,au -y
    xmake build DelayLamaPlugins

build-clap:
    xmake f -m {{mode}} --formats=clap --tests=n -y
    xmake build DelayLamaPlugins

verify-clap: build-clap
    {{clap_validator}} validate 'build/bundles/clap/Delay Lama.clap'

build-auv3:
    xmake f -m {{mode}} --formats=vst3,au,auv3 --target_minver={{auv3_macos_minimum_version}} -y
    xmake build DelayLamaPlugins
    xmake f -m {{mode}} --formats={{formats}} -y

[script]
sign-auv3-dev: build-auv3
    #!/bin/sh
    set -eu
    signing_identity="${AUV3_SIGNING_IDENTITY:-}"
    case "$signing_identity" in
        "Apple Development:"*) ;;
        *)
            printf '%s\n' 'AUV3_SIGNING_IDENTITY must name an Apple Development certificate; ad-hoc signing is unsupported.' >&2
            exit 1
            ;;
    esac
    codesign --force --sign "$signing_identity" --entitlements '{{auv3_entitlements}}' --timestamp=none '{{auv3_extension_bundle}}'
    codesign --force --sign "$signing_identity" --entitlements '{{auv3_entitlements}}' --timestamp=none '{{auv3_host_bundle}}'
    codesign --verify --deep --strict --verbose=2 '{{auv3_host_bundle}}'

[script]
register-auv3-dev: sign-auv3-dev
    #!/bin/sh
    set -eu
    repo_root="$(pwd -P)"
    expected_extension="$repo_root/{{auv3_extension_bundle}}"
    pluginkit -a "$expected_extension"
    just verify-auv3-registration

install-auv3-dev: register-auv3-dev

[script]
install-au-vst3-dev: build-au-vst3
    #!/bin/sh
    set -eu
    signing_identity="${AUDIO_PLUGIN_SIGNING_IDENTITY:-}"
    case "$signing_identity" in
        "Apple Development:"*) ;;
        *)
            printf '%s\n' 'AUDIO_PLUGIN_SIGNING_IDENTITY must name an Apple Development certificate; ad-hoc signing is unsupported.' >&2
            exit 1
            ;;
    esac
    repo_root="$(pwd -P)"
    au_source="$repo_root/build/bundles/au/Delay Lama.component"
    vst3_source="$repo_root/build/bundles/vst3/Delay Lama.vst3"
    au_destination="$HOME/Library/Audio/Plug-Ins/Components/Delay Lama.component"
    vst3_destination="$HOME/Library/Audio/Plug-Ins/VST3/Delay Lama.vst3"
    test -f "$au_source/Contents/Resources/artifact-manifest.json"
    test -f "$vst3_source/Contents/Resources/artifact-manifest.json"
    rm -rf "$au_destination" "$vst3_destination"
    mkdir -p "$(dirname "$au_destination")" "$(dirname "$vst3_destination")"
    ditto "$au_source" "$au_destination"
    ditto "$vst3_source" "$vst3_destination"
    codesign --force --deep --sign "$signing_identity" --timestamp=none "$au_destination"
    codesign --force --deep --sign "$signing_identity" --timestamp=none "$vst3_destination"
    codesign --verify --deep --strict --verbose=2 "$au_destination"
    codesign --verify --deep --strict --verbose=2 "$vst3_destination"

[script]
uninstall-au-vst3-dev:
    #!/bin/sh
    set -eu
    rm -rf "$HOME/Library/Audio/Plug-Ins/Components/Delay Lama.component"
    rm -rf "$HOME/Library/Audio/Plug-Ins/VST3/Delay Lama.vst3"

[script]
verify-auv3-registration:
    #!/bin/sh
    set -eu
    repo_root="$(pwd -P)"
    expected_extension="$repo_root/{{auv3_extension_bundle}}"
    matches="$(pluginkit -m -v -A -D -i '{{auv3_extension_identifier}}' || true)"
    if ! printf '%s\n' "$matches" | grep -F -- "$expected_extension" >/dev/null; then
        printf '%s\n' 'AUv3 registration was not found in the PlugInKit registry.' >&2
        printf 'Expected extension identifier: %s\n' '{{auv3_extension_identifier}}' >&2
        printf 'Expected nested extension path: %s\n' "$expected_extension" >&2
        if test -n "$matches"; then
            printf '%s\n' "$matches" | head -n 20 >&2
        fi
        exit 1
    fi
    printf '%s\n' "$matches" | grep -F -- "$expected_extension" | head -n 20

[script]
uninstall-auv3-dev:
    #!/bin/sh
    set -eu
    repo_root="$(pwd -P)"
    pluginkit -r "$repo_root/{{auv3_extension_bundle}}"

test:
    xmake f -m {{mode}} --formats={{formats}} --tests=y -y
    xmake build DelayLamaDspTests DelayLamaHostTests DelayLamaEditorTests DelayLamaPlatformTests
    xmake run DelayLamaDspTests
    xmake run DelayLamaHostTests
    xmake run DelayLamaEditorTests
    xmake run DelayLamaPlatformTests

format-check:
    find src tests -type f \( -name '*.c' -o -name '*.cc' -o -name '*.cpp' -o -name '*.mm' -o -name '*.h' -o -name '*.hpp' \) -print0 | xargs -0 clang-format --dry-run --Werror

[script]
tidy:
    #!/bin/sh
    set -eu
    tidy="${CLANG_TIDY:-$(command -v clang-tidy || true)}"
    if test -z "$tidy" && test -x /opt/homebrew/opt/llvm/bin/clang-tidy; then
        tidy=/opt/homebrew/opt/llvm/bin/clang-tidy
    fi
    if test -z "$tidy"; then
        printf '%s\n' 'clang-tidy not found; set CLANG_TIDY to its executable path' >&2
        exit 1
    fi
    find src tests -type f \( -name '*.cpp' -o -name '*.mm' \) -print0 | xargs -0 "$tidy" -p . --config-file=.clang-tidy --header-filter='(^|.*/)(src|tests)/.*'

python-check:
    ruff check scripts
    ruff format --check scripts
    pyright scripts
    mypy --strict scripts
    python3 -m compileall -q scripts

python-test:
    python3 -m unittest discover -s tests/tooling -t . -p '*_contract.py'

check: python-check python-test format-check tidy

alias b := build
alias t := test
alias c := check
