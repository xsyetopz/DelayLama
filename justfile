set shell := ["sh", "-cu"]

auv3_host_bundle := "target/bundles/Delay Lama.app"
auv3_extension_bundle := "target/bundles/Delay Lama.app/Contents/PlugIns/AUExt.appex"
auv3_extension_identifier := "com.audionerdz.delaylama.v3.ext"
auv3_audio_component_cache_key := "7-'aumu'-'DLma'-'xsyT'-0x10000"
auv3_logic_component_cache_key := "aumu-DLma-xsyT"
auv3_entitlements := "config/entitlements/auv3.plist"

_default:
    @just --list

rust-test:
    cargo test

rust-build:
    cargo build --release

rust-bundles:
    cargo truce build --clap --vst3 --au2 -p delaylama-truce-plugin

build-auv3:
    python3 scripts/auv3.py

universal-macos-package:
    python3 scripts/auv3.py --universal-package

[script]
sign-auv3-dev: build-auv3
    #!/bin/sh
    set -eu
    signing_identity="${AUV3_SIGNING_IDENTITY:-}"
    case "$signing_identity" in
        "Apple Development:"*) ;;
        *) printf '%s\n' 'AUV3_SIGNING_IDENTITY must name an Apple Development certificate.' >&2; exit 1 ;;
    esac
    codesign --force --sign "$signing_identity" --entitlements '{{auv3_entitlements}}' --timestamp=none '{{auv3_extension_bundle}}'
    codesign --force --sign "$signing_identity" --entitlements '{{auv3_entitlements}}' --timestamp=none '{{auv3_host_bundle}}'
    codesign --verify --deep --strict --verbose=2 '{{auv3_host_bundle}}'

[script]
ensure-logic-is-closed:
    #!/bin/sh
    set -eu
    if pgrep -x 'Logic Pro' >/dev/null; then
        printf '%s\n' 'Quit Logic Pro before replacing its AUv3 extension.' >&2
        exit 1
    fi

[script]
clear-auv3-host-caches: ensure-logic-is-closed
    #!/bin/sh
    set -eu
    defaults delete com.apple.audio.AudioComponentCache '{{auv3_audio_component_cache_key}}' >/dev/null 2>&1 || true
    defaults delete com.apple.logic10 '{{auv3_logic_component_cache_key}}' >/dev/null 2>&1 || true

[script]
register-auv3-dev: sign-auv3-dev ensure-logic-is-closed
    #!/bin/sh
    set -eu
    pluginkit -a "$(pwd -P)/{{auv3_extension_bundle}}"
    pluginkit -e use -i '{{auv3_extension_identifier}}'
    just clear-auv3-host-caches
    just verify-auv3-registration

[script]
install-auv3-dev: sign-auv3-dev ensure-logic-is-closed
    #!/bin/sh
    set -eu
    install_root="${AUV3_INSTALL_ROOT:-/Applications}"
    source_app="$(pwd -P)/{{auv3_host_bundle}}"
    installed_app="$install_root/Delay Lama.app"
    installed_extension="$installed_app/Contents/PlugIns/AUExt.appex"
    test -d "$source_app"
    mkdir -p "$install_root"
    if test -e "$installed_app"; then
        backup_root="$(mktemp -d "${TMPDIR:-/tmp}/delaylama-auv3.XXXXXX")"
        mv "$installed_app" "$backup_root/Delay Lama.app"
    fi
    ditto "$source_app" "$installed_app"
    codesign --verify --deep --strict --verbose=2 "$installed_app"
    pluginkit -a "$installed_extension"
    pluginkit -e use -i '{{auv3_extension_identifier}}'
    just clear-auv3-host-caches
    sleep 2
    AUV3_EXTENSION_PATH="$installed_extension" just verify-auv3-registration

[script]
verify-auv3-registration:
    #!/bin/sh
    set -eu
    expected="${AUV3_EXTENSION_PATH:-$(pwd -P)/{{auv3_extension_bundle}}}"
    attempt=0
    while test "$attempt" -lt 60; do
        test -d "$expected" && pluginkit -a "$expected" >/dev/null 2>&1 || true
        pluginkit -e use -i '{{auv3_extension_identifier}}'
        matches="$(pluginkit -m -v -A -D -i '{{auv3_extension_identifier}}' || true)"
        active_match="$(printf '%s\n' "$matches" | awk -v expected="$expected" '$0 ~ /^[+]/ && index($0, expected) { print; exit }')"
        if test -n "$active_match"; then
            printf '%s\n' "$active_match"
            exit 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done
    printf 'Active AUv3 registration not found after 60 seconds: %s\n' "$expected" >&2
    exit 1

[script]
uninstall-auv3-dev:
    #!/bin/sh
    set -eu
    pluginkit -r "$(pwd -P)/{{auv3_extension_bundle}}" || true
