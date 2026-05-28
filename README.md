<p align="center">
  <img src="usr/share/icons/hicolor/scalable/apps/org.communitybig.bigrecorder.svg" width="128" height="128" alt="Big Recorder icon">
</p>

<h1 align="center">Big Recorder</h1>

<p align="center">Modern local voice recorder for Linux, built with Rust, GTK4, libadwaita, and GStreamer.</p>

<p align="center">
  <code>Rust</code>
  <code>GTK4</code>
  <code>libadwaita</code>
  <code>GStreamer</code>
  <code>Gettext</code>
  <code>Linux</code>
</p>

## Features

- Local microphone recording with pause, resume, and stop.
- Live waveform animation while recording.
- Saved recordings list with in-app playback and progress display.
- Fixed tools footer for quick access to editing actions.
- Audio tools: trim, volume, speed, pitch, voice effects, 10-band equalizer, reverse, merge, and export.
- Export formats: WAV, MP3, Opus, and FLAC.
- Gettext-ready interface with English source strings and external translations.

## Build

Native requirements:

- Rust toolchain
- GTK4 development files
- libadwaita development files
- GStreamer development files
- GStreamer audio plugins: good, bad, ugly, libav

```sh
cargo build
```

```sh
cargo run
```

## Runtime Data

Recordings are stored in:

```text
~/Music/BigRecorder
```

Application ID:

```text
org.communitybig.bigrecorder
```

Binary name:

```text
bigrecorder
```

## Project Layout

```text
src/audio/      Recording, playback, processing, export
src/ui/         GTK4/libadwaita interface and drawing widgets
po/             Gettext catalogs
usr/share/      Desktop file, icon, metainfo
pkgbuild/       BigCommunity package files
```

## Validation

```sh
cargo fmt --check
cargo clippy --offline -- -D warnings
cargo test --offline
desktop-file-validate usr/share/applications/org.communitybig.bigrecorder.desktop
```

## License

GPL-3.0-or-later
