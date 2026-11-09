import { describe, expect, it } from 'vitest'
import { derailerOperationOptions, operationWord, pointKindOptions, pointKindWord, signalTypeOptions } from './labels.js'

const values = (options) => options.map(o => o.value)
const t = (key) => key

describe('what the editors offer to choose', () => {
  it('works a derailer remote, local or not at all — the domain\'s electric ones, as the export writes them', () => {
    expect(derailerOperationOptions('de', 'electricRemote').map(o => [o.value, o.label])).toEqual([
      ['electricRemote', 'ferngestellt'], ['electricLocal', 'ortsgestellt'], ['notWorked', 'nicht stellbar'],
    ])
    expect(operationWord({ operation: 'electricLocal', derailer: {} }, 'de')).toBe('ortsgestellt')
    expect(operationWord({ operation: 'electricLocal', switchId: 's' }, 'de')).toBe('elektrisch ortsgestellt')
  })

  it('sets a detection point as an axle counter or an insulated joint, the two-rail one', () => {
    expect(pointKindOptions('de', 'axleCounter').map(o => [o.value, o.label])).toEqual([
      ['axleCounter', 'Achszähler'], ['insulatedJointTwoRail', 'Isolierstoß'],
    ])
    expect(pointKindWord('insulatedJointTwoRail', 'de')).toBe('Isolierstoß')
  })

  it('gives a Ks signal the main, multi-section and distant kinds and the repeater, no shunting variant', () => {
    expect(values(signalTypeOptions('ks', 'de', t, 'main'))).toEqual(['main', 'multiSection', 'distant', 'distantRepeater'])
    expect(values(signalTypeOptions('sh', 'de', t, 'shunting'))).toEqual(['shunting', 'other'])
  })

  it('keeps what a read-in object has where it is none of these, in its full words', () => {
    expect(derailerOperationOptions('de', 'mechanicalLocal').at(-1)).toEqual({ value: 'mechanicalLocal', label: 'mechanisch ortsgestellt' })
    expect(pointKindOptions('de', 'electricalJoint').at(-1)).toEqual({ value: 'electricalJoint', label: 'Elektrischer Trennstoß' })
    expect(signalTypeOptions('ks', 'de', t, 'mainShunting').at(-1)).toEqual({ value: 'mainShunting', label: 'Ks-Hauptsperrsignal' })
  })
})
