import { sha256Hex, randomHex } from "./password";

/** 生成默认头像用的随机 seed */
export function generateAvatarSeed(): string {
  return randomHex(8);
}

function hashBytes(input: string): number[] {
  // 同步轻量哈希：用 seed 的 hex 推导多组字节（确定性）
  // 外部再结合 sha256 可选；这里用 FNV 扩展保证 Worker 无 async 依赖即可出图
  let h1 = 0x811c9dc5;
  let h2 = 0x01000193;
  const out: number[] = [];
  const src = input + "|avatar";
  for (let i = 0; i < src.length; i++) {
    h1 ^= src.charCodeAt(i);
    h1 = Math.imul(h1, 0x01000193) >>> 0;
    h2 = (h2 + src.charCodeAt(i) * (i + 7)) >>> 0;
  }
  for (let i = 0; i < 16; i++) {
    h1 ^= h1 << 13;
    h1 >>>= 0;
    h1 ^= h1 >>> 17;
    h1 ^= h1 << 5;
    h1 >>>= 0;
    h2 = (Math.imul(h2, 1664525) + 1013904223) >>> 0;
    out.push((h1 ^ h2) & 0xff);
  }
  return out;
}

function hsl(h: number, s: number, l: number): string {
  const H = ((h % 360) + 360) % 360;
  const S = Math.min(100, Math.max(0, s)) / 100;
  const L = Math.min(100, Math.max(0, l)) / 100;
  const a = S * Math.min(L, 1 - L);
  const f = (n: number) => {
    const k = (n + H / 30) % 12;
    const value = L - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
    const byte = Math.round(255 * Math.min(1, Math.max(0, value)));
    return byte.toString(16).padStart(2, "0");
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

/**
 * 根据 seed 生成 SVG identicon（5 列对称九宫风格）。
 * 同一 seed 始终得到同一图像。
 */
export function renderIdenticonSvg(seed: string, size = 128): string {
  const bytes = hashBytes(seed || "anonymous");
  const bgH = bytes[0] % 360;
  const fgH = (bgH + 140 + (bytes[1] % 80)) % 360;
  const bg = hsl(bgH, 28, 93);
  const fg = hsl(fgH, 55, 42);
  const fg2 = hsl((fgH + 20) % 360, 45, 58);

  const cells: string[] = [];
  const unit = 20;
  // 5x5，左右对称
  for (let y = 0; y < 5; y++) {
    for (let x = 0; x < 3; x++) {
      const bit = bytes[2 + y + x] % 3;
      if (bit === 0) continue;
      const color = bit === 1 ? fg : fg2;
      const x1 = x * unit;
      const x2 = (4 - x) * unit;
      const y1 = y * unit;
      if (x === 2) {
        cells.push(
          `<rect x="${x1}" y="${y1}" width="${unit}" height="${unit}" fill="${color}"/>`
        );
      } else {
        cells.push(
          `<rect x="${x1}" y="${y1}" width="${unit}" height="${unit}" fill="${color}"/>` +
            `<rect x="${x2}" y="${y1}" width="${unit}" height="${unit}" fill="${color}"/>`
        );
      }
    }
  }

  // 中心圆点装饰
  const dot = bytes[9] % 2 === 0;
  const deco = dot
    ? `<circle cx="50" cy="50" r="14" fill="${bg}" opacity="0.55"/>`
    : `<rect x="36" y="36" width="28" height="28" rx="6" fill="${bg}" opacity="0.45"/>`;

  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 100 100" role="img" aria-label="User avatar">` +
    `<rect width="100" height="100" fill="${bg}"/>` +
    cells.join("") +
    deco +
    `</svg>`
  );
}

/** 可选：基于用户 id 的稳定 seed（无自定义 seed 时） */
export async function stableSeedFromId(userId: string): Promise<string> {
  return (await sha256Hex(userId)).slice(0, 16);
}
