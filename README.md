# DelayLama

C++20 software instrument for VST3, CLAP, AU, AUv3, AAX, and LV2 hosts.

## Build

Requires Xmake and a C++20 compiler. AU and AUv3 require Xcode. AAX requires the Avid SDK.

```sh
xmake require -y juce clap clap-helpers clap-juce-extensions
xmake f -m release --formats=vst3,clap,au -y
xmake build DelayLamaPlugins
```

Set `--formats` to any comma-separated selection of `vst3`, `clap`, `au`, `auv3`, `aax`, and `lv2`. Bundles are written to `build/bundles/`.

AUv3 examples:

```sh
xmake f -c -m release --formats=vst3,au,auv3 --target_minver=13.0 -y
xmake f -c -p iphoneos -a arm64 -m release --formats=auv3 --tests=n --target_minver=13.0 -y
```

## Test

```sh
just test
just check
```

## Install on macOS

```sh
AUDIO_PLUGIN_SIGNING_IDENTITY='Apple Development: …' just install-au-vst3-dev
AUV3_SIGNING_IDENTITY='Apple Development: …' just install-auv3-dev
```

Remove installed development builds with `just uninstall-au-vst3-dev` and `just uninstall-auv3-dev`.

## License

AudioNerdz distributed Delay Lama as freeware, but that statement does not grant a source-code license. This repository does not apply MIT, Unlicense, or another open-source license. Redistribution requires permission from the relevant code, name, and artwork rights holders.
