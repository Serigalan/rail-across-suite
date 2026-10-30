import { useState } from 'react'
import { api, blobUrl } from '../../api'
import { FormDialog } from '../../Dialog'
import { useAction } from '../../useAction'
import { useTexts } from '../../i18n'
import { exportedDocument, exportFileName, pairDefaults } from '../../projects'
import { PROJECT_IMAGES, projectImagePicture, splitDataUrl } from '../../projectImages'
import ImagePicker from './ImagePicker'

// The dialogs of the projects: a new one, its title and description, a
// backup file of a variant, a variant branched off, renamed or archived, a
// project deleted — and the two variants a tool compares or merges. Each
// calls `onDone` once the server has it, so the list is read again.

const liveVariants = (project) => project.variants.filter(v => !v.archived)

function VariantSelect({ label, variants, value, onChange }) {
  return (
    <label className="collab-field">
      <span>{label}</span>
      <select value={value} onChange={e => onChange(e.target.value)}>
        {variants.map(v => <option key={v.id} value={v.id}>{v.name}</option>)}
      </select>
    </label>
  )
}

/** A new project — or, for an admin, a new template —, its picture uploaded first. */
export function NewProjectDialog({ template = false, onCancel, onDone }) {
  const { t } = useTexts()
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [picture, setPicture] = useState({ kind: 'none' })
  const { busy, error, run } = useAction()
  const submit = () => run(async () => {
    if (!title.trim()) return
    const preset = picture.kind === 'preset' ? PROJECT_IMAGES.find(i => i.key === picture.key) : null
    const image = preset ? await projectImagePicture(preset)
      : picture.kind === 'file' ? splitDataUrl(picture.dataUrl) : null
    let payload
    if (image) {
      const { hash } = await api.uploadImage(image.mime, image.data)
      payload = { tracks: [], switches: [], platforms: [], imageHash: hash }
    }
    await api.createProject({ title: title.trim(), description: description.trim(), template, ...(payload ? { payload } : {}) })
    await onDone()
  })
  return (
    <FormDialog title={t(template ? 'project_new_template_title' : 'project_new_title')} submitLabel={t('project_create')}
      busy={busy} canSubmit={Boolean(title.trim())} error={error} onCancel={onCancel} onSubmit={submit}>
      <label className="collab-field">
        <span>{t('project_field_title')} *</span>
        <input value={title} onChange={e => setTitle(e.target.value)} placeholder={t('project_field_title_placeholder')} required />
      </label>
      <label className="collab-field">
        <span>{t('project_field_description')}</span>
        <textarea rows={3} value={description} onChange={e => setDescription(e.target.value)} placeholder={t('project_field_description_placeholder')} />
      </label>
      <ImagePicker value={picture} onChange={setPicture} disabled={busy} />
    </FormDialog>
  )
}

/** A project's title and description. */
export function RenameProjectDialog({ project, onCancel, onDone }) {
  const { t } = useTexts()
  const [title, setTitle] = useState(project.title)
  const [description, setDescription] = useState(project.description ?? '')
  const { busy, error, run } = useAction()
  return (
    <FormDialog title={t('project_rename')} submitLabel={t('save')} busy={busy} canSubmit={Boolean(title.trim())} error={error} onCancel={onCancel}
      onSubmit={() => run(async () => { await api.patchProject(project.id, { title: title.trim(), description: description.trim() }); await onDone() })}>
      <label className="collab-field">
        <span>{t('project_field_title')} *</span>
        <input value={title} onChange={e => setTitle(e.target.value)} required />
      </label>
      <label className="collab-field">
        <span>{t('project_field_description')}</span>
        <textarea rows={3} value={description} onChange={e => setDescription(e.target.value)} />
      </label>
    </FormDialog>
  )
}

async function imageAsDataUrl(hash) {
  const res = await fetch(blobUrl(hash), { credentials: 'same-origin' })
  if (!res.ok) return null
  const blob = await res.blob()
  return await new Promise((resolve) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result)
    reader.onerror = () => resolve(null)
    reader.readAsDataURL(blob)
  })
}

function download(doc, fileName) {
  const url = URL.createObjectURL(new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' }))
  const a = Object.assign(document.createElement('a'), { href: url, download: fileName })
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 0)
}

