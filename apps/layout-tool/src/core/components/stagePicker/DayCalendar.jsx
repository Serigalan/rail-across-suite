import { useState } from 'react'
import { useI18n } from '../../locales/i18nContext'
import { dayOf, monthAfter, monthWeeks } from '../../utils/stage'

const today = () => { const d = new Date(); return dayOf(d.getFullYear(), d.getMonth() + 1, d.getDate()) }

/**
 * A month to pick a day in: `value` marked, the days in `marked` (those
 * something is put into or taken out of service on) underlined, today
 * outlined. Opens at `value`, else at `start`, else at today; months and
 * years to step through. `onPick(day)` with the day clicked (YYYY-MM-DD).
 */
export default function DayCalendar({ value, start, marked, onPick }) {
  const { t, language } = useI18n()
  const now = today()
  const [[y, m], setMonth] = useState(() => (value ?? start ?? now).split('-').slice(0, 2).map(Number))
  const step = (n) => setMonth(monthAfter(y, m, n))
  const title = new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString(language, { timeZone: 'UTC', month: 'long', year: 'numeric' })
  // Monday to Sunday: 2024-01-01 was a Monday.
  const weekdays = Array.from({ length: 7 }, (_, i) => new Date(Date.UTC(2024, 0, 1 + i)).toLocaleDateString(language, { timeZone: 'UTC', weekday: 'narrow' }))
  return (
    <div className="day-calendar">
      <div className="day-calendar-head">
        <button type="button" onClick={() => step(-12)} title={t('calendar_year_back')} aria-label={t('calendar_year_back')}>«</button>
        <button type="button" onClick={() => step(-1)} title={t('calendar_month_back')} aria-label={t('calendar_month_back')}>‹</button>
        <span className="day-calendar-title">{title}</span>
        <button type="button" onClick={() => step(1)} title={t('calendar_month_next')} aria-label={t('calendar_month_next')}>›</button>
        <button type="button" onClick={() => step(12)} title={t('calendar_year_next')} aria-label={t('calendar_year_next')}>»</button>
      </div>
      <table className="day-calendar-grid">
        <thead><tr>{weekdays.map((w, i) => <th key={i} scope="col">{w}</th>)}</tr></thead>
        <tbody>
          {monthWeeks(y, m).map((week, i) => (
            <tr key={i}>
              {week.map((day, j) => (
                <td key={j}>
                  {day && (
                    <button type="button" aria-pressed={day === value}
                      className={[marked?.has(day) && 'day-calendar-marked', day === now && 'day-calendar-today'].filter(Boolean).join(' ') || undefined}
                      onClick={() => onPick(day)}>
                      {Number(day.slice(8))}
                    </button>
                  )}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
