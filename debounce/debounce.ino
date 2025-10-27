// currently useing the Rust style guides found: 
// https://doc.rust-lang.org/nightly/style-guide/index.html

#include <Arduino.h>
#include <Servo.h>

// --- Constants ---
const int servoPin = 3;

// --- Enums ---
/// State of the goal of the hand
enum EmgState {
  Relaxed,
  Intermediate,
  Clenched
};

/// "Random" (psudo) number generator
struct RNG {

  private:
    /// storage for the random number genorator
    uint32_t value;

  public:
    /// Construct the Random Number Generator
    RNG(uint32_t seed) {
      value = seed;
    }

    /// Advance RNG state and return new value
    uint32_t next() {

      value = value * 0x6C8E9CF5UL;
      value ^= value >> 13;
      value = value + 0xB5297A4DUL;
      value ^= value << 17;
      value = value - 0xD6E8FEB8UL;
      value ^= value >> 5;
      return value;
    }

    /// Return bounded random number [0, bound)
    uint32_t next_bounded(uint32_t bound) {
      return next() % bound;
    }
};

/// Electromyography Simulator 
struct EMGSimulator {

  private:
    /// how many times updated, also changes the state
    uint32_t step_count = 0;
    /// current goal of the hand
    EmgState emg_state = Relaxed;
    /// for making harsh above and below the line
    uint16_t phase = 0;
    /// How large the spike is
    uint8_t spike_remaining = 0;
    /// The Random Number Geneator
    RNG number_generator;

  public:
    EMGSimulator(RNG generator) : number_generator(generator) {}

    /// Get the next EMG value
    uint16_t next() {
      uint16_t noise = number_generator.next_bounded(1024);

      step_count++;
      phase += 17;

      if ((step_count % 1000) == 0) {
        uint16_t r = noise % 100;
        if (r < 50) emg_state = Relaxed;
        else if (r < 80) emg_state = Intermediate;
        else emg_state = Clenched;
      }

      if (spike_remaining == 0 && (noise % 200) == 0) {
        spike_remaining = (noise % 5) + 1;
      }

      if (spike_remaining > 0) {
        spike_remaining--;
        return 1023;
      }

      uint16_t baseline = 0;
      uint16_t amplitude = 0;

      switch (emg_state) {
        case Relaxed: baseline = 200; amplitude = 50; break;
        case Intermediate: baseline = 620; amplitude = 30; break;
        case Clenched: baseline = 940; amplitude = 10; break;
      }

      int16_t jitter = ((noise % (2 * amplitude)) - amplitude);
      int16_t artifact = ((phase % 256) < 128) ? 3 : -3;
      int16_t signal = baseline + jitter + artifact;

      if (signal < 0) signal = 0;
      if (signal > 1023) signal = 1023;

      return (uint16_t)signal;
    }
};

/// Smoothing for a chaotic system
struct ExponentalMovingAverage {
  private:
    float ema = 0.0;
    float alpha;
    float last_input = 0.0;

  public:
    /// EMA Constructor
    ExponentalMovingAverage(float alpha) {
      this->alpha = alpha;
    }

    /// Get next smoothed value based on the last average
    uint16_t update(uint16_t input) {
      float input_f = (float)input;
      float max_slope = abs(last_input - ema);

      float go_to = alpha * input_f + (1.0f - alpha) * ema;
      float slope = go_to - ema;

      if (slope > max_slope) slope = max_slope;
      else if (slope < -max_slope) slope = -max_slope;

      ema += slope;
      last_input = input_f;

      return (uint16_t)ema;
    }
};

// --- Globals for EMG Simulator ---
EMGSimulator emg_simulator = EMGSimulator(RNG(42));
ExponentalMovingAverage ema = ExponentalMovingAverage(0.15);

// Servo object
Servo finger;

// --- Functions ---

/// Convert 0-1023 to 0-90 degrees
uint16_t from_1023_to_90(uint16_t number) {
  return (uint16_t)(((uint32_t)number * 90) / 1023);
}

void setup() {
  Serial.begin(9600);
  finger.attach(servoPin);
  pinMode(A0, INPUT);

  // Calibration: hold servo at 0 degrees for ~5 seconds
  for (int i = 0; i <= 90; i++) {
    finger.write(0);
    delay(55);
  }
}

void loop() {
  uint16_t raw = analogRead(A0);
  uint16_t smoothed = ema.update(raw);

  uint16_t motor_out = from_1023_to_90(smoothed);
  finger.write(motor_out);

  Serial.print("raw:");
  Serial.print(raw);
  Serial.print(", smoothed:");
  Serial.print(smoothed);
  Serial.print(", motor:");
  Serial.println(motor_out);
}
