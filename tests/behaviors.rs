macro_rules! timer_factory {
    () => {
        static TIMER: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

        #[derive(Clone, Default)]
        struct TestTimer;

        impl TestTimer {
            fn new() -> Self {
                TestTimer
            }

            fn set_ms(ms: u32) {
                TIMER.store(ms, core::sync::atomic::Ordering::SeqCst);
            }

            fn increment_ms(ms: u32) {
                TIMER.fetch_add(ms, core::sync::atomic::Ordering::SeqCst);
            }
        }

        impl groundhog::RollingTimer for TestTimer {
            type Tick = u32;

            const TICKS_PER_SECOND: u32 = 1000;

            fn get_ticks(&self) -> Self::Tick {
                TIMER.load(core::sync::atomic::Ordering::SeqCst)
            }

            fn is_initialized(&self) -> bool {
                true
            }
        }
    };
}

use choreographer::{
    engine::{LoopBehavior, Sequence},
    script,
    colors,
};
use groundhog::RollingTimer;

#[test]
fn smoke() {
    timer_factory!();

    TestTimer::set_ms(0);

    // Create a script for a single LED with up to
    // eight different steps in the sequence
    let mut script: Sequence<TestTimer, 8> = Sequence::empty();

    // This script will:
    // * Keep the LED black for 1s
    // * Fade from black to white and back in a sine pattern over 2.5s
    // * Remain at black for 1s
    // * End the sequence
    script.set(&script! {
        | action |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
        |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
        |    sin |  WHITE |        2500 |      2500.0 |               0 |   once |
        |  solid |  BLACK |        1000 |         0.0 |               0 |   once |
    }, LoopBehavior::OneShot);

    let timer = TestTimer::new();

    // Stay off for the first 1000ms...
    while timer.get_ticks() <= 1000 {
        assert_eq!(Some(colors::BLACK), script.poll());
        TestTimer::increment_ms(10);
    }

    let mut last = colors::BLACK;

    // Then monotonically increase for 1/2 of 2500...
    while timer.get_ticks() <= 2250 {
        let color = script.poll().unwrap();
        assert!(color.r >= last.r);
        assert!(color.g >= last.g);
        assert!(color.b >= last.b);
        last = color;

        TestTimer::increment_ms(10);
    }

    // Then monotonically decrease for the second 1/2 of 2500...
    while timer.get_ticks() <= 3500 {
        let color = script.poll().unwrap();
        println!("last: {:?}, color: {:?}", last, color);
        assert!(color.r <= last.r);
        assert!(color.g <= last.g);
        assert!(color.b <= last.b);
        last = color;

        TestTimer::increment_ms(10);
    }

    // Then stay off for 1000ms
    while timer.get_ticks() < 4500 {
        assert_eq!(Some(colors::BLACK), script.poll());
        TestTimer::increment_ms(10);
    }

    // At 4500, we're done!
    assert_eq!(timer.get_ticks(), 4500);
    assert!(script.poll().is_none());
}

#[test]
fn fade_up() {
    timer_factory!();

    TestTimer::set_ms(0);

    // Create a script for a single LED with up to
    // eight different steps in the sequence
    let mut script: Sequence<TestTimer, 8> = Sequence::empty();

    // This script will:
    // * Keep the LED black for 1s
    // * Fade from black to white and back in a sine pattern over 2.5s
    // * Remain at black for 1s
    // * End the sequence
    script.set(&script! {
        | action  |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
        | fade_up |  WHITE |        1000 |         0.0 |               0 |   once |
    }, LoopBehavior::OneShot);

    let timer = TestTimer::new();

    let mut last = script.poll().unwrap();
    assert_eq!(last, colors::BLACK);

    while timer.get_ticks() < 1000 {
        let color = script.poll().unwrap();
        assert!(color.r >= last.r);
        assert!(color.g >= last.g);
        assert!(color.b >= last.b);
        last = color;

        TestTimer::increment_ms(10);
    }

    assert_eq!(last, colors::WHITE);

    assert_eq!(timer.get_ticks(), 1000);
    assert!(script.poll().is_none());
}

#[test]
fn fade_down() {
    timer_factory!();

    TestTimer::set_ms(0);

    // Create a script for a single LED with up to
    // eight different steps in the sequence
    let mut script: Sequence<TestTimer, 8> = Sequence::empty();

    // This script will:
    // * Keep the LED black for 1s
    // * Fade from black to white and back in a sine pattern over 2.5s
    // * Remain at black for 1s
    // * End the sequence
    script.set(&script! {
        | action    |  color | duration_ms | period_ms_f | phase_offset_ms | repeat |
        | solid     |  WHITE |          10 |         0.0 |               0 |   once |
        | fade_down |  WHITE |        1000 |         0.0 |               0 |   once |
    }, LoopBehavior::OneShot);

    let timer = TestTimer::new();

    let mut last = script.poll().unwrap();
    assert_eq!(last, colors::WHITE);
    TestTimer::increment_ms(10);

    while timer.get_ticks() < 1010 {
        let color = script.poll().unwrap();
        println!("last: {:?}, color: {:?}", last, color);
        // assert!(color.r <= last.r);
        // assert!(color.g <= last.g);
        // assert!(color.b <= last.b);
        last = color;

        TestTimer::increment_ms(10);
    }

    assert_eq!(last, colors::BLACK);
    assert_eq!(timer.get_ticks(), 1010);
    assert!(script.poll().is_none());

    panic!()
}
