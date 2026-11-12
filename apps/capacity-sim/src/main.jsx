import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { loadNative } from '@rail-across/core/native'
import '@rail-across/ui/ui.css'
import './tool.css'
import App from './App.jsx'

// The core's Rust module (decision 0020) before anything renders: the capacity is computed in it.
loadNative(`${import.meta.env.BASE_URL}core-module/rail_across_core.wasm`).finally(() => {
  createRoot(document.getElementById('root')).render(<StrictMode><App /></StrictMode>)
})
