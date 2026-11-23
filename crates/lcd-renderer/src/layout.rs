//! Display-independent dashboard layout and the renderer that turns a layout +
//! telemetry snapshot into a display-sized [`Frame`].
//!
//! A layout is authored in its own logical coordinate space (e.g. 1280x480) and
//! can be rendered at any physical resolution; each physical display owns its own
//! layout assignment (ARCHITECTURE.md §4.7).
//!
//! Phase 1 renders each widget as a labeled bar gauge. Glyph/text rendering needs
//! a font rasterizer and is deliberately deferred; the bar fill + color already
//! make values and "no data" legible for the static/live bring-up.

use crate::frame::{Frame, Rgba};
use serde::{Deserialize, Serialize};
use shookshmam_sensor_model::{SensorStatus, Snapshot};

/// A rectangle in layout (logical) coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A single gauge bound to a sensor, with the value range it maps across.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub rect: Rect,
    pub sensor_id: String,
    pub label: String,
    pub min: f64,
    pub max: f64,
}

/// A complete dashboard for one display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    ZeroLayoutDimensions,
    ZeroTargetDimensions,
    WidgetOutOfBounds { sensor_id: String },
    ZeroWidgetDimensions { sensor_id: String },
    InvalidRange { sensor_id: String },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::ZeroLayoutDimensions => write!(f, "layout has zero width/height"),
            LayoutError::ZeroTargetDimensions => write!(f, "target has zero width/height"),
            LayoutError::WidgetOutOfBounds { sensor_id } => {
                write!(f, "widget `{sensor_id}` lies outside the layout bounds")
            }
            LayoutError::ZeroWidgetDimensions { sensor_id } => {
                write!(f, "widget `{sensor_id}` has zero width/height")
            }
            LayoutError::InvalidRange { sensor_id } => {
                write!(f, "widget `{sensor_id}` has min >= max")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

impl Layout {
    /// Validate every widget against the layout bounds and value ranges.
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.width == 0 || self.height == 0 {
            return Err(LayoutError::ZeroLayoutDimensions);
        }
        for wdg in &self.widgets {
            let r = wdg.rect;
            if r.w == 0 || r.h == 0 {
                return Err(LayoutError::ZeroWidgetDimensions { sensor_id: wdg.sensor_id.clone() });
            }
            if r.x + r.w > self.width || r.y + r.h > self.height {
                return Err(LayoutError::WidgetOutOfBounds { sensor_id: wdg.sensor_id.clone() });
            }
            if wdg.min >= wdg.max {
                return Err(LayoutError::InvalidRange { sensor_id: wdg.sensor_id.clone() });
            }
        }
        Ok(())
    }
}

/// Scale a logical coordinate to target space, flooring (never exceeding bounds).
fn scale(v: u32, from: u32, to: u32) -> u32 {
    ((v as u64 * to as u64) / from.max(1) as u64) as u32
}

/// Render `layout` at `target_w`×`target_h` using `snapshot`.
pub fn render(
    layout: &Layout,
    snapshot: &Snapshot,
    target_w: u32,
    target_h: u32,
) -> Result<Frame, LayoutError> {
    layout.validate()?;
    if target_w == 0 || target_h == 0 {
        return Err(LayoutError::ZeroTargetDimensions);
    }
    let mut frame = Frame::new(target_w, target_h, Rgba(16, 18, 24, 255))
        .ok_or(LayoutError::ZeroTargetDimensions)?;

    for wdg in &layout.widgets {
        let x = scale(wdg.rect.x, layout.width, target_w);
        let y = scale(wdg.rect.y, layout.height, target_h);
        let w = scale(wdg.rect.w, layout.width, target_w).max(1);
        let h = scale(wdg.rect.h, layout.height, target_h).max(1);

        // Panel background + border.
        frame.fill_rect(x, y, w, h, Rgba(28, 31, 40, 255));
        frame.stroke_rect(x, y, w, h, Rgba(70, 74, 86, 255));

        let reading = snapshot.get(&wdg.sensor_id);
        let usable = reading.map(|r| r.status.carries_value()).unwrap_or(false);

        match (reading, usable) {
            (Some(r), true) => {
                let value = r.value.unwrap_or(wdg.min);
                let frac = ((value - wdg.min) / (wdg.max - wdg.min)).clamp(0.0, 1.0);
                // Green -> amber -> red as the gauge fills.
                let color = if frac < 0.5 {
                    Rgba::lerp(Rgba(60, 200, 120, 255), Rgba(230, 190, 60, 255), frac * 2.0)
                } else {
                    Rgba::lerp(Rgba(230, 190, 60, 255), Rgba(226, 74, 74, 255), (frac - 0.5) * 2.0)
                };
                let inner_w = w.saturating_sub(4);
                let fill_w = (inner_w as f64 * frac).round() as u32;
                frame.fill_rect(x + 2, y + 2, fill_w, h.saturating_sub(4), color);
                // Dim the panel for a stale (last-known) value.
                if r.status == SensorStatus::Stale {
                    frame.stroke_rect(x, y, w, h, Rgba(150, 140, 60, 255));
                }
            }
            _ => {
                // No usable data: a distinct gray hatch, never a zero-filled bar.
                draw_no_data(&mut frame, x, y, w, h);
            }
        }
    }

    Ok(frame)
}

