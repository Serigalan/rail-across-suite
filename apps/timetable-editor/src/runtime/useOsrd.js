import { useEffect, useState } from 'react'
import { osrdNet } from '@rail-across/running-times'
import { api } from '../api.js'

// OSRD's nets for computing a running time: the list the server holds, and
// one of them loaded and made ready to run over (osrdNet), kept while the page
// is open.

const loaded = new Map()   // id → Promise of the net

/** The nets the server holds: { nets, failed }; none until they are there. */
export function useOsrdNets() {
  const [state, setState] = useState({ nets: [], failed: false })
  useEffect(() => {
    let gone = false
    api.osrdNets().then(nets => { if (!gone) setState({ nets, failed: false }) }, () => { if (!gone) setState({ nets: [], failed: true }) })
    return () => { gone = true }
  }, [])
  return state
}

/** The net `id` ready to run over: { net, loading, error } — nothing asked for where `id` is null. */
export function useOsrdNet(id) {
  const [state, setState] = useState({ id: null, net: null, error: null })
  useEffect(() => {
    if (id == null) return undefined
    let gone = false
    if (!loaded.has(id)) loaded.set(id, api.osrdNet(id).then(osrdNet))
    loaded.get(id).then(
      net => { if (!gone) setState({ id, net, error: null }) },
      err => { loaded.delete(id); if (!gone) setState({ id, net: null, error: err.code ?? String(err.message ?? err) }) },
    )
    return () => { gone = true }
  }, [id])
  const current = id != null && state.id === id
  return { net: current ? state.net : null, loading: id != null && !current, error: current ? state.error : null }
}
