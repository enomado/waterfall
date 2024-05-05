# Android eframe template
A basic eframe template with android support.

## Compile for Desktop
Run cargo as normal
```
cargo build
```

## Compile for Web
use [Trunk](https://trunkrs.dev/#install).
```
trunk build
```

## Compile for Android
use [xbuild](https://github.com/rust-mobile/xbuild).
```
x build --platform android --arch arm64
```

If this fails check that you have the required external build tools installed with
```
x doctor
```
