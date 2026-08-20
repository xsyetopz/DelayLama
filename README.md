# Delay Lama

![Delay Lama (VST3) on FL Studio 2026](image.png)

**Delay Lama** is a monophonic vocal synthesizer rebuilt in Rust from the AudioNerdz plug-in. It shapes each note with a vowel/formant model. The monk interface follows the current voice.

Play it from a MIDI keyboard or the built-in XY pad. The pad controls pitch on the horizontal axis and vowel on the vertical axis. Four host parameters shape the sound:

- **Vowel** moves through the formant range.
- **Portamento** sets the glide time between notes.
- **Delay** controls the stereo delay mix.
- **Voice** changes the vocal character.

The project builds CLAP, VST3, AUv2, and AUv3 plug-in bundles.

## Build

The root Rust package owns DSP, host state, editor state, artwork, and format exports:

```sh
just rust-test
just rust-build
just rust-bundles
```

Editor artwork is stored as lossless QOI and decoded directly into RGBA texture data.

Build universal macOS raw bundles and a local installer for Apple Silicon and Intel:

```sh
set -a; . ./.env; set +a
export DEVELOPER_DIR=/Applications/Xcode-26.6.0.app/Contents/Developer
just universal-macos-package
```

The installer is written to `target/dist/`, and the raw bundles are copied to
`target/universal-bundles/`. The bundles use `TRUCE_SIGNING_IDENTITY`. The installer
is not notarized and is signed only when `TRUCE_INSTALLER_SIGNING_IDENTITY` is set.

Build the AUv3 with full Xcode:

```sh
set -a; . ./.env; set +a
export DEVELOPER_DIR=/Applications/Xcode-26.6.0.app/Contents/Developer
just install-auv3-dev
just verify-auv3-registration
```

## Test

```sh
just rust-test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
```

Remove the development AUv3 registration with `just uninstall-auv3-dev`.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

AudioNerdz distributed Delay Lama as freeware. Redistribution requires permission from the relevant code, name, and artwork rights holders.
