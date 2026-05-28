use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use anyhow::Context;
use gtk::gio;
use gtk::glib;

use crate::audio::player::{PlaybackStatus, Player};
use crate::audio::processing::{
    self, EQUALIZER_BANDS, ExportChannelMode, ExportFormat, ExportOptions, ExportQuality,
    ExportSampleRate, VoiceEffectPreset,
};
use crate::audio::recorder::{Recorder, saved_recordings};
use crate::i18n::{format_message, gettext};
use crate::ui::playback_bar::PlaybackBar;
use crate::ui::waveform::Waveform;

#[derive(Clone, Copy, Eq, PartialEq)]
enum RecordingMode {
    Idle,
    Recording,
    Paused,
}

struct Session {
    mode: RecordingMode,
    started_at: Option<Instant>,
    accumulated: Duration,
    current_path: Option<PathBuf>,
}

#[derive(Clone)]
struct RecorderWidgets {
    window: adw::ApplicationWindow,
    page_stack: gtk::Stack,
    start_button: gtk::Button,
    record_button: gtk::Button,
    record_symbol: gtk::Stack,
    record_label: gtk::Label,
    stop_button: gtk::Button,
    status_label: gtk::Label,
    timer_label: gtk::Label,
    path_label: gtk::Label,
    waveform: Waveform,
}

#[derive(Clone)]
struct RecordingRow {
    row_widget: gtk::Box,
    path: PathBuf,
    playback_symbol: gtk::Stack,
    playback_bar: PlaybackBar,
}

#[derive(Clone)]
struct ModeControls {
    page_stack: gtk::Stack,
    start_button: gtk::Button,
    record_button: gtk::Button,
    record_symbol: gtk::Stack,
    record_label: gtk::Label,
    stop_button: gtk::Button,
    status_label: gtk::Label,
}

#[derive(Clone, Copy)]
enum ToolAction {
    Trim,
    Volume,
    Speed,
    Pitch,
    VoiceEffects,
    Equalizer,
    Reverse,
    Merge,
    Export,
}

#[derive(Clone, Copy)]
enum ToolIllustration {
    Trim,
    Volume,
    Speed,
    Pitch,
    VoiceEffects,
    Reverse,
    Merge,
    Export,
}

#[derive(Clone)]
struct ToolContext {
    window: adw::ApplicationWindow,
    recordings_box: gtk::Box,
    empty_recordings_label: gtk::Label,
    recording_rows: RecordingRows,
    selected_recording: SelectedRecording,
    player: Rc<RefCell<Player>>,
}

type RecordingRows = Rc<RefCell<Vec<RecordingRow>>>;
type SelectedRecording = Rc<RefCell<Option<PathBuf>>>;

impl Session {
    fn new() -> Self {
        Self {
            mode: RecordingMode::Idle,
            started_at: None,
            accumulated: Duration::ZERO,
            current_path: None,
        }
    }

    fn elapsed(&self) -> Duration {
        match self.started_at {
            Some(started_at) => self.accumulated + started_at.elapsed(),
            None => self.accumulated,
        }
    }

    fn start(&mut self, path: PathBuf) {
        self.mode = RecordingMode::Recording;
        self.started_at = Some(Instant::now());
        self.accumulated = Duration::ZERO;
        self.current_path = Some(path);
    }

    fn pause(&mut self) {
        self.accumulated = self.elapsed();
        self.started_at = None;
        self.mode = RecordingMode::Paused;
    }

    fn resume(&mut self) {
        self.started_at = Some(Instant::now());
        self.mode = RecordingMode::Recording;
    }

    fn stop(&mut self) {
        self.accumulated = self.elapsed();
        self.started_at = None;
        self.mode = RecordingMode::Idle;
    }
}

