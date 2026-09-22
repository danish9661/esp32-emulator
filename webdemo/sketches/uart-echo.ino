void setup() {
  Serial.begin(115200);
  Serial.println("=== UART ECHO ===");
  Serial.println("Type into the input box below and press Send.");
  Serial.println("ECHO_READY");
}
void loop() {
  while (Serial.available()) {
    int c = Serial.read();
    if (c == '\r') continue;
    if (c == '
') { Serial.println(); Serial.print("ECHO> "); }
    else Serial.write(c);
  }
  delay(5);
}
