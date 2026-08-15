set shell := ["sh", "-cu"]

auv3_host_bundle := "target/bundles/Delay Lama.app"
auv3_extension_bundle := "target/bundles/Delay Lama.app/Contents/PlugIns/AUExt.appex"
auv3_extension_identifier := "com.audionerdz.delaylama.v3.ext"
auv3_entitlements := "config/entitlements/auv3.plist"

_default:
    @just --list

rust-test:
    cargo test --workspace

rust-build:
    cargo build --workspace --release

rust-bundles:
    cargo truce build --clap --vst3 --au2 -p delaylama-truce-plugin

build-auv3:
    python3 scripts/build_rust_auv3.py

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
register-auv3-dev: sign-auv3-dev
    #!/bin/sh
    set -eu
    pluginkit -a "$(pwd -P)/{{auv3_extension_bundle}}"
    just verify-auv3-registration

install-auv3-dev: register-auv3-dev

[script]
verify-auv3-registration:
    #!/bin/sh
    set -eu
    expected="$(pwd -P)/{{auv3_extension_bundle}}"
    matches="$(pluginkit -m -v -A -D -i '{{auv3_extension_identifier}}' || true)"
    printf '%s\n' "$matches" | grep -F -- "$expected" >/dev/null || { printf 'AUv3 registration not found: %s\n' "$expected" >&2; exit 1; }
    printf '%s\n' "$matches" | grep -F -- "$expected" | head -n 20

[script]
uninstall-auv3-dev:
    #!/bin/sh
    set -eu
    pluginkit -r "$(pwd -P)/{{auv3_extension_bundle}}" || true

check: rust-test

test: rust-test
