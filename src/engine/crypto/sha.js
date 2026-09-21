import { ShaAlgorithm } from "../../peripherals/common/enums.js";
import { uint32ToHex } from "../../peripherals/common/helpers.js";

export let digestWordCounts = {
    [ShaAlgorithm.SHA1]: 5,
    [ShaAlgorithm.SHA224]: 7,
    [ShaAlgorithm.SHA256]: 8,
    [ShaAlgorithm.SHA384]: 12,
    [ShaAlgorithm.SHA512]: 16,
    [ShaAlgorithm.SHA512_224]: 7,
    [ShaAlgorithm.SHA512_256]: 8,
  },
  initialHashValues = {
    [ShaAlgorithm.SHA1]: [
      0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0,
    ],
    [ShaAlgorithm.SHA224]: [
      0xc1059ed8, 0x367cd507, 0x3070dd17, 0xf70e5939, 0xffc00b31, 0x68581511,
      0x64f98fa7, 0xbefa4fa4,
    ],
    [ShaAlgorithm.SHA256]: [
      0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
      0x1f83d9ab, 0x5be0cd19,
    ],
    [ShaAlgorithm.SHA384]: [
      0xcbbb9d5d, 0xc1059ed8, 0x629a292a, 0x367cd507, 0x9159015a, 0x3070dd17,
      0x152fecd8, 0xf70e5939, 0x67332667, 0xffc00b31, 0x8eb44a87, 0x68581511,
      0xdb0c2e0d, 0x64f98fa7, 0x47b5481d, 0xbefa4fa4,
    ],
    [ShaAlgorithm.SHA512]: [
      0x6a09e667, 0xf3bcc908, 0xbb67ae85, 0x84caa73b, 0x3c6ef372, 0xfe94f82b,
      0xa54ff53a, 0x5f1d36f1, 0x510e527f, 0xade682d1, 0x9b05688c, 0x2b3e6c1f,
      0x1f83d9ab, 0xfb41bd6b, 0x5be0cd19, 0x137e2179,
    ],
    [ShaAlgorithm.SHA512_224]: [
      0x8c3d37c8, 0x19544da2, 0x73e19966, 0x89dcd4d6, 0x1dfab7ae, 0x32ff9c82,
      0x679dd514, 0x582f9fcf, 0xf6d2b69, 0x7bd44da8, 0x77e36f73, 0x4c48942,
      0x3f9d85a8, 0x6a1d36c8, 0x1112e6ad, 0x91d692a1,
    ],
    [ShaAlgorithm.SHA512_256]: [
      0x22312194, 0xfc2bf72c, 0x9f555fa3, 0xc84c64c2, 0x2393b86b, 0x6f53b151,
      0x96387719, 0x5940eabd, 0x96283ee2, 0xa88effe3, 0xbe5e1e25, 0x53863992,
      0x2b0199fc, 0x2c85b8aa, 0xeb72ddc, 0x81c52ca2,
    ],
  },
  sha256RoundConstants = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0xfc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x6ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ],
  sha512RoundConstants = [
    0x428a2f98, 0xd728ae22, 0x71374491, 0x23ef65cd, 0xb5c0fbcf, 0xec4d3b2f,
    0xe9b5dba5, 0x8189dbbc, 0x3956c25b, 0xf348b538, 0x59f111f1, 0xb605d019,
    0x923f82a4, 0xaf194f9b, 0xab1c5ed5, 0xda6d8118, 0xd807aa98, 0xa3030242,
    0x12835b01, 0x45706fbe, 0x243185be, 0x4ee4b28c, 0x550c7dc3, 0xd5ffb4e2,
    0x72be5d74, 0xf27b896f, 0x80deb1fe, 0x3b1696b1, 0x9bdc06a7, 0x25c71235,
    0xc19bf174, 0xcf692694, 0xe49b69c1, 0x9ef14ad2, 0xefbe4786, 0x384f25e3,
    0xfc19dc6, 0x8b8cd5b5, 0x240ca1cc, 0x77ac9c65, 0x2de92c6f, 0x592b0275,
    0x4a7484aa, 0x6ea6e483, 0x5cb0a9dc, 0xbd41fbd4, 0x76f988da, 0x831153b5,
    0x983e5152, 0xee66dfab, 0xa831c66d, 0x2db43210, 0xb00327c8, 0x98fb213f,
    0xbf597fc7, 0xbeef0ee4, 0xc6e00bf3, 0x3da88fc2, 0xd5a79147, 0x930aa725,
    0x6ca6351, 0xe003826f, 0x14292967, 0xa0e6e70, 0x27b70a85, 0x46d22ffc,
    0x2e1b2138, 0x5c26c926, 0x4d2c6dfc, 0x5ac42aed, 0x53380d13, 0x9d95b3df,
    0x650a7354, 0x8baf63de, 0x766a0abb, 0x3c77b2a8, 0x81c2c92e, 0x47edaee6,
    0x92722c85, 0x1482353b, 0xa2bfe8a1, 0x4cf10364, 0xa81a664b, 0xbc423001,
    0xc24b8b70, 0xd0f89791, 0xc76c51a3, 0x654be30, 0xd192e819, 0xd6ef5218,
    0xd6990624, 0x5565a910, 0xf40e3585, 0x5771202a, 0x106aa070, 0x32bbd1b8,
    0x19a4c116, 0xb8d2d0c8, 0x1e376c08, 0x5141ab53, 0x2748774c, 0xdf8eeb99,
    0x34b0bcb5, 0xe19b48a8, 0x391c0cb3, 0xc5c95a63, 0x4ed8aa4a, 0xe3418acb,
    0x5b9cca4f, 0x7763e373, 0x682e6ff3, 0xd6b2b8a3, 0x748f82ee, 0x5defb2fc,
    0x78a5636f, 0x43172f60, 0x84c87814, 0xa1f0ab72, 0x8cc70208, 0x1a6439ec,
    0x90befffa, 0x23631e28, 0xa4506ceb, 0xde82bde9, 0xbef9a3f7, 0xb2c67915,
    0xc67178f2, 0xe372532b, 0xca273ece, 0xea26619c, 0xd186b8c7, 0x21c0c207,
    0xeada7dd6, 0xcde0eb1e, 0xf57d4f7f, 0xee6ed178, 0x6f067aa, 0x72176fba,
    0xa637dc5, 0xa2c898a6, 0x113f9804, 0xbef90dae, 0x1b710b35, 0x131c471b,
    0x28db77f5, 0x23047d84, 0x32caab7b, 0x40c72493, 0x3c9ebe0a, 0x15c9bebc,
    0x431d67c4, 0x9c100d4c, 0x4cc5d4be, 0xcb3e42b6, 0x597f299c, 0xfc657e2a,
    0x5fcb6fab, 0x3ad6faec, 0x6c44198c, 0x4a475817,
  ];
