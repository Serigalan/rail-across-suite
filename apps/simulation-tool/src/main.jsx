import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@rail-across/ui/ui.css'
import './tool.css'
import App from './App.jsx'

createRoot(document.getElementById('root')).render(<StrictMode><App /></StrictMode>)
