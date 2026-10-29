import '@fontsource-variable/inter'
import '@fontsource-variable/jetbrains-mono'
import '@fontsource/newsreader/400.css'
import '@fontsource/newsreader/400-italic.css'
import './styles.css'
import { createRoot } from 'react-dom/client'
import { StrictMode, type ReactNode } from 'react'

export function mount(node: ReactNode) {
  createRoot(document.getElementById('root')!).render(<StrictMode>{node}</StrictMode>)
}
