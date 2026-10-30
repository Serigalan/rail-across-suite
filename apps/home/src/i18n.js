import { createContext, useContext } from 'react'

// The start page's texts, German and English — as the Open-Layout-Tool has
// them. The language is the one chosen here before, else the browser's.

const de = {
  suite: 'Rail-Across-Suite',
  tagline: 'Eine gemeinsame Beschreibung der Eisenbahn — entworfen, geplant und simuliert in einem Werkzeugkasten.',
  editors: 'Editoren',
  simulations: 'Simulationen',
  available: 'Verfügbar',
  planned: 'In Planung',
  phase: 'Roadmap, Phase {n}',
  back: 'Zur Übersicht',

  module_infrastructure: 'Infrastruktur-Editor',
  module_infrastructure_desc: 'Gleise, Weichen, Bahnsteige — Signale und Oberleitung darauf.',
  module_vehicles: 'Fahrzeug-Editor',
  module_vehicles_desc: 'Triebfahrzeuge und Wagen, Zugverbände, ihre Fahrdynamik.',
  module_timetable: 'Fahrplan-Editor',
  module_timetable_desc: 'Zugfahrten, Halte und Fahrzeiten auf der geplanten Infrastruktur.',
  module_timetableSim: 'Fahrplan-Sim',
  module_timetableSim_desc: 'Den Fahrplan durchfahren: Fahrzeiten, Anschlüsse, Verspätungen.',
  module_capacitySim: 'Kapazitäts-Sim',
  module_capacitySim_desc: 'Wie viele Züge eine Strecke trägt — gerechnet mit OSRD.',
  module_constructionSim: 'Baubetriebs-Sim',
  module_constructionSim_desc: 'Bauzustände, Sperrungen und Langsamfahrstellen, Zustand für Zustand.',
  planned_text: 'Dieses Modul gibt es noch nicht. Es liest und schreibt dieselbe Beschreibung der Eisenbahn wie alle anderen, sobald es da ist.',

  tool_layout: 'Layout-Tool',
  tool_layout_desc: 'Trassierung: Gleise, Weichen, Höhen, Bahnsteige',
  tool_signalling: 'Signalling-Tool',
  tool_signalling_desc: 'Signale, Gleisfreimeldung, Fahrstraßen',
  tool_catenary: 'Catenary-Tool',
  tool_catenary_desc: 'Maste, Ausleger, Fahrdraht, Schaltabschnitte',
  tool_soon: 'kommt in Phase {n}',
  opens_in: 'Projekte öffnen im {tool}',
  opens_not_yet: 'Das {tool} gibt es noch nicht — Projekte lassen sich darin noch nicht öffnen.',

  projects: 'Projekte',
  projects_loading: 'Projekte werden geladen …',
  projects_empty: 'Noch keine Projekte. Ein neues legst du im Layout-Tool an.',
  projects_error: 'Die Projekte ließen sich nicht laden.',
  projects_manage: 'Projekte verwalten',
  revision: 'Rev. {n}',
  open: 'Öffnen',

  sign_in: 'Anmelden',
  sign_in_hint: 'Melde dich an, um die Projekte zu sehen.',
  sign_out: 'Abmelden',
  login: 'Benutzername',
  password: 'Passwort',
  signing_in: 'Anmelden …',
  invalid_credentials: 'Benutzername oder Passwort stimmt nicht.',
  too_many_attempts: 'Zu viele Versuche — bitte warte einen Moment.',
  sign_in_failed: 'Die Anmeldung hat nicht geklappt.',
  must_change_password: 'Dein Passwort muss geändert werden, bevor es weitergeht.',
  change_password: 'Passwort ändern',
  language: 'Sprache',
}

const en = {
  suite: 'Rail-Across-Suite',
  tagline: 'One shared description of the railway — designed, planned and simulated in one toolkit.',
  editors: 'Editors',
  simulations: 'Simulations',
  available: 'Available',
  planned: 'Planned',
  phase: 'Roadmap, phase {n}',
  back: 'Back to the overview',

  module_infrastructure: 'Infrastructure editor',
  module_infrastructure_desc: 'Tracks, switches, platforms — signals and catenary on them.',
  module_vehicles: 'Rolling stock editor',
  module_vehicles_desc: 'Locomotives and coaches, train formations, their dynamics.',
  module_timetable: 'Timetable editor',
  module_timetable_desc: 'Train runs, stops and running times on the planned infrastructure.',
  module_timetableSim: 'Timetable sim',
  module_timetableSim_desc: 'Run the timetable: running times, connections, delays.',
  module_capacitySim: 'Capacity sim',
  module_capacitySim_desc: 'How many trains a line carries — computed with OSRD.',
  module_constructionSim: 'Construction sim',
  module_constructionSim_desc: 'Construction stages, closures and speed restrictions, stage by stage.',
  planned_text: 'This module does not exist yet. Once it does, it reads and writes the same description of the railway as all the others.',

  tool_layout: 'Layout-Tool',
  tool_layout_desc: 'Alignment: tracks, switches, heights, platforms',
  tool_signalling: 'Signalling-Tool',
  tool_signalling_desc: 'Signals, train detection, routes',
  tool_catenary: 'Catenary-Tool',
  tool_catenary_desc: 'Masts, cantilevers, contact wire, sections',
  tool_soon: 'comes in phase {n}',
  opens_in: 'Projects open in the {tool}',
  opens_not_yet: 'The {tool} does not exist yet — projects cannot be opened in it.',

  projects: 'Projects',
  projects_loading: 'Loading the projects …',
  projects_empty: 'No projects yet. Create one in the Layout-Tool.',
  projects_error: 'The projects could not be loaded.',
  projects_manage: 'Manage projects',
  revision: 'Rev. {n}',
  open: 'Open',

  sign_in: 'Sign in',
  sign_in_hint: 'Sign in to see the projects.',
  sign_out: 'Sign out',
  login: 'User name',
  password: 'Password',
  signing_in: 'Signing in …',
  invalid_credentials: 'User name or password is wrong.',
  too_many_attempts: 'Too many attempts — please wait a moment.',
  sign_in_failed: 'Signing in did not work.',
  must_change_password: 'Your password has to be changed before you go on.',
  change_password: 'Change password',
  language: 'Language',
}

export const TEXTS = { de, en }
export const LANGUAGES = ['de', 'en']
const KEY = 'rail-across-language'

/** The language chosen before, else the browser's (German where it asks for German). */
export function initialLanguage() {
  try {
    const kept = localStorage.getItem(KEY)
    if (LANGUAGES.includes(kept)) return kept
  } catch { /* no storage: the browser's language */ }
  const asked = globalThis.navigator?.language ?? 'de'
  return asked.toLowerCase().startsWith('de') ? 'de' : 'en'
}

export function keepLanguage(language) {
  try { localStorage.setItem(KEY, language) } catch { /* not kept: chosen again next time */ }
}

/** t(key) and fill(key, { name: value }) for a language. */
export function textsFor(language) {
  const table = TEXTS[language] ?? de
  const t = (key) => table[key] ?? de[key] ?? key
  const fill = (key, params) => t(key).replace(/\{(\w+)\}/g, (_, name) => String(params?.[name] ?? ''))
  return { language, t, fill }
}

export const I18n = createContext(textsFor('de'))
export const useTexts = () => useContext(I18n)
