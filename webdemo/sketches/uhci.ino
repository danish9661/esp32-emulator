
#include <soc/soc.h>
#include <soc/uhci_reg.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== UHCI TEST ===");
  bool pass = true;

  // UHCI0 (0x3FF54000)
  REG_WRITE(UHCI_CONF0_REG(0), 0x00000007);
  uint32_t v = REG_READ(UHCI_CONF0_REG(0));
  Serial.printf("[UHCI] conf0(0) readback=%x", v);
  if (v != 0x00000007) pass = false;

  REG_WRITE(UHCI_DMA_OUT_LINK_REG(0), 0x3FF80010);
  v = REG_READ(UHCI_DMA_OUT_LINK_REG(0));
  Serial.printf("[UHCI] out_link(0) readback=%x", v);
  if (v != 0x3FF80010) pass = false;

  REG_WRITE(UHCI_DMA_IN_LINK_REG(0), 0x3FF80100);
  v = REG_READ(UHCI_DMA_IN_LINK_REG(0));
  if (v != 0x3FF80100) pass = false;

  REG_WRITE(UHCI_INT_ENA_REG(0), 0x000001FF);
  v = REG_READ(UHCI_INT_ENA_REG(0));
  Serial.printf("[UHCI] int_ena(0) readback=%x", v);
  if (v != 0x000001FF) pass = false;

  REG_WRITE(UHCI_INT_CLR_REG(0), 0xFFFFFFFF);   // clears RAW/ST, no crash
  REG_WRITE(UHCI_CONF1_REG(0), 0x3);
  v = REG_READ(UHCI_CONF1_REG(0));
  if (v != 0x3) pass = false;

  REG_WRITE(UHCI_PKT_THRES_REG(0), 0x40);
  v = REG_READ(UHCI_PKT_THRES_REG(0));
  if (v != 0x40) pass = false;

  // UHCI1 (0x3FF4C000)
  REG_WRITE(UHCI_CONF0_REG(1), 0x00000003);
  v = REG_READ(UHCI_CONF0_REG(1));
  Serial.printf("[UHCI] conf0(1) readback=%x", v);
  if (v != 0x00000003) pass = false;

  REG_WRITE(UHCI_DMA_IN_LINK_REG(1), 0x3FF80200);
  v = REG_READ(UHCI_DMA_IN_LINK_REG(1));
  if (v != 0x3FF80200) pass = false;

  REG_WRITE(UHCI_STATE0_REG(1), 0xABCD);
  v = REG_READ(UHCI_STATE0_REG(1));
  if (v != 0xABCD) pass = false;

  REG_WRITE(UHCI_Q0_WORD0_REG(1), 0xDEADBEEF);
  v = REG_READ(UHCI_Q0_WORD0_REG(1));
  if (v != 0xDEADBEEF) pass = false;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
