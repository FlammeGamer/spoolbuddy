import { useState, useEffect, useCallback } from 'preact/hooks'
import { api, CatalogEntry } from '../lib/api'
import { useToast } from '../lib/toast'
import { Database, Plus, Trash2, RotateCcw, Loader2, Edit2, Check, X, Search, Download, Upload } from 'lucide-preact'
import { useRef } from 'preact/hooks'

export function SpoolCatalogSettings() {
  const { showToast } = useToast()
  const [catalog, setCatalog] = useState<CatalogEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [search, setSearch] = useState('')
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Add/Edit form state
  const [showAddForm, setShowAddForm] = useState(false)
  const [editingId, setEditingId] = useState<number | null>(null)
  const [formName, setFormName] = useState('')
  const [formWeight, setFormWeight] = useState('')
  const [saving, setSaving] = useState(false)

  // Delete confirmation state
  const [deleteId, setDeleteId] = useState<number | null>(null)
  const [deleteName, setDeleteName] = useState('')
  const [showResetConfirm, setShowResetConfirm] = useState(false)

  // Load catalog
  const loadCatalog = useCallback(async () => {
    try {
      const entries = await api.getSpoolCatalog()
      setCatalog(entries)
    } catch {
      showToast('error', 'Failed to load spool catalog')
    } finally {
      setLoading(false)
    }
  }, [showToast])

  useEffect(() => {
    loadCatalog()
  }, [loadCatalog])

  // Filter catalog based on search
  const filteredCatalog = catalog.filter(entry =>
    entry.name.toLowerCase().includes(search.toLowerCase())
  )

  // Handle add entry
  const handleAdd = async () => {
    if (!formName.trim() || !formWeight) {
      showToast('error', 'Name and weight are required')
      return
    }
    setSaving(true)
    try {
      const entry = await api.addCatalogEntry({
        name: formName.trim(),
        weight: parseInt(formWeight)
      })
      setCatalog(prev => [...prev, entry].sort((a, b) => a.name.localeCompare(b.name)))
      setShowAddForm(false)
      setFormName('')
      setFormWeight('')
      showToast('success', 'Entry added')
    } catch {
      showToast('error', 'Failed to add entry')
    } finally {
      setSaving(false)
    }
  }

  // Handle edit entry
  const startEdit = (entry: CatalogEntry) => {
    setEditingId(entry.id)
    setFormName(entry.name)
    setFormWeight(entry.weight.toString())
  }

  const cancelEdit = () => {
    setEditingId(null)
    setFormName('')
    setFormWeight('')
  }

  const handleUpdate = async (id: number) => {
    if (!formName.trim() || !formWeight) {
      showToast('error', 'Name and weight are required')
      return
    }
    setSaving(true)
    try {
      const updated = await api.updateCatalogEntry(id, {
        name: formName.trim(),
        weight: parseInt(formWeight)
      })
      setCatalog(prev =>
        prev.map(e => e.id === id ? updated : e).sort((a, b) => a.name.localeCompare(b.name))
      )
      setEditingId(null)
      setFormName('')
      setFormWeight('')
      showToast('success', 'Entry updated')
    } catch {
      showToast('error', 'Failed to update entry')
    } finally {
      setSaving(false)
    }
  }

  // Handle delete entry
  const confirmDelete = (entry: CatalogEntry) => {
    setDeleteId(entry.id)
    setDeleteName(entry.name)
  }

  const handleDelete = async () => {
    if (!deleteId) return
    try {
      await api.deleteCatalogEntry(deleteId)
      setCatalog(prev => prev.filter(e => e.id !== deleteId))
      showToast('success', 'Entry deleted')
    } catch {
      showToast('error', 'Failed to delete entry')
    } finally {
      setDeleteId(null)
      setDeleteName('')
    }
  }

  // Handle reset to defaults
  const handleReset = async () => {
    setShowResetConfirm(false)
    setLoading(true)
    try {
      await api.resetSpoolCatalog()
      await loadCatalog()
      showToast('success', 'Catalog reset to defaults')
    } catch {
      showToast('error', 'Failed to reset catalog')
      setLoading(false)
    }
  }

  // Export catalog to JSON file
  const handleExport = () => {
    const exportData = catalog.map(({ name, weight }) => ({ name, weight }))
    const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'spool-catalog.json'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    showToast('success', `Exported ${catalog.length} entries`)
  }

  // Import catalog from JSON file
  const handleImport = async (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (!file) return

    try {
      const text = await file.text()
      const data = JSON.parse(text) as Array<{ name: string; weight: number }>

      if (!Array.isArray(data)) {
        throw new Error('Invalid format: expected array')
      }

      let added = 0
      let skipped = 0

      for (const item of data) {
        if (!item.name || typeof item.weight !== 'number') {
          skipped++
          continue
        }
        // Check if entry already exists
        const exists = catalog.some(c => c.name.toLowerCase() === item.name.toLowerCase())
        if (exists) {
          skipped++
          continue
        }
        try {
          const entry = await api.addCatalogEntry({ name: item.name, weight: item.weight })
          setCatalog(prev => [...prev, entry].sort((a, b) => a.name.localeCompare(b.name)))
          added++
        } catch {
          skipped++
        }
      }

      showToast('success', `Imported ${added} entries (${skipped} skipped)`)
    } catch {
      showToast('error', 'Failed to import: invalid JSON format')
    }

    // Reset file input
    if (fileInputRef.current) {
      fileInputRef.current.value = ''
    }
  }

  return (
    <div class="card">
      <div class="px-6 py-4 border-b border-[var(--border-color)]">
        <div class="flex items-center gap-2 mb-3">
          <Database class="w-5 h-5 text-[var(--text-muted)]" />
          <h2 class="text-lg font-medium text-[var(--text-primary)]">Spool Catalog</h2>
          <span class="text-sm text-[var(--text-muted)]">({catalog.length})</span>
        </div>
        <div class="flex items-center gap-2 flex-wrap">
          <button
            onClick={handleExport}
            class="btn flex items-center gap-1.5"
            title="Export catalog to JSON"
          >
            <Download class="w-4 h-4" />
            <span class="hidden sm:inline">Export</span>
          </button>
          <button
            onClick={() => fileInputRef.current?.click()}
            class="btn flex items-center gap-1.5"
            title="Import catalog from JSON"
          >
            <Upload class="w-4 h-4" />
            <span class="hidden sm:inline">Import</span>
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json"
            class="hidden"
            onChange={handleImport}
          />
          <button
            onClick={() => setShowResetConfirm(true)}
            class="btn flex items-center gap-1.5"
            title="Reset to defaults"
          >
            <RotateCcw class="w-4 h-4" />
            <span class="hidden sm:inline">Reset</span>
          </button>
          <button
            onClick={() => setShowAddForm(true)}
            class="btn btn-primary flex items-center gap-1.5"
          >
            <Plus class="w-4 h-4" />
            <span class="hidden sm:inline">Add</span>
          </button>
        </div>
      </div>

      <div class="p-6 space-y-4">
        <p class="text-sm text-[var(--text-secondary)]">
          Empty spool weights by brand/type. Used for automatic weight lookup when adding spools.
        </p>

        {/* Search */}
        <div class="relative">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-muted)]" />
          <input
            type="text"
            class="input input-with-icon"
            placeholder="Search catalog..."
            value={search}
            onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
          />
        </div>

        {/* Add form */}
        {showAddForm && (
          <div class="p-4 bg-[var(--bg-tertiary)] rounded-lg border border-[var(--border-color)]">
            <h3 class="text-sm font-medium text-[var(--text-primary)] mb-3">Add New Entry</h3>
            <div class="flex gap-2 items-center">
              <div class="flex-1 min-w-0">
                <input
                  type="text"
                  class="input w-full"
                  placeholder="Name (e.g., Bambu Lab - Plastic)"
                  value={formName}
                  onInput={(e) => setFormName((e.target as HTMLInputElement).value)}
                />
              </div>
              <input
                type="number"
                class="input w-20 max-w-[80px] text-center shrink-0"
                placeholder="g"
                value={formWeight}
                onInput={(e) => setFormWeight((e.target as HTMLInputElement).value)}
              />
              <span class="text-[var(--text-muted)] shrink-0">g</span>
              <button
                onClick={handleAdd}
                disabled={saving}
                class="btn btn-primary flex items-center gap-1 shrink-0"
              >
                {saving ? <Loader2 class="w-4 h-4 animate-spin" /> : <Check class="w-4 h-4" />}
                Add
              </button>
              <button
                onClick={() => { setShowAddForm(false); setFormName(''); setFormWeight('') }}
                class="btn shrink-0"
              >
                <X class="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {/* Catalog list */}
        {loading ? (
          <div class="flex items-center justify-center py-8 text-[var(--text-muted)]">
            <Loader2 class="w-5 h-5 animate-spin mr-2" />
            Loading catalog...
          </div>
        ) : (
          <div class="max-h-[400px] overflow-y-auto border border-[var(--border-color)] rounded-lg">
            <table class="w-full text-sm">
              <thead class="bg-[var(--bg-tertiary)] sticky top-0">
                <tr>
                  <th class="px-4 py-2 text-left text-[var(--text-secondary)] font-medium">Name</th>
                  <th class="px-4 py-2 text-right text-[var(--text-secondary)] font-medium w-24">Weight</th>
                  <th class="px-4 py-2 text-center text-[var(--text-secondary)] font-medium w-20">Type</th>
                  <th class="px-4 py-2 w-24"></th>
                </tr>
              </thead>
              <tbody>
                {filteredCatalog.length === 0 ? (
                  <tr>
                    <td colSpan={4} class="px-4 py-8 text-center text-[var(--text-muted)]">
                      {search ? 'No entries match your search' : 'No entries in catalog'}
                    </td>
                  </tr>
                ) : (
                  filteredCatalog.map(entry => (
                    <tr
                      key={entry.id}
                      class="border-t border-[var(--border-color)] hover:bg-[var(--bg-secondary)]"
                    >
                      {editingId === entry.id ? (
                        <>
                          <td class="px-4 py-2">
                            <input
                              type="text"
                              class="input w-full"
                              value={formName}
                              onInput={(e) => setFormName((e.target as HTMLInputElement).value)}
                            />
                          </td>
                          <td class="px-4 py-2">
                            <input
                              type="number"
                              class="input w-full text-right"
                              value={formWeight}
                              onInput={(e) => setFormWeight((e.target as HTMLInputElement).value)}
                            />
                          </td>
                          <td class="px-4 py-2 text-center">
                            <span class="text-xs text-[var(--text-muted)]">-</span>
                          </td>
                          <td class="px-4 py-2">
                            <div class="flex justify-end gap-1">
                              <button
                                onClick={() => handleUpdate(entry.id)}
                                disabled={saving}
                                class="p-1.5 rounded hover:bg-green-500/20 text-green-500"
                                title="Save"
                              >
                                {saving ? <Loader2 class="w-4 h-4 animate-spin" /> : <Check class="w-4 h-4" />}
                              </button>
                              <button
                                onClick={cancelEdit}
                                class="p-1.5 rounded hover:bg-[var(--bg-tertiary)] text-[var(--text-muted)]"
                                title="Cancel"
                              >
                                <X class="w-4 h-4" />
                              </button>
                            </div>
                          </td>
                        </>
                      ) : (
                        <>
                          <td class="px-4 py-2 text-[var(--text-primary)]">{entry.name}</td>
                          <td class="px-4 py-2 text-right font-mono text-[var(--text-primary)]">{entry.weight}g</td>
                          <td class="px-4 py-2 text-center">
                            {entry.is_default ? (
                              <span class="text-xs px-2 py-0.5 rounded bg-[var(--bg-tertiary)] text-[var(--text-muted)]">
                                Default
                              </span>
                            ) : (
                              <span class="text-xs px-2 py-0.5 rounded bg-[var(--accent-color)]/20 text-[var(--accent-color)]">
                                Custom
                              </span>
                            )}
                          </td>
                          <td class="px-4 py-2">
                            <div class="flex justify-end gap-1">
                              <button
                                onClick={() => startEdit(entry)}
                                class="p-1.5 rounded hover:bg-[var(--bg-tertiary)] text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                                title="Edit"
                              >
                                <Edit2 class="w-4 h-4" />
                              </button>
                              <button
                                onClick={() => confirmDelete(entry)}
                                class="p-1.5 rounded bg-red-500/10 hover:bg-red-500/20 text-red-500"
                                title="Delete"
                              >
                                <Trash2 class="w-4 h-4" />
                              </button>
                            </div>
                          </td>
                        </>
                      )}
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Delete confirmation modal */}
      {deleteId && (
        <div class="fixed inset-0 z-50 flex items-center justify-center">
          <div class="absolute inset-0 bg-black/50" onClick={() => setDeleteId(null)} />
          <div class="relative bg-[var(--bg-primary)] rounded-lg shadow-xl p-6 max-w-sm mx-4">
            <h3 class="text-lg font-medium text-[var(--text-primary)] mb-2">Delete Entry</h3>
            <p class="text-sm text-[var(--text-secondary)] mb-4">
              Are you sure you want to delete "<span class="font-medium">{deleteName}</span>"?
            </p>
            <div class="flex justify-end gap-2">
              <button
                onClick={() => setDeleteId(null)}
                class="btn"
              >
                Cancel
              </button>
              <button
                onClick={handleDelete}
                class="px-4 py-2 rounded-lg font-medium bg-red-500 hover:bg-red-600 text-white transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Reset confirmation modal */}
      {showResetConfirm && (
        <div class="fixed inset-0 z-50 flex items-center justify-center">
          <div class="absolute inset-0 bg-black/50" onClick={() => setShowResetConfirm(false)} />
          <div class="relative bg-[var(--bg-primary)] rounded-lg shadow-xl p-6 max-w-sm mx-4">
            <h3 class="text-lg font-medium text-[var(--text-primary)] mb-2">Reset Catalog</h3>
            <p class="text-sm text-[var(--text-secondary)] mb-4">
              Reset catalog to defaults? This will remove all custom entries.
            </p>
            <div class="flex justify-end gap-2">
              <button
                onClick={() => setShowResetConfirm(false)}
                class="btn"
              >
                Cancel
              </button>
              <button
                onClick={handleReset}
                class="px-4 py-2 rounded-lg font-medium bg-red-500 hover:bg-red-600 text-white transition-colors"
              >
                Reset
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
