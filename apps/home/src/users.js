// What the user administration needs besides its page.

/** A start password: 16 characters a person can read out (no 0/O, 1/l/I). */
export function startPassword() {
  const alphabet = 'abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  // Bytes above the last whole multiple of the alphabet are thrown away, so
  // every character is equally likely.
  const limit = 256 - (256 % alphabet.length)
  let out = ''
  while (out.length < 16) {
    for (const b of crypto.getRandomValues(new Uint8Array(32))) {
      if (b < limit && out.length < 16) out += alphabet[b % alphabet.length]
    }
  }
  return out
}

/** A size in bytes as people read it. */
export function sizeText(bytes) {
  const units = ['B', 'kB', 'MB', 'GB', 'TB']
  let v = Number(bytes) || 0, u = 0
  while (v >= 1000 && u < units.length - 1) { v /= 1000; u++ }
  return `${v >= 100 || u === 0 ? Math.round(v) : v.toFixed(1)} ${units[u]}`
}
