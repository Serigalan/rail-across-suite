import { describe, it, expect } from 'vitest'
import { signalSymbolOf, symbolLength } from './symbols.js'

const screen = [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }] }]
const signal = (...logicals) => ({ id: 's', kind: 'real', logicalSignals: logicals.map(([type, fn = 'entry']) => ({ system: 'ks', type, function: fn, frames: screen })) })
const board = (aspect, symbol) => ({ id: 'b', kind: 'real', frames: [{ kind: 'plate', aspects: [{ aspect, ...(symbol ? { symbol } : {}) }] }] })

describe('the symbols of the schematic view', () => {
  it('draws a signal by its strongest screen', () => {
    expect(signalSymbolOf(signal(['main']))).toMatchObject({ kind: 'signal', family: 'main', shunt: false, open: false })
    expect(signalSymbolOf(signal(['multiSection']))).toMatchObject({ family: 'multi' })
    expect(signalSymbolOf(signal(['distant']))).toMatchObject({ family: 'distant' })
    expect(signalSymbolOf(signal(['shunting']))).toMatchObject({ family: 'shunting', shunt: false })
  })

  it('marks a main shunting signal, and draws a distant repeater as a distant signal', () => {
    expect(signalSymbolOf(signal(['mainShunting']))).toMatchObject({ family: 'main', shunt: true })
    expect(signalSymbolOf(signal(['distantRepeater']))).toMatchObject({ family: 'distant', shunt: false })
  })

  it('fills every screen, as the domain does not say who works a signal', () => {
    expect(signalSymbolOf(signal(['multiSection', 'block']))).toMatchObject({ open: false })
  })

  it('draws a board by the aspect it shows, with its figure', () => {
    expect(signalSymbolOf(board('Ne_1'))).toEqual({ kind: 'board', shape: 'trapezoid', text: '', hanging: false })
    expect(signalSymbolOf(board('Lf_6', '6'))).toMatchObject({ kind: 'board', shape: 'triangle', text: '6' })
    expect(signalSymbolOf(board('Lf_2'))).toMatchObject({ kind: 'board', shape: 'square', text: 'A' })
    expect(signalSymbolOf(board('So_20'))).toEqual({ kind: 'board', shape: 'plate', text: 'So 20' })
  })

  it('hangs the ETCS stop marker where it is on a bracket or gantry', () => {
    expect(signalSymbolOf(board('Ne_14'))).toMatchObject({ shape: 'etcsStop', hanging: false })
    expect(signalSymbolOf({ ...board('Ne_14'), mounting: 'gantry' })).toMatchObject({ shape: 'etcsStop', hanging: true })
  })

  it('knows a virtual signal and how long each symbol is', () => {
    expect(signalSymbolOf({ id: 'v', kind: 'virtual' })).toEqual({ kind: 'virtual' })
    expect(symbolLength(signalSymbolOf(signal(['multiSection'])))).toBeGreaterThan(symbolLength(signalSymbolOf(signal(['main']))))
  })
})
