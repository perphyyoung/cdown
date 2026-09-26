// 占位图标生成器：1024×1024 深色圆角方块 + 白环 + 红色进度弧 + 指针。
// 用法：node scripts/gen-icon.mjs → 生成 app-icon.png，再 pnpm tauri icon app-icon.png。
// 正式图标可直接替换 app-icon.png 后重跑 pnpm tauri icon。
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import path from "node:path";

const S = 1024;
const C = S / 2;
const CORNER = 180;
const RING_OUT = 380;
const RING_IN = 320;
const BG = [15, 23, 42]; // slate-900
const WHITE = [226, 232, 240];
const RED = [239, 68, 68];

const px = Buffer.alloc(S * S * 4, 0);

function blend(i, [r, g, b], a) {
  px[i] = Math.round(px[i] * (1 - a) + r * a);
  px[i + 1] = Math.round(px[i + 1] * (1 - a) + g * a);
  px[i + 2] = Math.round(px[i + 2] * (1 - a) + b * a);
  px[i + 3] = Math.max(px[i + 3], Math.round(a * 255));
}

for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const i = (y * S + x) * 4;
    // 圆角方块遮罩（带 1px 抗锯齿）
    const dx = Math.max(Math.abs(x - C) - (C - CORNER), 0);
    const dy = Math.max(Math.abs(y - C) - (C - CORNER), 0);
    const edge = Math.hypot(dx, dy) - CORNER;
    const mask = Math.max(0, Math.min(1, 0.5 - edge));
    if (mask <= 0) continue;

    const dist = Math.hypot(x - C, y - C);
    // 指针：中心 → 12 点方向的竖条
    if (Math.abs(x - C) <= 16 && y >= C - (RING_IN - 20) && y <= C) {
      blend(i, WHITE, mask);
      continue;
    }
    if (dist <= 26) {
      blend(i, WHITE, mask); // 中心点
      continue;
    }
    if (dist <= RING_OUT && dist >= RING_IN) {
      // 红色弧：从 12 点顺时针 5/6 圈（屏幕坐标 y 向下，顺时针即 atan2 增大）
      let rel = Math.atan2(y - C, x - C) + Math.PI / 2;
      if (rel < 0) rel += Math.PI * 2;
      blend(i, rel <= (Math.PI * 5) / 6 ? RED : WHITE, mask);
      continue;
    }
    blend(i, BG, mask);
  }
}

// —— 最小 PNG 编码（RGBA8，filter 0）——
function crc32(buf) {
  let c = ~0;
  for (const byte of buf) {
    c ^= byte;
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return ~c >>> 0;
}

function chunk(type, data) {
  const out = Buffer.alloc(8 + data.length + 4);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, "ascii");
  data.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA

const raw = Buffer.alloc(S * (S * 4 + 1));
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0; // filter none
  px.copy(raw, y * (S * 4 + 1) + 1, y * S * 4, (y + 1) * S * 4);
}

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const out = path.join(import.meta.dirname, "..", "app-icon.png");
writeFileSync(out, png);
console.log(`已生成 ${out}（${S}×${S}）`);
