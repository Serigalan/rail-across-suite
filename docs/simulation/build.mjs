#!/usr/bin/env node
// The assumptions of the simulation (docs/simulation), in English and in its German translation, each as Typst and
// as Markdown, from the main text text/<lang>.md — the companion of the format description rail-across-1.0
// (docs/format): that one says what is stored, this one what the tools compute from it. Run again whenever a text
// changes.
//
//   node docs/simulation/build.mjs           write simulation-assumptions.{typ,md} and simulation-assumptions.de.{typ,md}
//   node docs/simulation/build.mjs --check   exit 1 where a file is not what the sources make
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { mainMd, mainTyp, preambleTyp, writeOrCheck } from '../markup.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const DATE = { en: '16 November 2026', de: '16. November 2026', short: { en: '2026-11-16', de: '16.11.2026' } }
const SOURCE = (lang) => `docs/simulation/build.mjs from text/${lang}.md`

const WORDS = {
  en: {
    title: 'Assumptions of the simulation', subtitle: 'How the Rail-Across-Suite runs trains, holds blocks, works out capacity and tries construction scenarios',
    docTitle: 'Rail-Across-Suite — Assumptions of the simulation', status: `As of ${DATE.en} · companion of rail-across-1.0`,
    header: `As of ${DATE.short.en}`, lang: 'en',
  },
  de: {
    title: 'Annahmen der Simulation', subtitle: 'Wie die Rail-Across-Suite Züge fährt, Blöcke belegt, Kapazität rechnet und Bauszenarien prüft',
    docTitle: 'Rail-Across-Suite — Annahmen der Simulation', status: `Stand ${DATE.de} · Begleitdokument zu rail-across-1.0 · Übersetzung des englischen Originals`,
    header: `Stand ${DATE.short.de}`, lang: 'de',
  },
}

const files = {}
for (const [lang, base] of [['en', 'simulation-assumptions'], ['de', 'simulation-assumptions.de']]) {
  const W = WORDS[lang]
  const main = readFileSync(join(HERE, 'text', `${lang}.md`), 'utf8')
  files[`${base}.typ`] = `${preambleTyp(W, SOURCE(lang))}\n\n${mainTyp(main)}`.replace(/\n{3,}/g, '\n\n').trimEnd() + '\n'
  files[`${base}.md`] = mainMd(W, SOURCE(lang), main).replace(/\n{3,}/g, '\n\n').trimEnd() + '\n'
}
process.exitCode = writeOrCheck(HERE, 'docs/simulation', 'docs/simulation/build.mjs', files, process.argv[2] === '--check') ? 1 : 0
