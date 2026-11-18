import { CONTAINER_TYPE, fileText, toContainer } from '@rail-across/core/document'

/** Hand a blob to the browser as a download. */
export function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob)
  const a   = document.createElement('a')
  a.href     = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

export function downloadJSON(data, filename) {
  const content = typeof data === 'string' ? data : JSON.stringify(data)
  downloadBlob(new Blob([content], { type: 'application/json' }), filename)
}

/** Download a file rail-across-1.0 as its container `.railacross` (decision 0002). */
export function downloadContainer(file, filename) {
  downloadBlob(new Blob([toContainer(file)], { type: CONTAINER_TYPE }), filename)
}

/** Download a text file — an SVG sheet, say. */
export function downloadText(text, filename, type = 'text/plain') {
  downloadBlob(new Blob([text], { type }), filename)
}

/** A chosen file's text. */
export const readFileText = (file) => file.text()

/** A chosen file's JSON text: the file a container `.railacross` holds, or a JSON file as it is. */
export const readProjectFileText = async (file) => fileText(new Uint8Array(await file.arrayBuffer()))

/** A chosen file's bytes. */
export const readFileBuffer = (file) => file.arrayBuffer()
