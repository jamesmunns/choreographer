use crate::engine::Context;
use crate::LossyIntoF32;
use groundhog::RollingTimer;
use micromath::F32Ext;
use smart_leds::RGB8;

#[derive(Clone, Debug, Default)]
pub struct StayColor;

impl StayColor {
    pub fn new() -> Self {
        StayColor
    }

    pub fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
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

#[derive(Debug, Clone, Copy)]
pub enum AutoIncr {
    Never,
    Once,
    Forever,
}

impl Default for AutoIncr {
    fn default() -> Self {
        AutoIncr::Never
    }
}

#[derive(Clone)]
pub struct Cycler {
    func: fn(f32) -> f32,
}

// Methods:
//
// reinit(): reinitialize with the current time
// poll() -> Option<RGB8>: Some if updated color, None if action is complete

impl Cycler {
    pub fn new() -> Self {
        Self { func: <f32 as F32Ext>::sin }
    }

    pub fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
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

    pub fn start_high(&mut self) {
        self.func = <f32 as F32Ext>::cos
    }

    pub fn start_low(&mut self) {
        self.func = <f32 as F32Ext>::sin
    }
}


#[derive(Clone)]
pub struct SeekColor;

// Methods:
//
// reinit(): reinitialize with the current time
// poll() -> Option<RGB8>: Some if updated color, None if action is complete

impl SeekColor {
    pub fn new() -> Self {
        Self
    }

    pub fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
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


        Some(RGB8 { r: norm_r, g: norm_g, b: norm_b })
    }
}

#[derive(Clone)]
pub struct FadeColor {
    pub cycler: Cycler,
}

impl FadeColor {
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

    pub fn poll<R>(&self, context: &Context<R>) -> Option<RGB8>
    where
        R: RollingTimer<Tick = u32> + Default + Clone,
    {
        self.cycler.poll(context)
    }

    pub fn inner_mut(&mut self) -> &mut Cycler {
        &mut self.cycler
    }
}
