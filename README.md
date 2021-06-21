# Choreographer

A color pattern sequencer, intended for groups of RGB LEDs

## Example

Check out the video [in this tweet](https://twitter.com/bitshiftmask/status/1404633529179377673)

```rust
use choreographer::{
    engine::{Behavior, Sequence},
    script,
};
use groundhog::std_timer::Timer;
use std::thread::sleep;
use std::time::Duration;

// Timer with 1us ticks
type MicroTimer = Timer<1_000_000>;

// Create a script for a single LED with up to
// eight different steps in the sequence
let mut script: Sequence<MicroTimer, 8> = Sequence::empty();

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
}, Behavior::OneShot);

// Poll the script and update the LED until the
// script has completed (4.5s or so)
while let Some(color) = script.poll() {
    println!("Color: {:?}", color);
    set_led(color);
    sleep(Duration::from_millis(10));
}

// Now we could leave the LED off, or set a
// new sequence on some event!
```

## License

This project is licensed under the [Mozilla Public License v2.0](https://www.mozilla.org/en-US/MPL/2.0/).
