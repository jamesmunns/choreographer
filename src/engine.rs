//!
//! This engine is intended to provide behavior over time
//! for one or more RGB LEDs (though could also be useful
//! for any kind of color sequencing).
//!
//! In most cases:
//!
//! * Each LED will get a [`Sequence`] of some max length `N`.
//! * Each [`Sequence`] contains up to `N` [`Action`]s
//! * Each [`Action`] consists of:
//!   * A [`Context`], which contains information like the color,
//!       start time, and duration of an Action
//!   * A [behavior], which is what the LED will do over time
//!   * A [`LoopBehavior`], which describes whether the individual
//!       Action will repeat in some way
//! * The [`Sequence`] also contains a [`LoopBehavior`], which describes
//!       whether the ENTIRE SEQUENCE will repeat in some way
//!
//! A typical usage example would be to:
//!
//! 1. Create an array of [`Sequence`]s, one for each LED
//! 1. Use either the [`ActionBuilder`] or [`script!()`] macro
//!     to define each [`Action`] in each [`Sequence`]
//! 1. Periodically poll each [`Sequence`], using the resulting
//!     color to update the physical LED
//! 1. Modify or replace the [`Sequence`]s as necessary, for example
//!     at a regular interval, or in response to some event, such as
//!     a button press or received packet
//!
//! [`Sequence`]: crate::engine::Sequence
//! [`Action`]: crate::engine::Action
//! [`Context`]: crate::engine::Context
//! [behavior]: crate::behaviors
//! [`script!()`]: crate::script
//! [`ActionBuilder`]: crate::engine::ActionBuilder
//! [`LoopBehavior`]: crate::engine::LoopBehavior

use core::cmp::min;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::behaviors::{Cycler, FadeColor, SeekColor, StayColor};
use crate::LossyIntoF32;
use groundhog::RollingTimer;
use heapless::Vec;
use smart_leds::colors::BLACK;
use smart_leds::RGB8;

/// A sequence of [`Action`]s with maximum length N.
///
/// The total [`Sequence`] also has a [`LoopBehavior`]
/// that describes the looping behavior of the entire
/// sequence (e.g. play the sequence N times, once,
/// forever).
///
/// [`Action`]: Action
/// [`LoopBehavior`]: LoopBehavior)
/// [`Sequence`]: Sequence
#[derive(Clone)]
pub struct Sequence<R, const N: usize> {
    seq: Vec<Action<R>, N>,
    position: usize,
    behavior: LoopBehavior,
    never_run: bool,
}

impl<R, const N: usize> Sequence<R, N> {
    const INIT: Sequence<R, N> = Sequence::new();

    /// Create a new, empty sequence
    pub const fn new() -> Self {
        Self {
            seq: Vec::new(),
            position: 0,
            behavior: LoopBehavior::Nop,
            never_run: true,
        }
    }

    /// Create an array of new, empty sequences.
    ///
    /// This is often useful when creating an array for multiple
    /// LEDs
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

impl<R, const N: usize> Sequence<R, N>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    /// Create a new, empty sequence
    pub fn empty() -> Self {
        Self {
            seq: Vec::new(),
            position: 0,
            behavior: LoopBehavior::OneShot,
            never_run: true,
        }
    }

    /// Clear the current set of actions, leaving the
    /// Sequence empty
    pub fn clear(&mut self) {
        self.seq.clear();
        self.position = 0;
    }

    /// Clear the current set of actions, and fill the sequence with
    /// the given new actions.
    ///
    /// If `actions` is larger than the capacity of `self`, remaining items
    /// will be ignored
    pub fn set(&mut self, actions: &[Action<R>], behavior: LoopBehavior) {
        let amt = min(N, actions.len());
        self.clear();

        self.never_run = true;

        self.seq.extend_from_slice(&actions[..amt]).ok();
        self.behavior = behavior;
    }

    /// Poll the currently active Action, potentially also moving
    /// to the next Action if necessary.
    ///
    /// When any Action is active, an RGB8 will be returned
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

