
#include "SD_MMC.h"

void setup() {
  Serial.begin(115200);
  Serial.println("=== SDCARD TEST ===");
  bool pass = true;

  bool ok = SD_MMC.begin("/sdcard", true, true);
  Serial.print("SD_BEGIN="); Serial.println(ok ? "PASS" : "FAIL");
  if (!ok) pass = false;

  uint64_t sectors = SD_MMC.numSectors();
  uint64_t cardSize = SD_MMC.cardSize();
  Serial.printf("SECTORS=%llu CARDSIZE=%llu", sectors, cardSize);
  if (sectors == 0 || cardSize == 0) pass = false;

  if (ok) {
    File f = SD_MMC.open("/hello.txt", FILE_WRITE);
    if (!f) { Serial.println("OPEN_W=FAIL"); pass = false; }
    else {
      f.println("esp32emu-sd-test-12345");
      f.close();
      Serial.println("OPEN_W=PASS");
    }

    File r = SD_MMC.open("/hello.txt", FILE_READ);
    if (!r) { Serial.println("OPEN_R=FAIL"); pass = false; }
    else {
      String s = r.readString();
      r.close();
      bool hit = s.indexOf("esp32emu-sd-test-12345") >= 0;
      Serial.print("READBACK="); Serial.println(hit ? "PASS" : "FAIL");
      if (!hit) pass = false;
    }

    // Multi-block file (4KB + 100B spans 9 blocks).
    File w = SD_MMC.open("/big.bin", FILE_WRITE);
    if (!w) { Serial.println("BIG_W=FAIL"); pass = false; }
    else {
      uint32_t cksum = 0;
      for (int i = 0; i < 4196; i++) { uint8_t b = (i * 37 + 11) & 0xFF; w.write(b); cksum += b; }
      w.close();
      Serial.printf("BIG_W=PASS cksum=%u", cksum);
    }
    File br = SD_MMC.open("/big.bin", FILE_READ);
    if (!br) { Serial.println("BIG_R=FAIL"); pass = false; }
    else {
      uint32_t cksum = 0, n = 0;
      while (br.available()) { cksum += br.read(); n++; }
      br.close();
      bool hit = (n == 4196 && cksum == 534714);
      Serial.printf("BIG_R n=%u cksum=%u %s", n, cksum, hit ? "PASS" : "FAIL");
      if (!hit) pass = false;
    }

    File root = SD_MMC.open("/");
    int entries = 0;
    if (root) {
      while (true) { File e = root.openNextFile(); if (!e) break; entries++; e.close(); }
      root.close();
    }
    Serial.printf("LS entries=%d %s", entries, entries >= 2 ? "PASS" : "FAIL");
    if (entries < 2) pass = false;
  }

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
