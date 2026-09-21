import { byteToHex } from "../../peripherals/common/helpers.js";

function toBigInt(value) {
  return typeof value === "number" ? BigInt(value) : value;
}

export function iabs(value) {
  return value >= 0 ? value : -value;
}

// Extended Euclidean algorithm. Returns { g: gcd(a,b), x, y } with a*x + b*y = g.
export function extendedGcd(a, b) {
  a = toBigInt(a);
  b = toBigInt(b);
  if (a <= 0n || b <= 0n) throw RangeError("a and b MUST be > 0");
  let x0 = 0n;
  let x1 = 1n;
  let y0 = 1n;
  let y1 = 0n;
  while (a !== 0n) {
    const q = b / a;
    const r = b % a;
    const x0Next = x0 - x1 * q;
    const y0Next = y0 - y1 * q;
    b = a;
    a = r;
    x0 = x1;
    y0 = y1;
    x1 = x0Next;
    y1 = y0Next;
  }
  return { g: b, x: x0, y: y0 };
}

// Positive modulo: result in [0, m).
export function modPositive(value, m) {
  const v = toBigInt(value);
  const mod = toBigInt(m);
  if (mod <= 0n) throw RangeError("modulus must be > 0");
  const r = v % mod;
  return r < 0n ? r + mod : r;
}

export function modInverse(value, m) {
  const { g, x } = extendedGcd(modPositive(value, m), m);
  if (g !== 1n)
    throw RangeError(
      `${value.toString()} does not have inverse modulo ${m.toString()}`,
    );
  return modPositive(x, m);
}

export function chineseRemainderTheorem(remainders, modulos, product) {
  if (remainders.length !== modulos.length)
    throw RangeError(
      "The remainders and modulos arrays should have the same length",
    );
  const M = product ?? modulos.reduce((acc, m) => acc * m, 1n);
  return modulos.reduce((acc, m, i) => {
    const Mi = M / m;
    return modPositive(
      acc + (((Mi * modInverse(Mi, m)) % M) * remainders[i]) % M,
      M,
    );
  }, 0n);
}

export function modProduct(values, m) {
  const mod = BigInt(m);
  return modPositive(
    values.map((v) => BigInt(v) % mod).reduce((acc, v) => (acc * v) % mod, 1n),
    mod,
  );
}

export function eulerTotient(factors) {
  return factors
    .map(([p, k]) => p ** (k - 1n) * (p - 1n))
    .reduce((acc, v) => v * acc, 1n);
}

// Build [[prime, exponent], ...] from a flat list of primes or [prime, exp] pairs.
function factorCounts(factors) {
  const map = {};
  for (const f of factors) {
    if (typeof f === "bigint" || typeof f === "number") {
      const key = String(f);
      if (map[key] === undefined) map[key] = { p: BigInt(f), k: 1n };
      else map[key].k += 1n;
    } else {
      const key = String(f[0]);
      if (map[key] === undefined) map[key] = { p: BigInt(f[0]), k: BigInt(f[1]) };
      else map[key].k += BigInt(f[1]);
    }
  }
  return Object.values(map).map((e) => [e.p, e.k]);
}

export function modPow(base, exp, modulus, factors) {
  base = toBigInt(base);
  exp = toBigInt(exp);
  modulus = toBigInt(modulus);
  if (modulus <= 0n) throw RangeError("modulus must be > 0");
  if (modulus === 1n) return 0n;
  base = modPositive(base, modulus);
  if (exp < 0n) return modInverse(modPow(base, iabs(exp), modulus, factors), modulus);
  if (factors !== undefined) {
    const factorList = factorCounts(factors);
    const primePows = factorList.map(([p, k]) => p ** k);
    const phiList = factorList.map(([p, k]) => eulerTotient([[p, k]]));
    const remainders = factorList.map((_, i) =>
      modPow(base, exp % phiList[i], primePows[i]),
    );
    return chineseRemainderTheorem(remainders, primePows, modulus);
  }
  let result = 1n;
  let b = base;
  let e = exp;
  while (e > 0n) {
    if (e % 2n === 1n) result = (result * b) % modulus;
    e /= 2n;
    b = (b * b) % modulus;
  }
  return result;
}

export function bytesToBigInt(bytes) {
  return BigInt(
    "0x" +
      Array.from(bytes)
        .reverse()
        .map((b) => byteToHex(b))
        .join(""),
  );
}

export function bigIntToBytes(out, value, length) {
  const hex = value.toString(16);
  let len = hex.length;
  for (let i = 0; i < length; i++, len -= 2) {
    if (len > 1) out[i] = parseInt(hex.substring(len - 2, len), 16);
    else if (len === 1) out[i] = parseInt(hex.substring(len - 1, len), 16);
    else out[i] = 0;
  }
}

export function modInverseAlt(a, m) {
  a = ((BigInt(a) % m) + m) % m;
  if (a === 0n || m < 2n) return 0n;
  const steps = [];
  let x = a;
  let b = m;
  while (b) {
    [x, b] = [b, x % b];
    steps.push({ a: x, b });
  }
  if (x !== 1n) return 0n;
  let inv = 1n;
  let prev = 0n;
  for (let i = steps.length - 2; i >= 0; i--)
    [inv, prev] = [prev, inv - prev * (steps[i].a / steps[i].b)];
  return (prev % m + m) % m;
}
