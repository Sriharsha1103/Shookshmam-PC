//! Effect rendering engine: compute per-LED colors for an effect at time `t_ms`.
//! Pure and deterministic, so effects are unit-tested without hardware. The
//! output is adapted to the zone's LED count rather than assuming a universal
//! protocol (PHASE-4-SPEC §5).

use crate::effect::Effect;
use crate::zone::Rgb;
use std::f64::consts::PI;

/// Map speed (1=slowest .. 10=fastest) to an animation period in milliseconds.
fn period_ms(speed: u8) -> u64 {
    let s = speed.clamp(1, 10) as u64;
    4000 - (s - 1) * 360 // 4000ms .. 760ms
}

/// Sample a multi-stop gradient at parameter `u` in `[0,1]`.
fn sample_gradient(colors: &[Rgb], u: f64) -> Rgb {
    if colors.is_empty() {
        return Rgb::BLACK;
    }
    if colors.len() == 1 {
        return colors[0];
    }
    let u = u.clamp(0.0, 1.0);
    let scaled = u * (colors.len() - 1) as f64;
    let i = scaled.floor() as usize;
    if i >= colors.len() - 1 {
        return colors[colors.len() - 1];
    }
    Rgb::lerp(colors[i], colors[i + 1], scaled - i as f64)
}

/// Render `led_count` colors for `effect` at `t_ms`.
pub fn render(effect: &Effect, led_count: u32, t_ms: u64) -> Vec<Rgb> {
    let n = led_count as usize;
    match effect {
        Effect::Off => vec![Rgb::BLACK; n],
        Effect::Static { color } => vec![*color; n],
        Effect::Breathing { color, speed } => {
            let period = period_ms(*speed);
            let phase = (t_ms % period) as f64 / period as f64;
            // Smooth 0 -> 1 -> 0 breathing curve.
            let brightness = 0.5 * (1.0 - (2.0 * PI * phase).cos());
            vec![color.scaled(brightness); n]
        }
        Effect::ColorCycle { speed, brightness } => {
            let period = period_ms(*speed);
            let hue = (t_ms % period) as f64 / period as f64 * 360.0;
            let v = (*brightness as f64 / 100.0).clamp(0.0, 1.0);
            vec![Rgb::from_hsv(hue, 1.0, v); n]
        }
        Effect::Gradient { colors, speed } => {
            let period = period_ms(*speed);
            let offset = (t_ms % period) as f64 / period as f64;
            (0..n)
                .map(|i| {
                    let pos = if n <= 1 { 0.0 } else { i as f64 / (n - 1) as f64 };
                    let u = (pos + offset).rem_euclid(1.0);
                    sample_gradient(colors, u)
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_and_static() {
        assert_eq!(render(&Effect::Off, 3, 0), vec![Rgb::BLACK; 3]);
        let c = Rgb::new(10, 20, 30);
        assert_eq!(render(&Effect::Static { color: c }, 4, 123), vec![c; 4]);
    }

    #[test]
    fn breathing_dims_and_brightens() {
        let e = Effect::Breathing { color: Rgb::new(255, 255, 255), speed: 5 };
        let period = period_ms(5);
        // Phase 0 -> brightness 0 (fully dim).
        assert_eq!(render(&e, 1, 0)[0], Rgb::BLACK);
        // Half period -> brightness 1 (full).
        assert_eq!(render(&e, 1, period / 2)[0], Rgb::new(255, 255, 255));
    }

    #[test]
    fn color_cycle_changes_over_time() {
        let e = Effect::ColorCycle { speed: 5, brightness: 100 };
        let a = render(&e, 1, 0)[0];
        let b = render(&e, 1, period_ms(5) / 3)[0];
        assert_ne!(a, b);
    }

    #[test]
    fn gradient_starts_at_first_stop_and_spans_colors() {
        // Cyclic (scrolling) gradient: at t=0 the first LED is the first stop,
        // and the second color appears along the strip.
        let colors = vec![Rgb::new(0, 0, 0), Rgb::new(255, 0, 0)];
        let out = render(&Effect::Gradient { colors, speed: 1 }, 5, 0);
        assert_eq!(out[0], Rgb::new(0, 0, 0));
        assert!(out.iter().any(|c| c.r > 0), "the red stop should appear on the strip");
    }

    #[test]
    fn gradient_sampling_midpoint() {
        let colors = [Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)];
        assert_eq!(sample_gradient(&colors, 0.5), Rgb::new(128, 128, 128));
    }
}
