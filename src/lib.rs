//! # Choreographer
//!
//! A color pattern sequencer, intended for groups of RGB LEDs
//!
//! ## Example
//!
//! Check out the video [in this tweet](https://twitter.com/bitshiftmask/status/1404633529179377673)
//!
//! ## License
//!
//! This project is licensed under the [Mozilla Public License v2.0](https://www.mozilla.org/en-US/MPL/2.0/).


#![cfg_attr(not(test), no_std)]
// #![deny(missing_docs)]

pub mod behaviors;
pub mod engine;
pub use smart_leds::colors;

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

#[macro_export]
macro_rules! script {
    (| action | color | (duration_ms) | (period_ms_f) | (phase_offset_ms) | repeat | $(| $action:ident | $color:ident | ($duration_ms:expr) | ($period_ms_f:expr) | ($phase_offset_ms:expr) | $repeat:ident |)+) => {
        {
            #[allow(unused_imports)]
            use $crate::reexports::colors::*;
            use $crate::engine::PhaseIncr::*;
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
            use $crate::reexports::colors::*;
            use $crate::engine::PhaseIncr::*;
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