pub fn build(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Big Recorder")
        .default_width(560)
        .default_height(700)
        .resizable(false)
        .build();

    let recorder = Rc::new(RefCell::new(Recorder::new()));
    let player = Rc::new(RefCell::new(Player::new()));
    let session = Rc::new(RefCell::new(Session::new()));
    let recording_rows: RecordingRows = Rc::new(RefCell::new(Vec::new()));
    let selected_recording: SelectedRecording = Rc::new(RefCell::new(None));

    let header = adw::HeaderBar::new();
    header.add_css_class("flat-header");
    let subtitle = gettext("Local voice recorder");
    header.set_title_widget(Some(&adw::WindowTitle::new("Big Recorder", &subtitle)));

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(gettext("Menu"))
        .build();
    let menu = gtk::gio::Menu::new();
    menu.append(Some(&gettext("About")), Some("win.about"));
    menu.append(Some(&gettext("Close")), Some("win.close"));
    menu_button.set_menu_model(Some(&menu));
    header.pack_end(&menu_button);
    install_window_actions(&window);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_css_class("flat-toolbar");
    toolbar.add_top_bar(&header);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("app-bg");

    let page_stack = gtk::Stack::new();
    page_stack.set_vexpand(true);
    page_stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);

    let recordings_page = gtk::Overlay::new();
    recordings_page.set_vexpand(true);

    let recordings_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let recordings_clamp = adw::Clamp::builder()
        .maximum_size(560)
        .tightening_threshold(560)
        .build();
    recordings_clamp.set_margin_top(14);
    recordings_clamp.set_margin_bottom(92);
    recordings_clamp.set_margin_start(14);
    recordings_clamp.set_margin_end(14);

    let recordings_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    recordings_content.set_hexpand(true);

    let recordings_title = gtk::Label::new(Some(&gettext("Recordings")));
    recordings_title.add_css_class("section-title");
    recordings_title.set_halign(gtk::Align::Start);

    let recordings_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    recordings_box.set_hexpand(true);

    let empty_recordings_label = gtk::Label::new(Some(&gettext("No recordings yet")));
    empty_recordings_label.add_css_class("empty-list");
    empty_recordings_label.set_halign(gtk::Align::Center);
    recordings_box.append(&empty_recordings_label);

    recordings_content.append(&recordings_title);
    recordings_content.append(&recordings_box);
    recordings_clamp.set_child(Some(&recordings_content));
    recordings_scroll.set_child(Some(&recordings_clamp));
    recordings_page.set_child(Some(&recordings_scroll));

    let start_dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    start_dot.add_css_class("record-dot");
    start_dot.set_halign(gtk::Align::Center);
    start_dot.set_valign(gtk::Align::Center);

    let start_button = gtk::Button::builder()
        .tooltip_text(gettext("Record"))
        .halign(gtk::Align::Center)
        .valign(gtk::Align::End)
        .margin_bottom(18)
        .build();
    start_button.set_child(Some(&start_dot));
    start_button.add_css_class("record-fab");
    recordings_page.add_overlay(&start_button);

    let recording_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    recording_page.set_vexpand(true);

    let recording_clamp = adw::Clamp::builder()
        .maximum_size(560)
        .tightening_threshold(560)
        .build();
    recording_clamp.set_margin_top(14);
    recording_clamp.set_margin_bottom(14);
    recording_clamp.set_margin_start(14);
    recording_clamp.set_margin_end(14);

    let recording_content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    recording_content.set_hexpand(true);

    let recorder_surface = gtk::Box::new(gtk::Orientation::Vertical, 8);
    recorder_surface.add_css_class("recorder-surface");

    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    status_row.set_halign(gtk::Align::Center);

    let status_label = gtk::Label::new(Some(&gettext("Ready")));
    status_label.add_css_class("status-pill");
    status_row.append(&status_label);

    let timer_label = gtk::Label::new(Some("00:00:00"));
    timer_label.add_css_class("timer");
    timer_label.set_halign(gtk::Align::Center);

    let waveform = Waveform::new();

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    controls.set_halign(gtk::Align::Center);

    let record_symbol = gtk::Stack::new();
    record_symbol.set_halign(gtk::Align::Center);
    record_symbol.set_valign(gtk::Align::Center);
    record_symbol.set_size_request(24, 24);

    let record_dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    record_dot.add_css_class("record-dot");
    record_dot.set_halign(gtk::Align::Center);
    record_dot.set_valign(gtk::Align::Center);

    let pause_icon = gtk::Image::from_icon_name("media-playback-pause-symbolic");
    pause_icon.set_pixel_size(24);
    pause_icon.set_halign(gtk::Align::Center);
    pause_icon.set_valign(gtk::Align::Center);

    let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
    play_icon.set_pixel_size(26);
    play_icon.set_halign(gtk::Align::Center);
    play_icon.set_valign(gtk::Align::Center);

    record_symbol.add_named(&record_dot, Some("record"));
    record_symbol.add_named(&pause_icon, Some("pause"));
    record_symbol.add_named(&play_icon, Some("play"));
    record_symbol.set_visible_child_name("record");

    let record_label = gtk::Label::new(Some(&gettext("Record")));
    record_label.add_css_class("control-label");
    record_label.set_visible(false);

    let record_content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    record_content.set_halign(gtk::Align::Center);
    record_content.set_valign(gtk::Align::Center);
    record_content.append(&record_symbol);
    record_content.append(&record_label);

    let record_button = gtk::Button::builder()
        .tooltip_text(gettext("Record"))
        .build();
    record_button.set_child(Some(&record_content));
    record_button.add_css_class("record-fab");

    let stop_icon = gtk::Image::from_icon_name("media-playback-stop-symbolic");
    stop_icon.set_pixel_size(18);
    stop_icon.set_halign(gtk::Align::Center);
    stop_icon.set_valign(gtk::Align::Center);

    let stop_label = gtk::Label::new(Some(&gettext("Stop")));
    stop_label.add_css_class("control-label");

    let stop_content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    stop_content.set_halign(gtk::Align::Center);
    stop_content.set_valign(gtk::Align::Center);
    stop_content.append(&stop_icon);
    stop_content.append(&stop_label);

    let stop_button = gtk::Button::builder()
        .tooltip_text(gettext("Stop"))
        .sensitive(false)
        .visible(false)
        .build();
    stop_button.set_child(Some(&stop_content));
    stop_button.add_css_class("stop-pill");

    controls.append(&record_button);
    controls.append(&stop_button);

    let path_label = gtk::Label::new(Some(&gettext("Recordings are saved in Music/BigRecorder")));
    path_label.add_css_class("muted");
    path_label.set_wrap(true);
    path_label.set_justify(gtk::Justification::Center);
    path_label.set_halign(gtk::Align::Center);

    recorder_surface.append(&status_row);
    recorder_surface.append(waveform.widget());
    recorder_surface.append(&timer_label);
    recorder_surface.append(&controls);
    recorder_surface.append(&path_label);

    recording_content.append(&recorder_surface);
    recording_clamp.set_child(Some(&recording_content));
    recording_page.append(&recording_clamp);

    page_stack.add_named(&recordings_page, Some("recordings"));
    page_stack.add_named(&recording_page, Some("recording"));
    page_stack.set_visible_child_name("recordings");

    let tools_footer = gtk::Box::new(gtk::Orientation::Vertical, 6);
    tools_footer.add_css_class("tools-footer");

    let tools_revealer = gtk::Revealer::new();
    tools_revealer.set_transition_type(gtk::RevealerTransitionType::SlideUp);
    tools_revealer.set_transition_duration(180);
    tools_revealer.set_reveal_child(false);

    let tools_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tools_panel.add_css_class("tools-panel");
    tools_panel.set_halign(gtk::Align::Center);

    let tools_title = gtk::Label::new(Some(&gettext("Tools")));
    tools_title.add_css_class("section-title");
    tools_title.set_halign(gtk::Align::Start);

    let tools_grid = gtk::Grid::builder()
        .column_spacing(8)
        .row_spacing(8)
        .hexpand(false)
        .halign(gtk::Align::Center)
        .column_homogeneous(true)
        .build();
    tools_grid.add_css_class("tool-grid");

    let tools = [
        ("edit-cut-symbolic", gettext("Trim"), ToolAction::Trim),
        (
            "audio-volume-high-symbolic",
            gettext("Volume"),
            ToolAction::Volume,
        ),
        (
            "media-seek-forward-symbolic",
            gettext("Speed"),
            ToolAction::Speed,
        ),
        (
            "audio-x-generic-symbolic",
            gettext("Pitch"),
            ToolAction::Pitch,
        ),
        (
            "applications-multimedia-symbolic",
            gettext("Effects"),
            ToolAction::VoiceEffects,
        ),
        (
            "view-more-symbolic",
            gettext("Equalizer"),
            ToolAction::Equalizer,
        ),
        (
            "object-flip-horizontal-symbolic",
            gettext("Reverse"),
            ToolAction::Reverse,
        ),
        ("list-add-symbolic", gettext("Merge"), ToolAction::Merge),
        (
            "document-save-symbolic",
            gettext("Export"),
            ToolAction::Export,
        ),
    ];

    let tool_context = ToolContext {
        window: window.clone(),
        recordings_box: recordings_box.clone(),
        empty_recordings_label: empty_recordings_label.clone(),
        recording_rows: Rc::clone(&recording_rows),
        selected_recording: Rc::clone(&selected_recording),
        player: Rc::clone(&player),
    };

    let tools_toggle = gtk::ToggleButton::builder()
        .tooltip_text(gettext("Tools"))
        .halign(gtk::Align::Center)
        .build();
    tools_toggle.add_css_class("tools-toggle");
    tools_toggle.set_child(Some(&toggle_button_content(
        "view-grid-symbolic",
        &gettext("Tools"),
    )));

    let tools_revealer_clone = tools_revealer.clone();
    tools_toggle.connect_toggled(move |button| {
        tools_revealer_clone.set_reveal_child(button.is_active());
    });

    for (index, (icon, label, action)) in tools.iter().enumerate() {
        let button = compact_tool_button(icon, label);
        button.set_tooltip_text(Some(&tool_tooltip(*action)));
        let tool_context = tool_context.clone();
        let tools_toggle_clone = tools_toggle.clone();
        let action = *action;
        button.connect_clicked(move |_| {
            run_tool(action, &tool_context);
            tools_toggle_clone.set_active(false);
        });
        tools_grid.attach(&button, (index % 3) as i32, (index / 3) as i32, 1, 1);
    }

    tools_panel.append(&tools_title);
    tools_panel.append(&tools_grid);
    tools_revealer.set_child(Some(&tools_panel));
    tools_footer.append(&tools_revealer);
    tools_footer.append(&tools_toggle);

    root.append(&page_stack);
    root.append(&tools_footer);
    toolbar.set_content(Some(&root));
    window.set_content(Some(&toolbar));

    load_saved_recordings(&tool_context);

    connect_controls(
        RecorderWidgets {
            window: window.clone(),
            page_stack,
            start_button,
            record_button,
            record_symbol,
            record_label,
            stop_button,
            status_label,
            timer_label,
            path_label,
            waveform,
        },
        recorder,
        player,
        session,
        recording_rows,
        tool_context.clone(),
    );

    window.present();
}

fn install_window_actions(window: &adw::ApplicationWindow) {
    let about_action = gtk::gio::SimpleAction::new("about", None);
    let window_clone = window.clone();
    about_action.connect_activate(move |_, _| {
        let dialog = adw::AboutDialog::builder()
            .application_name("Big Recorder")
            .application_icon(crate::app::APP_ID)
            .developer_name("BigLinux Team")
            .version(crate::app::APP_VERSION)
            .comments(gettext(
                "Local voice recorder built with Rust, GTK4, libadwaita, and GStreamer.",
            ))
            .license_type(gtk::License::MitX11)
            .build();
        dialog.present(Some(&window_clone));
    });
    window.add_action(&about_action);

    let close_action = gtk::gio::SimpleAction::new("close", None);
    let window_clone = window.clone();
    close_action.connect_activate(move |_, _| {
        window_clone.close();
    });
    window.add_action(&close_action);
}

