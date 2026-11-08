import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { loadNative } from '@rail-across/core/native'
import App from './App'

// The core's Rust module (decision 0020) before anything renders; without it
// the core runs on its JavaScript.
loadNative(`${import.meta.env.BASE_URL}core-module/rail_across_core.wasm`).finally(() => {
  createRoot(document.getElementById('root')).render(<StrictMode><App /></StrictMode>)
})
