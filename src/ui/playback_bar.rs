use std::cell::Cell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use gtk::prelude::*;

const PLAYBACK_BINS: usize = 96;
const MIN_VISIBLE_LEVEL: f64 = 0.03;
const MIN_PROGRESS_DELTA: f64 = 0.00005;

#[derive(Clone)]
pub struct PlaybackBar {
    area: gtk::DrawingArea,
    progress: Rc<Cell<f64>>,
}

impl PlaybackBar {
    pub fn new(path: &Path) -> Self {
        let progress = Rc::new(Cell::new(0.0));
        let segments = Rc::new(analyze_path(path).unwrap_or_else(default_segments));

        let area = gtk::DrawingArea::builder()
            .height_request(12)
            .hexpand(true)
            .build();
        area.add_css_class("playback-progress");

        let draw_progress = Rc::clone(&progress);
        let draw_segments = Rc::clone(&segments);
        area.set_draw_func(move |area, context, width, height| {
            draw_playback_bar(
                area,
                context,
                f64::from(width),
                f64::from(height),
                &draw_segments,
                draw_progress.get(),
            );
        });

        Self { area, progress }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.area
    }

    pub fn set_progress(&self, progress: f64) {
        let progress = progress.clamp(0.0, 1.0);
        if (self.progress.get() - progress).abs() < MIN_PROGRESS_DELTA {
            return;
        }

        self.progress.set(progress);
        self.area.queue_draw();
    }
}

fn draw_playback_bar(
    widget: &gtk::DrawingArea,
    context: &gtk::cairo::Context,
    width: f64,
    height: f64,
    segments: &[f64],
    progress: f64,
) {
    if segments.is_empty() || width <= 1.0 || height <= 1.0 {
        return;
    }

    let padding = 2.0;
    let available_width = (width - padding * 2.0).max(1.0);
    let slot = available_width / segments.len() as f64;
    let gap = (slot * 0.34).clamp(1.0, 3.0);
    let segment_width = (slot - gap).max(1.0);
    let center_y = height / 2.0;
    let progress_x = padding + available_width * progress.clamp(0.0, 1.0);
    let fg = css_color(widget, "window_fg_color", (0.70, 0.75, 0.84));
    let accent = css_color(widget, "accent_bg_color", (0.80, 0.84, 0.35));

    context.set_line_cap(gtk::cairo::LineCap::Round);
    context.set_line_width(3.5);

    for (index, segment) in segments.iter().enumerate() {
        if *segment <= 0.0 {
            continue;
        }

        let x1 = padding + index as f64 * slot;
        let x2 = (x1 + segment_width).min(padding + available_width);
        let alpha = if *segment >= MIN_VISIBLE_LEVEL {
            0.54
        } else {
            0.22
        };

        context.set_source_rgba(fg.0, fg.1, fg.2, alpha);
        context.move_to(x1, center_y);
        context.line_to(x2, center_y);
        let _ = context.stroke();
    }

    for (index, segment) in segments.iter().enumerate() {
        if *segment < MIN_VISIBLE_LEVEL {
            continue;
        }

        let x1 = padding + index as f64 * slot;
        if x1 >= progress_x {
            break;
        }

        let x2 = (x1 + segment_width).min(padding + available_width);
        let clipped_x2 = x2.min(progress_x);
        if clipped_x2 <= x1 {
            continue;
        }

        context.set_source_rgba(accent.0, accent.1, accent.2, 0.78 + segment * 0.18);
        context.move_to(x1, center_y);
        context.line_to(clipped_x2, center_y);
        let _ = context.stroke();
    }

    if (0.01..0.99).contains(&progress) {
        context.set_line_width(4.5);
        context.set_source_rgba(accent.0, accent.1, accent.2, 0.96);
        context.move_to(progress_x, center_y - 5.0);
        context.line_to(progress_x, center_y + 5.0);
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

fn analyze_path(path: &Path) -> Option<Vec<f64>> {
    let bytes = fs::read(path).ok()?;
    analyze_wav_bytes(&bytes)
}

fn analyze_wav_bytes(bytes: &[u8]) -> Option<Vec<f64>> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut format = None;
    let mut data_range = None;
    let mut offset = 12;

    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = read_u32(bytes, offset + 4)? as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start.checked_add(chunk_size)?;
        if chunk_end > bytes.len() {
            break;
        }

        match chunk_id {
            b"fmt " => format = parse_format(&bytes[chunk_start..chunk_end]),
            b"data" => data_range = Some(chunk_start..chunk_end),
            _ => {}
        }

        offset = chunk_end + (chunk_size % 2);
    }

    let format = format?;
    let data = &bytes[data_range?];
    let bytes_per_sample = usize::from(format.bits_per_sample / 8);
    if format.channels == 0 || bytes_per_sample == 0 {
        return None;
    }

    let frame_size = bytes_per_sample.checked_mul(usize::from(format.channels))?;
    let frame_count = data.len() / frame_size;
    if frame_count == 0 {
        return Some(vec![0.0; PLAYBACK_BINS]);
    }

    let mut peaks = vec![0.0_f64; PLAYBACK_BINS];
    for frame_index in 0..frame_count {
        let bin = frame_index * PLAYBACK_BINS / frame_count;
        let frame_start = frame_index * frame_size;
        let mut frame_peak = 0.0_f64;

        for channel in 0..usize::from(format.channels) {
            let sample_start = frame_start + channel * bytes_per_sample;
            let sample_end = sample_start + bytes_per_sample;
            if let Some(sample) =
                sample_amplitude(&data[sample_start..sample_end], format.audio_format)
            {
                frame_peak = frame_peak.max(sample);
            }
        }

        peaks[bin] = peaks[bin].max(frame_peak);
    }

    Some(normalize_peaks(peaks))
}

