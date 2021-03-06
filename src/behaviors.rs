//!
//! Behaviors are the smallest unit of action that
//! can be choreographed. They describe a pattern
//! that is polled for some amount of time before
//! moving to the next step.
//!
//! Note that all Behaviors are "color agnostic",
//! in that they do not contain any information
//! including color, duration, etc.
//!
//! This information is instead stored in the
//! [`Context`] structure, and passed in on each
//! poll event.
//!
//! [`Context`]: crate::engine::Context

use crate::engine::Context;
use crate::LossyIntoF32;
use groundhog::RollingTimer;
use micromath::F32Ext;
use smart_leds::RGB8;

/// StayColor - A solid constant color
///
/// This is the simplest behavior
#[derive(Clone, Debug, Default)]
pub struct StayColor;

impl StayColor {
    /// Create a new StayColor instance
    pub fn new() -> Self {
        StayColor
    }

    pub(crate) fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        let timer = R::default();
        if timer.millis_since(context.start_tick) >= context.duration_ms {
            None
        } else {
            Some(context.color)
        }
    }
}

/// Cycler - A sine/cosine wave oscillator
///
/// A cycler can either "start low" as a sine wave, or
/// "start high", as a cosine wave.
#[derive(Clone)]
pub struct Cycler {
    func: fn(f32) -> f32,
}

impl Cycler {
    /// Create a new cycler starting low, using a sine function
    pub fn new() -> Self {
        Self {
            func: <f32 as F32Ext>::sin,
        }
    }

    pub(crate) fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        let timer = R::default();
        let delta = timer.millis_since(context.start_tick);

        if delta >= context.duration_ms {
            return None;
        }

        // Since we "rectify" the sine wave, it actually has a period that
        // looks half as long.
        let period = context.period_ms * 2.0;

        let deltaf = delta.wrapping_add(context.phase_offset_ms).lossy_into();
        let normalized = deltaf / period;
        let rad_norm = normalized * 2.0 * core::f32::consts::PI;
        let out_norm = (self.func)(rad_norm);
        let abs_out = out_norm.abs();

        let retval = RGB8 {
            r: (abs_out * (context.color.r as f32)) as u8,
            g: (abs_out * (context.color.g as f32)) as u8,
            b: (abs_out * (context.color.b as f32)) as u8,
        };

        Some(retval)
    }

    /// Start the Cycler high, e.g. using a cosine function
    pub fn start_high(&mut self) {
        self.func = <f32 as F32Ext>::cos
    }

    /// Start the Cycler low, e.g. using a sine function
    pub fn start_low(&mut self) {
        self.func = <f32 as F32Ext>::sin
    }
}

/// SeekColor - Linearly fade from the last color to a new color
///
/// This behavior linearly fades all r/g/b channels from the
/// previous color to the new color
#[derive(Clone)]
pub struct SeekColor;

impl SeekColor {
    /// Create a new SeekColor
    pub fn new() -> Self {
        Self
    }

    pub(crate) fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        let timer = R::default();
        let delta = timer.millis_since(context.start_tick);

        if delta >= context.duration_ms {
            return None;
        }

        let delta_r = ((context.color.r as i16) - (context.last_color.r as i16)) as f32;
        let delta_g = ((context.color.g as i16) - (context.last_color.g as i16)) as f32;
        let delta_b = ((context.color.b as i16) - (context.last_color.b as i16)) as f32;
        let norm_dt = (delta as f32) / (context.duration_ms as f32);
        let norm_r = ((context.last_color.r as i16) + ((delta_r * norm_dt) as i16)) as u8;
        let norm_g = ((context.last_color.g as i16) + ((delta_g * norm_dt) as i16)) as u8;
        let norm_b = ((context.last_color.b as i16) + ((delta_b * norm_dt) as i16)) as u8;

        Some(RGB8 {
            r: norm_r,
            g: norm_g,
            b: norm_b,
        })
    }
}

/// FadeColor - Fade Up to a color or Fade down from a color to black
///
/// FadeColor is similar to a [`Cycler`](Cycler), but is intended for
/// cases when you don't want a repeating sinusoid, but rather just
/// want to "fade in" or "fade out" then hold a color.
#[derive(Clone)]
pub struct FadeColor {
    cycler: Cycler,
}

impl FadeColor {
    /// Create a new FadeColor, fading up to a color from black
    pub fn new_fade_up<R>(context: &mut Context<R>) -> Self
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        let mut cycler = Cycler::new();
        cycler.start_low();

        // TODO: This might be better to remove later? Probably
        // conside how to handle these "hacks", or abstract over
        // the cycler type more reasonably
        context.period_ms = context.duration_ms.lossy_into() * 2.0;

        Self { cycler }
    }

    /// Create a new FadeColor, fading down from a color to black
    pub fn new_fade_down<R>(context: &mut Context<R>) -> Self
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        let mut cycler = Cycler::new();
        cycler.start_high();

        // TODO: This might be better to remove later? Probably
        // conside how to handle these "hacks", or abstract over
        // the cycler type more reasonably
        context.period_ms = context.duration_ms.lossy_into() * 2.0;

        Self { cycler }
    }

    pub(crate) fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        self.cycler.poll(context)
    }
}
