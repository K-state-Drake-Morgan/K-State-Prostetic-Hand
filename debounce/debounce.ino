// currently useing the Rust style guides found:
// https://doc.rust-lang.org/nightly/style-guide/index.html

const unsigned long MAX_UNSIGNED_LONG = 4294967295UL;
const unsigned long CHANGE_STATE_INTERVAL = 250; // 0.25 of a second to register should change
const int HIGH_THREASHOLD = 500;

// Enums and Structs

enum DebounceState {
  Low,
  PossibleHigh,
  High,
  PossibleLow
};

struct ChangeAble {
  bool can_change;
  unsigned long last_time;
};

struct Debouncer {
  DebounceState state;
  ChangeAble change;
  uint8_t pin; // store the pin here so we don't have to pass it around all over the place.
};

// Default Constructors (Rust-style naming)

/// The default state is Low, should be a trait but that is a rust thing
DebounceState default_debounce_state() {
  return DebounceState::Low;
}

/// creates a ChangeAble Object from the current time according to the board
ChangeAble default_change_able() {
  return ChangeAble {
    true,
    millis() // using millis here so we can call default without needing another constructor
  };
}

/// creates a new Debouncer object from a given pin
Debouncer new_debouncer(uint8_t pin) {
  return Debouncer {
    default_debounce_state(),
    default_change_able(),
    pin
  };
}

// Logic

/// Returns true if High or Possible Low and updates the timeline base on the State
/// We only need to use the can_change for the Possible states because that allows more control
/// over when to stop the state
bool is_high(Debouncer debouncer) {
  int current_input = analogRead(debouncer.pin);
  Serial.print(current_input);
  Serial.print(",");
  if (current_input > 155) {
    current_input = HIGH;
  } else {
    current_input = LOW;
  }

  switch (debouncer.state) {
    case Low:
      if (current_input == HIGH) { // the current reading is high
         debouncer.change = default_change_able();
         debouncer.state = PossibleHigh;
      }
      return false;
    case PossibleHigh:
      if (current_input == HIGH && can_change(debouncer.change, CHANGE_STATE_INTERVAL)) {
        debouncer.state = High;
        debouncer.change = default_change_able();
        return true;
      } else if (current_input == LOW) {
        debouncer.state = Low;
        debouncer.change = default_change_able();
      }
      return false;
    case High:
      if (current_input == Low) {
        debouncer.change = default_change_able();
        debouncer.state = PossibleLow;
      }
      return true;
    case PossibleLow:
      if (current_input == LOW && can_change(debouncer.change, CHANGE_STATE_INTERVAL)) {
        debouncer.change = default_change_able();
        debouncer.state = Low;
        return false;
      } else if (current_input == HIGH) {
        debouncer.state = High;
        debouncer.change = default_change_able();
      }
      return true;
  }
  printf("%i] got to unreachable area and isn't signal high!");
  return false; // something really bad happened if we go here!
}

/// Returns true if enough time has elapsed for use to update the State
/// Handles an overflow for millis
bool can_change(ChangeAble c, unsigned long interval_milli_seconds) {
  if (c.can_change) {
    return true;
  }

  unsigned long current_time = millis();
  if (current_time < c.last_time) {
    // millis() overflowed
    unsigned long total_time = MAX_UNSIGNED_LONG;
    unsigned long diffrence = total_time - c.last_time;
    unsigned long current_total_time = diffrence + current_time;
    if (current_total_time > interval_milli_seconds) {
      c.can_change = true;
      return true;
    }

  } else if (current_time - c.last_time >= interval_milli_seconds) {
    c.can_change = true;
    return true;
  }

  return false;
}

// Pins (Reading from EMG's incase anyone reading this forgot)

uint8_t refrence_pin = 14; // temp value for now
uint8_t bend_pin = 17;     // temp value for now
uint8_t unbend_pin = 16;   // temp value for now

// debouncers based on the pins
Debouncer bend_pin_debouncer = new_debouncer(bend_pin);
Debouncer unbend_pin_debouncer = new_debouncer(unbend_pin);

// Setup & Loop

void setup() {
  pinMode(refrence_pin, INPUT);
  pinMode(bend_pin, INPUT);
  pinMode(unbend_pin, INPUT);

  // for testing
  pinMode(13, OUTPUT);
  Serial.begin(9600);
}

void loop() {
  bool finger_should_bend = is_high(bend_pin_debouncer);
  Serial.println(finger_should_bend);
  // bool finger_should_extend = is_high(unbend_pin_debouncer);
  if (finger_should_bend) {
    digitalWrite(13, HIGH);
  } else {
    digitalWrite(13, LOW);
  }
}
