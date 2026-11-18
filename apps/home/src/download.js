import { CONTAINER_TYPE, fileText, toContainer } from '@rail-across/core/document'

/** A file rail-across-1.0 saved by the browser, as its container `.railacross` (decision 0002). */
export function download(file, fileName) {
  const url = URL.createObjectURL(new Blob([toContainer(file)], { type: CONTAINER_TYPE }))
  const a = Object.assign(document.createElement('a'), { href: url, download: fileName })
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 0)
}

/** A chosen file's JSON text: the file a container holds, or a JSON file as it is. */
export const chosenText = async (file) => fileText(new Uint8Array(await file.arrayBuffer()))

/** What the file pickers take: containers, and JSON files as before. */
export const FILE_ACCEPT = '.railacross,.json,application/json'