        use LoopBehavior::*;
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

/// A single behavior step
///
/// An Action is a single describable step in a sequence
/// of events.
///
/// * Each [`Action`] consists of:
///   * A [`Context`], which contains information like the color,
///       start time, and duration of an Action
///   * A [behavior], which is what the LED will do over time
///   * A [`LoopBehavior`], which describes whether the individual
///       Action will repeat in some way
///
/// Actions are typically constructed through either the
/// [`ActionBuilder`] or [`script!()`] macro.
///
/// Actions are not usually interacted with directly, but rather
/// as part of a [`Sequence`].
///
/// [`Sequence`]: crate::engine::Sequence
/// [`Action`]: crate::engine::Action
/// [`Context`]: crate::engine::Context
/// [behavior]: crate::behaviors
/// [`script!()`]: crate::script
/// [`ActionBuilder`]: crate::engine::ActionBuilder
/// [`LoopBehavior`]: crate::engine::LoopBehavior
#[derive(Clone)]
pub struct Action<R> {
    action: InnerAction<R>,
    behavior: LoopBehavior,
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
    /// Return an ActionBuilder structure to configure a new
    /// Action
    pub fn build() -> ActionBuilder<R> {
        ActionBuilder::new()
    }

    pub(crate) fn reinit(&mut self, start: R::Tick, end_ph: R::Tick, last_color: RGB8) {
        self.action.reinit(start, end_ph, last_color);

        use LoopBehavior::*;
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

    pub(crate) fn poll(&mut self) -> Option<RGB8> {
        use LoopBehavior::*;

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

/// The behavior-independent information of an [`Action`]
///
/// The Context structure contains information like the color,
/// start time, and duration of an [`Action`].
///
/// It is not usually necessary to interact with a Context directly.
///
/// [`Action`]: crate::engine::Action
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
    pub(crate) fn calc_end(&self) -> R::Tick {
        self.start_tick
            .wrapping_add(self.duration_ms * (R::TICKS_PER_SECOND / 1000))
    }

    pub(crate) fn calc_end_phase(&self) -> R::Tick {
        self.phase_offset_ms.wrapping_add(self.duration_ms)
    }

    pub(crate) fn reinit(&mut self, start: R::Tick, start_ph: R::Tick, last_color: RGB8) {
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
struct InnerAction<R> {
    context: Context<R>,
    kind: InnerActionKind,
}

impl<R> Default for InnerAction<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    fn default() -> Self {
        Self {
            context: Context::default(),
            kind: InnerActionKind::Static(StayColor::new()),
        }
    }
}

impl<R> Deref for InnerAction<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    type Target = Context<R>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<R> DerefMut for InnerAction<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<R> InnerAction<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    pub fn poll(&self) -> Option<RGB8> {
        use InnerActionKind::*;
        match &self.kind {
            Sin(s) => s.poll(&self.context),
            Static(s) => s.poll(&self.context),
            Fade(f) => f.poll(&self.context),
            Seek(s) => s.poll(&self.context),
        }
    }
}

#[derive(Clone)]
enum InnerActionKind {
    Sin(Cycler),
    Static(StayColor),
    Fade(FadeColor),
    Seek(SeekColor),
}

/// A description of the looping behavior of an [`Action`] or [`Sequence`]
///
/// [`Sequence`]: crate::engine::Sequence
/// [`Action`]: crate::engine::Action
#[derive(Clone)]
pub enum LoopBehavior {
    /// Execute this action or sequence exactly once
    OneShot,

    /// Loop this action or sequence endlessly
    LoopForever,

    /// Loop this action or sequence N times
    LoopN {
        /// The current iteration
        current: usize,

        /// The total number of iterations
        cycles: usize,
    },

    /// This action will immediately yield to the next
    Nop,
}

impl Default for LoopBehavior {
    fn default() -> Self {
        LoopBehavior::Nop
    }
}

/// A builder for the [`Action`] structure
///
/// [`Action`]: crate::engine::Action
pub struct ActionBuilder<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    act: Action<R>,
}

// Builder Methods
impl<R> ActionBuilder<R>
where
    R: RollingTimer<Tick = u32> + Default + Clone,
{
    /// Create a new ActionBuilder with default settings
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            act: Action {
                action: InnerAction::default(),
                behavior: LoopBehavior::default(),
            },
        }
    }

    /// Finalize the ActionBuilder into an Action
    #[inline(always)]
    pub fn finish(self) -> Action<R> {
        self.act
    }

