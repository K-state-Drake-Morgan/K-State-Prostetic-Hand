
/*
  Auto-Calibrating MyoWare Sensor for Raw Data with Low-Pass Filtering on
  the Signal and the State Decision, plus Serial-Controlled Calibration
  -----------------------------------------------------------------------------------------------
  This sketch reads raw sensor values from the MyoWare 2.0 and creates its own
  envelope signal. It first calibrates the raw offset in the relaxed state and then
  calibrates a flexed envelope (using rectified data). The threshold is set to
  flexedEnvelope/2.

  In normal operation the rectified sensor data is low-pass filtered to obtain an envelope.
  Additionally, the state decision (flexed vs relaxed) is low-pass filtered via a variable
  called filteredState so that the reported muscle state changes only after the new state has persisted
  for a longer period of time.
*/

// --- Motor Control Definitions ---
#define MUSCLE_RELAXED 0
#define MUSCLE_FLEXED  1

const int MOTOR_P = 39;
const int MOTOR_N = 29;

void extend() {
  digitalWrite(MOTOR_P, HIGH);
  digitalWrite(MOTOR_N, LOW);
}
void retract() {
  digitalWrite(MOTOR_P, LOW);
  digitalWrite(MOTOR_N, HIGH);
}
void stopMotor() {
  digitalWrite(MOTOR_P, HIGH);
  digitalWrite(MOTOR_N, HIGH);
}

// --- Sensor Pin ---
const int sensorPin = 36; // Ensure this matches your wiring

// --- Low-Pass Filter Parameter for Envelope Extraction ---
const float lpf_alpha = 0.05; // Envelope extraction low-pass filter coefficient (0: slow, 1: fast)
float envelope = 0;           // Running envelope value

// --- Low-Pass Filter for State Decision ---
const float state_filter_alpha = 0.02;   // Lower value makes state decision change more slowly
float filteredState = 0; // This variable (ranging approximately 0 to 1) will be low-pass filtered from the binary decision

// Instead of a debounce counter, we now low-pass the state decision.
// Define thresholds for switching:
const float flexedStateThresh = 0.7;    // Filtered state must exceed 0.7 to decide "flexed"
const float relaxedStateThresh = 0.3;   // Filtered state must drop below 0.3 to decide "relaxed"

// --- Sampling Parameters (200Hz) ---
unsigned long lastSampleTime = 0;
const unsigned long sampleInterval = 5000;  // 5000 microseconds (5ms interval)

// --- Calibration Variables ---
unsigned long calibrationDuration = 5000;  // Calibration period in milliseconds (5 seconds)
unsigned long calibrationStartTime = 0;
long calibrationSum = 0;
unsigned long calibrationCount = 0;

float rawOffset = 0;       // Determined during relaxed calibration
float flexedEnvelope = 0;  // Determined during flexed calibration
float threshold = 0;       // Set to flexedEnvelope/2

// --- Calibration State Machine ---
enum SystemState { WAIT_RELAXED, CALIBRATE_RELAXED, WAIT_FLEXED, CALIBRATE_FLEXED, RUNNING };
SystemState systemState = WAIT_RELAXED;

// For reporting the final (decided) state
int reportedState = MUSCLE_RELAXED;

void setup() {
  Serial.begin(115200);
  pinMode(sensorPin, INPUT);
  pinMode(MOTOR_P, OUTPUT);
  pinMode(MOTOR_N, OUTPUT);

  // Initialize rawOffset by taking one sample (this will be updated during calibration)
  int initialValue = analogRead(sensorPin);
  rawOffset = initialValue;

  Serial.println("Auto-Calibrating MyoWare Sensor from Raw Data with Low-Pass Filtering on Signal & State");
  Serial.println("-----------------------------------------------------------------------------------------");
  Serial.println("Place your muscle in the relaxed position and type 'r' to start relaxed calibration.");
}

