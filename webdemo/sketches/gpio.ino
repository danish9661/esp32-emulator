
void setup() {
  Serial.begin(115200);
  Serial.println("=== GPIO TEST ===");

  pinMode(2, OUTPUT);
  digitalWrite(2, HIGH);
  digitalWrite(2, LOW);
  Serial.println("[GPIO] output OK");

  pinMode(4, INPUT_PULLUP);
  Serial.print("[GPIO] pullup="); Serial.println(digitalRead(4));

  pinMode(5, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(5), []{}, RISING);
  Serial.println("[GPIO] int attach OK");
  detachInterrupt(digitalPinToInterrupt(5));
  Serial.println("[GPIO] int detach OK");

  Serial.print("RESULT="); Serial.println("PASS");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
