#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::{port::Pin, prelude::*};
use core::cell;
use embedded_hal::digital::InputPin;
use panic_halt as _;


#[derive(Debug)]
enum ChangeAble {
    CanChange,
    Waiting { start_time: u32 },
}

impl ChangeAble {
    fn default() -> Self {
        ChangeAble::Waiting {
            start_time: millis(),
        }
    }

    fn can_change(&self, interval: u32) -> bool {
        match self {
            ChangeAble::CanChange => true,
            ChangeAble::Waiting { start_time } => {
                let now = millis();
                let elapsed = now.wrapping_sub(*start_time);
                elapsed >= interval
            }
        }
    }
}

#[derive(Debug, Default)]
enum DebounceState {
    #[default]
    Low,
    PossibleHigh(ChangeAble),
    High,
    PossibleLow(ChangeAble),
}

impl DebounceState {
    pub fn update(&mut self, high: bool) {
        *self = match core::mem::replace(self, DebounceState::Low) {
            DebounceState::Low => {
                if high {
                    DebounceState::PossibleHigh(ChangeAble::default())
                } else {
                    DebounceState::Low
                }
            }
            DebounceState::PossibleHigh(change) => {
                if high {
                    if change.can_change(250) {
                        DebounceState::High
                    } else {
                        DebounceState::PossibleHigh(change)
                    }
                } else {
                    DebounceState::Low
                }
            }
            DebounceState::High => {
                if !high {
                    DebounceState::PossibleLow(ChangeAble::default())
                } else {
                    DebounceState::High
                }
            }
            DebounceState::PossibleLow(change) => {
                if !high {
                    if change.can_change(250) {
                        DebounceState::Low
                    } else {
                        DebounceState::PossibleLow(change)
                    }
                } else {
                    DebounceState::High
                }
            }
        };
    }

    pub fn is_high(&self) -> bool {
        matches!(self, DebounceState::High | DebounceState::PossibleLow(_))
    }
}

struct Debouncer<PIN>
where
    PIN: InputPin,
{
    state: DebounceState,
    pin: PIN,
}

impl<PIN> Debouncer<PIN>
where
    PIN: InputPin,
{
    pub fn new(pin: PIN) -> Self {
        Self {
            state: DebounceState::default(),
            pin,
        }
    }

    pub fn is_high(&mut self) -> bool {
        let pin_high = self.pin.is_high().unwrap_or(false);
        self.state.update(pin_high);
        self.state.is_high()
    }
}

// ================== Millis() ====================
//
/*!
 * A basic implementation of the `millis()` function from Arduino:
 *
 *     https://www.arduino.cc/reference/en/language/functions/time/millis/
 *
 * Uses timer TC0 and one of its interrupts to update a global millisecond
 * counter.  A walkthough of this code is available here:
 *
 *     https://blog.rahix.de/005-avr-hal-millis/
 */
// Possible Values:
//
// ╔═══════════╦══════════════╦═══════════════════╗
// ║ PRESCALER ║ TIMER_COUNTS ║ Overflow Interval ║
// ╠═══════════╬══════════════╬═══════════════════╣
// ║        64 ║          250 ║              1 ms ║
// ║       256 ║          125 ║              2 ms ║
// ║       256 ║          250 ║              4 ms ║
// ║      1024 ║          125 ║              8 ms ║
// ║      1024 ║          250 ║             16 ms ║
// ╚═══════════╩══════════════╩═══════════════════╝
const PRESCALER: u32 = 1024;
const TIMER_COUNTS: u32 = 125;

const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

static MILLIS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

fn millis_init(tc0: arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

fn millis() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

// ----------------------------------------------------------------------------

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    millis_init(dp.TC0);

    // Enable interrupts globally
    unsafe { avr_device::interrupt::enable() }; // only use millis after this line!

    let mut bend_debouncer = Debouncer::new(pins.d2.into_pull_up_input()); // example pins
    let mut unbend_debouncer = Debouncer::new(pins.d3.into_pull_up_input());

    loop {
        let should_bend = bend_debouncer.is_high();
        let should_unbend = unbend_debouncer.is_high();

        if should_bend {
            // do something
        }

        if should_unbend {
            // do something else
        }
    }
}
