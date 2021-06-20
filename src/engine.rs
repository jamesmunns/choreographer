use core::cmp::min;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::behaviors::AutoIncr;
use crate::behaviors::{Cycler, FadeColor, SeekColor, StayColor};
use crate::LossyIntoF32;
use groundhog::RollingTimer;
use heapless::Vec;
use smart_leds::colors::BLACK;
use smart_leds::RGB8;

#[derive(Clone)]
pub struct Sequence<R, const N: usize> {
    seq: Vec<Action<R>, N>,
    position: usize,
    behavior: Behavior,
    never_run: bool,
}

impl<R, const N: usize> Sequence<R, N> {
    const INIT: Sequence<R, N> = Sequence::new();

    pub const fn new() -> Self {
        Self {
            seq: Vec::new(),
            position: 0,
            behavior: Behavior::Nop,
            never_run: true,
        }
    }

    pub const fn new_array<const M: usize>() -> [Self; M] {
        [Self::INIT; M]
    }
}

impl<R, const N: usize> Default for Sequence<R, N>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Action<R> {
    action: Actions<R>,
    behavior: Behavior,
}

#[derive(Clone, Default)]
pub struct Context<R> {
    pub(crate) start_tick: u32, // TODO: Hack - Not R::Tick because const init
    pub(crate) auto_incr_phase: AutoIncr,
    pub(crate) period_ms: f32,
    pub(crate) duration_ms: u32, // TODO: Hack - Not R::Tick because const init
    pub(crate) phase_offset_ms: u32, // TODO: Hack - Not R::Tick because const init
    pub(crate) last_color: RGB8,
    pub(crate) color: RGB8,
    _pd: PhantomData<R>,
}

impl<R> Context<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    pub fn calc_end(&self) -> R::Tick {
        self.start_tick
            .wrapping_add(self.duration_ms * (R::TICKS_PER_SECOND / 1000))
    }

    pub fn calc_end_phase(&self) -> R::Tick {
        self.phase_offset_ms.wrapping_add(self.duration_ms)
    }

    pub fn reinit(&mut self, start: R::Tick, start_ph: R::Tick, last_color: RGB8) {
        self.start_tick = start;
        self.last_color = last_color;
        match self.auto_incr_phase {
            AutoIncr::Never => {}
            AutoIncr::Once => {
                self.phase_offset_ms = start_ph;
                self.auto_incr_phase = AutoIncr::Never;
            }
            AutoIncr::Forever => {
                self.phase_offset_ms = start_ph;
            }
        }
    }
}

#[derive(Clone)]
pub struct Actions<R> {
    context: Context<R>,
    kind: ActionsKind,
}

#[derive(Clone)]
pub enum ActionsKind {
    Sin(Cycler),
    Static(StayColor),
    Fade(FadeColor),
    Seek(SeekColor),
}

#[derive(Clone)]
pub enum Behavior {
    OneShot,
    LoopForever,

    #[allow(dead_code)]
    LoopN {
        current: usize,
        cycles: usize,
    },
    Nop,
}

pub struct ActionBuilder<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    act: Action<R>,
}

impl Default for Behavior {
    fn default() -> Self {
        Behavior::Nop
    }
}

impl<R> Default for Actions<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    fn default() -> Self {
        Self {
            context: Context::default(),
            kind: ActionsKind::Static(StayColor::new()),
        }
    }
}

