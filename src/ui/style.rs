const STYLE: &str = r#"
window,
.app-bg {
  background-color: @window_bg_color;
  color: @window_fg_color;
}

.flat-header,
.flat-toolbar,
headerbar,
windowhandle,
.titlebar {
  background-color: @window_bg_color;
  color: @window_fg_color;
  border: 0;
  box-shadow: none;
}

.recorder-surface {
  background-color: @card_bg_color;
  color: @window_fg_color;
  border: 1px solid alpha(@accent_bg_color, 0.26);
  border-radius: 8px;
  padding: 12px;
}

.timer {
  font-size: 34px;
  font-weight: 500;
  font-feature-settings: "tnum";
  color: @window_fg_color;
}

.status-pill {
  background-color: alpha(@accent_bg_color, 0.12);
  color: @accent_color;
  border: 1px solid alpha(@accent_bg_color, 0.28);
  border-radius: 999px;
  padding: 3px 10px;
  font-weight: 650;
}

.status-pill.recording {
  background-color: alpha(@accent_bg_color, 0.18);
  color: @accent_color;
  border-color: alpha(@accent_bg_color, 0.36);
}

.status-pill.paused {
  background-color: alpha(@accent_bg_color, 0.12);
  color: @accent_color;
  border-color: alpha(@accent_bg_color, 0.30);
}

.record-fab {
  min-width: 68px;
  min-height: 68px;
  border-radius: 999px;
  background-color: @destructive_bg_color;
  color: @destructive_fg_color;
  border: 0;
  -gtk-icon-size: 28px;
}

.record-fab:hover {
  background-color: shade(@destructive_bg_color, 1.08);
}

.control-pill {
  min-width: 150px;
  min-height: 52px;
  border-radius: 999px;
  background-color: @card_bg_color;
  color: @window_fg_color;
  border: 1px solid @borders;
  padding: 0 18px;
  -gtk-icon-size: 24px;
}

.control-pill:hover {
  background-color: alpha(@accent_bg_color, 0.10);
}

.control-pill.resume-pill {
  background-color: alpha(@accent_bg_color, 0.12);
  color: @accent_color;
}

.record-dot {
  min-width: 18px;
  min-height: 18px;
  border-radius: 999px;
  background-color: alpha(black, 0.55);
}

.stop-pill {
  min-width: 150px;
  min-height: 52px;
  border-radius: 999px;
  background-color: @destructive_bg_color;
  color: @destructive_fg_color;
  border: 0;
  padding: 0 18px;
  -gtk-icon-size: 20px;
}

.stop-pill:hover {
  background-color: shade(@destructive_bg_color, 1.08);
}

.control-label {
  font-size: 15px;
  font-weight: 650;
}

.waveform {
  background-color: @view_bg_color;
  border-radius: 8px;
}

.section-title {
  font-weight: 700;
  color: @window_fg_color;
  margin-top: 2px;
}

.tools-footer {
  background-color: @window_bg_color;
  padding: 6px 14px 10px;
}

.tools-panel {
  min-width: 424px;
  background-color: @card_bg_color;
  border: 1px solid @borders;
  border-radius: 8px;
  padding: 10px;
}

.tools-toggle {
  min-width: 126px;
  min-height: 36px;
  border-radius: 999px;
  background-color: @card_bg_color;
  color: @window_fg_color;
  border: 1px solid @borders;
  padding: 0 14px;
}

.tools-toggle:checked {
  background-color: alpha(@accent_bg_color, 0.14);
  color: @accent_color;
  border-color: alpha(@accent_bg_color, 0.42);
}

.muted {
  color: alpha(@window_fg_color, 0.62);
  font-size: 12px;
}

.empty-list {
  color: alpha(@window_fg_color, 0.62);
  padding: 18px;
}

.recording-row {
  background-color: @card_bg_color;
  color: @window_fg_color;
  border: 1px solid @borders;
  border-radius: 8px;
  padding: 12px 14px;
}