/** The head of one variant as a self-contained backup file, its picture embedded. */
export function ExportDialog({ project, onCancel, onDone }) {
  const { t } = useTexts()
  const variants = liveVariants(project)
  const [variantId, setVariantId] = useState(variants[0]?.id ?? '')
  const { busy, error, run } = useAction()
  const submit = () => run(async () => {
    const { payload, variant } = await api.head(variantId)
    const image = payload.imageHash ? await imageAsDataUrl(payload.imageHash) : null
    download(exportedDocument(payload, { title: project.title, image }), exportFileName(project, variant))
    onDone()
  })
  return (
    <FormDialog title={t('project_export_title')} submitLabel={t('project_export')} busy={busy} error={error} onCancel={onCancel} onSubmit={submit}>
      <VariantSelect label={t('variant')} variants={variants} value={variantId} onChange={setVariantId} />
    </FormDialog>
  )
}

/** A new variant branched off one of the project's variants, at its head. */
export function BranchDialog({ project, onCancel, onDone }) {
  const { t } = useTexts()
  const variants = liveVariants(project)
  const [name, setName] = useState('')
  const [from, setFrom] = useState(variants[0]?.id ?? '')
  const { busy, error, run } = useAction()
  return (
    <FormDialog title={t('branch_title')} submitLabel={t('branch_submit')} busy={busy} canSubmit={Boolean(name.trim())} error={error} onCancel={onCancel}
      onSubmit={() => run(async () => { await api.branch(project.id, { name: name.trim(), fromVariant: from }); await onDone() })}>
      <label className="collab-field">
        <span>{t('branch_name')} *</span>
        <input value={name} onChange={e => setName(e.target.value)} placeholder="2030" required />
      </label>
      <VariantSelect label={t('branch_from')} variants={variants} value={from} onChange={setFrom} />
    </FormDialog>
  )
}

/** A variant renamed, or archived (and taken back). */
export function VariantDialog({ variant, onCancel, onDone }) {
  const { t } = useTexts()
  const [name, setName] = useState(variant.name)
  const [archived, setArchived] = useState(Boolean(variant.archived))
  const { busy, error, run } = useAction()
  return (
    <FormDialog title={t('variant_edit')} submitLabel={t('save')} busy={busy} canSubmit={Boolean(name.trim())} error={error} onCancel={onCancel}
      onSubmit={() => run(async () => { await api.patchVariant(variant.id, { name: name.trim(), archived }); await onDone() })}>
      <label className="collab-field">
        <span>{t('variant_name')}</span>
        <input value={name} onChange={e => setName(e.target.value)} required />
      </label>
      <label className="ra-check">
        <input type="checkbox" checked={archived} onChange={e => setArchived(e.target.checked)} />
        {t('variant_archived')}
      </label>
    </FormDialog>
  )
}

/** A project deleted with all its variants, once asked. */
export function DeleteProjectDialog({ project, onCancel, onDone }) {
  const { t, fill } = useTexts()
  const { busy, error, run } = useAction()
  return (
    <FormDialog title={fill('project_delete_confirm', { title: project.title })} submitLabel={t('project_delete')} danger
      busy={busy} error={error} onCancel={onCancel}
      onSubmit={() => run(async () => { await api.deleteProject(project.id); await onDone() })} />
  )
}

/**
 * Two variants of a project for the tool: the two states it compares
 * (earlier, later), or the source whose changes it merges into the target.
 * `onChoose(from, to)` goes on to the tool.
 */
export function PairDialog({ kind, project, toolName, onCancel, onChoose }) {
  const { t, fill } = useTexts()
  const variants = liveVariants(project)
  const start = pairDefaults(variants)
  const [from, setFrom] = useState(start.from)
  const [to, setTo] = useState(start.to)
  const merge = kind === 'merge'
  return (
    <FormDialog title={t(merge ? 'merge_title' : 'compare_title')} submitLabel={fill('in_tool', { action: t(merge ? 'merge' : 'compare'), tool: toolName })}
      busy={false} canSubmit={Boolean(from && to && from !== to)} onCancel={onCancel} onSubmit={() => onChoose(from, to)}>
      {merge && <p className="collab-muted">{t('merge_desc')}</p>}
      <VariantSelect label={t(merge ? 'merge_source' : 'compare_before')} variants={variants} value={from} onChange={setFrom} />
      <VariantSelect label={t(merge ? 'merge_target' : 'compare_after')} variants={variants} value={to} onChange={setTo} />
    </FormDialog>
  )
}
