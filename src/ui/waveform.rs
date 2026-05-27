use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::audio::recorder::WAVEFORM_BINS;

const FRESH_BARS: usize = 3;

#[derive(Clone)]
pub struct Waveform {
    area: gtk::DrawingArea,
    active: Rc<Cell<bool>>,
    target_level: Rc<Cell<f64>>,
    target_peaks: Rc<RefCell<[f64; WAVEFORM_BINS]>>,
}

impl Waveform {
    pub fn new() -> Self {
        let active = Rc::new(Cell::new(false));
        let target_level = Rc::new(Cell::new(0.0));
        let current_level = Rc::new(Cell::new(0.0));
        let target_peaks = Rc::new(RefCell::new([0.0_f64; WAVEFORM_BINS]));
        let current_peaks = Rc::new(RefCell::new([0.0_f64; WAVEFORM_BINS]));

        let area = gtk::DrawingArea::builder()
            .height_request(108)
            .hexpand(true)
            .build();
        area.add_css_class("waveform");

        let draw_active = Rc::clone(&active);
        let draw_level = Rc::clone(&current_level);
        let draw_peaks = Rc::clone(&current_peaks);
        area.set_draw_func(move |area, context, width, height| {
            draw_waveform(
                area,
                context,
                f64::from(width),
                f64::from(height),
                draw_active.get(),
                draw_level.get(),
                &draw_peaks.borrow(),
            );
        });

        let tick_area = area.clone();
        let tick_active = Rc::clone(&active);
        let tick_target = Rc::clone(&target_level);
        let tick_level = Rc::clone(&current_level);
        let tick_target_peaks = Rc::clone(&target_peaks);
        let tick_current_peaks = Rc::clone(&current_peaks);
        glib::timeout_add_local(Duration::from_millis(33), move || {
            let target = if tick_active.get() {
                tick_target.get()
            } else {
                0.0
            };
            let current = tick_level.get();
            tick_level.set(current + (target - current) * 0.22);

            let target_peaks = tick_target_peaks.borrow();
            let mut current_peaks = tick_current_peaks.borrow_mut();
            for (index, current_peak) in current_peaks.iter_mut().enumerate() {
                let target_peak = if tick_active.get() {
                    target_peaks[index]
                } else {
                    0.0
                };
                let smoothing = if target_peak > *current_peak {
                    0.48
                } else {
                    0.18
                };
                *current_peak += (target_peak - *current_peak) * smoothing;
                if *current_peak < 0.003 {
                    *current_peak = 0.0;
                }
            }

            tick_area.queue_draw();
            glib::ControlFlow::Continue
        });

        Self {
            area,
            active,
            target_level,
            target_peaks,
        }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.area
    }

    pub fn set_active(&self, active: bool) {
        self.active.set(active);
        if !active {
            self.target_level.set(0.0);
            self.target_peaks.borrow_mut().fill(0.0);
        }
    }

    pub fn set_level(&self, level: f64) {
        self.target_level.set(level.clamp(0.0, 1.0));
    }

    pub fn set_peaks(&self, peaks: [f64; WAVEFORM_BINS]) {
        let mut target_peaks = self.target_peaks.borrow_mut();
        target_peaks.rotate_left(FRESH_BARS);

        for index in 0..FRESH_BARS {
            let chunk_start = index * peaks.len() / FRESH_BARS;
            let chunk_end = (index + 1) * peaks.len() / FRESH_BARS;
            let peak = peaks[chunk_start..chunk_end]
                .iter()
                .copied()
                .fold(0.0_f64, f64::max);
            target_peaks[WAVEFORM_BINS - FRESH_BARS + index] = peak.clamp(0.0, 1.0);
        }
    }
}

fn draw_waveform(
    widget: &gtk::DrawingArea,
    context: &gtk::cairo::Context,
    width: f64,
    height: f64,
    active: bool,
    level: f64,
    peaks: &[f64; WAVEFORM_BINS],
) {
    let padding = 14.0;
    let center = height / 2.0;
    let available_width = width - padding * 2.0;
    let slot = available_width / peaks.len() as f64;
    let view_bg = css_color(widget, "view_bg_color", (0.075, 0.082, 0.110));
    let accent = css_color(widget, "accent_bg_color", (0.30, 0.84, 1.0));

    context.set_source_rgb(view_bg.0, view_bg.1, view_bg.2);
    context.rectangle(0.0, 0.0, width, height);
    let _ = context.fill();

    context.set_line_cap(gtk::cairo::LineCap::Round);
    context.set_line_width((slot * 0.50).clamp(4.0, 7.0));
    for (index, peak) in peaks.iter().enumerate() {
        if *peak <= 0.0 {
            continue;
        }

        let bar_height = ((height - padding * 2.0) * peak).max(if active { 5.0 } else { 0.0 });
        let x = padding + index as f64 * slot + slot / 2.0;
        let y1 = center - bar_height / 2.0;
        let y2 = center + bar_height / 2.0;

        if index % 2 == 0 {
            context.set_source_rgba(accent.0, accent.1, accent.2, 0.86 + level * 0.10);
        } else {
            context.set_source_rgba(0.76, 0.47, 1.0, 0.84 + level * 0.10);
        }

        context.move_to(x, y1);
        context.line_to(x, y2);
        let _ = context.stroke();
    }
}

fn css_color(widget: &gtk::DrawingArea, name: &str, fallback: (f64, f64, f64)) -> (f64, f64, f64) {
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
