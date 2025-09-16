#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::pac::tc0::TCNT0;
use arduino_hal::{port::Pin, prelude::*};
use core::cell;
use core::ops::{Add, Div, Mul, Sub};
use core::panic::PanicInfo;
use embedded_hal::digital::InputPin;
use num_traits::one;
use num_traits::SaturatingMul;
use num_traits::SaturatingSub;
// use panic_halt as _;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // Write panic message ASAP
    let _ = ufmt::uwriteln!(&mut serial, "Panic!");

    loop {}
}

// ================== Tseting =====================
/// This is a simulator for when we don't have an EMG to test with, it uses random walks to get a seemingly resable graph for and EMG
struct EmgSimulator {
    val: i16,
    decay: u8,
}

impl EmgSimulator {
    pub fn new() -> Self {
        Self { val: 512, decay: 0 }
    }

    pub fn next(&mut self, noise: u16) -> u16 {
        if noise == 0 {
            self.val += 500;
            self.decay = 10;
        } else {
            // max step by this meathod is 7 units, but also everything between -7 and 7
            if noise > u16::MAX / 2 {
                self.val = self.val.saturating_add_unsigned(noise & 111);
            } else {
                self.val = self.val.saturating_sub_unsigned(noise & 111);
            }
        }

        // this should make a random spike imedently go down sometimes
        if self.decay > 0 {
            if noise % 7 != 0 {
                self.decay -= 1;
                self.val = self.val.saturating_sub_unsigned(50);
            } else {
                self.val = self.val.saturating_sub_unsigned(50 * self.decay);
                self.decay = 0;
            }
        }

        // Clamp to 0..=1023 and cast to u16 safely
        self.val = self.val.clamp(0, 1023);
        self.val as u16
    }
}

/// A very bad random number generator that works with no_std
pub struct LcgRng {
    state: u32,
}

impl LcgRng {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    pub fn rand_bounded_u32(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }
}
// ------------------------------------------------

// ================== Millis() ====================
//
/**
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

// A rolling average for data over time
pub struct ExponentialMovingAverage {
    pub ema: f32,
    pub alpha: f32,
}

impl ExponentialMovingAverage {
    pub fn update(&mut self, input: u16) -> u16 {
        let input_f32: f32 = input as f32;
        self.ema = (input_f32 * self.alpha) + (self.ema * (1.0 - self.alpha));
        return self.ema as u16;
    }

    pub fn new(alpha: f32) -> ExponentialMovingAverage {
        ExponentialMovingAverage {
            ema: 0.0,
            alpha: alpha,
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let a0 = pins.a0.into_analog_input(dp.ADC);

    // setup millis
    millis_init(dp.TC0);

    // Enable interrupts globally
    unsafe { avr_device::interrupt::enable() };
    // the millis function is now avable

    // ========================== Testing ===================================
    let mut rng = LcgRng::new(42);
    let mut emg_sim = EmgSimulator::new();
    let mut ema = ExponentialMovingAverage::new(0.25); // alpha is 0.25 with the scaler

    let mut start_time = millis();
    let mut next_time = start_time.wrapping_add(10);
    // ======================== Testing: End ================================

    loop {
        if millis() < next_time {
            continue;
        }

        // use rng for testing and read for functional
        let input = rng.rand_bounded_u32(1023) as u16;
        // let input = a0.analog_read(dp.ADC);

        let raw = emg_sim.next(input);
        let smoothed = ema.update(raw.clone());

        ufmt::uwriteln!(&mut serial, "{}, {}", raw, smoothed).unwrap();

        // from looking at the code provided in EMG_HAND_CM.ino (TEAMS GENERAL)
        // it seems that the servo rotates between 0 and 90
        // so we need a function that takes balues from 0 to 1023
        // to be from 0 to 90 for the hand to function

        next_time = millis().wrapping_add(10);
    }
}