fn connect_controls(
    widgets: RecorderWidgets,
    recorder: Rc<RefCell<Recorder>>,
    player: Rc<RefCell<Player>>,
    session: Rc<RefCell<Session>>,
    recording_rows: RecordingRows,
    tool_context: ToolContext,
) {
    let timer_session = Rc::clone(&session);
    let timer_label_clone = widgets.timer_label.clone();
    glib::timeout_add_seconds_local(1, move || {
        timer_label_clone.set_label(&format_duration(timer_session.borrow().elapsed()));
        glib::ControlFlow::Continue
    });

    let monitor_recorder = Rc::clone(&recorder);
    let monitor_session = Rc::clone(&session);
    let monitor_waveform = widgets.waveform.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        let mode = monitor_session.borrow().mode;
        monitor_waveform.set_active(mode == RecordingMode::Recording);

        if mode == RecordingMode::Recording {
            match monitor_recorder.borrow_mut().poll_frame() {
                Ok(Some(frame)) => {
                    monitor_waveform.set_level(frame.level);
                    monitor_waveform.set_peaks(frame.peaks);
                }
                Ok(None) => {}
                Err(err) => eprintln!("Failed to read audio level: {err}"),
            }
        }

        glib::ControlFlow::Continue
    });

    let playback_player = Rc::clone(&player);
    let playback_rows = Rc::clone(&recording_rows);
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let poll_result = {
            let mut player = playback_player.borrow_mut();
            player.poll_finished()
        };

        match poll_result {
            Ok(true) => refresh_playback_rows(&playback_rows, None),
            Ok(false) => {
                let status = playback_player.borrow().status();
                refresh_playback_rows(&playback_rows, status.as_ref());
            }
            Err(err) => {
                eprintln!("Failed to play recording: {err}");
                refresh_playback_rows(&playback_rows, None);
            }
        }

        glib::ControlFlow::Continue
    });

    let mode_controls = ModeControls {
        page_stack: widgets.page_stack.clone(),
        start_button: widgets.start_button.clone(),
        record_button: widgets.record_button.clone(),
        record_symbol: widgets.record_symbol.clone(),
        record_label: widgets.record_label.clone(),
        stop_button: widgets.stop_button.clone(),
        status_label: widgets.status_label.clone(),
    };

    let window_clone = widgets.window.clone();
    let mode_controls_clone = mode_controls.clone();
    let timer_label_clone = widgets.timer_label.clone();
    let path_label_clone = widgets.path_label.clone();
    let recorder_clone = Rc::clone(&recorder);
    let player_clone = Rc::clone(&player);
    let session_clone = Rc::clone(&session);
    let recording_rows_clone = Rc::clone(&recording_rows);

    widgets.start_button.connect_clicked(move |_| {
        if session_clone.borrow().mode != RecordingMode::Idle {
            return;
        }

        if let Err(err) = player_clone.borrow_mut().stop() {
            show_error(&window_clone, &gettext("Could not stop playback"), &err);
            return;
        }
        refresh_playback_rows(&recording_rows_clone, None);

        match recorder_clone.borrow_mut().start() {
            Ok(path) => {
                session_clone.borrow_mut().start(path.clone());
                let path_text = path.display().to_string();
                path_label_clone.set_label(&format_message(
                    &gettext("Recording to {path}"),
                    &[("{path}", &path_text)],
                ));
                apply_mode(RecordingMode::Recording, &mode_controls_clone);
            }
            Err(err) => show_error(&window_clone, &gettext("Could not start recording"), &err),
        }

        timer_label_clone.set_label(&format_duration(session_clone.borrow().elapsed()));
    });

    let window_clone = widgets.window.clone();
    let mode_controls_clone = mode_controls.clone();
    let timer_label_clone = widgets.timer_label.clone();
    let recorder_clone = Rc::clone(&recorder);
    let session_clone = Rc::clone(&session);

    widgets.record_button.connect_clicked(move |_| {
        let mode = session_clone.borrow().mode;

        match mode {
            RecordingMode::Recording => match recorder_clone.borrow().pause() {
                Ok(()) => {
                    session_clone.borrow_mut().pause();
                    apply_mode(RecordingMode::Paused, &mode_controls_clone);
                }
                Err(err) => show_error(&window_clone, &gettext("Could not pause"), &err),
            },
            RecordingMode::Paused => match recorder_clone.borrow().resume() {
                Ok(()) => {
                    session_clone.borrow_mut().resume();
                    apply_mode(RecordingMode::Recording, &mode_controls_clone);
                }
                Err(err) => show_error(&window_clone, &gettext("Could not resume"), &err),
            },
            RecordingMode::Idle => {}
        }

        timer_label_clone.set_label(&format_duration(session_clone.borrow().elapsed()));
    });

    let window_clone = widgets.window.clone();
    let mode_controls_clone = mode_controls.clone();
    let timer_label_clone = widgets.timer_label.clone();
    let path_label_clone = widgets.path_label.clone();
    let recorder_clone = Rc::clone(&recorder);
    let session_clone = Rc::clone(&session);
    let tool_context_clone = tool_context.clone();

    widgets.stop_button.connect_clicked(move |_| {
        let final_duration = session_clone.borrow().elapsed();

        match recorder_clone.borrow_mut().stop() {
            Ok(Some(path)) => {
                session_clone.borrow_mut().stop();
                timer_label_clone.set_label(&format_duration(final_duration));
                let path_text = path.display().to_string();
                path_label_clone.set_label(&format_message(
                    &gettext("Saved to {path}"),
                    &[("{path}", &path_text)],
                ));
                apply_mode(RecordingMode::Idle, &mode_controls_clone);
                append_recording_row(&tool_context_clone, path, Some(final_duration), true);
            }
            Ok(None) => {}
            Err(err) => {
                session_clone.borrow_mut().stop();
                timer_label_clone.set_label(&format_duration(final_duration));
                apply_mode(RecordingMode::Idle, &mode_controls_clone);
                show_error(&window_clone, &gettext("Could not finish recording"), &err);
            }
        }
    });
}

fn apply_mode(mode: RecordingMode, controls: &ModeControls) {
    let page_stack = &controls.page_stack;
    let start_button = &controls.start_button;
    let record_button = &controls.record_button;
    let record_symbol = &controls.record_symbol;
    let record_label = &controls.record_label;
    let stop_button = &controls.stop_button;
    let status_label = &controls.status_label;

    status_label.remove_css_class("recording");
    status_label.remove_css_class("paused");
    record_button.remove_css_class("record-fab");
    record_button.remove_css_class("control-pill");
    record_button.remove_css_class("resume-pill");

    match mode {
        RecordingMode::Idle => {
            page_stack.set_visible_child_name("recordings");
            start_button.set_sensitive(true);
            status_label.set_label(&gettext("Ready"));
            record_symbol.set_visible_child_name("record");
            record_label.set_visible(false);
            record_button.set_tooltip_text(Some(&gettext("Record")));
            record_button.add_css_class("record-fab");
            stop_button.set_sensitive(false);
            stop_button.set_visible(false);
        }
        RecordingMode::Recording => {
            page_stack.set_visible_child_name("recording");
            start_button.set_sensitive(false);
            status_label.set_label(&gettext("Recording"));
            status_label.add_css_class("recording");
            record_symbol.set_visible_child_name("pause");
            record_label.set_label(&gettext("Pause"));
            record_label.set_visible(true);
            record_button.set_tooltip_text(Some(&gettext("Pause")));
            record_button.add_css_class("control-pill");
            stop_button.set_sensitive(true);
            stop_button.set_visible(true);
        }
        RecordingMode::Paused => {
            page_stack.set_visible_child_name("recording");
            start_button.set_sensitive(false);
            status_label.set_label(&gettext("Paused"));
            status_label.add_css_class("paused");
            record_symbol.set_visible_child_name("play");
            record_label.set_label(&gettext("Resume"));
            record_label.set_visible(true);
            record_button.set_tooltip_text(Some(&gettext("Resume")));
            record_button.add_css_class("control-pill");
            record_button.add_css_class("resume-pill");
            stop_button.set_sensitive(true);
            stop_button.set_visible(true);
        }
    }
}

fn load_saved_recordings(context: &ToolContext) {
    match saved_recordings() {
        Ok(paths) => {
            for path in paths.into_iter().rev() {
                append_recording_row(context, path, None, false);
            }
            if let Some(path) = context
                .recording_rows
                .borrow()
                .last()
                .map(|row| row.path.clone())
            {
                select_recording(&context.recording_rows, &context.selected_recording, &path);
            }
        }
        Err(err) => eprintln!("Failed to load saved recordings: {err}"),
    }
}

