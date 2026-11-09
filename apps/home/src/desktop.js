// The desktop app (crates/desktop): the suite in its window, the project
// server in its process. There the start page reaches the app's own commands —
// the start screen again, and the language for the next start.
const tauri = () => (typeof window === 'undefined' ? null : window.__TAURI__?.core ?? null)

export const onDesktop = () => tauri() !== null

/** The start screen: language, working files, exports — a change restarts the app. */
export const openSettings = () => tauri()?.invoke('open_settings')

/** The language the app starts in next time, as chosen in the suite. */
export const rememberLanguage = (language) => tauri()?.invoke('set_language', { language }).catch(() => {})