    /// Set the LoopBehavior to repeat `ct` times
    #[inline(always)]
    pub fn times(mut self, ct: usize) -> Self {
        self.act.behavior = LoopBehavior::LoopN {
            current: 0,
            cycles: ct,
        };
        self
    }

    /// Set the LoopBehavior to repeat never
    #[inline(always)]
    pub fn once(mut self) -> Self {
        self.act.behavior = LoopBehavior::OneShot;
        self
    }

    /// Set the LoopBehavior to loop forever
    #[inline(always)]
    pub fn forever(mut self) -> Self {
        self.act.behavior = LoopBehavior::LoopForever;
        self
    }

    /// Set the color
    #[inline(always)]
    pub fn color(mut self, color: RGB8) -> Self {
        self.act.action.context.color = color;
        self
    }

    /// Set the duration in milliseconds
    #[inline(always)]
    pub fn for_ms(mut self, duration: R::Tick) -> Self {
        self.act.action.context.duration_ms = duration;

        // TODO: This might be better to remove later? Probably
        // conside how to handle these "hacks", or abstract over
        // the cycler type more reasonably
        if let InnerActionKind::Fade(_) = self.act.action.kind {
            self.act.action.context.period_ms = duration.lossy_into() * 4.0;
        }
        self
    }

    /// Set the phase offset behavior
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

    /// Set the duration and period in one step
    #[inline(always)]
    pub fn dur_per_ms(mut self, duration: R::Tick, period_ms: f32) -> Self {
        self.act.action.context.duration_ms = duration;

        // TODO: fix hax?
        self.act.action.context.period_ms = match self.act.action.kind {
            InnerActionKind::Sin(_) => period_ms * 2.0,
            InnerActionKind::Static(_) => period_ms,
            InnerActionKind::Fade(_) => duration.lossy_into() * 4.0,
            InnerActionKind::Seek(_) => period_ms,
        };

        self
    }

    /// Set the period, in milliseconds (as an f32)
    #[inline(always)]
    pub fn period_ms(mut self, duration: f32) -> Self {
        self.act.action.context.period_ms = duration;
        self
    }

    /// Convert the current ActionBuilder to produce a Sine Cycler
    #[inline(always)]
    pub fn sin(mut self) -> Self {
        let mut sin = Cycler::new();
        sin.start_low();
        self.act.action.kind = InnerActionKind::Sin(sin);
        self
    }

    /// Convert the current ActionBuilder to produce a SeekColor behavior
    #[inline(always)]
    pub fn seek(mut self) -> Self {
        self.act.action.kind = InnerActionKind::Seek(SeekColor);
        self
    }

    /// Convert the current ActionBuilder to produce a Cosine Cycler
    #[inline(always)]
    pub fn cos(mut self) -> Self {
        let mut cos = Cycler::new();
        cos.start_high();
        self.act.action.kind = InnerActionKind::Sin(cos);
        self
    }

    /// Convert the current ActionBuilder to produce a StayColor action
    #[inline(always)]
    pub fn solid(mut self) -> Self {
        self.act.action.kind = InnerActionKind::Static(StayColor::new());
        self
    }

    /// Convert the current ActionBuilder to produce a Fade Up action
    #[inline(always)]
    pub fn fade_up(mut self) -> Self {
        self.act.action.kind =
            InnerActionKind::Fade(FadeColor::new_fade_up(&mut self.act.action.context));
        self
    }

    /// Convert the current ActionBuilder to produce a Fade Down action
    #[inline(always)]
    pub fn fade_down(mut self) -> Self {
        self.act.action.kind =
            InnerActionKind::Fade(FadeColor::new_fade_down(&mut self.act.action.context));
        self
    }
}

/// A description of Phase Increment Behavior
pub enum PhaseIncr {
    /// A specific phase increment, in milliseconds
    Millis(u32),

    /// Increment the phase from the last elapsed phase
    /// on every action transition
    AutoIncr,

    /// Increment the phase from the last elapsed phase
    /// on the first action transition
    AutoIncrOnStart,
}

impl From<u32> for PhaseIncr {
    fn from(data: u32) -> Self {
        PhaseIncr::Millis(data)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AutoIncr {
    Never,
    Once,
    Forever,
}

impl Default for AutoIncr {
    fn default() -> Self {
        AutoIncr::Never
    }
}
