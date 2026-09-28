// Generates a 1024x1024 PNG app icon (gradient + "F") without any
// dependencies, so `npx tauri icon` can derive every platform icon.
// Run: node scripts/gen-icon.mjs [output.png]
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const S = 1024;

// --- minimal PNG encoder ------------------------------------------------
const CRC_TABLE = new Int32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  CRC_TABLE[n] = c;
}
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
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
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  const raw = Buffer.alloc(h * (1 + w * 4));
  for (let y = 0; y < h; y++) {
    raw[y * (1 + w * 4)] = 0; // filter: none
    rgba.copy(raw, y * (1 + w * 4) + 1, y * w * 4, (y + 1) * w * 4);
  }
  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// --- drawing -------------------------------------------------------------
const px = Buffer.alloc(S * S * 4);
const R = 180; // corner radius

function inRoundRect(x, y) {
  const cx = Math.min(Math.max(x, R), S - R);
  const cy = Math.min(Math.max(y, R), S - R);
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= R * R;
}

// "F" glyph (white), centered, thickness t
const t = 96;
const fx0 = 330;
const fy0 = 190;
const fw = 364; // width of horizontals
const fh = 644; // height of the stem
function inF(x, y) {
  if (x < fx0 - t || x > fx0 + t) return false; // stem column
  const stem = x >= fx0 - t && x <= fx0 + t && y >= fy0 && y <= fy0 + fh;
  const top = y >= fy0 && y <= fy0 + t && x >= fx0 - t && x <= fx0 - t + fw;
  const mid = y >= fy0 + 240 && y <= fy0 + 240 + t && x >= fx0 - t && x <= fx0 - t + 320;
  return stem || top || mid;
}

for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const i = (y * S + x) * 4;
    if (!inRoundRect(x, y)) {
      px[i + 3] = 0;
      continue;
    }
    // diagonal gradient: indigo -> sky
    const g = (x + y) / (2 * S);
    px[i] = Math.round(79 + (14 - 79) * g); // R: #4f46e5 -> #0ea5e9
    px[i + 1] = Math.round(70 + (165 - 70) * g);
    px[i + 2] = Math.round(229 + (233 - 229) * g);
    px[i + 3] = 255;
    if (inF(x, y)) {
      px[i] = 255;
      px[i + 1] = 255;
      px[i + 2] = 255;
    }
  }
}

const out = process.argv[2] ?? "icon.png";
writeFileSync(out, encodePng(px, S, S));
console.log(`wrote ${out} (${S}x${S})`);
