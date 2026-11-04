// Binary53 toward-zero arithmetic for the source's _controlfp(10000,30000)
// and _controlfp(300,300). All helpers operate on finite normal values in the
// bounded cloth domain; no native code is executed. Round-to-nearest is used
// internally only for exact compensated residuals, never as a hidden tolerance.
const storage64 = new ArrayBuffer(8);
const values64 = new Float64Array(storage64);
const words64 = new Uint32Array(storage64);
const storage32 = new ArrayBuffer(4);
const values32 = new Float32Array(storage32);
const words32 = new Uint32Array(storage32);
const SPLITTER = 134217729;

function nextTowardZero(value) {
  if (value === 0) return value;
  values64[0] = value;
  if (words64[0] === 0) words64[1]--;
  words64[0]--;
  return values64[0];
}

function productError(a, b, product) {
  const sa = SPLITTER * a, sb = SPLITTER * b;
  const ah = sa - (sa - a), bh = sb - (sb - b);
  const al = a - ah, bl = b - bh;
  return ((ah * bh - product) + ah * bl + al * bh) + al * bl;
}

function additionError(a, b, sum) {
  const fractionB = sum - a;
  return (a - (sum - fractionB)) + (b - fractionB);
}

function truncateRounded(value, error) {
  return (value > 0 && error < 0) || (value < 0 && error > 0) ? nextTowardZero(value) : value;
}

export function store32(value) {
  values32[0] = value;
  if (Math.abs(values32[0]) > Math.abs(value)) words32[0]--;
  return values32[0];
}

export function add(a, b) {
  const sum = a + b;
  return truncateRounded(sum, additionError(a, b, sum));
}

export function sub(a, b) { return add(a, -b); }

export function mul(a, b) {
  const product = a * b;
  return truncateRounded(product, productError(a, b, product));
}

export function div(a, b) {
  if (b === 0) throw new RangeError('Zero source arithmetic divisor');
  const quotient = a / b;
  const product = quotient * b;
  const residual = (a - product) - productError(quotient, b, product);
  return truncateRounded(quotient, residual * Math.sign(b));
}

export function sqrt(value) {
  if (value < 0) throw new RangeError('Negative source arithmetic square root');
  const root = Math.sqrt(value);
  const square = root * root;
  const residual = (value - square) - productError(root, root, square);
  return residual < 0 ? nextTowardZero(root) : root;
}

export const CHOP53 = Object.freeze({store32, add, sub, mul, div, sqrt, name: 'binary53-toward-zero'});
export const NEAREST53 = Object.freeze({store32: Math.fround, add: (a,b)=>a+b, sub: (a,b)=>a-b, mul: (a,b)=>a*b, div: (a,b)=>a/b, sqrt: Math.sqrt, name: 'binary53-nearest'});
