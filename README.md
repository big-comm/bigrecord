<p align="center">
  <img src="usr/share/icons/hicolor/scalable/apps/org.communitybig.bigrecorder.svg" width="128" height="128" alt="Big Recorder icon">
</p>

<h1 align="center">Big Recorder</h1>

<p align="center">Modern local voice recorder for Linux, built with Rust, GTK4, libadwaita, and GStreamer.</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-f74c00?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="GTK4" src="https://img.shields.io/badge/GTK4-3584e4?style=for-the-badge&logo=gnome&logoColor=white">
  <img alt="libadwaita" src="https://img.shields.io/badge/libadwaita-9141ac?style=for-the-badge&logo=gnome&logoColor=white">
  <img alt="GStreamer" src="https://img.shields.io/badge/GStreamer-e95420?style=for-the-badge&logo=gstreamer&logoColor=white">
  <img alt="Gettext" src="https://img.shields.io/badge/Gettext-26a269?style=for-the-badge">
  <img alt="License MIT" src="https://img.shields.io/badge/License-MIT-2ec27e?style=for-the-badge">
</p>

<p align="center">
  <strong>Local recording</strong> ·
  <strong>Waveform playback</strong> ·
  <strong>Audio editing</strong> ·
  <strong>Export tools</strong> ·
  <strong>Translations</strong>
</p>

## Overview

Big Recorder is a native Linux voice recorder focused on fast local capture,
simple playback, and practical editing tools. It records from the microphone,
stores audio in the user's Music directory, and keeps the workflow local without
uploading recordings to external services.

The interface follows GTK4 and libadwaita conventions, supports light and dark
themes, and keeps frequently used tools available from the main window.

## Features

- Local microphone recording with pause, resume, and stop.
- Live waveform animation during recording.
- Saved recordings list with in-app playback and progress display.
- Recording deletion through the system trash.
- Compact fixed tools footer for quick editing actions.
- Trim, volume, speed, pitch, voice effects, equalizer, reverse, merge, and export tools.
- Export formats: WAV, MP3, Opus, and FLAC.
- Gettext-ready interface with English source strings and external translations.

## Audio Tools

Big Recorder includes focused tools for common voice recording adjustments:

- **Trim**: cut unwanted parts from a recording.
- **Volume**: increase or reduce the recording level.
- **Speed**: change playback/export speed.
- **Pitch**: adjust tonal height without changing the source file directly.
- **Voice effects**: apply character presets such as robot, child, deep voice,
  radio, echo, and telephone-style effects.
- **Equalizer**: shape frequencies with a 10-band visual control.
- **Reverse**: create a reversed copy of the selected recording.
- **Merge**: combine recordings into one file.
- **Export**: save processed audio as WAV, MP3, Opus, or FLAC.

## Interface

- Main page with recordings, playback controls, and a central record button.
- Dedicated recording page with live visual feedback while speaking.
- Playback progress bars that show spoken and silent segments.
- Dialogs with native-style controls and symbolic status icons.
- Automatic adaptation to the active system color scheme.

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

## Install Layout

Packaged installations use these application files:

```text
usr/bin/bigrecorder
usr/share/applications/org.communitybig.bigrecorder.desktop
usr/share/icons/hicolor/scalable/apps/org.communitybig.bigrecorder.svg
usr/share/metainfo/org.communitybig.bigrecorder.metainfo.xml
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

## Localization

The source code uses English strings with gettext. Translation catalogs live in
`po/`, and compiled catalogs are installed as `bigrecorder.mo`.

## Validation

```sh
cargo fmt --check
cargo clippy --offline -- -D warnings
cargo test --offline
desktop-file-validate usr/share/applications/org.communitybig.bigrecorder.desktop
```

## License

MIT