fn append_recording_row(
    context: &ToolContext,
    path: PathBuf,
    duration: Option<Duration>,
    auto_select: bool,
) {
    context.empty_recordings_label.set_visible(false);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("recording-row");
    row.set_hexpand(true);

    let row_body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    row_body.set_hexpand(true);

    let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    row_content.set_hexpand(true);

    let text_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text_box.set_hexpand(true);

    let title = gtk::Label::new(Some(&recording_title(&path)));
    title.add_css_class("recording-title");
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let subtitle = gtk::Label::new(Some(&recording_subtitle(duration)));
    subtitle.add_css_class("recording-subtitle");
    subtitle.set_xalign(0.0);

    text_box.append(&title);
    text_box.append(&subtitle);

    let duration_label = gtk::Label::new(duration.map(format_compact_duration).as_deref());
    duration_label.add_css_class("recording-duration");
    duration_label.set_valign(gtk::Align::Start);

    let playback_symbol = gtk::Stack::new();
    playback_symbol.set_size_request(24, 24);
    playback_symbol.set_halign(gtk::Align::Center);
    playback_symbol.set_valign(gtk::Align::Center);
    playback_symbol.add_named(&row_playback_symbol(false), Some("play"));
    playback_symbol.add_named(&row_playback_symbol(true), Some("pause"));
    playback_symbol.set_visible_child_name("play");

    let play_button = gtk::Button::builder()
        .tooltip_text(gettext("Play"))
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    play_button.add_css_class("row-play-button");
    play_button.set_child(Some(&playback_symbol));

    let delete_icon = gtk::Image::from_icon_name("user-trash-symbolic");
    delete_icon.set_pixel_size(18);
    delete_icon.set_halign(gtk::Align::Center);
    delete_icon.set_valign(gtk::Align::Center);

    let delete_button = gtk::Button::builder()
        .tooltip_text(gettext("Move to Trash"))
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    delete_button.add_css_class("row-delete-button");
    delete_button.set_child(Some(&delete_icon));

    let playback_bar = PlaybackBar::new(&path);

    row_content.append(&text_box);
    row_content.append(&duration_label);
    row_body.append(&row_content);
    row_body.append(playback_bar.widget());
    row.append(&row_body);
    row.append(&delete_button);
    row.append(&play_button);
    context.recordings_box.prepend(&row);

    context.recording_rows.borrow_mut().push(RecordingRow {
        row_widget: row.clone(),
        path: path.clone(),
        playback_symbol: playback_symbol.clone(),
        playback_bar: playback_bar.clone(),
    });

    let select_rows = Rc::clone(&context.recording_rows);
    let select_recording_ref = Rc::clone(&context.selected_recording);
    let select_path = path.clone();
    let gesture = gtk::GestureClick::new();
    gesture.connect_pressed(move |_, _, _, _| {
        select_recording(&select_rows, &select_recording_ref, &select_path);
    });
    row.add_controller(gesture);

    let delete_context = context.clone();
    let delete_path = path.clone();
    delete_button.connect_clicked(move |_| {
        select_recording(
            &delete_context.recording_rows,
            &delete_context.selected_recording,
            &delete_path,
        );
        confirm_trash_recording(&delete_context, delete_path.clone());
    });

    let player_clone = Rc::clone(&context.player);
    let recording_rows_clone = Rc::clone(&context.recording_rows);
    let selected_recording_clone = Rc::clone(&context.selected_recording);
    let window_clone = context.window.clone();
    let play_path = path.clone();
    play_button.connect_clicked(move |_| {
        select_recording(&recording_rows_clone, &selected_recording_clone, &play_path);
        let result = {
            let mut player = player_clone.borrow_mut();
            player.toggle(&play_path)
        };

        match result {
            Ok(()) => {
                let status = player_clone.borrow().status();
                refresh_playback_rows(&recording_rows_clone, status.as_ref());
            }
            Err(err) => show_error(&window_clone, &gettext("Could not play recording"), &err),
        }
    });

    if auto_select {
        select_recording(&context.recording_rows, &context.selected_recording, &path);
    }
}

fn confirm_trash_recording(context: &ToolContext, path: PathBuf) {
    let dialog = adw::AlertDialog::new(
        Some(&gettext("Move recording to trash?")),
        Some(&format_message(
            &gettext("{recording} will be moved to the system trash."),
            &[("{recording}", &recording_title(&path))],
        )),
    );
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("trash", &gettext("Move to Trash"));
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("trash", adw::ResponseAppearance::Destructive);

    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "trash" {
            trash_recording(&action_context, &path);
        }
    });
    dialog.present(Some(&context.window));
}

fn trash_recording(context: &ToolContext, path: &Path) {
    if let Err(err) = context.player.borrow_mut().stop() {
        show_error(&context.window, &gettext("Could not stop playback"), &err);
        return;
    }
    refresh_playback_rows(&context.recording_rows, None);

    match move_recording_to_trash(path) {
        Ok(()) => remove_recording_row(context, path),
        Err(err) => show_error(
            &context.window,
            &gettext("Could not move recording to trash"),
            &err,
        ),
    }
}

fn move_recording_to_trash(path: &Path) -> anyhow::Result<()> {
    let file = gio::File::for_path(path);
    file.trash(gio::Cancellable::NONE)
        .with_context(|| format!("Failed to move recording to trash: {}", path.display()))
}

fn remove_recording_row(context: &ToolContext, path: &Path) {
    let deleted_selected = context
        .selected_recording
        .borrow()
        .as_ref()
        .is_some_and(|selected_path| selected_path == path);

    let Some((row_widget, replacement_path, list_is_empty)) = ({
        let mut rows = context.recording_rows.borrow_mut();
        if let Some(index) = rows.iter().position(|row| row.path == path) {
            let removed_row = rows.remove(index);
            let replacement_path = if deleted_selected {
                rows.get(index)
                    .or_else(|| rows.last())
                    .map(|row| row.path.clone())
            } else {
                None
            };

            Some((removed_row.row_widget, replacement_path, rows.is_empty()))
        } else {
            None
        }
    }) else {
        return;
    };

    context.recordings_box.remove(&row_widget);

    if list_is_empty {
        context.empty_recordings_label.set_visible(true);
        context.selected_recording.replace(None);
    } else if let Some(path) = replacement_path {
        select_recording(&context.recording_rows, &context.selected_recording, &path);
    }
}

fn select_recording(
    recording_rows: &RecordingRows,
    selected_recording: &SelectedRecording,
    path: &Path,
) {
    selected_recording.replace(Some(path.to_path_buf()));

    for row in recording_rows.borrow().iter() {
        if row.path == path {
            row.row_widget.add_css_class("selected");
        } else {
            row.row_widget.remove_css_class("selected");
        }
    }
}

fn selected_recording_path(context: &ToolContext) -> Option<PathBuf> {
    if let Some(path) = context.selected_recording.borrow().clone()
        && context
            .recording_rows
            .borrow()
            .iter()
            .any(|row| row.path == path)
    {
        return Some(path);
    }

    context
        .recording_rows
        .borrow()
        .last()
        .map(|row| row.path.clone())
}

fn run_tool(action: ToolAction, context: &ToolContext) {
    match action {
        ToolAction::Trim => show_trim_dialog(context),
        ToolAction::Volume => show_volume_dialog(context),
        ToolAction::Speed => show_speed_dialog(context),
        ToolAction::Pitch => show_pitch_dialog(context),
        ToolAction::VoiceEffects => show_voice_effects_dialog(context),
        ToolAction::Equalizer => show_equalizer_dialog(context),
        ToolAction::Reverse => show_reverse_dialog(context),
        ToolAction::Merge => show_merge_dialog(context),
        ToolAction::Export => export_selected_recording(context),
    }
}

fn selected_or_notice(context: &ToolContext) -> Option<PathBuf> {
    let path = selected_recording_path(context);
    if path.is_none() {
        show_notice(
            &context.window,
            &gettext("Select a recording first"),
            &gettext("Click a recording in the list before using a tool."),
        );
    }

    path
}

fn show_trim_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };
    let duration = match processing::duration(&path) {
        Ok(duration) if duration > Duration::ZERO => duration,
        Ok(_) => {
            show_notice(
                &context.window,
                &gettext("Could not trim recording"),
                &gettext("This recording has no audio data."),
            );
            return;
        }
        Err(err) => {
            show_error(&context.window, &gettext("Could not trim recording"), &err);
            return;
        }
    };

    let max_seconds = duration.as_secs_f64();
    let start_spin = number_spin(0.0, max_seconds, 0.1, 2, 0.0);
    let end_spin = number_spin(0.0, max_seconds, 0.1, 2, max_seconds);
    let content = tool_content_with_illustration(
        ToolIllustration::Trim,
        &[
            (&gettext("Start seconds"), &start_spin),
            (&gettext("End seconds"), &end_spin),
        ],
    );
    let dialog = tool_dialog(
        &gettext("Trim recording"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let task_path = path.clone();
        let start = Duration::from_secs_f64(start_spin.value());
        let end = Duration::from_secs_f64(end_spin.value());
        run_processed_task(
            &action_context,
            gettext("Could not trim recording"),
            move || processing::trim(&task_path, start, end),
        );
    });
    dialog.present(Some(&window));
}

fn show_volume_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };
    let (gain_control, gain_adjustment) = horizontal_value_control(0.0, 300.0, 5.0, 0, 100.0, "%");
    let content = tool_content_with_illustration(
        ToolIllustration::Volume,
        &[(&gettext("Volume percent"), &gain_control)],
    );
    let dialog = tool_dialog(
        &gettext("Change volume"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let task_path = path.clone();
        let gain = (gain_adjustment.value() / 100.0) as f32;
        run_processed_task(
            &action_context,
            gettext("Could not change volume"),
            move || processing::change_volume(&task_path, gain),
        );
    });
    dialog.present(Some(&window));
}

fn show_speed_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };
    let (speed_control, speed_adjustment) = horizontal_value_control(0.25, 4.0, 0.05, 2, 1.25, "x");
    let content = tool_content_with_illustration(
        ToolIllustration::Speed,
        &[(&gettext("Speed factor"), &speed_control)],
    );
    let dialog = tool_dialog(
        &gettext("Change speed"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let task_path = path.clone();
        let speed = speed_adjustment.value() as f32;
        run_processed_task(
            &action_context,
            gettext("Could not change speed"),
            move || processing::change_speed(&task_path, speed),
        );
    });
    dialog.present(Some(&window));
}