export class ShaEngine {
  constructor() {
    ((this.hash = new Uint32Array(16)),
      (this.hashBytes = new Uint8Array(this.hash.buffer)));
  }
  initialize(cpuVal) {
    (this.hash.fill(0), this.hash.set(initialHashValues[cpuVal]));
  }
  initialize512t(cpuVal) {
    let tmpVal = `SHA-512/${cpuVal}`,
      idxVal = new Uint8Array(128);
    for (let cpuVal = 0; cpuVal < tmpVal.length; cpuVal++)
      idxVal[cpuVal] = tmpVal.charCodeAt(cpuVal);
    ((idxVal[tmpVal.length] = 128), (idxVal[127] = 8 * tmpVal.length));
    let ClockEvent = initialHashValues[ShaAlgorithm.SHA512];
    for (let cpuVal = 0; cpuVal < 16; cpuVal++)
      this.hash[cpuVal] = 0xa5a5a5a5 ^ ClockEvent[cpuVal];
    this.updateSha512(idxVal);
  }
  update(cpuVal, tmpVal) {
    switch (cpuVal) {
      case ShaAlgorithm.SHA1:
        return this.updateSha1(tmpVal);
      case ShaAlgorithm.SHA224:
      case ShaAlgorithm.SHA256:
        return this.updateSha256(tmpVal);
      default:
        return this.updateSha512(tmpVal);
    }
  }
  updateSha1(cpuVal) {
    let [tmpVal, idxVal, ClockEvent, SimulationClock, regVal] = this.hash,
      [argVal, RegisterType, cfgVal] = [0, 0, 0],
      hVal = new DataView(cpuVal.buffer, cpuVal.byteOffset, cpuVal.byteLength),
      offVal = new Uint32Array(80);
    for (let cpuVal = 0; cpuVal < 16; cpuVal++)
      offVal[cpuVal] = hVal.getUint32(4 * cpuVal);
    for (RegisterType = 16; RegisterType < 80; ++RegisterType)
      ((cfgVal =
        offVal[RegisterType - 3] ^
        offVal[RegisterType - 8] ^
        offVal[RegisterType - 14] ^
        offVal[RegisterType - 16]),
        (offVal[RegisterType] = (cfgVal << 1) | (cfgVal >>> 31)));
    for (RegisterType = 0; RegisterType < 20; RegisterType += 5)
      ((argVal = (idxVal & ClockEvent) | (~idxVal & SimulationClock)),
        (regVal =
          ((cfgVal = (tmpVal << 5) | (tmpVal >>> 27)) +
            argVal +
            regVal +
            0x5a827999 +
            offVal[RegisterType]) |
          0),
        (argVal =
          (tmpVal & (idxVal = (idxVal << 30) | (idxVal >>> 2))) |
          (~tmpVal & ClockEvent)),
        (SimulationClock =
          ((cfgVal = (regVal << 5) | (regVal >>> 27)) +
            argVal +
            SimulationClock +
            0x5a827999 +
            offVal[RegisterType + 1]) |
          0),
        (argVal =
          (regVal & (tmpVal = (tmpVal << 30) | (tmpVal >>> 2))) |
          (~regVal & idxVal)),
        (ClockEvent =
          ((cfgVal = (SimulationClock << 5) | (SimulationClock >>> 27)) +
            argVal +
            ClockEvent +
            0x5a827999 +
            offVal[RegisterType + 2]) |
          0),
        (argVal =
          (SimulationClock & (regVal = (regVal << 30) | (regVal >>> 2))) |
          (~SimulationClock & tmpVal)),
        (idxVal =
          ((cfgVal = (ClockEvent << 5) | (ClockEvent >>> 27)) +
            argVal +
            idxVal +
            0x5a827999 +
            offVal[RegisterType + 3]) |
          0),
        (argVal =
          (ClockEvent &
            (SimulationClock =
              (SimulationClock << 30) | (SimulationClock >>> 2))) |
          (~ClockEvent & regVal)),
        (tmpVal =
          ((cfgVal = (idxVal << 5) | (idxVal >>> 27)) +
            argVal +
            tmpVal +
            0x5a827999 +
            offVal[RegisterType + 4]) |
          0),
        (ClockEvent = (ClockEvent << 30) | (ClockEvent >>> 2)));
    for (; RegisterType < 40; RegisterType += 5)
      ((argVal = idxVal ^ ClockEvent ^ SimulationClock),
        (regVal =
          ((cfgVal = (tmpVal << 5) | (tmpVal >>> 27)) +
            argVal +
            regVal +
            0x6ed9eba1 +
            offVal[RegisterType]) |
          0),
        (argVal =
          tmpVal ^ (idxVal = (idxVal << 30) | (idxVal >>> 2)) ^ ClockEvent),
        (SimulationClock =
          ((cfgVal = (regVal << 5) | (regVal >>> 27)) +
            argVal +
            SimulationClock +
            0x6ed9eba1 +
            offVal[RegisterType + 1]) |
          0),
        (argVal = regVal ^ (tmpVal = (tmpVal << 30) | (tmpVal >>> 2)) ^ idxVal),
        (ClockEvent =
          ((cfgVal = (SimulationClock << 5) | (SimulationClock >>> 27)) +
            argVal +
            ClockEvent +
            0x6ed9eba1 +
            offVal[RegisterType + 2]) |
          0),
        (argVal =
          SimulationClock ^
          (regVal = (regVal << 30) | (regVal >>> 2)) ^
          tmpVal),
        (idxVal =
          ((cfgVal = (ClockEvent << 5) | (ClockEvent >>> 27)) +
            argVal +
            idxVal +
            0x6ed9eba1 +
            offVal[RegisterType + 3]) |
          0),
        (argVal =
          ClockEvent ^
          (SimulationClock =
            (SimulationClock << 30) | (SimulationClock >>> 2)) ^
          regVal),
        (tmpVal =
          ((cfgVal = (idxVal << 5) | (idxVal >>> 27)) +
            argVal +
            tmpVal +
            0x6ed9eba1 +
            offVal[RegisterType + 4]) |
          0),
        (ClockEvent = (ClockEvent << 30) | (ClockEvent >>> 2)));
    for (; RegisterType < 60; RegisterType += 5)
      ((argVal =
        (idxVal & ClockEvent) |
        (idxVal & SimulationClock) |
        (ClockEvent & SimulationClock)),
        (regVal =
          ((cfgVal = (tmpVal << 5) | (tmpVal >>> 27)) +
            argVal +
            regVal -
            0x70e44324 +
            offVal[RegisterType]) |
          0),
        (argVal =
          (tmpVal & (idxVal = (idxVal << 30) | (idxVal >>> 2))) |
          (tmpVal & ClockEvent) |
          (idxVal & ClockEvent)),
        (SimulationClock =
          ((cfgVal = (regVal << 5) | (regVal >>> 27)) +
            argVal +
            SimulationClock -
            0x70e44324 +
            offVal[RegisterType + 1]) |
          0),
        (argVal =
          (regVal & (tmpVal = (tmpVal << 30) | (tmpVal >>> 2))) |
          (regVal & idxVal) |
          (tmpVal & idxVal)),
        (ClockEvent =
          ((cfgVal = (SimulationClock << 5) | (SimulationClock >>> 27)) +
            argVal +
            ClockEvent -
            0x70e44324 +
            offVal[RegisterType + 2]) |
          0),
        (argVal =
          (SimulationClock & (regVal = (regVal << 30) | (regVal >>> 2))) |
          (SimulationClock & tmpVal) |
          (regVal & tmpVal)),
        (idxVal =
          ((cfgVal = (ClockEvent << 5) | (ClockEvent >>> 27)) +
            argVal +
            idxVal -
            0x70e44324 +
            offVal[RegisterType + 3]) |
          0),
        (argVal =
          (ClockEvent &
            (SimulationClock =
              (SimulationClock << 30) | (SimulationClock >>> 2))) |
          (ClockEvent & regVal) |
          (SimulationClock & regVal)),
        (tmpVal =
          ((cfgVal = (idxVal << 5) | (idxVal >>> 27)) +
            argVal +
            tmpVal -
            0x70e44324 +
            offVal[RegisterType + 4]) |
          0),
        (ClockEvent = (ClockEvent << 30) | (ClockEvent >>> 2)));
    for (; RegisterType < 80; RegisterType += 5)
      ((argVal = idxVal ^ ClockEvent ^ SimulationClock),
        (regVal =
          ((cfgVal = (tmpVal << 5) | (tmpVal >>> 27)) +
            argVal +
            regVal -
            0x359d3e2a +
            offVal[RegisterType]) |
          0),
        (argVal =
          tmpVal ^ (idxVal = (idxVal << 30) | (idxVal >>> 2)) ^ ClockEvent),
        (SimulationClock =
          ((cfgVal = (regVal << 5) | (regVal >>> 27)) +
            argVal +
            SimulationClock -
            0x359d3e2a +
            offVal[RegisterType + 1]) |
          0),
        (argVal = regVal ^ (tmpVal = (tmpVal << 30) | (tmpVal >>> 2)) ^ idxVal),
        (ClockEvent =
          ((cfgVal = (SimulationClock << 5) | (SimulationClock >>> 27)) +
            argVal +
            ClockEvent -
            0x359d3e2a +
            offVal[RegisterType + 2]) |
          0),
        (argVal =
          SimulationClock ^
          (regVal = (regVal << 30) | (regVal >>> 2)) ^
          tmpVal),
        (idxVal =
          ((cfgVal = (ClockEvent << 5) | (ClockEvent >>> 27)) +
            argVal +
            idxVal -
            0x359d3e2a +
            offVal[RegisterType + 3]) |
          0),
        (argVal =
          ClockEvent ^
          (SimulationClock =
            (SimulationClock << 30) | (SimulationClock >>> 2)) ^
          regVal),
        (tmpVal =
          ((cfgVal = (idxVal << 5) | (idxVal >>> 27)) +
            argVal +
            tmpVal -
            0x359d3e2a +
            offVal[RegisterType + 4]) |
          0),
        (ClockEvent = (ClockEvent << 30) | (ClockEvent >>> 2)));
    ((this.hash[0] += tmpVal),
      (this.hash[1] += idxVal),
      (this.hash[2] += ClockEvent),
      (this.hash[3] += SimulationClock),
      (this.hash[4] += regVal));
  }
  updateSha256(cpuVal) {
    let [
        tmpVal,
        idxVal,
        ClockEvent,
        SimulationClock,
        regVal,
        argVal,
        RegisterType,
        cfgVal,
      ] = this.hash,
      [hVal, offVal, lenVal, valVal, flag, TVal, data, IVal, RVal, xVal] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      ],
      CVal = new DataView(cpuVal.buffer, cpuVal.byteOffset, cpuVal.byteLength),
      SVal = new Uint32Array(64);
    for (let cpuVal = 0; cpuVal < 16; cpuVal++)
      SVal[cpuVal] = CVal.getUint32(4 * cpuVal);
    for (let cpuVal = 16; cpuVal < 64; ++cpuVal)
      ((hVal =
        (((valVal = SVal[cpuVal - 15]) >>> 7) | (valVal << 25)) ^
        ((valVal >>> 18) | (valVal << 14)) ^
        (valVal >>> 3)),
        (offVal =
          (((valVal = SVal[cpuVal - 2]) >>> 17) | (valVal << 15)) ^
          ((valVal >>> 19) | (valVal << 13)) ^
          (valVal >>> 10)),
        (SVal[cpuVal] =
          (SVal[cpuVal - 16] + hVal + SVal[cpuVal - 7] + offVal) | 0));
    xVal = idxVal & ClockEvent;
    for (let cpuVal = 0; cpuVal < 64; cpuVal += 4)
      ((hVal =
        ((tmpVal >>> 2) | (tmpVal << 30)) ^
        ((tmpVal >>> 13) | (tmpVal << 19)) ^
        ((tmpVal >>> 22) | (tmpVal << 10))),
        (offVal =
          ((regVal >>> 6) | (regVal << 26)) ^
          ((regVal >>> 11) | (regVal << 21)) ^
          ((regVal >>> 25) | (regVal << 7))),
        (lenVal = (data = tmpVal & idxVal) ^ (tmpVal & ClockEvent) ^ xVal),
        (valVal =
          cfgVal +
          offVal +
          (TVal = (regVal & argVal) ^ (~regVal & RegisterType)) +
          sha256RoundConstants[cpuVal] +
          SVal[cpuVal]),
        (flag = hVal + lenVal),
        (cfgVal = (SimulationClock + valVal) | 0),
        (hVal =
          (((SimulationClock = (valVal + flag) | 0) >>> 2) |
            (SimulationClock << 30)) ^
          ((SimulationClock >>> 13) | (SimulationClock << 19)) ^
          ((SimulationClock >>> 22) | (SimulationClock << 10))),
        (offVal =
          ((cfgVal >>> 6) | (cfgVal << 26)) ^
          ((cfgVal >>> 11) | (cfgVal << 21)) ^
          ((cfgVal >>> 25) | (cfgVal << 7))),
        (lenVal =
          (IVal = SimulationClock & tmpVal) ^
          (SimulationClock & idxVal) ^
          data),
        (valVal =
          RegisterType +
          offVal +
          (TVal = (cfgVal & regVal) ^ (~cfgVal & argVal)) +
          sha256RoundConstants[cpuVal + 1] +
          SVal[cpuVal + 1]),
        (flag = hVal + lenVal),
        (RegisterType = (ClockEvent + valVal) | 0),
        (hVal =
          (((ClockEvent = (valVal + flag) | 0) >>> 2) | (ClockEvent << 30)) ^
          ((ClockEvent >>> 13) | (ClockEvent << 19)) ^
          ((ClockEvent >>> 22) | (ClockEvent << 10))),
        (offVal =
          ((RegisterType >>> 6) | (RegisterType << 26)) ^
          ((RegisterType >>> 11) | (RegisterType << 21)) ^
          ((RegisterType >>> 25) | (RegisterType << 7))),
        (lenVal =
          (RVal = ClockEvent & SimulationClock) ^ (ClockEvent & tmpVal) ^ IVal),
        (valVal =
          argVal +
          offVal +
          (TVal = (RegisterType & cfgVal) ^ (~RegisterType & regVal)) +
          sha256RoundConstants[cpuVal + 2] +
          SVal[cpuVal + 2]),
        (flag = hVal + lenVal),
        (argVal = (idxVal + valVal) | 0),
        (hVal =
          (((idxVal = (valVal + flag) | 0) >>> 2) | (idxVal << 30)) ^
          ((idxVal >>> 13) | (idxVal << 19)) ^
          ((idxVal >>> 22) | (idxVal << 10))),
        (offVal =
          ((argVal >>> 6) | (argVal << 26)) ^
          ((argVal >>> 11) | (argVal << 21)) ^
          ((argVal >>> 25) | (argVal << 7))),
        (lenVal =
          (xVal = idxVal & ClockEvent) ^ (idxVal & SimulationClock) ^ RVal),
        (valVal =
          regVal +
          offVal +
          (TVal = (argVal & RegisterType) ^ (~argVal & cfgVal)) + // eslint-disable-line no-unused-vars
          sha256RoundConstants[cpuVal + 3] +
          SVal[cpuVal + 3]),
        (flag = hVal + lenVal),
        (regVal = (tmpVal + valVal) | 0),
        (tmpVal = (valVal + flag) | 0));
    ((this.hash[0] += tmpVal),
      (this.hash[1] += idxVal),
      (this.hash[2] += ClockEvent),
      (this.hash[3] += SimulationClock),
      (this.hash[4] += regVal),
      (this.hash[5] += argVal),
      (this.hash[6] += RegisterType),
      (this.hash[7] += cfgVal));
  }
  updateSha512(cpuVal) {
    let [
        tmpVal,
        idxVal,
        ClockEvent,
        SimulationClock,
        regVal,
        argVal,
        RegisterType,
        cfgVal,
        hVal,
        offVal,
        lenVal,
        valVal,
        flag,
        TVal,
        data,
        IVal,
      ] = this.hash,
      [
        RVal,
        xVal,
        CVal,
        SVal,
        ptrVal,
        AVal,
        NVal,
        EVal,
        gpio,
        PVal,
        DVal,
        UVal,
        byte,
        MVal,
        mode,
        LVal,
        OVal,
        word,
        FVal,
        kdxVal,
        yVal,
        BVal,
        GVal,
        WVal,
      ] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      ],
      HVal = new DataView(cpuVal.buffer, cpuVal.byteOffset, cpuVal.byteLength),
      value = new Uint32Array(160);
    for (let cpuVal = 0; cpuVal < 32; cpuVal++)
      value[cpuVal] = HVal.getUint32(4 * cpuVal);
    for (let cpuVal = 32; cpuVal < 160; cpuVal += 2)
      ((RVal =
        (((FVal = value[cpuVal - 30]) >>> 1) |
          ((kdxVal = value[cpuVal - 29]) << 31)) ^
        ((FVal >>> 8) | (kdxVal << 24)) ^
        (FVal >>> 7)),
        (xVal =
          ((kdxVal >>> 1) | (FVal << 31)) ^
          ((kdxVal >>> 8) | (FVal << 24)) ^
          ((kdxVal >>> 7) | (FVal << 25))),
        (CVal =
          (((FVal = value[cpuVal - 4]) >>> 19) |
            ((kdxVal = value[cpuVal - 3]) << 13)) ^
          ((kdxVal >>> 29) | (FVal << 3)) ^
          (FVal >>> 6)),
        (SVal =
          ((kdxVal >>> 19) | (FVal << 13)) ^
          ((FVal >>> 29) | (kdxVal << 3)) ^
          ((kdxVal >>> 6) | (FVal << 26))),
        (FVal = value[cpuVal - 32]),
        (kdxVal = value[cpuVal - 31]),
        (yVal = value[cpuVal - 14]),
        (ptrVal =
          (65535 & (BVal = value[cpuVal - 13])) +
          (65535 & kdxVal) +
          (65535 & xVal) +
          (65535 & SVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          (65535 & RVal) +
          (65535 & CVal) +
          ((AVal =
            (BVal >>> 16) +
            (kdxVal >>> 16) +
            (xVal >>> 16) +
            (SVal >>> 16) +
            (ptrVal >>> 16)) >>>
            16)),
        (EVal =
          (yVal >>> 16) +
          (FVal >>> 16) +
          (RVal >>> 16) +
          (CVal >>> 16) +
          (NVal >>> 16)),
        (value[cpuVal] = (EVal << 16) | (65535 & NVal)),
        (value[cpuVal + 1] = (AVal << 16) | (65535 & ptrVal)));
    let KVal = tmpVal,
      XVal = idxVal,
      VVal = ClockEvent,
      queue = SimulationClock,
      YVal = regVal,
      zVal = argVal,
      QVal = RegisterType,
      $Val = cfgVal,
      JVal = hVal,
      ZVal = offVal,
      jdxVal = lenVal,
      eeVal = valVal,
      etVal = flag,
      eiVal = TVal,
      esVal = data,
      enVal = IVal;
    ((mode = VVal & YVal), (LVal = queue & zVal));
    for (let cpuVal = 0; cpuVal < 160; cpuVal += 8)
      ((RVal =
        ((KVal >>> 28) | (XVal << 4)) ^
        ((XVal >>> 2) | (KVal << 30)) ^
        ((XVal >>> 7) | (KVal << 25))),
        (xVal =
          ((XVal >>> 28) | (KVal << 4)) ^
          ((KVal >>> 2) | (XVal << 30)) ^
          ((KVal >>> 7) | (XVal << 25))),
        (CVal =
          ((JVal >>> 14) | (ZVal << 18)) ^
          ((JVal >>> 18) | (ZVal << 14)) ^
          ((ZVal >>> 9) | (JVal << 23))),
        (SVal =
          ((ZVal >>> 14) | (JVal << 18)) ^
          ((ZVal >>> 18) | (JVal << 14)) ^
          ((JVal >>> 9) | (ZVal << 23))),
        (gpio = KVal & VVal),
        (PVal = XVal & queue),
        (OVal = gpio ^ (KVal & YVal) ^ mode),
        (word = PVal ^ (XVal & zVal) ^ LVal),
        (GVal = (JVal & jdxVal) ^ (~JVal & etVal)),
        (WVal = (ZVal & eeVal) ^ (~ZVal & eiVal)),
        (FVal = value[cpuVal]),
        (kdxVal = value[cpuVal + 1]),
        (yVal = sha512RoundConstants[cpuVal]),
        (ptrVal =
          (65535 & (BVal = sha512RoundConstants[cpuVal + 1])) +
          (65535 & kdxVal) +
          (65535 & WVal) +
          (65535 & SVal) +
          (65535 & enVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          (65535 & GVal) +
          (65535 & CVal) +
          (65535 & esVal) +
          ((AVal =
            (BVal >>> 16) +
            (kdxVal >>> 16) +
            (WVal >>> 16) +
            (SVal >>> 16) +
            (enVal >>> 16) +
            (ptrVal >>> 16)) >>>
            16)),
        (FVal =
          ((EVal =
            (yVal >>> 16) +
            (FVal >>> 16) +
            (GVal >>> 16) +
            (CVal >>> 16) +
            (esVal >>> 16) +
            (NVal >>> 16)) <<
            16) |
          (65535 & NVal)),
        (kdxVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & word) + (65535 & xVal)),
        (NVal =
          (65535 & OVal) +
          (65535 & RVal) +
          ((AVal = (word >>> 16) + (xVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (yVal =
          ((EVal = (OVal >>> 16) + (RVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (BVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & $Val) + (65535 & kdxVal)),
        (NVal =
          (65535 & QVal) +
          (65535 & FVal) +
          ((AVal = ($Val >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (esVal =
          ((EVal = (QVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (enVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & BVal) + (65535 & kdxVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          ((AVal = (BVal >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (RVal =
          (((QVal =
            ((EVal = (yVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
            (65535 & NVal)) >>>
            28) |
            (($Val = (AVal << 16) | (65535 & ptrVal)) << 4)) ^
          (($Val >>> 2) | (QVal << 30)) ^
          (($Val >>> 7) | (QVal << 25))),
        (xVal =
          (($Val >>> 28) | (QVal << 4)) ^
          ((QVal >>> 2) | ($Val << 30)) ^
          ((QVal >>> 7) | ($Val << 25))),
        (CVal =
          ((esVal >>> 14) | (enVal << 18)) ^
          ((esVal >>> 18) | (enVal << 14)) ^
          ((enVal >>> 9) | (esVal << 23))),
        (SVal =
          ((enVal >>> 14) | (esVal << 18)) ^
          ((enVal >>> 18) | (esVal << 14)) ^
          ((esVal >>> 9) | (enVal << 23))),
        (DVal = QVal & KVal),
        (UVal = $Val & XVal),
        (OVal = DVal ^ (QVal & VVal) ^ gpio),
        (word = UVal ^ ($Val & queue) ^ PVal),
        (GVal = (esVal & JVal) ^ (~esVal & jdxVal)),
        (WVal = (enVal & ZVal) ^ (~enVal & eeVal)),
        (FVal = value[cpuVal + 2]),
        (kdxVal = value[cpuVal + 3]),
        (yVal = sha512RoundConstants[cpuVal + 2]),
        (ptrVal =
          (65535 & (BVal = sha512RoundConstants[cpuVal + 3])) +
          (65535 & kdxVal) +
          (65535 & WVal) +
          (65535 & SVal) +
          (65535 & eiVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          (65535 & GVal) +
          (65535 & CVal) +
          (65535 & etVal) +
          ((AVal =
            (BVal >>> 16) +
            (kdxVal >>> 16) +
            (WVal >>> 16) +
            (SVal >>> 16) +
            (eiVal >>> 16) +
            (ptrVal >>> 16)) >>>
            16)),
        (FVal =
          ((EVal =
            (yVal >>> 16) +
            (FVal >>> 16) +
            (GVal >>> 16) +
            (CVal >>> 16) +
            (etVal >>> 16) +
            (NVal >>> 16)) <<
            16) |
          (65535 & NVal)),
        (kdxVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & word) + (65535 & xVal)),
        (NVal =
          (65535 & OVal) +
          (65535 & RVal) +
          ((AVal = (word >>> 16) + (xVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (yVal =
          ((EVal = (OVal >>> 16) + (RVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (BVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & zVal) + (65535 & kdxVal)),
        (NVal =
          (65535 & YVal) +
          (65535 & FVal) +
          ((AVal = (zVal >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (etVal =
          ((EVal = (YVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (eiVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & BVal) + (65535 & kdxVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          ((AVal = (BVal >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (RVal =
          (((YVal =
            ((EVal = (yVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
            (65535 & NVal)) >>>
            28) |
            ((zVal = (AVal << 16) | (65535 & ptrVal)) << 4)) ^
          ((zVal >>> 2) | (YVal << 30)) ^
          ((zVal >>> 7) | (YVal << 25))),
        (xVal =
          ((zVal >>> 28) | (YVal << 4)) ^
          ((YVal >>> 2) | (zVal << 30)) ^
          ((YVal >>> 7) | (zVal << 25))),
        (CVal =
          ((etVal >>> 14) | (eiVal << 18)) ^
          ((etVal >>> 18) | (eiVal << 14)) ^
          ((eiVal >>> 9) | (etVal << 23))),
        (SVal =
          ((eiVal >>> 14) | (etVal << 18)) ^
          ((eiVal >>> 18) | (etVal << 14)) ^
          ((etVal >>> 9) | (eiVal << 23))),
        (byte = YVal & QVal),
        (MVal = zVal & $Val),
        (OVal = byte ^ (YVal & KVal) ^ DVal),
        (word = MVal ^ (zVal & XVal) ^ UVal),
        (GVal = (etVal & esVal) ^ (~etVal & JVal)),
        (WVal = (eiVal & enVal) ^ (~eiVal & ZVal)),
        (FVal = value[cpuVal + 4]),
        (kdxVal = value[cpuVal + 5]),
        (yVal = sha512RoundConstants[cpuVal + 4]),
        (ptrVal =
          (65535 & (BVal = sha512RoundConstants[cpuVal + 5])) +
          (65535 & kdxVal) +
          (65535 & WVal) +
          (65535 & SVal) +
          (65535 & eeVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          (65535 & GVal) +
          (65535 & CVal) +
          (65535 & jdxVal) +
          ((AVal =
            (BVal >>> 16) +
            (kdxVal >>> 16) +
            (WVal >>> 16) +
            (SVal >>> 16) +
            (eeVal >>> 16) +
            (ptrVal >>> 16)) >>>
            16)),
        (FVal =
          ((EVal =
            (yVal >>> 16) +
            (FVal >>> 16) +
            (GVal >>> 16) +
            (CVal >>> 16) +
            (jdxVal >>> 16) +
            (NVal >>> 16)) <<
            16) |
          (65535 & NVal)),
        (kdxVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & word) + (65535 & xVal)),
        (NVal =
          (65535 & OVal) +
          (65535 & RVal) +
          ((AVal = (word >>> 16) + (xVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (yVal =
          ((EVal = (OVal >>> 16) + (RVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (BVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & queue) + (65535 & kdxVal)),
        (NVal =
          (65535 & VVal) +
          (65535 & FVal) +
          ((AVal = (queue >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (jdxVal =
          ((EVal = (VVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (eeVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & BVal) + (65535 & kdxVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          ((AVal = (BVal >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (RVal =
          (((VVal =
            ((EVal = (yVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
            (65535 & NVal)) >>>
            28) |
            ((queue = (AVal << 16) | (65535 & ptrVal)) << 4)) ^
          ((queue >>> 2) | (VVal << 30)) ^
          ((queue >>> 7) | (VVal << 25))),
        (xVal =
          ((queue >>> 28) | (VVal << 4)) ^
          ((VVal >>> 2) | (queue << 30)) ^
          ((VVal >>> 7) | (queue << 25))),
        (CVal =
          ((jdxVal >>> 14) | (eeVal << 18)) ^
          ((jdxVal >>> 18) | (eeVal << 14)) ^
          ((eeVal >>> 9) | (jdxVal << 23))),
        (SVal =
          ((eeVal >>> 14) | (jdxVal << 18)) ^
          ((eeVal >>> 18) | (jdxVal << 14)) ^
          ((jdxVal >>> 9) | (eeVal << 23))),
        (mode = VVal & YVal),
        (LVal = queue & zVal),
        (OVal = mode ^ (VVal & QVal) ^ byte),
        (word = LVal ^ (queue & $Val) ^ MVal),
        (GVal = (jdxVal & etVal) ^ (~jdxVal & esVal)),
        (WVal = (eeVal & eiVal) ^ (~eeVal & enVal)),
        (FVal = value[cpuVal + 6]),
        (kdxVal = value[cpuVal + 7]),
        (yVal = sha512RoundConstants[cpuVal + 6]),
        (ptrVal =
          (65535 & (BVal = sha512RoundConstants[cpuVal + 7])) +
          (65535 & kdxVal) +
          (65535 & WVal) +
          (65535 & SVal) +
          (65535 & ZVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          (65535 & GVal) +
          (65535 & CVal) +
          (65535 & JVal) +
          ((AVal =
            (BVal >>> 16) +
            (kdxVal >>> 16) +
            (WVal >>> 16) +
            (SVal >>> 16) +
            (ZVal >>> 16) +
            (ptrVal >>> 16)) >>>
            16)),
        (FVal =
          ((EVal =
            (yVal >>> 16) +
            (FVal >>> 16) +
            (GVal >>> 16) +
            (CVal >>> 16) +
            (JVal >>> 16) +
            (NVal >>> 16)) <<
            16) |
          (65535 & NVal)),
        (kdxVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & word) + (65535 & xVal)),
        (NVal =
          (65535 & OVal) +
          (65535 & RVal) +
          ((AVal = (word >>> 16) + (xVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (yVal =
          ((EVal = (OVal >>> 16) + (RVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (BVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & XVal) + (65535 & kdxVal)),
        (NVal =
          (65535 & KVal) +
          (65535 & FVal) +
          ((AVal = (XVal >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (JVal =
          ((EVal = (KVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (ZVal = (AVal << 16) | (65535 & ptrVal)),
        (ptrVal = (65535 & BVal) + (65535 & kdxVal)),
        (NVal =
          (65535 & yVal) +
          (65535 & FVal) +
          ((AVal = (BVal >>> 16) + (kdxVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
        (KVal =
          ((EVal = (yVal >>> 16) + (FVal >>> 16) + (NVal >>> 16)) << 16) |
          (65535 & NVal)),
        (XVal = (AVal << 16) | (65535 & ptrVal)));
    ((ptrVal = (65535 & idxVal) + (65535 & XVal)),
      (NVal =
        (65535 & tmpVal) +
        (65535 & KVal) +
        ((AVal = (idxVal >>> 16) + (XVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (tmpVal >>> 16) + (KVal >>> 16) + (NVal >>> 16)),
      (this.hash[0] = (EVal << 16) | (65535 & NVal)),
      (this.hash[1] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & SimulationClock) + (65535 & queue)),
      (NVal =
        (65535 & ClockEvent) +
        (65535 & VVal) +
        ((AVal =
          (SimulationClock >>> 16) + (queue >>> 16) + (ptrVal >>> 16)) >>>
          16)),
      (EVal = (ClockEvent >>> 16) + (VVal >>> 16) + (NVal >>> 16)),
      (this.hash[2] = (EVal << 16) | (65535 & NVal)),
      (this.hash[3] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & argVal) + (65535 & zVal)),
      (NVal =
        (65535 & regVal) +
        (65535 & YVal) +
        ((AVal = (argVal >>> 16) + (zVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (regVal >>> 16) + (YVal >>> 16) + (NVal >>> 16)),
      (this.hash[4] = (EVal << 16) | (65535 & NVal)),
      (this.hash[5] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & cfgVal) + (65535 & $Val)),
      (NVal =
        (65535 & RegisterType) +
        (65535 & QVal) +
        ((AVal = (cfgVal >>> 16) + ($Val >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (RegisterType >>> 16) + (QVal >>> 16) + (NVal >>> 16)),
      (this.hash[6] = (EVal << 16) | (65535 & NVal)),
      (this.hash[7] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & offVal) + (65535 & ZVal)),
      (NVal =
        (65535 & hVal) +
        (65535 & JVal) +
        ((AVal = (offVal >>> 16) + (ZVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (hVal >>> 16) + (JVal >>> 16) + (NVal >>> 16)),
      (this.hash[8] = (EVal << 16) | (65535 & NVal)),
      (this.hash[9] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & valVal) + (65535 & eeVal)),
      (NVal =
        (65535 & lenVal) +
        (65535 & jdxVal) +
        ((AVal = (valVal >>> 16) + (eeVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (lenVal >>> 16) + (jdxVal >>> 16) + (NVal >>> 16)),
      (this.hash[10] = (EVal << 16) | (65535 & NVal)),
      (this.hash[11] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & TVal) + (65535 & eiVal)),
      (NVal =
        (65535 & flag) +
        (65535 & etVal) +
        ((AVal = (TVal >>> 16) + (eiVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (flag >>> 16) + (etVal >>> 16) + (NVal >>> 16)),
      (this.hash[12] = (EVal << 16) | (65535 & NVal)),
      (this.hash[13] = (AVal << 16) | (65535 & ptrVal)),
      (ptrVal = (65535 & IVal) + (65535 & enVal)),
      (NVal =
        (65535 & data) +
        (65535 & esVal) +
        ((AVal = (IVal >>> 16) + (enVal >>> 16) + (ptrVal >>> 16)) >>> 16)),
      (EVal = (data >>> 16) + (esVal >>> 16) + (NVal >>> 16)),
      (this.hash[14] = (EVal << 16) | (65535 & NVal)),
      (this.hash[15] = (AVal << 16) | (65535 & ptrVal)));
  }
  digest(cpuVal, tmpVal) {
    let idxVal = digestWordCounts[cpuVal];
    for (let cpuVal = 0; cpuVal < idxVal; cpuVal++)
      tmpVal.setUint32(4 * cpuVal, this.hash[cpuVal]);
  }
  hexDigest(cpuVal) {
    let tmpVal = digestWordCounts[cpuVal],
      idxVal = "";
    for (let cpuVal = 0; cpuVal < tmpVal; cpuVal++)
      idxVal += uint32ToHex(this.hash[cpuVal]);
    return idxVal;
  }
}

