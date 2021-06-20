#![cfg_attr(not(test), no_std)]

pub mod behaviors;
pub mod engine;

pub trait LossyIntoF32 {
    fn lossy_into(&self) -> f32;
}

pub mod reexports {
    pub use smart_leds::colors;
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
