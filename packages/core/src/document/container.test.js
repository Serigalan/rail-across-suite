import { describe, it, expect } from 'vitest'
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate'
import { CONTAINER_ENTRY, CONTAINER_TYPE, fileText, isContainer, toContainer } from './container.js'
import { PayloadError } from './payloadError.js'

const file = { format: 'rail-across', version: '1.0', infraData: { tracks: [{ id: 't1', element_type: ['arc', 'straight', 'straight'] }] } }

describe('the container .railacross', () => {
  it('starts with the media type stored at byte 38', () => {
    const bytes = toContainer(file)
    expect(strFromU8(bytes.subarray(0, 2))).toBe('PK')
    expect(bytes[8] | (bytes[9] << 8)).toBe(0) // method: stored
    expect(bytes[28] | (bytes[29] << 8)).toBe(0) // no extra field
    expect(strFromU8(bytes.subarray(30, 38))).toBe('mimetype')
    expect(strFromU8(bytes.subarray(38, 38 + CONTAINER_TYPE.length))).toBe(CONTAINER_TYPE)
    expect(Object.keys(unzipSync(bytes))).toEqual(['mimetype', CONTAINER_ENTRY])
  })

  it('gives back the file byte for byte', () => {
    expect(fileText(toContainer(file))).toBe(JSON.stringify(file))
    expect(fileText(toContainer(file).buffer)).toBe(JSON.stringify(file))
  })

  it('reads a plain JSON file as before', () => {
    const text = JSON.stringify(file, null, 2)
    expect(isContainer(strToU8(text))).toBe(false)
    expect(fileText(strToU8(text))).toBe(text)
  })

  it('takes no other zip for a container, and refuses one without its file', () => {
    const other = zipSync({ [CONTAINER_ENTRY]: strToU8('{}') })
    expect(isContainer(other)).toBe(false)
    const empty = zipSync({ mimetype: [strToU8(CONTAINER_TYPE), { level: 0 }] })
    expect(() => fileText(empty)).toThrow(PayloadError)
  })
})
