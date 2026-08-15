# DelayLama

Rust software instrument for CLAP, VST3, AUv2, and AUv3 hosts.

## Build

The Rust workspace owns DSP, host state, editor state, artwork, and format exports:

```sh
just rust-test
just rust-build
just rust-bundles
```

Build the AUv3 with full Xcode:

```sh
set -a; . ./.env; set +a
export DEVELOPER_DIR=/Applications/Xcode-26.6.0.app/Contents/Developer
just install-auv3-dev
just verify-auv3-registration
```

## Test

```sh
just test
just check
```

Remove the development AUv3 registration with `just uninstall-auv3-dev`.

## License

AudioNerdz distributed Delay Lama as freeware. Redistribution requires permission from the relevant code, name, and artwork rights holders.