#[derive(Clone, Copy)]
struct WaveFormat {
    audio_format: AudioFormat,
    channels: u16,
    bits_per_sample: u16,
}

#[derive(Clone, Copy)]
enum AudioFormat {
    Pcm,
    Float,
}

fn parse_format(bytes: &[u8]) -> Option<WaveFormat> {
    if bytes.len() < 16 {
        return None;
    }

    let raw_format = read_u16(bytes, 0)?;
    let channels = read_u16(bytes, 2)?;
    let bits_per_sample = read_u16(bytes, 14)?;
    let audio_format = match raw_format {
        1 => AudioFormat::Pcm,
        3 => AudioFormat::Float,
        0xfffe => extensible_audio_format(bytes)?,
        _ => return None,
    };

    Some(WaveFormat {
        audio_format,
        channels,
        bits_per_sample,
    })
}

fn extensible_audio_format(bytes: &[u8]) -> Option<AudioFormat> {
    if bytes.len() < 40 {
        return None;
    }

    match &bytes[24..40] {
        [1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113] => Some(AudioFormat::Pcm),
        [3, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113] => Some(AudioFormat::Float),
        _ => None,
    }
}

fn sample_amplitude(bytes: &[u8], format: AudioFormat) -> Option<f64> {
    match (format, bytes.len()) {
        (AudioFormat::Float, 4) => {
            let value = f32::from_le_bytes(bytes.try_into().ok()?) as f64;
            value.is_finite().then_some(value.abs().clamp(0.0, 1.0))
        }
        (AudioFormat::Pcm, 1) => Some(((f64::from(bytes[0]) - 128.0) / 128.0).abs()),
        (AudioFormat::Pcm, 2) => Some(
            (f64::from(i16::from_le_bytes(bytes.try_into().ok()?)) / f64::from(i16::MAX)).abs(),
        ),
        (AudioFormat::Pcm, 3) => {
            let value =
                i32::from(bytes[0]) | (i32::from(bytes[1]) << 8) | (i32::from(bytes[2]) << 16);
            let signed = (value << 8) >> 8;
            Some((f64::from(signed) / 8_388_608.0).abs())
        }
        (AudioFormat::Pcm, 4) => Some(
            (f64::from(i32::from_le_bytes(bytes.try_into().ok()?)) / f64::from(i32::MAX)).abs(),
        ),
        _ => None,
    }
}

fn normalize_peaks(mut peaks: Vec<f64>) -> Vec<f64> {
    let max_peak = peaks.iter().copied().fold(0.0_f64, f64::max);
    if max_peak <= 0.0001 {
        return peaks;
    }

    let silence_threshold = (max_peak * 0.12).clamp(0.0008, 0.035);
    let usable_range = (max_peak - silence_threshold).max(0.0001);
    for peak in &mut peaks {
        if *peak <= silence_threshold {
            *peak = 0.0;
        } else {
            let normalized = ((*peak - silence_threshold) / usable_range).clamp(0.0, 1.0);
            *peak = normalized.powf(0.45).max(0.18);
        }
    }

    peaks
}

fn default_segments() -> Vec<f64> {
    vec![1.0; PLAYBACK_BINS]
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_active_and_silent_pcm_regions() {
        let mut samples = vec![0_i16; 960];
        for sample in &mut samples[180..360] {
            *sample = 10_000;
        }
        for sample in &mut samples[620..760] {
            *sample = -14_000;
        }

        let segments = analyze_wav_bytes(&pcm_wav(&samples)).expect("valid wav");

        assert_eq!(segments.len(), PLAYBACK_BINS);
        assert!(segments.iter().any(|segment| *segment == 0.0));
        assert!(segments.iter().any(|segment| *segment > 0.75));
    }

    fn pcm_wav(samples: &[i16]) -> Vec<u8> {
        let data_size = (samples.len() * 2) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&96_000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }
}
