import { describe, expect, it } from 'vitest'
import { isTimetablesFile, parseTimetablesFile, timetablesFile, timetablesFileName, timetablesOfText } from './exchange.js'

const listed = [
  { id: 'a', name: 'Takt', version: 3, data: { trains: [{ id: 't1' }], dispatch: { mode: 'wait' } }, mayEdit: true },
  { id: 'b', name: 'Nacht', version: 1, data: { trains: [] } },
]

describe('a file of timetables', () => {
  it('holds their names and data, not what the server keeps besides, and reads back as it went out', () => {
    const file = timetablesFile(listed, { project: 'Halle–Könnern' })
    expect(file).toEqual({
      format: 'rail-across-timetables', version: '1.0', project: 'Halle–Könnern',
      timetables: [{ name: 'Takt', data: { trains: [{ id: 't1' }], dispatch: { mode: 'wait' } } }, { name: 'Nacht', data: { trains: [] } }],
    })
    expect(isTimetablesFile(file)).toBe(true)
    expect(timetablesOfText(JSON.stringify(file))).toEqual(file.timetables)
    expect(timetablesFileName('Halle–Könnern')).toBe('Halle–Könnern – Fahrpläne.json')
  })

  it('is refused where it is another version, another file or not of its shape', () => {
    const file = timetablesFile(listed)
    const code = (f) => { try { f(); return null } catch (err) { return err.code } }
    expect(code(() => parseTimetablesFile({ ...file, version: '2.0' }))).toBe('timetables_version')
    expect(code(() => parseTimetablesFile({ format: 'rail-across', domains: {} }))).toBe('timetables_invalid')
    expect(code(() => parseTimetablesFile({ ...file, timetables: [{ name: ' ', data: { trains: [] } }] }))).toBe('timetables_invalid')
    expect(code(() => parseTimetablesFile({ ...file, timetables: [{ name: 'F', data: {} }] }))).toBe('timetables_invalid')
    expect(code(() => timetablesOfText('{'))).toBe('timetables_invalid')
    expect(isTimetablesFile(null)).toBe(false)
  })
})