impl<R, const N: usize> Sequence<R, N>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    pub fn empty() -> Self {
        Self {
            seq: Vec::new(),
            position: 0,
            behavior: Behavior::OneShot,
            never_run: true,
        }
    }

    pub fn clear(&mut self) {
        self.seq.clear();
        self.position = 0;
    }

    pub fn set(&mut self, actions: &[Action<R>], behavior: Behavior) {
        let amt = min(N, actions.len());
        self.clear();

        self.never_run = true;

        self.seq.extend_from_slice(&actions[..amt]).ok();
        self.behavior = behavior;
    }

    pub fn poll(&mut self) -> Option<RGB8> {
        if self.seq.is_empty() || (self.position >= self.seq.len()) {
            return None;
        }

        let behavior = &mut self.behavior;
        let seq = &mut self.seq;
        let position = &mut self.position;

        // If we are running this sequence for the first time,
        // re-initialize to ensure time is current
        if self.never_run {
            let ph = seq[*position].action.context.phase_offset_ms;
            let timer = R::default();
            seq[*position].reinit(timer.get_ticks(), ph, BLACK);
            self.never_run = false;
        }

        use Behavior::*;
        match behavior {
            OneShot => seq[*position].poll().or_else(|| {
                let end = seq[*position].calc_end();
                let end_ph = seq[*position].calc_end_phase();
                let last_color = seq[*position].color;
                *position += 1;
                if *position < seq.len() {
                    seq[*position].reinit(end, end_ph, last_color);
                    seq[*position].poll()
                } else {
                    None
                }
            }),
            LoopForever => seq[*position].poll().or_else(|| {
                let end = seq[*position].calc_end();
                let end_ph = seq[*position].calc_end_phase();
                let last_color = seq[*position].color;
                *position += 1;

                if *position >= seq.len() {
                    *position = 0;
                }

                seq[*position].reinit(end, end_ph, last_color);
                seq[*position].poll()
            }),
            LoopN {
                ref mut current,
                cycles,
            } => seq[*position].poll().or_else(|| {
                let end = seq[*position].calc_end();
                let end_ph = seq[*position].calc_end_phase();
                let last_color = seq[*position].color;
                *position += 1;

                if *position >= seq.len() {
                    if *current < *cycles {
                        *position = 0;
                        *current += 1;
                        seq[*position].reinit(end, end_ph, last_color);
                        seq[*position].poll()
                    } else {
                        None
                    }
                } else {
                    seq[*position].reinit(end, end_ph, last_color);
                    seq[*position].poll()
                }
            }),
            Nop => None,
        }
    }
}

impl<R> Deref for Action<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    type Target = Context<R>;

    fn deref(&self) -> &Self::Target {
        &self.action.context
    }
}

impl<R> Action<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    pub fn new(action: Actions<R>, behavior: Behavior) -> Self {
        Self { action, behavior }
    }

    pub fn build() -> ActionBuilder<R> {
        ActionBuilder::new()
    }

    pub fn reinit(&mut self, start: R::Tick, end_ph: R::Tick, last_color: RGB8) {
        self.action.reinit(start, end_ph, last_color);

        use Behavior::*;
        match &mut self.behavior {
            OneShot => {}
            LoopForever => {}
            Nop => {}
            LoopN {
                ref mut current, ..
            } => {
                *current = 0;
            }
        }
    }

    pub fn poll(&mut self) -> Option<RGB8> {
        use Behavior::*;

        let action = &mut self.action;
        let behavior = &mut self.behavior;

        match behavior {
            OneShot => action.poll(),
            LoopForever => action.poll().or_else(|| {
                let end = action.calc_end();
                let end_ph = action.calc_end_phase();
                let last_color = action.context.color;
                action.reinit(end, end_ph, last_color);
                action.poll()
            }),
            LoopN {
                ref mut current,
                cycles,
            } => action.poll().or_else(|| {
                if *current < *cycles {
                    *current += 1;
                    // TODO: Reinit as above?
                    action.poll()
                } else {
                    None
                }
            }),
            Nop => None,
        }
    }
}

pub enum PhaseIncr {
    Millis(u32),
    AutoIncr,
    AutoIncrOnStart,
}

impl From<u32> for PhaseIncr {
    fn from(data: u32) -> Self {
        PhaseIncr::Millis(data)
    }
}