fn show_pitch_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };
    let (pitch_control, pitch_adjustment) =
        horizontal_value_control(-12.0, 12.0, 1.0, 0, 2.0, " st");
    let content = tool_content_with_illustration(
        ToolIllustration::Pitch,
        &[(&gettext("Semitones"), &pitch_control)],
    );
    let dialog = tool_dialog(
        &gettext("Change pitch"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let task_path = path.clone();
        let semitones = pitch_adjustment.value() as f32;
        run_processed_task(
            &action_context,
            gettext("Could not change pitch"),
            move || processing::change_pitch(&task_path, semitones),
        );
    });
    dialog.present(Some(&window));
}

fn show_voice_effects_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };

    let effect_combo = gtk::ComboBoxText::new();
    effect_combo.append_text(&gettext("Child voice"));
    effect_combo.append_text(&gettext("Deep voice"));
    effect_combo.append_text(&gettext("Robot voice"));
    effect_combo.append_text(&gettext("Radio voice"));
    effect_combo.append_text(&gettext("Echo voice"));
    effect_combo.set_active(Some(0));

    let (amount_control, amount_adjustment) =
        horizontal_value_control(0.0, 100.0, 5.0, 0, 70.0, "%");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.add_css_class("tool-dialog");
    content.append(&tool_illustration(ToolIllustration::VoiceEffects));

    let fields = gtk::Box::new(gtk::Orientation::Vertical, 10);
    fields.append(&tool_row(&gettext("Effect"), &effect_combo));
    fields.append(&tool_row(&gettext("Intensity"), &amount_control));
    content.append(&fields);

    let dialog = tool_dialog(
        &gettext("Voice effects"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let task_path = path.clone();
        let preset = voice_effect_from_combo(&effect_combo);
        let amount = (amount_adjustment.value() / 100.0) as f32;
        run_processed_task(
            &action_context,
            gettext("Could not apply voice effect"),
            move || processing::apply_voice_effect(&task_path, preset, amount),
        );
    });
    dialog.present(Some(&window));
}

fn show_equalizer_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };

    let adjustments: Vec<gtk::Adjustment> = EQUALIZER_BANDS
        .iter()
        .map(|_| gtk::Adjustment::new(0.0, -12.0, 12.0, 1.0, 3.0, 0.0))
        .collect();
    let content = equalizer_dialog_content(&adjustments);
    let dialog = tool_dialog(
        &gettext("Equalizer"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let task_path = path.clone();
        let gains = std::array::from_fn(|index| adjustments[index].value() as f32);
        run_processed_task(
            &action_context,
            gettext("Could not equalize recording"),
            move || processing::equalize(&task_path, gains),
        );
    });
    dialog.present(Some(&window));
}

fn show_reverse_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };
    let dialog = adw::AlertDialog::new(
        Some(&gettext("Reverse recording")),
        Some(&gettext("Create a new recording with the audio reversed.")),
    );
    let content = tool_illustration_content(ToolIllustration::Reverse);
    dialog.set_extra_child(Some(&content));
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("apply", &gettext("Reverse"));
    dialog.set_default_response(Some("apply"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let result = processing::reverse(&path);
        finish_processed_recording(
            &action_context,
            result,
            &gettext("Could not reverse recording"),
        );
    });
    dialog.present(Some(&window));
}

fn show_merge_dialog(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };
    let options: Vec<PathBuf> = context
        .recording_rows
        .borrow()
        .iter()
        .filter(|row| row.path != path)
        .map(|row| row.path.clone())
        .collect();
    if options.is_empty() {
        show_notice(
            &context.window,
            &gettext("Could not merge recordings"),
            &gettext("There is no second recording to merge."),
        );
        return;
    }

    let combo = gtk::ComboBoxText::new();
    for option in &options {
        combo.append_text(&recording_title(option));
    }
    combo.set_active(Some(0));
    let content = tool_content_with_illustration(
        ToolIllustration::Merge,
        &[(&gettext("Append recording"), &combo)],
    );
    let dialog = tool_dialog(
        &gettext("Merge recordings"),
        Some(&recording_title(&path)),
        &content,
    );

    let window = context.window.clone();
    let action_context = context.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let index = combo.active().unwrap_or(0) as usize;
        let result = options
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("No recording selected"))
            .and_then(|second_path| processing::merge(&path, second_path));
        finish_processed_recording(
            &action_context,
            result,
            &gettext("Could not merge recordings"),
        );
    });
    dialog.present(Some(&window));
}

fn export_selected_recording(context: &ToolContext) {
    let Some(path) = selected_or_notice(context) else {
        return;
    };

    let format_combo = gtk::ComboBoxText::new();
    format_combo.append_text(&gettext("MP3 audio (.mp3)"));
    format_combo.append_text(&gettext("WAV audio (.wav)"));
    format_combo.append_text(&gettext("Opus audio (.opus)"));
    format_combo.append_text(&gettext("FLAC audio (.flac)"));
    format_combo.set_active(Some(0));

    let quality_combo = gtk::ComboBoxText::new();
    quality_combo.append_text(&gettext("Low"));
    quality_combo.append_text(&gettext("Medium"));
    quality_combo.append_text(&gettext("High"));
    quality_combo.set_active(Some(1));

    let channel_combo = gtk::ComboBoxText::new();
    channel_combo.append_text(&gettext("Original"));
    channel_combo.append_text(&gettext("Mono"));
    channel_combo.append_text(&gettext("Stereo"));
    channel_combo.set_active(Some(1));

    let sample_rate_combo = gtk::ComboBoxText::new();
    sample_rate_combo.append_text(&gettext("Original"));
    sample_rate_combo.append_text(&gettext("48 kHz"));
    sample_rate_combo.append_text(&gettext("44.1 kHz"));
    sample_rate_combo.append_text(&gettext("24 kHz"));
    sample_rate_combo.append_text(&gettext("16 kHz"));
    sample_rate_combo.set_active(Some(0));

    let content = tool_content_with_illustration(
        ToolIllustration::Export,
        &[
            (&gettext("Format"), &format_combo),
            (&gettext("Quality"), &quality_combo),
            (&gettext("Channels"), &channel_combo),
            (&gettext("Sample rate"), &sample_rate_combo),
        ],
    );
    let dialog = adw::AlertDialog::new(
        Some(&gettext("Export recording")),
        Some(&recording_title(&path)),
    );
    dialog.set_extra_child(Some(&content));
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("apply", &gettext("Export"));
    dialog.set_default_response(Some("apply"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);

    let window = context.window.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "apply" {
            return;
        }

        let options = ExportOptions {
            format: export_format_from_combo(&format_combo),
            quality: export_quality_from_combo(&quality_combo),
            channel_mode: export_channel_from_combo(&channel_combo),
            sample_rate: export_sample_rate_from_combo(&sample_rate_combo),
        };
        show_export_file_dialog(&window, path.clone(), options);
    });
    dialog.present(Some(&context.window));
}

fn show_export_file_dialog(window: &adw::ApplicationWindow, path: PathBuf, options: ExportOptions) {
    let dialog = gtk::FileChooserNative::new(
        Some(&gettext("Export recording")),
        Some(window),
        gtk::FileChooserAction::Save,
        Some(&gettext("Export")),
        Some(&gettext("Cancel")),
    );
    dialog.set_current_name(&default_export_name(&path, options.format));

    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&export_filter_name(options.format)));
    filter.add_pattern(&format!("*.{}", options.format.extension()));
    dialog.add_filter(&filter);

    let window = window.clone();
    dialog.run_async(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                let Some(destination) = file.path() else {
                    show_notice(
                        &window,
                        &gettext("Could not export recording"),
                        &gettext("The selected destination is not a local file."),
                    );
                    dialog.destroy();
                    return;
                };
                let destination = destination_with_extension(&destination, options.format);

                run_export_task(&window, path.clone(), destination, options);
            } else {
                show_notice(
                    &window,
                    &gettext("Could not export recording"),
                    &gettext("The selected destination is not a local file."),
                );
            }
        }
        dialog.destroy();
    });
}

fn export_format_from_combo(combo: &gtk::ComboBoxText) -> ExportFormat {
    match combo.active().unwrap_or(0) {
        1 => ExportFormat::Wav,
        2 => ExportFormat::OggOpus,
        3 => ExportFormat::Flac,
        _ => ExportFormat::Mp3,
    }
}

fn export_quality_from_combo(combo: &gtk::ComboBoxText) -> ExportQuality {
    match combo.active().unwrap_or(1) {
        0 => ExportQuality::Low,
        2 => ExportQuality::High,
        _ => ExportQuality::Medium,
    }
}

fn export_channel_from_combo(combo: &gtk::ComboBoxText) -> ExportChannelMode {
    match combo.active().unwrap_or(1) {
        0 => ExportChannelMode::Original,
        2 => ExportChannelMode::Stereo,
        _ => ExportChannelMode::Mono,
    }
}

