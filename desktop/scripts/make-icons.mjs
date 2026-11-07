#!/usr/bin/env node
// Generate the app icons of the local package (desktop/src-tauri/icons/) — a
// white track (two rails, sleepers) on the suite's indigo #303383, drawn as
// raw pixels and encoded as PNG, no dependencies. Run once, the PNGs are
// committed:  node scripts/make-icons.mjs
// For .icns/.ico (macOS/Windows bundles) afterwards:  npx tauri icon src-tauri/icons/icon.png
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { deflateSync } from 'node:zlib'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const ICONS = join(HERE, '..', 'src-tauri', 'icons')

const BG = [0x30, 0x33, 0x83, 0xff]   // #303383, the suite's indigo
const FG = [0xff, 0xff, 0xff, 0xff]   // white

const CRC_TABLE = (() => {
  const t = new Int32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    t[n] = c
  }
  return t
})()

const crc32 = (buf) => {
  let c = ~0
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8)
  return ~c >>> 0
}

function chunk(type, data) {
  const out = Buffer.alloc(8 + data.length + 4)
  out.writeUInt32BE(data.length, 0)
  out.write(type, 4, 'ascii')
  data.copy(out, 8)
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length)
  return out
}

function encodePng(width, height, rgba) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8; ihdr[9] = 6 // 8 bit, RGBA
  const raw = Buffer.alloc((width * 4 + 1) * height)
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0 // filter: none
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4)
  }
  return Buffer.concat([sig, chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))])
}

/** The icon at `size` px: an indigo rounded square, a white track across it. */
function draw(size) {
  const px = Buffer.alloc(size * size * 4)
  const set = (x, y, [r, g, b, a]) => {
    if (x < 0 || y < 0 || x >= size || y >= size) return
    const i = (y * size + x) * 4
    px[i] = r; px[i + 1] = g; px[i + 2] = b; px[i + 3] = a
  }
  const radius = size / 6
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      // rounded corners: a pixel in a corner square but outside its circle stays transparent
      const px = x + 0.5, py = y + 0.5
      const inCornerX = px < radius || px > size - radius
      const inCornerY = py < radius || py > size - radius
      if (inCornerX && inCornerY) {
        const dx = px < radius ? radius - px : px - (size - radius)
        const dy = py < radius ? radius - py : py - (size - radius)
        if (Math.hypot(dx, dy) > radius) continue
      }
      set(x, y, BG)
    }
  }
  // the track: two rails and the sleepers between them
  const rail = Math.max(1, Math.round(size * 0.05))
  const y1 = Math.round(size * 0.38), y2 = Math.round(size * 0.62)
  const x1 = Math.round(size * 0.16), x2 = Math.round(size * 0.84)
  for (let x = x1; x <= x2; x++) {
    for (let dy = -Math.floor(rail / 2); dy <= Math.ceil(rail / 2) - 1; dy++) {
      set(x, y1 + dy, FG)
      set(x, y2 + dy, FG)
    }
  }
  const sleeper = Math.max(1, Math.round(size * 0.045))
  for (let x = x1 + Math.round(size * 0.05); x <= x2; x += Math.max(2, Math.round(size * 0.1))) {
    for (let dx = 0; dx < sleeper; dx++) {
      for (let y = y1; y <= y2; y++) set(x + dx, y, FG)
    }
  }
  return px
}

mkdirSync(ICONS, { recursive: true })
const files = { '32x32.png': 32, '128x128.png': 128, '128x128@2x.png': 256, 'icon.png': 512 }
for (const [name, size] of Object.entries(files)) {
  writeFileSync(join(ICONS, name), encodePng(size, size, draw(size)))
  console.log(`icons/${name} (${size}px)`)
}
