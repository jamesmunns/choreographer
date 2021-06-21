//! # Choreographer
//!
//! A color pattern sequencer, intended for groups of RGB LEDs
//!
//! ## Example
//!
//! Check out the video [in this tweet](https://twitter.com/bitshiftmask/status/1404633529179377673)
//!
//! ```rust
//! # fn set_led(_: choreographer::RGB8) { }
//! #
//! use choreographer::{
//!     engine::{LoopBehavior, Sequence},
//!     script,
//! };
//! use groundhog::std_timer::Timer;
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! // Timer with 1us ticks
//! type MicroTimer = Timer<1_000_000>;
//!
//! // Create a script for a single LED with up to
//! // eight different steps in the sequence
//! let mut script: Sequence<MicroTimer, 8> = Sequence::empty();
//!
//! // This script will:
//! // * Keep the LED black for 1s
//! // * Fade from black to white and back in a sine pattern over 2.5s
//! // * Remain at black for 1s
//! // * End the sequence
//! script.set(&script! {
//!     | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
//!     |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
//!     |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
//!     |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
//! }, LoopBehavior::OneShot);
//!
//! // Poll the script and update the LED until the
//! // script has completed (4.5s or so)
//! while let Some(color) = script.poll() {
//!     println!("Color: {:?}", color);
//!     set_led(color);
//!     sleep(Duration::from_millis(10));
//! }
//!
//! // Now we could leave the LED off, or set a
//! // new sequence on some event!
//! ```
//!
//! ## License
//!
//! This project is licensed under the [Mozilla Public License v2.0](https://www.mozilla.org/en-US/MPL/2.0/).

#![cfg_attr(not(test), no_std)]
#![deny(missing_docs)]

/// Individual color behavior steps
pub mod behaviors;

/// The choreographer sequencing engine
pub mod engine;

/// The color types from the [`smart-leds`](https://docs.rs/smart-leds) crate
pub use smart_leds::colors;

/// The RGB8 type from the [`smart-leds`](https://docs.rs/smart-leds) crate
pub use smart_leds::RGB8;

/// A trait to convert integers into `f32`s
///
/// This conversion may be lossy, but we're not *too* worried
/// about precision here.
pub trait LossyIntoF32 {
    /// Convert a number into a float, possibly losing precision
    /// or accumulating error
    fn lossy_into(&self) -> f32;
}

impl LossyIntoF32 for u64 {
    fn lossy_into(&self) -> f32 {
        // oops
        *self as f32
    }
}

impl LossyIntoF32 for u32 {
    fn lossy_into(&self) -> f32 {
        // oops
        *self as f32
    }
}

impl LossyIntoF32 for u16 {
    fn lossy_into(&self) -> f32 {
        (*self).into()
    }
}

impl LossyIntoF32 for u8 {
    fn lossy_into(&self) -> f32 {
        (*self).into()
    }
}

/// The `script!()` macro for defining [`Action`]s for a [`Sequence`]
///
/// [`Action`]: crate::engine::Action
/// [`Sequence`]: crate::engine::Sequence
#[macro_export]
macro_rules! script {
    (| action | color | (duration_ms) | (period_ms_f) | (phase_offset_ms) | repeat | $(| $action:ident | $color:ident | ($duration_ms:expr) | ($period_ms_f:expr) | ($phase_offset_ms:expr) | $repeat:ident |)+) => {
        {
            #[allow(unused_imports)]
            use $crate::{
                colors::*,
                engine::PhaseIncr::*,
            };
            [
                $(
                    $crate::engine::Action::build()
                        .$action()
                        .color($color)
                        .for_ms($duration_ms)
                        .period_ms($period_ms_f)
                        .phase_offset_ms($phase_offset_ms.into())
                        .$repeat()
                        .finish(),
                )+
            ]
        }
    };
    (| action | color | duration_ms | period_ms_f | phase_offset_ms | repeat | $(| $action:ident | $color:ident | $duration_ms:literal | $period_ms_f:literal | $phase_offset_ms:literal | $repeat:ident |)+) => {
        {
            #[allow(unused_imports)]
            use $crate::{
                colors::*,
                engine::PhaseIncr::*,
            };
            [
                $(
                    $crate::engine::Action::build()
                        .$action()
                        .color($color)
                        .for_ms($duration_ms)
                        .period_ms($period_ms_f)
                        .phase_offset_ms($phase_offset_ms.into())
                        .$repeat()
                        .finish(),
                )+
            ]
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        engine::{LoopBehavior, Sequence},
        script,
    };
    use groundhog::std_timer::Timer;
    use std::thread::sleep;
    use std::time::Duration;

    // Timer with 1us ticks
    type MicroTimer = Timer<1_000_000>;

    #[test]
    fn foo() {
        // Create a script for a single LED with up to
        // eight different steps in the sequence
        let mut script: Sequence<MicroTimer, 8> = Sequence::empty();

        // This script will:
        // * Keep the LED black for 1s
        // * Fade from black to white and back in a sine pattern over 2.5s
        // * Remain at black for 1s
        // * End the sequence
        script.set(
            &script! {
                | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
                |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
                |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
                |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
            },
            LoopBehavior::OneShot,
        );

        // Poll the script and update the LED until the
        // script has completed (4.5s or so)
        while let Some(_color) = script.poll() {
            sleep(Duration::from_millis(10));
        }

        // Now we could leave the LED off, or set a
        // new sequence on some event!
    }
}