// Builder Methods
impl<R> ActionBuilder<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            act: Action {
                action: Actions::default(),
                behavior: Behavior::default(),
            },
        }
    }

    #[inline(always)]
    pub fn finish(self) -> Action<R> {
        self.act
    }

    #[inline(always)]
    pub fn times(mut self, ct: usize) -> Self {
        self.act.behavior = Behavior::LoopN {
            current: 0,
            cycles: ct,
        };
        self
    }

    #[inline(always)]
    pub fn once(mut self) -> Self {
        self.act.behavior = Behavior::OneShot;
        self
    }

    #[inline(always)]
    pub fn forever(mut self) -> Self {
        self.act.behavior = Behavior::LoopForever;
        self
    }

    #[inline(always)]
    pub fn color(mut self, color: RGB8) -> Self {
        self.act.action.context.color = color;
        self
    }

    #[inline(always)]
    pub fn for_ms(mut self, duration: R::Tick) -> Self {
        self.act.action.context.duration_ms = duration;

        // TODO: This might be better to remove later? Probably
        // conside how to handle these "hacks", or abstract over
        // the cycler type more reasonably
        if let ActionsKind::Fade(_) = self.act.action.kind {
            self.act.action.context.period_ms = duration.lossy_into() * 4.0;
        }
        self
    }

    #[inline(always)]
    pub fn phase_offset_ms(mut self, phase_offset_ms: PhaseIncr) -> Self {
        let (phase_offset_ms, incr) = match phase_offset_ms {
            PhaseIncr::Millis(ms) => (ms, AutoIncr::Never),
            PhaseIncr::AutoIncr => (0, AutoIncr::Forever),
            PhaseIncr::AutoIncrOnStart => (0, AutoIncr::Once),
        };
        self.act.action.context.phase_offset_ms = phase_offset_ms;
        self.act.action.context.auto_incr_phase = incr;
        self
    }

    #[inline(always)]
    pub fn dur_per_ms(mut self, duration: R::Tick, period_ms: f32) -> Self {
        self.act.action.context.duration_ms = duration;

        // TODO: fix hax?
        self.act.action.context.period_ms = match self.act.action.kind {
            ActionsKind::Sin(_) => period_ms * 2.0,
            ActionsKind::Static(_) => period_ms,
            ActionsKind::Fade(_) => duration.lossy_into() * 4.0,
            ActionsKind::Seek(_) => period_ms,
        };

        self
    }

    #[inline(always)]
    pub fn period_ms(mut self, duration: f32) -> Self {
        self.act.action.context.period_ms = duration;
        self
    }

    #[inline(always)]
    pub fn sin(mut self) -> Self {
        let mut sin = Cycler::new();
        sin.start_low();
        self.act.action.kind = ActionsKind::Sin(sin);
        self
    }

    #[inline(always)]
    pub fn seek(mut self) -> Self {
        self.act.action.kind = ActionsKind::Seek(SeekColor);
        self
    }

    #[inline(always)]
    pub fn cos(mut self) -> Self {
        let mut cos = Cycler::new();
        cos.start_high();
        self.act.action.kind = ActionsKind::Sin(cos);
        self
    }

    #[inline(always)]
    pub fn solid(mut self) -> Self {
        self.act.action.kind = ActionsKind::Static(StayColor::new());
        self
    }

    #[inline(always)]
    pub fn fade_up(mut self) -> Self {
        self.act.action.kind =
            ActionsKind::Fade(FadeColor::new_fade_up(&mut self.act.action.context));
        self
    }

    #[inline(always)]
    pub fn fade_down(mut self) -> Self {
        self.act.action.kind =
            ActionsKind::Fade(FadeColor::new_fade_down(&mut self.act.action.context));
        self
    }
}

impl<R> Deref for Actions<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    type Target = Context<R>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<R> DerefMut for Actions<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<R> Actions<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    pub fn poll(&self) -> Option<RGB8> {
        use ActionsKind::*;
        match &self.kind {
            Sin(s) => s.poll(&self.context),
            Static(s) => s.poll(&self.context),
            Fade(f) => f.poll(&self.context),
            Seek(s) => s.poll(&self.context),
        }
    }
}
