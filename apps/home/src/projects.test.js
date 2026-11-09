import { describe, expect, it } from 'vitest'
import { timetablesFile } from '@rail-across/timetables/exchange'
import { projectDocument } from '@rail-across/core/project'
import { importedFiles } from './projects'

const record = { id: 'p', title: 'Strecke', tracks: [], switches: [], platforms: [] }
const backup = projectDocument(record, { carrySchemas: true })
const timetables = timetablesFile([{ name: 'Takt', data: { trains: [] } }], { project: 'Strecke' })

describe('the files chosen to import', () => {
  it('are told by what they hold: the projects of a backup file, the timetables of a file beside it', () => {
    const { projects, timetables: tt } = importedFiles([
      { name: 'Strecke – Fahrpläne.json', text: JSON.stringify(timetables) },
      { name: 'Strecke – Bestand.railacross.json', text: JSON.stringify(backup) },
    ])
    expect(projects.map(p => p.title)).toEqual(['Strecke'])
    expect(tt).toEqual([{ name: 'Takt', data: { trains: [] } }])
  })

  it('refuse a file that is neither, or timetables of another version', () => {
    const code = (files) => { try { importedFiles(files); return null } catch (err) { return err.code } }
    expect(code([{ name: 'x.json', text: '{' }])).toBe('import_invalid')
    expect(code([{ name: 'x.json', text: JSON.stringify({ ...timetables, version: '9.0' }) }])).toBe('timetables_version')
  })

  it('refuse an oltjson-1.1 file: the one format is infra-data-1.0 (decision 0021)', () => {
    const oltjson = { format: 'oltjson', version: '1.1', projects: [record] }
    const code = (files) => { try { importedFiles(files); return null } catch (err) { return err.code } }
    expect(code([{ name: 'Strecke.railacross.json', text: JSON.stringify(oltjson) }])).toBe('import_version')
  })
})
