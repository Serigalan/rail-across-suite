import { Suspense, lazy } from 'react'

// The panels of the server's MDB conversion as sections in core's data exchange
// (`panelTools`, `importSections`, Paket L). Each is its own chunk (R9.1),
// loaded the first time it is opened.
const MdbSection = lazy(() => import('./MdbSection'))

export function MdbImport({ onReport }) {
  return <Suspense fallback={null}><MdbSection onReport={onReport} /></Suspense>
}

export function MdbDbrefImport({ onReport }) {
  return <Suspense fallback={null}><MdbSection dbref onReport={onReport} /></Suspense>
}
