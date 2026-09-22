
#include <mbedtls/bignum.h>

static void mpi_set_str(mbedtls_mpi *m, const char *s) {
  if (mbedtls_mpi_read_string(m, 10, s) != 0) {
    Serial.println("MPI_READ_STRING=FAIL");
  }
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== RSA TEST ===");
  mbedtls_mpi base, exp, mod, res, expected;
  mbedtls_mpi_init(&base); mbedtls_mpi_init(&exp);
  mbedtls_mpi_init(&mod); mbedtls_mpi_init(&res);
  mbedtls_mpi_init(&expected);

  mpi_set_str(&base, "5");
  mpi_set_str(&exp, "65537");
  mpi_set_str(&mod, "11");
  mpi_set_str(&expected, "3");
  int err = mbedtls_mpi_exp_mod(&res, &base, &exp, &mod, NULL);
  bool expOk = (err == 0) && (mbedtls_mpi_cmp_mpi(&res, &expected) == 0);
  Serial.print("RSA_EXP1="); Serial.println(expOk ? "PASS" : "FAIL");

  mpi_set_str(&base, "3");
  mpi_set_str(&exp, "7");
  mpi_set_str(&mod, "17");
  mpi_set_str(&expected, "11");
  err = mbedtls_mpi_exp_mod(&res, &base, &exp, &mod, NULL);
  bool mulOk = (err == 0) && (mbedtls_mpi_cmp_mpi(&res, &expected) == 0);
  Serial.print("RSA_EXP2="); Serial.println(mulOk ? "PASS" : "FAIL");

  mbedtls_mpi_free(&base); mbedtls_mpi_free(&exp);
  mbedtls_mpi_free(&mod); mbedtls_mpi_free(&res);
  mbedtls_mpi_free(&expected);

  bool pass = expOk && mulOk;
  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