void loop() {
  // Check for serial commands for calibration triggers.
  if (Serial.available() > 0) {
    char cmd = Serial.read();
    if (systemState == WAIT_RELAXED && (cmd == 'r' || cmd == 'R')) {
      systemState = CALIBRATE_RELAXED;
      calibrationSum = 0;
      calibrationCount = 0;
      calibrationStartTime = millis();
      Serial.println("Starting calibration for relaxed state. Please keep the muscle relaxed...");
    }
    else if (systemState == WAIT_FLEXED && (cmd == 'f' || cmd == 'F')) {
      systemState = CALIBRATE_FLEXED;
      calibrationSum = 0;
      calibrationCount = 0;
      calibrationStartTime = millis();
      Serial.println("Starting calibration for flexed state. Please flex the muscle steadily...");
    }
  }

  // Sampling at 200Hz using micros()
  unsigned long currentMicros = micros();
  if (currentMicros - lastSampleTime >= sampleInterval) {
    lastSampleTime = currentMicros;

    int sensorValue = analogRead(sensorPin);

    // Process according to current system state.
    switch(systemState) {
      case WAIT_RELAXED:
        // Do nothing—just wait.
        break;

      case CALIBRATE_RELAXED:
        // Accumulate raw sensor readings for relaxed state.
        calibrationSum += sensorValue;
        calibrationCount++;
        if (millis() - calibrationStartTime >= calibrationDuration) {
          rawOffset = (float)calibrationSum / calibrationCount;
          Serial.print("Relaxed calibration complete. Raw offset: ");
          Serial.println(rawOffset, 1);
          systemState = WAIT_FLEXED;
          Serial.println("Now, flex the muscle and type 'f' to start flexed calibration.");
        }
        break;

      case WAIT_FLEXED:
        // Do nothing—just wait.
        break;

      case CALIBRATE_FLEXED:
      {
        // For flexed calibration, record the rectified signal (raw minus offset, then absolute).
        int diff = sensorValue - rawOffset;
        if(diff < 0) diff = -diff;

        calibrationSum += diff;
        calibrationCount++;
        if (millis() - calibrationStartTime >= calibrationDuration) {
          flexedEnvelope = (float)calibrationSum / calibrationCount;
          threshold = flexedEnvelope / 2.0;  // Set threshold to half the measured flexed envelope.
          Serial.print("Flexed calibration complete. Flexed envelope: ");
          Serial.println(flexedEnvelope, 1);
          Serial.print("Calculated threshold: ");
          Serial.println(threshold, 1);
          systemState = RUNNING;
          Serial.println("Calibration complete. Entering normal operation.");
          // Initialize the envelope filter and state filter.
          envelope = 0;
          filteredState = 0;  // We'll assume muscle is relaxed to start.
          reportedState = MUSCLE_RELAXED;
        }
        break;
      }

      case RUNNING:
      {
        // Compute the rectified difference from raw offset.
        int diff = sensorValue - rawOffset;
        if(diff < 0) diff = -diff;

        // Apply a low-pass filter to obtain an envelope.
        envelope = (1 - lpf_alpha) * envelope + lpf_alpha * diff;

        // Compute the instantaneous state: 1 if envelope exceeds threshold, otherwise 0.
        float measuredState = (envelope > threshold) ? 1.0 : 0.0;

        // Now low-pass filter the state decision.
        filteredState = (1 - state_filter_alpha) * filteredState + state_filter_alpha * measuredState;

        // Use separate thresholds for switching to ensure the state decision must persist.
        if (filteredState > flexedStateThresh) {
          reportedState = MUSCLE_FLEXED;
        }
        else if (filteredState < relaxedStateThresh) {
          reportedState = MUSCLE_RELAXED;
        }

        // Debug output.
        Serial.print("Raw:");
        Serial.print(sensorValue);
        Serial.print(",Env:");
        Serial.print(envelope, 1);
        Serial.print(",Thresh:");
        Serial.print(threshold, 1);
        Serial.print(",StateFilt:");
        Serial.print(filteredState, 2);
        Serial.print(",State:");
        Serial.println((reportedState == MUSCLE_FLEXED) ? 100 : 0);

        // Optionally, implement motor control here:
        // if (reportedState == MUSCLE_FLEXED) { extend(); } else { retract(); }
        break;
      }

    } // end switch
  } // end if (sampling interval)
}
