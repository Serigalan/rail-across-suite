/** A JSON file saved by the browser. */
export function download(doc, fileName) {
  const url = URL.createObjectURL(new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' }))
  const a = Object.assign(document.createElement('a'), { href: url, download: fileName })
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 0)
}
