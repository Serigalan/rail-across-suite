#!/usr/bin/env node
// The app icons from the logo (icons/*.svg): icon-small.svg for 16–64 px,
// where the words would not read, icon.svg from 128 px on — rasterized in
// headless Chrome (RA_CHROME, default google-chrome), packed into icon.ico and
// icon.icns as PNG entries. logo.svg is the logo itself (the start screen).
//
//   node scripts/icons.mjs
import { execFileSync } from 'node:child_process'
import { readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
const out = resolve(fileURLToPath(new URL('.', import.meta.url)), '../icons')
const SIZES = { 'icon-small.svg': [16, 24, 32, 48, 64], 'icon.svg': [128, 256, 512, 1024] }
const jobs = Object.entries(SIZES).flatMap(([f, ss]) => ss.map(s => [f, s, readFileSync(join(out, f), 'utf8')]))
const page = `<!doctype html><body><script>
const jobs = ${JSON.stringify(jobs)}
Promise.all(jobs.map(([f, size, svg]) => new Promise(ok => { const img = new Image()
  img.onload = () => { const c = document.createElement('canvas'); c.width = c.height = size
    c.getContext('2d').drawImage(img, 0, 0, size, size); ok([f, size, c.toDataURL('image/png')]) }
  img.src = 'data:image/svg+xml;base64,' + btoa(svg) }))).then(r => { const p = document.createElement('pre'); p.id = 'out'; p.textContent = JSON.stringify(r); document.body.append(p) })
</script>`
const tmp = join(tmpdir(), `ra-icons-${process.pid}.html`)
writeFileSync(tmp, page)
const dom = execFileSync(process.env.RA_CHROME ?? 'google-chrome', ['--headless=new', '--no-sandbox', '--disable-gpu', '--virtual-time-budget=5000', '--dump-dom', 'file://' + tmp], { encoding: 'utf8', maxBuffer: 1 << 28, stdio: ['ignore', 'pipe', 'ignore'] })
const json = dom.match(/<pre id="out">([\s\S]*?)<\/pre>/)[1].replace(/&quot;/g, '"').replace(/&amp;/g, '&')
const png = {}
for (const [, size, url] of JSON.parse(json)) png[size] = Buffer.from(url.split(',')[1], 'base64')
const save = (name, size) => writeFileSync(join(out, name), png[size])
save('32x32.png', 32); save('128x128.png', 128); save('128x128@2x.png', 256); save('icon.png', 512)
// ICO: PNG entries (Windows Vista and later)
const icoSizes = [16, 24, 32, 48, 64, 256]
const head = Buffer.alloc(6 + 16 * icoSizes.length)
head.writeUInt16LE(0, 0); head.writeUInt16LE(1, 2); head.writeUInt16LE(icoSizes.length, 4)
let off = head.length
icoSizes.forEach((s, i) => { const e = 6 + 16 * i
  head.writeUInt8(s >= 256 ? 0 : s, e); head.writeUInt8(s >= 256 ? 0 : s, e + 1)
  head.writeUInt16LE(1, e + 4); head.writeUInt16LE(32, e + 6)
  head.writeUInt32LE(png[s].length, e + 8); head.writeUInt32LE(off, e + 12); off += png[s].length })
writeFileSync(join(out, 'icon.ico'), Buffer.concat([head, ...icoSizes.map(s => png[s])]))
// ICNS: PNG entries
const types = [['icp4', 16], ['icp5', 32], ['icp6', 64], ['ic07', 128], ['ic08', 256], ['ic09', 512], ['ic10', 1024]]
const parts = types.map(([t, s]) => { const h = Buffer.alloc(8); h.write(t, 0, 'ascii'); h.writeUInt32BE(8 + png[s].length, 4); return Buffer.concat([h, png[s]]) })
const body = Buffer.concat(parts), ih = Buffer.alloc(8); ih.write('icns', 0, 'ascii'); ih.writeUInt32BE(8 + body.length, 4)
writeFileSync(join(out, 'icon.icns'), Buffer.concat([ih, body]))
rmSync(tmp)
console.log(`icons written to ${out}`)
