// The projects of the infrastructure as the suite manages them, whichever
// tool works on them: the list with its templates, each project's tree of
// variants, who may change what, and the backup files a project goes out
// and comes in as. All of it is the server's and the core's; nothing here
// knows a tool.
import { parseProjectsPayload, PayloadError, projectDocument, withImportSystem } from '@rail-across/core/project'

const byTitle = (a, b) => String(a.title).localeCompare(String(b.title), 'de', { sensitivity: 'base' })

/** The projects and the templates apart, each by title. */
export function splitProjects(projects) {
  const list = [...(projects ?? [])].sort(byTitle)
  return { regular: list.filter(p => !p.template), templates: list.filter(p => p.template) }
}

/** Variants in tree order, each with its depth under the variant it was branched off. */
export function variantTree(variants) {
  const children = new Map()
  for (const v of variants) {
    const key = v.parentVariantId ?? null
    if (!children.has(key)) children.set(key, [])
    children.get(key).push(v)
  }
  const ids = new Set(variants.map(v => v.id))
  const out = []
  const walk = (parent, depth) => {
    for (const v of children.get(parent) ?? []) {
      out.push({ variant: v, depth })
      walk(v.id, depth + 1)
    }
  }
  walk(null, 0)
  // A variant whose parent is not listed (archived away) starts a tree of its own.
  for (const v of variants) if (v.parentVariantId && !ids.has(v.parentVariantId)) { out.push({ variant: v, depth: 0 }); walk(v.id, 1) }
  return out
}

/**
 * What a user may do with a project (decision 127): its admin edits
 * everything, and deletes it; its creator deletes a project that is no
 * template; a template — its root variants — only an admin edits, everyone
 * else looks at it and branches a variant of their own off it.
 */
export function rightsOf(project, user) {
  const admin = user?.role === 'admin'
  return {
    canEdit: admin || !project.template,
    canDelete: admin || (!project.template && project.createdBy?.id === user?.id),
  }
}

/** Whether a variant is only looked at — a template's own, for anyone but an admin. */
export const readOnlyVariant = (variant, rights) => !rights.canEdit && !variant.parentVariantId

/**
 * The two variants a comparison or a merge starts with: a variant's parent
 * and the variant — a merge goes from the parent into it by default, a
 * comparison the same way round. Ids, or undefined where there are none.
 */
export function pairDefaults(variants) {
  const byId = new Map(variants.map(v => [v.id, v]))
  const child = variants.find(v => v.parentVariantId && byId.has(v.parentVariantId)) ?? variants[1] ?? variants[0]
  return {
    from: child?.parentVariantId && byId.has(child.parentVariantId) ? child.parentVariantId : variants[0]?.id,
    to: child?.id ?? variants[0]?.id,
  }
}

/**
 * The projects of a backup file, each as the server takes it — { title,
 * description, record, image } with its picture (a data URL) apart, for it is
 * uploaded first. Throws { code: 'import_version' | 'import_invalid' }.
 */
export function importedProjects(text, fileName = '') {
  let incoming
  try {
    incoming = parseProjectsPayload(JSON.parse(text)).projects
  } catch (err) {
    const code = err instanceof PayloadError && err.code === 'unsupported_version' ? 'import_version' : 'import_invalid'
    throw Object.assign(new Error(code), { code })
  }
  return incoming.map(p => {
    const { image, id: _id, ...record } = withImportSystem(p)
    return {
      title: p.title || fileName.replace(/(\.railacross)?\.json$/i, ''),
      description: p.description ?? '',
      record,
      image: image ?? null,
    }
  })
}

/**
 * A variant's head as a backup file: self-contained, the schemas carried,
 * the picture embedded (a data URL, or none) instead of its hash on the server.
 */
export function exportedDocument(record, { title, image = null }) {
  const { imageHash: _hash, ...rest } = record
  return projectDocument({ ...rest, title, ...(image ? { image } : {}) }, { carrySchemas: true })
}

/** The name a backup file is saved under. */
export const exportFileName = (project, variant) => `${project.title} – ${variant.name}.railacross.json`