fn export_sample_rate_from_combo(combo: &gtk::ComboBoxText) -> ExportSampleRate {
    match combo.active().unwrap_or(0) {
        1 => ExportSampleRate::Hz(48_000),
        2 => ExportSampleRate::Hz(44_100),
        3 => ExportSampleRate::Hz(24_000),
        4 => ExportSampleRate::Hz(16_000),
        _ => ExportSampleRate::Original,
    }
}

fn default_export_name(path: &Path, format: ExportFormat) -> String {
    format!("{}.{}", recording_title(path), format.extension())
}

fn destination_with_extension(destination: &Path, format: ExportFormat) -> PathBuf {
    if destination
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(format.extension()))
    {
        return destination.to_path_buf();
    }

    destination.with_extension(format.extension())
}

fn export_filter_name(format: ExportFormat) -> String {
    match format {
        ExportFormat::Wav => gettext("WAV audio"),
        ExportFormat::Mp3 => gettext("MP3 audio"),
        ExportFormat::OggOpus => gettext("Opus audio"),
        ExportFormat::Flac => gettext("FLAC audio"),
    }
}

fn voice_effect_from_combo(combo: &gtk::ComboBoxText) -> VoiceEffectPreset {
    match combo.active().unwrap_or(0) {
        1 => VoiceEffectPreset::Deep,
        2 => VoiceEffectPreset::Robot,
        3 => VoiceEffectPreset::Radio,
        4 => VoiceEffectPreset::Echo,
        _ => VoiceEffectPreset::Child,
    }
}

fn equalizer_dialog_content(adjustments: &[gtk::Adjustment]) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.add_css_class("tool-dialog");

    let graph = equalizer_curve(adjustments);
    content.append(&graph);

    let preset_combo = gtk::ComboBoxText::new();
    preset_combo.append_text(&gettext("Flat"));
    preset_combo.append_text(&gettext("Voice clarity"));
    preset_combo.append_text(&gettext("Warm"));
    preset_combo.append_text(&gettext("Bright"));
    preset_combo.append_text(&gettext("Bass cut"));
    preset_combo.set_active(Some(0));
    content.append(&tool_row(&gettext("Preset"), &preset_combo));

    let band_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    band_box.set_halign(gtk::Align::Center);
    band_box.add_css_class("equalizer-band-box");

    for (index, adjustment) in adjustments.iter().enumerate() {
        let column = gtk::Box::new(gtk::Orientation::Vertical, 4);
        column.set_halign(gtk::Align::Center);

        let value_label = gtk::Label::new(Some("0"));
        value_label.add_css_class("equalizer-value-label");

        let scale = gtk::Scale::new(gtk::Orientation::Vertical, Some(adjustment));
        scale.set_draw_value(false);
        scale.set_inverted(false);
        scale.set_size_request(24, 118);
        scale.add_css_class("equalizer-band-scale");

        let band_label = gtk::Label::new(Some(&equalizer_band_label(EQUALIZER_BANDS[index])));
        band_label.add_css_class("equalizer-band-label");

        let value_label_clone = value_label.clone();
        let graph_clone = graph.clone();
        adjustment.connect_value_changed(move |adjustment| {
            value_label_clone.set_label(&format!("{:+.0}", adjustment.value()));
            graph_clone.queue_draw();
        });

        column.append(&value_label);
        column.append(&scale);
        column.append(&band_label);
        band_box.append(&column);
    }

    let preset_adjustments = adjustments.to_vec();
    let graph_clone = graph.clone();
    preset_combo.connect_changed(move |combo| {
        let gains = equalizer_preset_gains(combo.active().unwrap_or(0));
        for (adjustment, gain) in preset_adjustments.iter().zip(gains) {
            adjustment.set_value(f64::from(gain));
        }
        graph_clone.queue_draw();
    });

    content.append(&band_box);
    content
}

fn equalizer_curve(adjustments: &[gtk::Adjustment]) -> gtk::DrawingArea {
    let values = adjustments.to_vec();
    let area = gtk::DrawingArea::builder()
        .content_width(360)
        .content_height(96)
        .halign(gtk::Align::Center)
        .build();
    area.add_css_class("equalizer-curve");
    area.set_draw_func(move |widget, cr, width, height| {
        draw_equalizer_curve(widget, cr, width, height, &values);
    });
    area
}

fn draw_equalizer_curve(
    widget: &gtk::DrawingArea,
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    adjustments: &[gtk::Adjustment],
) {
    let w = f64::from(width).max(1.0);
    let h = f64::from(height).max(1.0);
    let padding = 12.0;
    let plot_w = (w - padding * 2.0).max(1.0);
    let plot_h = (h - padding * 2.0).max(1.0);
    let fg = css_color(widget, "window_fg_color", (0.85, 0.88, 0.96));
    let accent = css_color(widget, "accent_bg_color", (0.45, 0.33, 1.0));

    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_line_width(1.0);
    cr.set_source_rgba(fg.0, fg.1, fg.2, 0.18);
    for line in 0..=4 {
        let y = padding + plot_h * f64::from(line) / 4.0;
        cr.move_to(padding, y);
        cr.line_to(w - padding, y);
    }
    let _ = cr.stroke();

    cr.set_line_width(2.4);
    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.92);
    for (index, adjustment) in adjustments.iter().enumerate() {
        let x =
            padding + plot_w * index as f64 / (adjustments.len().saturating_sub(1).max(1)) as f64;
        let normalized = ((adjustment.value() + 12.0) / 24.0).clamp(0.0, 1.0);
        let y = padding + plot_h * (1.0 - normalized);
        if index == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();
}

fn equalizer_preset_gains(index: u32) -> [f32; EQUALIZER_BANDS.len()] {
    match index {
        1 => [-8.0, -7.0, -5.0, -2.0, 0.0, 2.0, 4.0, 4.0, 2.0, 0.0],
        2 => [3.0, 3.0, 2.0, 1.0, 0.0, -1.0, -1.0, -2.0, -3.0, -3.0],
        3 => [-4.0, -3.0, -2.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 3.0],
        4 => [-12.0, -10.0, -8.0, -4.0, -2.0, 0.0, 1.0, 1.0, 0.0, 0.0],
        _ => [0.0; EQUALIZER_BANDS.len()],
    }
}

fn equalizer_band_label(frequency: f32) -> String {
    if frequency >= 1_000.0 {
        format!("{:.0}k", frequency / 1_000.0)
    } else {
        format!("{frequency:.0}")
    }
}

fn finish_processed_recording(
    context: &ToolContext,
    result: anyhow::Result<PathBuf>,
    error_heading: &str,
) {
    if let Err(err) = context.player.borrow_mut().stop() {
        show_error(&context.window, &gettext("Could not stop playback"), &err);
        return;
    }
    refresh_playback_rows(&context.recording_rows, None);

    match result {
        Ok(path) => {
            let duration = processing::duration(&path).ok();
            append_recording_row(context, path, duration, true);
        }
        Err(err) => show_error(&context.window, error_heading, &err),
    }
}

fn run_processed_task<F>(context: &ToolContext, error_heading: String, task: F)
where
    F: FnOnce() -> anyhow::Result<PathBuf> + Send + 'static,
{
    if let Err(err) = context.player.borrow_mut().stop() {
        show_error(&context.window, &gettext("Could not stop playback"), &err);
        return;
    }
    refresh_playback_rows(&context.recording_rows, None);

    let action_context = context.clone();
    let handle = gtk::gio::spawn_blocking(task);
    glib::MainContext::default().spawn_local(async move {
        match handle.await {
            Ok(result) => finish_processed_recording(&action_context, result, &error_heading),
            Err(_) => show_notice(
                &action_context.window,
                &error_heading,
                &gettext("The audio task stopped unexpectedly."),
            ),
        }
    });
}

fn run_export_task(
    window: &adw::ApplicationWindow,
    path: PathBuf,
    destination: PathBuf,
    options: ExportOptions,
) {
    let window = window.clone();
    let handle = gtk::gio::spawn_blocking(move || processing::export(&path, &destination, options));
    glib::MainContext::default().spawn_local(async move {
        match handle.await {
            Ok(Ok(())) => show_success(
                &window,
                &gettext("Recording exported"),
                &gettext("The file was exported successfully."),
            ),
            Ok(Err(err)) => show_error(&window, &gettext("Could not export recording"), &err),
            Err(_) => show_notice(
                &window,
                &gettext("Could not export recording"),
                &gettext("The export task stopped unexpectedly."),
            ),
        }
    });
}

fn tool_dialog(heading: &str, body: Option<&str>, content: &gtk::Box) -> adw::AlertDialog {
    let dialog = adw::AlertDialog::new(Some(heading), body);
    dialog.set_extra_child(Some(content));
    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("apply", &gettext("Apply"));
    dialog.set_default_response(Some("apply"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);
    dialog
}

fn tool_content_with_illustration(
    illustration: ToolIllustration,
    rows: &[(&str, &impl IsA<gtk::Widget>)],
) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.add_css_class("tool-dialog");
    content.append(&tool_illustration(illustration));

    let fields = gtk::Box::new(gtk::Orientation::Vertical, 10);
    fields.set_hexpand(true);
    for (label, widget) in rows {
        fields.append(&tool_row(label, *widget));
    }

    content.append(&fields);
    content
}

