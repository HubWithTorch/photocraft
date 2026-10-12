# Android alpha.2 validation — 2026-10-12

Base: PhotoCraft 0.7.0, commit 614d0689. Community build by @HubWithTorch.
APK: photocraft-android-v0.7.0-alpha.2.apk, ARM64, 58,726,588 bytes.
SHA256: 5BFB11ABC6389FAAA4C3788669DF7C9CF24E1FEC4B248F4FE3B8700D2253777D.

## Fixes

The initial local Android shell used independent uninitialized OnceLocks for picker
results, ended polling immediately, omitted import/export services and consumed a
single global save URI. It could neither reliably open nor repeatedly save documents.
The replacement uses a JVM global reference and Activity instance methods, starts
pickers on the Android UI thread, polls until selection/cancellation, reports errors,
closes streams, and retains per-target save capabilities. VIEW/SEND lifecycle events
are forwarded into the shared editor. The shell forwards eframe ui/logic/raw_input_hook.

Canvas touch navigation supports centroid zoom/pan and prevents stray brush strokes
when a second finger arrives or one finger remains. Three regression tests exercise
DPI/rotation/mirror/aspect, invalid input and an egui multi-touch event sequence. A host
service test exercises picker import, PNG export and two saves to the same target.

## Evidence

Lenovo TB378FC, Android 16, USB debugging: system picker PNG open, PSD Save As,
brush edit followed by Save, and Files cold launch of the saved PSD all succeeded.
The PSD grew from 46,494 to 50,501 bytes and reopened with the brush edit preserved.
Physical two-finger gesture feel is still awaiting user confirmation; synthetic touch
regressions are not a substitute for real multi-touch validation.

On the 0.7.0 base: Android service test (1), touch tests (3), UI unit tests (1953,
10 ignored), host and Android clippy, layers, wasm and xtask tests (48) passed.
An offscreen safe-GPU UI snapshot was visually inspected. Twelve host GPU integration
test targets failed with Microsoft Basic Render Driver; these are not claimed to pass.
Quick performance suite completed; some complex layout actions exceed 16 ms, and no
baseline was updated. These results apply to the tested 0.7.0 build, not future rebases.

## Source map

- src/android.rs: JNI bridge, document-provider streams, NativeActivity and intent loop.
- src/services.rs: import/export service wiring and host regression test.
- android/app/src/main/java/ai/storyteller/photocraft/FileHelper.java: system picker glue.
- crates/ui-egui/src/canvas.rs: shared touch navigation and tests.
- crates/text/src/{fonts,cjk}.rs: Android system font lookup.

The two existing unsafe JNI bootstrap calls are an explicit upstream-review item.
The original development notes are preserved locally in log/android-development-history.md.
