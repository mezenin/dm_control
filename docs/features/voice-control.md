# DM Control voice control

Personal-fork capability, added to Hex 3.1.1. macOS only, available in every build, off by default in Labs. Settings: `voice_control.enabled` and `voice_control.wake_phrase` (default `Hello Hex`, 2–5 English words). Commands support Russian and English through the selected Hex speech model.

```ts
Labs › Voice control on
├── one worker loads the bundled English KWS model on the CPU (one thread)
├── the existing microphone owner stays open; no second device stream
├── bounded audio projection → wake phrase
│   ├── stale audio, queue overflow, input discontinuity → invalidate generation
│   └── wake → visible recording HUD → command capture (at most 8 seconds)
├── speech plus 700 ms pause → recognize on Hex's shared engine
│   ├── dictation gets priority; a superseded command cannot execute
│   ├── while recognizing/executing, discard background input before queueing
│   └── exact local command grammar → explicit multimedia action
└── toggle off / change phrase / ordinary dictation → invalidate pending command
```

No command audio is persisted. Unknown, ambiguous, negated, out-of-range, or unsupported commands do nothing. Recognized speech never becomes an executable path, AppleScript source, or shell expression. Local subprocesses have a five-second deadline. The wake detector is not speaker authentication.

Actions: volume steps and 0–100% levels, mute/unmute, play/pause/resume/next/previous, external-display DDC brightness, Digital Light power/mode, lock, sleep, cancel. Sleep and lock require the explicit phrases listed by the parser. The app does not listen during system sleep.

Digital Light: confirmed from the existing DM Bot and the installed app's Swift source. Power uses `⌘⌥⌃⇧1`; mode uses `⌘⌥⌃⇧2`. Explicit on/off reads `mezenin.Digital-Light` / `saved_isOn`, requires the app to be running, and sends the power hotkey only if a state change is needed. Media uses the already installed `nowplaying-cli`; external brightness and unsupported system-volume outputs use the already installed `m1ddc`. The first version has no built-in-display brightness adapter.

The personal app uses bundle ID `mezenin.dm-control`, data in `Application Support/dm-control`, config in `~/.config/dm-control`, and its own permissions. Upstream self-updates are disabled. No legacy original-Hex TCC entries are reset.

Implementation: [runtime](../../src/voice_control/runtime.rs), [detector](../../src/voice_control/detector.rs), [command grammar](../../src/voice_control/commands.rs), [macOS actions](../../src/voice_control/platform.rs), [microphone projection](../../src/recognition.rs), [Labs UI](../../src/rune_ui/labs.rs).

## Observed verification

November 1, 2026, Apple M4 Pro (12 cores), 24 GB:

- Separate CPU-only sherpa-onnx 2.0.1 benchmark, int8 model, 30 seconds paced silence and 30 seconds paced public speech: 2.80% and 2.82% of one CPU core. Peak test-process RSS: 121.27 MiB, including Python and both model-variant trials. This is not a measurement of the finished native mode, microphone capture, or battery use.
- Native command check: “Hello Hex, громкость пятьдесят пять процентов” → `SetVolume(55)`, `executed: false`.
- Native detector on a 1.03-second system-TTS WAV of “Hello Hex” → one detection. This confirms tokenizer/model/native wiring, not recognition accuracy with the owner's voice or false activations during films.
- Source-shape and `cargo clippy --all-targets -- -D warnings` pass. The complete native suite passes through nextest: 1,904 tests passed, 26 skipped. Checks include command parsing, capture endpoints, queue backpressure, settings registration, and the original dictation behavior. Four macOS capture-permission goldens were reviewed and updated for the new product name. The expanded settings were visually reviewed in light and dark appearances, including a 720-point window; its language guidance was moved to a full-width row to avoid a narrow text column. The built `.app` has the correct name, bundle ID and icon, and passes strict signature verification.

Live microphone, physical display and Digital Light end-to-end checks require the new app's macOS permissions. No live sleep, volume, media playback, or lighting action was executed during automated verification.