.recording-row.selected {
  background-color: alpha(@accent_bg_color, 0.12);
  box-shadow: inset 0 0 0 1px alpha(@accent_bg_color, 0.48);
}

.recording-title {
  color: @window_fg_color;
  font-size: 15px;
  font-weight: 650;
}

.recording-subtitle,
.recording-duration {
  color: alpha(@window_fg_color, 0.62);
}

.recording-duration {
  font-feature-settings: "tnum";
}

.row-play-button,
.row-delete-button {
  min-width: 44px;
  min-height: 52px;
  padding: 0;
  border-radius: 8px;
  background-color: transparent;
  border: 0;
}

.row-play-button {
  color: @accent_color;
}

.row-delete-button {
  color: @destructive_color;
}

.row-play-button:hover {
  background-color: alpha(@accent_bg_color, 0.12);
}

.row-delete-button:hover {
  background-color: alpha(@destructive_bg_color, 0.12);
}

.playback-progress {
  min-height: 12px;
}

.tool-grid button {
  min-width: 126px;
  min-height: 34px;
  border-radius: 8px;
  background-color: @card_bg_color;
  color: @window_fg_color;
  border: 1px solid @borders;
  padding: 0 8px;
}

.tool-grid button:hover {
  background-color: alpha(@accent_bg_color, 0.12);
}

.tool-label {
  font-size: 12px;
  font-weight: 650;
}


.message-content {
  min-width: 240px;
  margin-top: 4px;
}

.message-icon {
  margin-bottom: 2px;
}

.message-icon.info {
  color: @accent_color;
}

.message-icon.success {
  color: @success_color;
}

.message-icon.error {
  color: @error_color;
}

.message-body {
  color: @window_fg_color;
}

.tool-dialog {
  margin-top: 12px;
  color: @window_fg_color;
}

.tool-dialog spinbutton,
.tool-dialog combobox {
  min-width: 150px;
}

.tool-illustration {
  min-width: 260px;
  min-height: 96px;
  border: 1px solid @borders;
  border-radius: 8px;
  background-color: @view_bg_color;
}

.horizontal-value-control {
  min-width: 280px;
}

.horizontal-value-label {
  color: @window_fg_color;
  font-weight: 700;
  font-feature-settings: "tnum";
}

.horizontal-tool-scale {
  min-width: 220px;
}

.horizontal-tool-scale trough {
  min-height: 6px;
  border-radius: 999px;
  background-color: alpha(@window_fg_color, 0.18);
}

.horizontal-tool-scale highlight {
  border-radius: 999px;
  background-color: @accent_bg_color;
}

.horizontal-tool-scale slider {
  min-width: 20px;
  min-height: 20px;
  border-radius: 999px;
  background-color: @window_bg_color;
  border: 1px solid alpha(@window_fg_color, 0.32);
}

.equalizer-curve {
  min-width: 360px;
  min-height: 96px;
  border: 1px solid @borders;
  border-radius: 8px;
  background-color: @view_bg_color;
}

.equalizer-band-box {
  padding-top: 2px;
}

.equalizer-band-scale trough {
  min-width: 6px;
  border-radius: 999px;
  background-color: alpha(@window_fg_color, 0.18);
}

.equalizer-band-scale highlight {
  border-radius: 999px;
  background-color: @accent_bg_color;
}

.equalizer-band-scale slider {
  min-width: 18px;
  min-height: 18px;
  border-radius: 999px;
  background-color: @window_bg_color;
  border: 1px solid alpha(@window_fg_color, 0.32);
}

.equalizer-value-label,
.equalizer-band-label {
  color: alpha(@window_fg_color, 0.72);
  font-size: 10px;
  font-feature-settings: "tnum";
}
"#;

pub fn load() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(STYLE);

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_search_path(local_icon_dir());
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn local_icon_dir() -> std::path::PathBuf {
    let local_dir = std::path::PathBuf::from("usr/share/icons");
    if local_dir.exists() {
        return local_dir;
    }

    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        for directory in parent.ancestors() {
            let candidate = directory.join("usr/share/icons");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    std::path::PathBuf::from("/usr/share/icons")
}
