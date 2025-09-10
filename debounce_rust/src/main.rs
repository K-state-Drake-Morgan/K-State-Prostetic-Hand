#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use arduino_hal::{hal::port::Pin, hal::port::mode::Input, hal::port::mode::PullUp};
use panic_halt as _;

const CHANGE_STATE_INTERVAL: u32 = 250;

#[derive(Copy, Clone)]
enum DebounceState {
    Low,
    PossibleHigh { start_time: u32 },
    High,
    PossibleLow { start_time: u32 },
}

struct Debouncer {
    state: DebounceState,
    pin: Pin<Input<PullUp>>,
}

impl Debouncer {
    fn new(pin: Pin<Input<PullUp>>) -> Self {
        Self {
            state: DebounceState::Low,
            pin,
        }
    }

    fn is_high(&mut self, current_time: u32) -> bool {
        let input_high = self.pin.is_high();

        match self.state {
            DebounceState::Low => {
                if input_high {
                    self.state = DebounceState::PossibleHigh {
                        start_time: current_time,
                    };
                }
                false
            }
            DebounceState::PossibleHigh { start_time } => {
                if input_high && current_time.wrapping_sub(start_time) >= CHANGE_STATE_INTERVAL {
                    self.state = DebounceState::High;
                    true
                } else if !input_high {
                    self.state = DebounceState::Low;
                    false
                } else {
                    false
                }
            }
            DebounceState::High => {
                if !input_high {
                    self.state = DebounceState::PossibleLow {
                        start_time: current_time,
                    };
                }
                true
            }
            DebounceState::PossibleLow { start_time } => {
                if !input_high && current_time.wrapping_sub(start_time) >= CHANGE_STATE_INTERVAL {
                    self.state = DebounceState::Low;
                    false
                } else if input_high {
                    self.state = DebounceState::High;
                    true
                } else {
                    true
                }
            }
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let _reference_pin = pins.d0.into_output();
    let bend_pin = pins.d1.into_pull_up_input();
    let unbend_pin = pins.d2.into_pull_up_input();

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut bend_debouncer = Debouncer::new(bend_pin);
    let mut unbend_debouncer = Debouncer::new(unbend_pin);

    loop {
        let now = arduino_hal::millis();

        let finger_should_bend = bend_debouncer.is_high(now);
        let finger_should_extend = unbend_debouncer.is_high(now);

        if finger_should_bend {
            ufmt::uwriteln!(&mut serial, "Bend!").ok();
        }
        if finger_should_extend {
            ufmt::uwriteln!(&mut serial, "Extend!").ok();
        }
    }
}
