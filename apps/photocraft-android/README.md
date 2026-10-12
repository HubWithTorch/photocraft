# PhotoCraft for Android (experimental)

Native eframe/wgpu application for ARM64 Android 9+ (API 28+). The editor, engine,
rendering and document codecs are Rust. A small NativeActivity Java adapter invokes
Android's Storage Access Framework and receives VIEW/SEND intents; it contains no UI
framework. No webview is used. This platform adapter currently needs two unsafe JNI
bootstrap calls, isolated in `src/android.rs`, to wrap android-activity's JVM and Activity
handles. This is a platform exception requiring upstream review.

## Build

Install the workspace Rust toolchain, cargo-ndk, Android SDK 35, NDK r27c and JDK 17+.
Set ANDROID_HOME (SDK), ANDROID_NDK_HOME (NDK) and JAVA_HOME (JDK).

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk --locked
cargo ndk -t arm64-v8a -o apps/photocraft-android/android/app/src/main/jniLibs build -p photocraft-android --release
cd apps/photocraft-android/android
./gradlew assembleDebug
```

On Windows use `gradlew.bat`. Debug APK: `app/build/outputs/apk/debug/app-debug.apk`.
For release use `assembleRelease`. Release signing is optional and uses only environment
variables: PHOTOCRAFT_KEYSTORE (absolute path), PHOTOCRAFT_STORE_PASSWORD,
PHOTOCRAFT_KEY_ALIAS and PHOTOCRAFT_KEY_PASSWORD. Supply all four to sign; otherwise
Gradle produces an unsigned release. Keep your key for future updates. Never commit
keystores, credentials, local.properties, compiled .so files, APKs or Gradle caches.

## Files and touch

File > Open uses ACTION_OPEN_DOCUMENT and supports multiple files. Save As uses
ACTION_CREATE_DOCUMENT. The Rust writer retains a separate document-provider URI for
each save target, so subsequent Save operations update the same file. Files received
from a file manager or Share are queued at cold launch and onNewIntent. Open dialogs
accept all MIME types because PSD/PSB/pcraft providers often use application/octet-stream.
No broad storage permission is requested. Reads are capped at 512 MiB; larger inputs
report an error. Display names are kept separately from provider URIs.

Two fingers zoom around their centroid and pan the canvas. A gesture cancels an
uncommitted brush preview and suppresses tool input until all fingers leave the screen.
DPI, view rotation, mirror and pixel aspect are accounted for.

## Validation and limitations

See [ANDROID_DEV.md](ANDROID_DEV.md) for the tested build and evidence. The desktop
panel layout is retained; phone portrait layout, larger touch targets and stylus
axis mapping still need work. Do not present this community Android build as an
upstream official release.
