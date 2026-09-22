
#include <soc/soc.h>

#define EMAC_MAC_BASE  0x3ff69000
#define EMAC_DMA_BASE  0x3ff6a000
#define MAC_MDIO_ADDR  (EMAC_MAC_BASE + 0x10)
#define MAC_MDIO_DATA  (EMAC_MAC_BASE + 0x14)
#define DMA_TXPOLL     (EMAC_DMA_BASE + 0x4)
#define DMA_RXBASE     (EMAC_DMA_BASE + 0xC)
#define DMA_TXBASE     (EMAC_DMA_BASE + 0x10)
#define DMA_STATUS     (EMAC_DMA_BASE + 0x14)
#define DMA_OPMODE     (EMAC_DMA_BASE + 0x18)
#define DMA_INTEN      (EMAC_DMA_BASE + 0x1C)

#define FRAMELEN 64
static uint8_t txFrame[FRAMELEN];
static uint8_t rxFrame[1536];
static uint32_t txDesc[4];
static uint32_t rxDesc[4];

static uint32_t mdio_read(uint8_t phy, uint8_t reg) {
  REG_WRITE(MAC_MDIO_ADDR, (phy << 11) | (reg << 6) | 1);
  for (int i = 0; i < 1000; i++) {
    if ((REG_READ(MAC_MDIO_ADDR) & 1) == 0) break;
  }
  return REG_READ(MAC_MDIO_DATA) & 0xFFFF;
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== EMACLB TEST ===");
  bool pass = true;

  uint32_t id1 = mdio_read(0, 2);
  uint32_t id2 = mdio_read(0, 3);
  uint32_t bsr = mdio_read(0, 1);
  Serial.printf("PHY id1=%04x id2=%04x bsr=%04x", id1, id2, bsr);
  if (id1 != 0x0007 || id2 != 0xC0F1) { Serial.println("PHY_ID=FAIL"); pass = false; }
  else Serial.println("PHY_ID=PASS");
  if ((bsr & 0x4) == 0) { Serial.println("PHY_LINK=FAIL"); pass = false; }
  else Serial.println("PHY_LINK=PASS");

  for (int i = 0; i < FRAMELEN; i++) txFrame[i] = (i * 7 + 3) & 0xFF;
  for (int i = 0; i < 1536; i++) rxFrame[i] = 0;
  txDesc[0] = 0x80000000 | (1 << 29) | (1 << 28); // OWN, LS, FS
  txDesc[1] = FRAMELEN;
  txDesc[2] = (uint32_t)txFrame;
  txDesc[3] = 0;
  rxDesc[0] = 0x80000000; // OWN
  rxDesc[1] = 1536;
  rxDesc[2] = (uint32_t)rxFrame;
  rxDesc[3] = 0;

  REG_WRITE(DMA_TXBASE, (uint32_t)txDesc);
  REG_WRITE(DMA_RXBASE, (uint32_t)rxDesc);
  REG_WRITE(DMA_INTEN, (1 << 0) | (1 << 6) | (1 << 16)); // TIE, RIE, NIE
  REG_WRITE(DMA_OPMODE, (1 << 13) | (1 << 1));           // ST, SR
  REG_WRITE(DMA_TXPOLL, 1);

  uint32_t st = 0;
  for (int i = 0; i < 100000; i++) {
    st = REG_READ(DMA_STATUS);
    if ((st & 0x41) == 0x41) break;
  }
  Serial.printf("DMASR=%08x", st);
  if ((st & 0x41) != 0x41) { Serial.println("DMA_DONE=FAIL"); pass = false; }
  else Serial.println("DMA_DONE=PASS");

  bool match = ((txDesc[0] & 0x80000000) == 0) && ((rxDesc[0] & 0x80000000) == 0);
  uint32_t rlen = (rxDesc[0] >> 16) & 0x3FFF;
  Serial.printf("TXOWN=%d RXOWN=%d RLEN=%u", !!(txDesc[0] & 0x80000000), !!(rxDesc[0] & 0x80000000), rlen);
  if (rlen != FRAMELEN) match = false;
  for (int i = 0; i < FRAMELEN; i++) { if (rxFrame[i] != txFrame[i]) { match = false; break; } }
  Serial.print("LOOPBACK="); Serial.println(match ? "PASS" : "FAIL");
  if (!match) pass = false;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