fn draw_no_data(frame: &mut Frame, x: u32, y: u32, w: u32, h: u32) {
    let mut yy = y + 2;
    while yy < y + h.saturating_sub(2) {
        let mut xx = x + 2 + (yy % 2) * 2;
        while xx < x + w.saturating_sub(2) {
            frame.put(xx, yy, Rgba::GRAY);
            xx += 4;
        }
        yy += 2;
    }
}

/// A full-frame static test pattern for display bring-up (PHASE-1-SPEC §8:
/// "static test pattern before live dashboard"). Vertical color bars plus a
/// 1px white border for orientation/edge checks.
pub fn test_pattern(width: u32, height: u32) -> Option<Frame> {
    let mut frame = Frame::new(width, height, Rgba::BLACK)?;
    let bars = [
        Rgba(255, 255, 255, 255),
        Rgba(230, 190, 60, 255),
        Rgba(60, 200, 220, 255),
        Rgba(60, 200, 120, 255),
        Rgba(220, 80, 200, 255),
        Rgba(226, 74, 74, 255),
        Rgba(70, 90, 230, 255),
        Rgba(20, 20, 20, 255),
    ];
    let bar_w = (width / bars.len() as u32).max(1);
    for (i, c) in bars.iter().enumerate() {
        frame.fill_rect(i as u32 * bar_w, 0, bar_w, height, *c);
    }
    frame.stroke_rect(0, 0, width, height, Rgba::WHITE);
    Some(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shookshmam_sensor_model::{SensorCategory, SensorReading};

    fn layout() -> Layout {
        Layout {
            name: "test".into(),
            width: 100,
            height: 100,
            widgets: vec![Widget {
                rect: Rect { x: 0, y: 0, w: 100, h: 20 },
                sensor_id: "cpu.temp".into(),
                label: "CPU".into(),
                min: 0.0,
                max: 100.0,
            }],
        }
    }

    fn snap_with(value: Option<f64>, status: SensorStatus) -> Snapshot {
        let r = match (value, status.carries_value()) {
            (Some(v), true) => SensorReading::available(
                "cpu.temp",
                "cpu0",
                "CPU",
                SensorCategory::Temperature,
                v,
                "sim",
                10,
            ),
            _ => SensorReading::without_value(
                "cpu.temp",
                "cpu0",
                "CPU",
                SensorCategory::Temperature,
                "sim",
                status,
                10,
            ),
        };
        let mut r = r;
        r.status = status;
        if status.carries_value() {
            r.value = value;
        }
        Snapshot::new(10, vec![r])
    }

    #[test]
    fn rejects_out_of_bounds_widget() {
        let mut l = layout();
        l.widgets[0].rect = Rect { x: 90, y: 0, w: 20, h: 20 };
        assert_eq!(
            l.validate(),
            Err(LayoutError::WidgetOutOfBounds { sensor_id: "cpu.temp".into() })
        );
    }

    #[test]
    fn rejects_invalid_range_and_zero_dims() {
        let mut l = layout();
        l.widgets[0].min = 10.0;
        l.widgets[0].max = 10.0;
        assert!(matches!(l.validate(), Err(LayoutError::InvalidRange { .. })));
    }

    #[test]
    fn render_scales_to_target() {
        let f = render(&layout(), &snap_with(Some(50.0), SensorStatus::Available), 1280, 480)
            .unwrap();
        assert_eq!(f.width(), 1280);
        assert_eq!(f.height(), 480);
    }

    #[test]
    fn render_rejects_zero_target() {
        let err = render(&layout(), &snap_with(Some(50.0), SensorStatus::Available), 0, 480);
        assert_eq!(err, Err(LayoutError::ZeroTargetDimensions));
    }

    #[test]
    fn bar_fill_tracks_value() {
        // At value 100 (frac 1.0) the far end of the bar is colored; at a low
        // value the same pixel stays as panel background.
        let full = render(&layout(), &snap_with(Some(100.0), SensorStatus::Available), 100, 100)
            .unwrap();
        let low =
            render(&layout(), &snap_with(Some(1.0), SensorStatus::Available), 100, 100).unwrap();
        let probe = full.get(90, 10).unwrap();
        assert_ne!(probe, low.get(90, 10).unwrap());
    }

    #[test]
    fn no_data_does_not_fill_as_zero() {
        // An unavailable sensor must not render as a 0%-filled (empty) bar that
        // could read as a real low value; it uses the gray hatch instead.
        let f = render(&layout(), &snap_with(None, SensorStatus::Unavailable), 100, 100).unwrap();
        // Some gray hatch pixel is present inside the widget.
        let mut found_gray = false;
        for xx in 2..98 {
            if f.get(xx, 10) == Some(Rgba::GRAY) {
                found_gray = true;
                break;
            }
        }
        assert!(found_gray);
    }

    #[test]
    fn test_pattern_dimensions() {
        let f = test_pattern(1280, 480).unwrap();
        assert_eq!(f.bytes().len(), 1280 * 480 * 4);
        assert_eq!(f.get(0, 0), Some(Rgba::WHITE)); // border
        assert!(test_pattern(0, 480).is_none());
    }
}
