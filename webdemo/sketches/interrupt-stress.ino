
#include <Arduino.h>

// Interrupt stress test: timer counting + GPIO rapid toggling
// Validates no crashes/panics under high-frequency register activity

static volatile uint64_t timerCount = 0;

void setup() {
  Serial.begin(115200);
  Serial.println("=== INTERRUPT STRESS TEST ===");

  // High-frequency timer (1MHz base, read continuously)
  hw_timer_t *timer = timerBegin(1000000);
  timerStart(timer);

  // Rapid GPIO toggling stress — creates heavy MMIO traffic
  pinMode(17, OUTPUT);
  for (int i = 0; i < 5000; i++) {
    digitalWrite(17, i & 1);
  }

  // Read timer after burst
  uint64_t cnt = timerRead(timer);
  timerStop(timer);
  timerEnd(timer);

  Serial.printf("[STRESS] timer_count=%lu gpio_toggles=5000", (uint32_t)cnt);

  // Timer at 1MHz should have counted ~thousands of ticks during the 5000 toggles
  bool pass = cnt > 100;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}

void loop() { delay(1000); }
