// Generates the fralsare-daily logo: a clean newspaper glyph on a blue
// tile. Dependency-free (hand-rolled PNG encoder), 2x2 supersampled for
// smooth edges. Run: node scripts/gen-logo.mjs [output.png]
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

// --- geometry (all coordinates in 1024 design units) ---------------------
const S = 1024; // output size
const SS = 2; // supersampling factor
const W = S * SS;

const u = (v) => v * SS;

// blue tile (rounded square, iOS-style corner radius)
const TILE_R = u(208);
// stacked pages
const BACK = { x0: 360, y0: 216, x1: 712, y1: 712, r: 40, a: 0.72 }; // white, translucent
const FRONT = { x0: 296, y0: 272, x1: 648, y1: 768, r: 40 }; // white
// page content: headline block + text lines
const HEADLINE = { x0: 340, y0: 316, x1: 604, y1: 404, r: 18 };
const LINES = [
  { x0: 340, y0: 452, x1: 604, y1: 500, r: 24 },
  { x0: 340, y0: 528, x1: 604, y1: 576, r: 24 },
  { x0: 340, y0: 604, x1: 604, y1: 652, r: 24 },
  { x0: 340, y0: 680, x1: 520, y1: 728, r: 24 }, // short last line
];

function inRect(x, y, b) {
  const x0 = u(b.x0), y0 = u(b.y0), x1 = u(b.x1), y1 = u(b.y1), r = u(b.r);
  if (x < x0 || x > x1 || y < y0 || y > y1) return false;
  // which corner is this pixel in?
  const cx = Math.min(Math.max(x, x0 + r), x1 - r);
  const cy = Math.min(Math.max(y, y0 + r), y1 - r);
  const dx = x - cx, dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}

// palette
const TOP = [53, 114, 240]; // #3572f0
const BOTTOM = [29, 78, 216]; // #1d4ed8
const HEADLINE_C = [30, 64, 175]; // #1e40af
const LINE_C = [148, 163, 184]; // #94a3b8

// --- render at supersampled size, then 2x2 box-downsample ----------------
const hi = Buffer.alloc(W * W * 4);
for (let y = 0; y < W; y++) {
  for (let x = 0; x < W; x++) {
    const i = (y * W + x) * 4;
    if (!inRect(x, y, { x0: 0, y0: 0, x1: 1024, y1: 1024, r: 208 })) {
      hi[i + 3] = 0;
      continue;
    }
    // vertical gradient tile
    const g = y / W;
    let r = TOP[0] + (BOTTOM[0] - TOP[0]) * g;
    let gg = TOP[1] + (BOTTOM[1] - TOP[1]) * g;
    let b = TOP[2] + (BOTTOM[2] - TOP[2]) * g;
    let a = 1;
    if (inRect(x, y, BACK)) [r, gg, b, a] = blend(r, gg, b, a, 255, 255, 255, BACK.a);
    if (inRect(x, y, FRONT)) [r, gg, b, a] = blend(r, gg, b, a, 255, 255, 255, 1);
    if (inRect(x, y, HEADLINE)) [r, gg, b, a] = blend(r, gg, b, a, HEADLINE_C[0], HEADLINE_C[1], HEADLINE_C[2], 1);
    for (const L of LINES) {
      if (inRect(x, y, L)) [r, gg, b, a] = blend(r, gg, b, a, LINE_C[0], LINE_C[1], LINE_C[2], 1);
    }
    hi[i] = Math.round(r);
    hi[i + 1] = Math.round(gg);
    hi[i + 2] = Math.round(b);
    hi[i + 3] = Math.round(a * 255);
  }
}

const lo = Buffer.alloc(S * S * 4);
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    let r = 0, g = 0, b = 0, a = 0;
    for (let dy = 0; dy < SS; dy++) {
      for (let dx = 0; dx < SS; dx++) {
        const j = ((y * SS + dy) * W + (x * SS + dx)) * 4;
        const la = hi[j + 3] / 255;
        r += hi[j] * la;
        g += hi[j + 1] * la;
        b += hi[j + 2] * la;
        a += la;
      }
    }
    const n = SS * SS;
    const o = (y * S + x) * 4;
    if (a <= 0) {
      lo[o + 3] = 0;
    } else {
      lo[o] = Math.round(r / a);
      lo[o + 1] = Math.round(g / a);
      lo[o + 2] = Math.round(b / a);
      lo[o + 3] = Math.round((a / n) * 255);
    }
  }
}

// straight alpha (premultiplied already above is intentional: the edges
// keep their background color instead of darkening)
function blend(br, bg, bb, ba, sr, sg, sb, sa) {
  return [sr * sa + br * ba * (1 - sa), sg * sa + bg * ba * (1 - sa), sb * sa + bb * ba * (1 - sa), ba + (1 - ba) * sa];
}

// --- minimal PNG encoder -------------------------------------------------
const CRC_TABLE = new Int32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  CRC_TABLE[n] = c;
}
function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}
function encodePng(rgba, w, h) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc(h * (1 + w * 4));
  for (let y = 0; y < h; y++) {
    raw[y * (1 + w * 4)] = 0;
    rgba.copy(raw, y * (1 + w * 4) + 1, y * w * 4, (y + 1) * w * 4);
  }
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0))]);
}

const out = process.argv[2] ?? "logo.png";
writeFileSync(out, encodePng(lo, S, S));
console.log(`wrote ${out} (${S}x${S})`);
