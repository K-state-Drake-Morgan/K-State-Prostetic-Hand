#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::delay_ms;
use arduino_hal::port::mode::PwmOutput;
use arduino_hal::port::Pin;

// #[panic_handler]
// fn panic(info: &core::panic::PanicInfo) -> ! {
//     let dp = match arduino_hal::Peripherals::take() {
//         Some(p) => p,
//         None => unsafe { Peripherals::steal() },
//     };
//     let pins = arduino_hal::pins!(dp);
//     let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
//
//     if let Some(location) = info.location() {
//         let _ = ufmt::uwriteln!(
//             &mut serial,
//             "Panic at {}:{}:{}",
//             location.file(),
//             location.line(),
//             location.column()
//         );
//     }
//
//     let _ = info.message().as_str().is_some_and(|msg| {
//         let _ = ufmt::uwriteln!(&mut serial, "{}", msg);
//         true
//     });
//
//     loop {}
// }

use arduino_hal::simple_pwm::IntoPwmPin;
use arduino_hal::simple_pwm::Timer2Pwm;
use panic_halt as _;

// ================== Testing =====================
/// This is a simulator for when we don't have an EMG to test with, it uses random walks to get a seemingly resable graph for and EMG

pub enum EmgState {
    Relaxed,
    Intermediate,
    Clenched,
}

pub struct EmgSimulator {
    step_count: u32,
    state: EmgState,
    phase: u16,
    spike_remaining: u8, // Counts how many steps left in spike
}

impl EmgSimulator {
    pub fn new() -> Self {
        Self {
            step_count: 0,
            state: EmgState::Relaxed,
            phase: 0,
            spike_remaining: 0,
        }
    }

    pub fn next(&mut self, noise: u16) -> u16 {
        self.step_count = self.step_count.wrapping_add(1);
        self.phase = self.phase.wrapping_add(17);

        // Change state every 1000 samples based on noise
        if self.step_count % 1000 == 0 {
            let r = noise % 100;
            self.state = if r < 50 {
                EmgState::Relaxed
            } else if r < 80 {
                EmgState::Intermediate
            } else {
                EmgState::Clenched
            };
        }

        // Trigger spike if none active and noise meets condition
        if self.spike_remaining == 0 && (noise % 200 == 0) {
            // spike length pseudo-random from 1 to 5 inclusive
            self.spike_remaining = (noise % 5 + 1) as u8;
        }

        // If in spike, output max value and decrement spike timer
        if self.spike_remaining > 0 {
            self.spike_remaining -= 1;
            return 1023;
        }

        // Normal signal calculation
        let (baseline, amplitude): (u16, u16) = match self.state {
            EmgState::Relaxed => (200, 50),
            EmgState::Intermediate => (620, 30),
            EmgState::Clenched => (940, 10),
        };

        let jitter = ((noise % (2 * amplitude)) as i16) - (amplitude as i16);

        let artifact = if (self.phase % 256) < 128 { 3 } else { -3 };

        let mut signal = baseline as i16 + jitter + artifact;

        signal = signal.clamp(0, 1023);

        signal as u16
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
        // Use a mix of wrapping mul, add, xor and shifts to scramble bits
        self.state = self.state.wrapping_mul(0x6C8E9CF5);
        self.state ^= self.state >> 13;
        self.state = self.state.wrapping_add(0xB5297A4D);
        self.state ^= self.state << 17;
        self.state = self.state.wrapping_sub(0xD6E8FEB8);
        self.state ^= self.state >> 5;
        self.state
    }

    pub fn rand_bounded_u32(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }
}

/// A rolling average for data over time
pub struct ExponentialMovingAverage {
    /// Stores the last value from the data
    pub ema: f32,
    /// How much the newest value effects the value.
    /// A lower alpha means a slower responce time.
    /// But a higher alpha has the ema follow the data more closly
    pub alpha: f32,
    last_input: f32,
}

impl ExponentialMovingAverage {
    /// update the value from new data
    pub fn update(&mut self, input: u16) -> u16 {
        let input_f32 = input as f32;
        let max_slope = (self.last_input - self.ema).abs();

        let go_to = self.alpha * input_f32 + (1.0 - self.alpha) * self.ema;
        let slope = go_to - self.ema;
        self.ema = self.ema + slope.clamp(-max_slope, max_slope);

        self.last_input = input_f32;

        self.ema as u16
    }

    pub fn new(alpha: f32) -> ExponentialMovingAverage {
        ExponentialMovingAverage {
            ema: 0.0,
            alpha: alpha,
            last_input: 0.0,
        }
    }
}

pub fn fron_1023_to_90(number: u16) -> u8 {
    ((number as u32).saturating_mul(90) / 1023) as u8
}

struct Servo {
    pin: Pin<PwmOutput<Timer2Pwm>, arduino_hal::hal::port::PD3>,
}

impl Servo {
    pub fn new(pin: Pin<PwmOutput<Timer2Pwm>, arduino_hal::hal::port::PD3>) -> Servo {
        Servo { pin }
    }

    pub fn set_angle(&mut self, angle: u8) {
        // let duty = 23 + ((angle as u32 * (31 - 23)) / 90) as u8;
        self.pin.set_duty(angle);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut timer = Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale1024);
    let mut servo_pin = pins.d3.into_output().into_pwm(&mut timer); // or use D3 instead

    // ========================== Testing ===================================
    let mut rng = LcgRng::new(42);
    let mut emg_sim = EmgSimulator::new();
    // ======================== Testing: End ================================

    let mut ema = ExponentialMovingAverage::new(0.15); // the alpha
                                                       // effects how much the new value is used

    servo_pin.enable();
    let mut s = Servo::new(servo_pin);

    // set the angle to 0 for callibration for 5 seconds
    for _ in 0..=90 {
        s.set_angle(0);
        delay_ms(55);
    }

    let mut u8_value = 0;

    loop {
        s.set_angle(u8_value);
        delay_ms(500);
        let _ = ufmt::uwriteln!(&mut serial, "u8_value:{}", u8_value);
        u8_value = u8_value.wrapping_add(1);

        // // use rng for testing and read for functional
        // let input = rng.rand_bounded_u32(1023) as u16;
        // // let input = a0.analog_read(adc);

        // let raw = emg_sim.next(input);
        // let smoothed = ema.update(raw.clone());

        // // from looking at the code provided in EMG_HAND_CM.ino (TEAMS GENERAL)
        // // it seems that the servo rotates between 0 and 90
        // // so we need a function that takes balues from 0 to 1023
        // // to be from 0 to 90 for the hand to function
        // let motor_out = fron_1023_to_90(smoothed);

        // s.set_angle(motor_out);

        // let _ = ufmt::uwriteln!(
        //     &mut serial,
        //     "raw:{}, smoothed:{}, motor:{}",
        //     raw,
        //     smoothed,
        //     motor_out
        // );
    }
}
