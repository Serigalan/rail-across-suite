import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import SignInPage from './SignInScreen'
import './home.css'

createRoot(document.getElementById('root')).render(<StrictMode><SignInPage /></StrictMode>)