fn tool_illustration_content(illustration: ToolIllustration) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.add_css_class("tool-dialog");
    content.append(&tool_illustration(illustration));
    content
}

fn tool_row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_hexpand(true);

    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_hexpand(true);

    row.append(&text);
    row.append(widget);
    row
}

fn horizontal_value_control(
    min: f64,
    max: f64,
    step: f64,
    digits: u32,
    value: f64,
    suffix: &'static str,
) -> (gtk::Box, gtk::Adjustment) {
    let adjustment = gtk::Adjustment::new(value, min, max, step, step * 10.0, 0.0);
    let scale = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adjustment));
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    scale.set_size_request(220, -1);
    scale.add_css_class("horizontal-tool-scale");

    let value_label = gtk::Label::new(Some(&slider_value_text(value, digits, suffix)));
    value_label.add_css_class("horizontal-value-label");
    value_label.set_width_chars(7);
    value_label.set_xalign(1.0);

    let value_label_clone = value_label.clone();
    adjustment.connect_value_changed(move |adjustment| {
        value_label_clone.set_label(&slider_value_text(adjustment.value(), digits, suffix));
    });

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    content.set_hexpand(true);
    content.set_valign(gtk::Align::Center);
    content.add_css_class("horizontal-value-control");
    content.append(&scale);
    content.append(&value_label);

    (content, adjustment)
}

fn slider_value_text(value: f64, digits: u32, suffix: &str) -> String {
    if digits == 0 {
        format!("{value:.0}{suffix}")
    } else {
        format!("{value:.precision$}{suffix}", precision = digits as usize)
    }
}

fn tool_illustration(kind: ToolIllustration) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::builder()
        .content_width(260)
        .content_height(96)
        .halign(gtk::Align::Center)
        .build();
    area.add_css_class("tool-illustration");
    area.set_draw_func(move |_, cr, width, height| {
        draw_tool_illustration(cr, width, height, kind);
    });
    area
}

fn draw_tool_illustration(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    kind: ToolIllustration,
) {
    let _ = cr.save();
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_line_join(gtk::cairo::LineJoin::Round);
    cr.translate(0.5, 0.5);

    let w = f64::from(width).max(1.0);
    let h = f64::from(height).max(1.0);
    draw_panel_grid(cr, (w, h));

    let drawing_width = 124.0;
    let _ = cr.save();
    cr.translate(((w - drawing_width) / 2.0).max(0.0), 0.0);
    match kind {
        ToolIllustration::Trim => draw_trim_illustration(cr, (drawing_width, h)),
        ToolIllustration::Volume => draw_volume_illustration(cr, (drawing_width, h)),
        ToolIllustration::Speed => draw_speed_illustration(cr, (drawing_width, h)),
        ToolIllustration::Pitch => draw_pitch_illustration(cr, (drawing_width, h)),
        ToolIllustration::VoiceEffects => draw_voice_effects_illustration(cr, (drawing_width, h)),
        ToolIllustration::Reverse => draw_reverse_illustration(cr, (drawing_width, h)),
        ToolIllustration::Merge => draw_merge_illustration(cr, (drawing_width, h)),
        ToolIllustration::Export => draw_export_illustration(cr, (drawing_width, h)),
    }
    let _ = cr.restore();

    let _ = cr.restore();
}

fn draw_panel_grid(cr: &gtk::cairo::Context, size: (f64, f64)) {
    let (w, h) = size;
    set_source(cr, (0.25, 0.29, 0.37, 0.38));
    cr.set_line_width(1.0);
    for line in 1..4 {
        let y = h * f64::from(line) / 4.0;
        cr.move_to(8.0, y);
        cr.line_to(w - 8.0, y);
    }
    let _ = cr.stroke();
}

fn draw_trim_illustration(cr: &gtk::cairo::Context, size: (f64, f64)) {
    draw_wave(
        cr,
        (13.0, 48.0),
        (98.0, 40.0),
        &[0.25, 0.6, 0.35, 0.85, 0.5, 0.75, 0.3],
        false,
    );
    draw_cut_line(cr, 40.0, size.1);
    draw_cut_line(cr, 82.0, size.1);
    draw_arrow(cr, (48.0, 68.0), (74.0, 68.0), (0.45, 0.33, 1.0, 0.95));
}

fn draw_volume_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_bars(
        cr,
        16.0,
        64.0,
        &[0.18, 0.3, 0.22, 0.28],
        (0.33, 0.86, 1.0, 0.9),
    );
    draw_arrow(cr, (52.0, 52.0), (72.0, 52.0), (0.45, 0.33, 1.0, 0.95));
    draw_bars(
        cr,
        82.0,
        64.0,
        &[0.35, 0.7, 0.45, 0.85],
        (0.71, 0.48, 1.0, 0.95),
    );
}

fn draw_speed_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_ticks(cr, 14.0, 48.0, 9.0, 6, (0.33, 0.86, 1.0, 0.9));
    draw_arrow(cr, (50.0, 48.0), (70.0, 48.0), (0.45, 0.33, 1.0, 0.95));
    draw_ticks(cr, 82.0, 48.0, 4.5, 7, (0.71, 0.48, 1.0, 0.95));
}

fn draw_pitch_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_sine(cr, (14.0, 58.0), (38.0, 18.0), 1.2, (0.33, 0.86, 1.0, 0.9));
    draw_arrow(cr, (54.0, 48.0), (72.0, 48.0), (0.45, 0.33, 1.0, 0.95));
    draw_sine(cr, (82.0, 45.0), (34.0, 28.0), 2.3, (0.71, 0.48, 1.0, 0.95));
}

fn draw_voice_effects_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_sine(cr, (12.0, 58.0), (34.0, 18.0), 1.4, (0.33, 0.86, 1.0, 0.9));
    draw_arrow(cr, (47.0, 48.0), (64.0, 48.0), (0.45, 0.33, 1.0, 0.95));
    draw_sine(cr, (72.0, 45.0), (22.0, 28.0), 2.8, (0.71, 0.48, 1.0, 0.95));
    set_source(cr, (0.33, 0.86, 1.0, 0.82));
    cr.set_line_width(2.0);
    cr.arc(104.0, 38.0, 7.0, 0.0, std::f64::consts::TAU);
    cr.arc(104.0, 61.0, 7.0, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();
}

fn draw_reverse_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_wave(
        cr,
        (14.0, 48.0),
        (38.0, 36.0),
        &[0.25, 0.45, 0.8, 0.35],
        false,
    );
    draw_arrow(cr, (72.0, 48.0), (52.0, 48.0), (0.45, 0.33, 1.0, 0.95));
    draw_wave(
        cr,
        (78.0, 48.0),
        (36.0, 36.0),
        &[0.35, 0.8, 0.45, 0.25],
        true,
    );
}

fn draw_merge_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_wave(
        cr,
        (12.0, 34.0),
        (36.0, 24.0),
        &[0.25, 0.55, 0.35, 0.65],
        false,
    );
    draw_wave(
        cr,
        (12.0, 64.0),
        (36.0, 24.0),
        &[0.65, 0.35, 0.55, 0.25],
        false,
    );
    draw_arrow(cr, (52.0, 48.0), (72.0, 48.0), (0.45, 0.33, 1.0, 0.95));
    draw_wave(
        cr,
        (78.0, 48.0),
        (36.0, 38.0),
        &[0.25, 0.55, 0.35, 0.65, 0.65, 0.35],
        false,
    );
}

fn draw_export_illustration(cr: &gtk::cairo::Context, _size: (f64, f64)) {
    draw_wave(
        cr,
        (12.0, 52.0),
        (42.0, 36.0),
        &[0.25, 0.65, 0.35, 0.85, 0.45],
        false,
    );
    draw_arrow(cr, (56.0, 52.0), (76.0, 52.0), (0.45, 0.33, 1.0, 0.95));
    set_source(cr, (0.71, 0.48, 1.0, 0.95));
    cr.set_line_width(2.0);
    cr.rectangle(84.0, 25.0, 25.0, 42.0);
    cr.move_to(99.0, 25.0);
    cr.line_to(109.0, 35.0);
    let _ = cr.stroke();
    draw_wave(
        cr,
        (88.0, 51.0),
        (17.0, 16.0),
        &[0.2, 0.55, 0.3, 0.45],
        false,
    );
}

fn draw_wave(
    cr: &gtk::cairo::Context,
    origin: (f64, f64),
    size: (f64, f64),
    amplitudes: &[f64],
    reversed: bool,
) {
    let color = if reversed {
        (0.71, 0.48, 1.0, 0.95)
    } else {
        (0.33, 0.86, 1.0, 0.9)
    };
    draw_bars(cr, origin.0, origin.1 + size.1 / 2.0, amplitudes, color);
}

fn draw_bars(
    cr: &gtk::cairo::Context,
    x: f64,
    center_y: f64,
    amplitudes: &[f64],
    color: (f64, f64, f64, f64),
) {
    set_source(cr, color);
    cr.set_line_width(3.0);
    for (index, amplitude) in amplitudes.iter().enumerate() {
        let bar_x = x + index as f64 * 8.0;
        let height = 34.0 * amplitude;
        cr.move_to(bar_x, center_y - height / 2.0);
        cr.line_to(bar_x, center_y + height / 2.0);
    }
    let _ = cr.stroke();
}

fn draw_ticks(
    cr: &gtk::cairo::Context,
    x: f64,
    center_y: f64,
    spacing: f64,
    count: usize,
    color: (f64, f64, f64, f64),
) {
    set_source(cr, color);
    cr.set_line_width(2.5);
    for index in 0..count {
        let tick_x = x + index as f64 * spacing;
        cr.move_to(tick_x, center_y - 15.0);
        cr.line_to(tick_x, center_y + 15.0);
    }
    let _ = cr.stroke();
}

fn draw_sine(
    cr: &gtk::cairo::Context,
    origin: (f64, f64),
    size: (f64, f64),
    cycles: f64,
    color: (f64, f64, f64, f64),
) {
    set_source(cr, color);
    cr.set_line_width(2.0);
    let steps = 28;
    for step in 0..=steps {
        let t = f64::from(step) / f64::from(steps);
        let x = origin.0 + t * size.0;
        let y = origin.1 + (t * cycles * std::f64::consts::TAU).sin() * size.1 / 2.0;
        if step == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();
}

fn draw_arrow(
    cr: &gtk::cairo::Context,
    start: (f64, f64),
    end: (f64, f64),
    color: (f64, f64, f64, f64),
) {
    set_source(cr, color);
    cr.set_line_width(2.0);
    cr.move_to(start.0, start.1);
    cr.line_to(end.0, end.1);
    let _ = cr.stroke();

    let direction = if end.0 >= start.0 { 1.0 } else { -1.0 };
    cr.move_to(end.0, end.1);
    cr.line_to(end.0 - direction * 7.0, end.1 - 5.0);
    cr.line_to(end.0 - direction * 7.0, end.1 + 5.0);
    cr.close_path();
    let _ = cr.fill();
}

fn draw_cut_line(cr: &gtk::cairo::Context, x: f64, height: f64) {
    set_source(cr, (0.96, 0.38, 0.42, 0.95));
    cr.set_line_width(2.0);
    cr.move_to(x, 18.0);
    cr.line_to(x, height - 18.0);
    let _ = cr.stroke();
}

fn set_source(cr: &gtk::cairo::Context, color: (f64, f64, f64, f64)) {
    cr.set_source_rgba(color.0, color.1, color.2, color.3);
}

fn css_color(
    widget: &impl IsA<gtk::Widget>,
    name: &str,
    fallback: (f64, f64, f64),
) -> (f64, f64, f64) {
    widget
        .style_context()
        .lookup_color(name)
        .map(|color| {
            (
                f64::from(color.red()),
                f64::from(color.green()),
                f64::from(color.blue()),
            )
        })
        .unwrap_or(fallback)
}

fn number_spin(min: f64, max: f64, step: f64, digits: u32, value: f64) -> gtk::SpinButton {
    let adjustment = gtk::Adjustment::new(value, min, max, step, step * 10.0, 0.0);
    gtk::SpinButton::new(Some(&adjustment), step, digits)
}

fn tool_tooltip(action: ToolAction) -> String {
    match action {
        ToolAction::Trim => gettext("Trim the selected recording"),
        ToolAction::Volume => gettext("Change the selected recording volume"),
        ToolAction::Speed => gettext("Change the selected recording speed"),
        ToolAction::Pitch => gettext("Change the selected recording pitch"),
        ToolAction::VoiceEffects => gettext("Apply a voice effect to the selected recording"),
        ToolAction::Equalizer => gettext("Adjust the selected recording frequency bands"),
        ToolAction::Reverse => gettext("Reverse the selected recording"),
        ToolAction::Merge => gettext("Merge the selected recording with another recording"),
        ToolAction::Export => gettext("Export the selected recording"),
    }
}

fn refresh_playback_rows(recording_rows: &RecordingRows, status: Option<&PlaybackStatus>) {
    let playing_path = status.map(|status| status.path.as_path());
    let progress = status.map(playback_progress).unwrap_or(0.0);

    for row in recording_rows.borrow().iter() {
        let is_playing = playing_path.is_some_and(|path| path == row.path.as_path());
        row.playback_symbol
            .set_visible_child_name(if is_playing { "pause" } else { "play" });
        row.playback_bar
            .set_progress(if is_playing { progress } else { 0.0 });
    }
}

fn playback_progress(status: &PlaybackStatus) -> f64 {
    let Some(duration) = status.duration else {
        return 0.0;
    };
    if duration <= Duration::ZERO {
        return 0.0;
    }

    (status.position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
}

fn recording_title(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| gettext("Recording"))
}

fn recording_subtitle(duration: Option<Duration>) -> String {
    if let Some(duration) = duration {
        let compact_duration = format_compact_duration(duration);
        return format_message(
            &gettext("Duration {duration}"),
            &[("{duration}", &compact_duration)],
        );
    }

    gettext("Saved recording")
}

fn row_playback_symbol(paused: bool) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::builder()
        .content_width(24)
        .content_height(24)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    area.set_draw_func(move |widget, cr, width, height| {
        let accent = css_color(widget, "accent_color", (0.45, 0.72, 1.0));
        cr.set_source_rgb(accent.0, accent.1, accent.2);
        let w = f64::from(width);
        let h = f64::from(height);
        let cx = w / 2.0;
        let cy = h / 2.0;

        if paused {
            let bar_width = 4.0;
            let bar_height = 13.0;
            let gap = 4.0;
            cr.rectangle(
                cx - gap / 2.0 - bar_width,
                cy - bar_height / 2.0,
                bar_width,
                bar_height,
            );
            cr.rectangle(cx + gap / 2.0, cy - bar_height / 2.0, bar_width, bar_height);
        } else {
            cr.move_to(cx - 4.0, cy - 7.0);
            cr.line_to(cx - 4.0, cy + 7.0);
            cr.line_to(cx + 8.0, cy);
            cr.close_path();
        }
        let _ = cr.fill();
    });
    area
}

fn toggle_button_content(icon: &str, label: &str) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);

    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(16);

    let text = gtk::Label::new(Some(label));
    text.add_css_class("tool-label");

    content.append(&image);
    content.append(&text);
    content
}

fn compact_tool_button(icon: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);

    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(16);

    let text = gtk::Label::new(Some(label));
    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.add_css_class("tool-label");

    content.append(&image);
    content.append(&text);
    button.set_child(Some(&content));
    button
}

#[derive(Clone, Copy)]
enum MessageKind {
    Info,
    Success,
    Error,
}

impl MessageKind {
    fn icon_name(self) -> &'static str {
        match self {
            Self::Info => "dialog-information-symbolic",
            Self::Success => "emblem-ok-symbolic",
            Self::Error => "dialog-error-symbolic",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Error => "error",
        }
    }
}

fn show_error(window: &adw::ApplicationWindow, heading: &str, err: &anyhow::Error) {
    show_message(window, heading, &err.to_string(), MessageKind::Error);
}

fn show_notice(window: &adw::ApplicationWindow, heading: &str, body: &str) {
    show_message(window, heading, body, MessageKind::Info);
}

fn show_success(window: &adw::ApplicationWindow, heading: &str, body: &str) {
    show_message(window, heading, body, MessageKind::Success);
}

fn show_message(window: &adw::ApplicationWindow, heading: &str, body: &str, kind: MessageKind) {
    let dialog = adw::AlertDialog::new(Some(heading), None);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.add_css_class("message-content");
    content.set_halign(gtk::Align::Center);

    let icon = gtk::Image::from_icon_name(kind.icon_name());
    icon.set_pixel_size(46);
    icon.add_css_class("message-icon");
    icon.add_css_class(kind.css_class());

    let body_label = gtk::Label::new(Some(body));
    body_label.add_css_class("message-body");
    body_label.set_wrap(true);
    body_label.set_justify(gtk::Justification::Center);
    body_label.set_xalign(0.5);

    content.append(&icon);
    content.append(&body_label);
    dialog.set_extra_child(Some(&content));
    dialog.add_response("ok", &gettext("OK"));
    dialog.set_default_response(Some("ok"));
    dialog.present(Some(window));
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn format_compact_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let minutes = seconds / 60;
    let seconds = seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}
