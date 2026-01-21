import { useState, useEffect, useMemo, useRef } from 'preact/hooks'
import { Modal } from './Modal'
import { Spool, SpoolInput, SlicerPreset, SpoolKProfile, CatalogEntry, api } from '../../lib/api'
import { X, MoreHorizontal, Trash2, Unlink, Archive, ArchiveRestore } from 'lucide-preact'
import { useToast } from '../../lib/toast'
import {
  // Types
  SpoolFormData,
  defaultFormData,
  PrinterWithCalibrations,
  ColorPreset,
  validateForm,
  // Utils
  extractBrandsFromPresets,
  buildFilamentOptions,
  findPresetOption,
  loadRecentColors,
  saveRecentColor,
  isMatchingCalibration,
  // Components
  FilamentSection,
  ColorSection,
  AdditionalSection,
  PAProfileSection,
} from './spool-form'

// Re-export for backwards compatibility
export type { PrinterWithCalibrations } from './spool-form'

interface AddSpoolModalProps {
  isOpen: boolean
  onClose: () => void
  onSave: (input: SpoolInput) => Promise<Spool>
  editSpool?: Spool | null
  onDelete?: (spool: Spool) => void
  onArchive?: (spool: Spool) => void
  onRestore?: (spool: Spool) => void
  onTagRemoved?: () => void
  printersWithCalibrations?: PrinterWithCalibrations[]
  initialTagId?: string | null
  initialWeight?: number | null
}

export function AddSpoolModal({
  isOpen,
  onClose,
  onSave,
  editSpool,
  onDelete,
  onArchive,
  onRestore,
  onTagRemoved,
  printersWithCalibrations = [],
  initialTagId,
  initialWeight,
}: AddSpoolModalProps) {
  // Form state
  const [formData, setFormData] = useState<SpoolFormData>(defaultFormData)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [errors, setErrors] = useState<Partial<Record<keyof SpoolFormData, string>>>({})
  const [activeTab, setActiveTab] = useState<'filament' | 'pa_profile'>('filament')

  // PA Profile state
  const [expandedPrinters, setExpandedPrinters] = useState<Set<string>>(new Set())
  const [selectedProfiles, setSelectedProfiles] = useState<Set<string>>(new Set())

  // Cloud presets
  const [cloudPresets, setCloudPresets] = useState<SlicerPreset[]>([])
  const [cloudAuthenticated, setCloudAuthenticated] = useState(false)
  const [loadingCloudPresets, setLoadingCloudPresets] = useState(false)
  const [presetInputValue, setPresetInputValue] = useState('')

  // Spool catalog & colors
  const [spoolCatalog, setSpoolCatalog] = useState<CatalogEntry[]>([])
  const [recentColors, setRecentColors] = useState<ColorPreset[]>([])

  // Secondary actions dropdown
  const [showActionsMenu, setShowActionsMenu] = useState(false)
  const actionsMenuRef = useRef<HTMLDivElement>(null)

  // Remove tag confirmation
  const [showRemoveTagConfirm, setShowRemoveTagConfirm] = useState(false)
  const [isRemovingTag, setIsRemovingTag] = useState(false)

  const { showToast, updateToast } = useToast()
  const isEditing = !!editSpool

  // Count only selected profiles that are actually visible (matching current formData and from connected printers)
  const validSelectedCount = useMemo(() => {
    let count = 0
    for (const { printer, calibrations } of printersWithCalibrations) {
      if (!printer.connected) continue
      for (const cal of calibrations) {
        if (!isMatchingCalibration(cal, formData)) continue
        const key = `${printer.serial}:${cal.cali_idx}:${cal.extruder_id ?? 'null'}`
        if (selectedProfiles.has(key)) {
          count++
        }
      }
    }
    return count
  }, [printersWithCalibrations, selectedProfiles, formData])

  // Load recent colors on mount
  useEffect(() => {
    setRecentColors(loadRecentColors())
  }, [])

  // Close actions menu on outside click
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (actionsMenuRef.current && !actionsMenuRef.current.contains(e.target as Node)) {
        setShowActionsMenu(false)
      }
    }
    if (showActionsMenu) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showActionsMenu])

  // Fetch cloud presets and catalog when modal opens
  useEffect(() => {
    if (isOpen) {
      const fetchData = async () => {
        setLoadingCloudPresets(true)
        try {
          const status = await api.getCloudStatus()
          setCloudAuthenticated(status.is_authenticated)
          if (status.is_authenticated) {
            const presets = await api.getFilamentPresets()
            setCloudPresets(presets)
          }
        } catch (e) {
          console.error('Failed to fetch cloud presets:', e)
          setCloudAuthenticated(false)
        } finally {
          setLoadingCloudPresets(false)
        }
      }
      fetchData()
      api.getSpoolCatalog().then(setSpoolCatalog).catch(console.error)
    }
  }, [isOpen])

  // Reset form when modal opens/closes or editSpool changes
  useEffect(() => {
    if (isOpen) {
      if (editSpool) {
        setFormData({
          material: editSpool.material || '',
          subtype: editSpool.subtype || '',
          brand: editSpool.brand || '',
          color_name: editSpool.color_name || '',
          rgba: editSpool.rgba ? (editSpool.rgba.startsWith('#') ? editSpool.rgba : `#${editSpool.rgba}`) : '#808080',
          label_weight: editSpool.label_weight || 1000,
          core_weight: editSpool.core_weight || 250,
          slicer_filament: editSpool.slicer_filament || '',
          location: editSpool.location || '',
          note: editSpool.note || '',
        })

        // Load K-profiles for this spool
        api.getSpoolKProfiles(editSpool.id).then(profiles => {
          const profileKeys = new Set<string>()
          for (const p of profiles) {
            const printerCals = printersWithCalibrations.find(pc => pc.printer.serial === p.printer_serial)
            if (printerCals && p.cali_idx !== null) {
              profileKeys.add(`${p.printer_serial}:${p.cali_idx}:${p.extruder ?? 'null'}`)
            }
          }
          setSelectedProfiles(profileKeys)
        }).catch(() => {})
      } else {
        setFormData(defaultFormData)
        setPresetInputValue('')
        setSelectedProfiles(new Set())
      }
      setErrors({})
      setActiveTab('filament')
      setShowRemoveTagConfirm(false)
      setShowActionsMenu(false)
      setExpandedPrinters(new Set(printersWithCalibrations.map(p => p.printer.serial)))
    }
  }, [isOpen, editSpool, printersWithCalibrations])

  // Update field helper
  const updateField = <K extends keyof SpoolFormData>(key: K, value: SpoolFormData[K]) => {
    setFormData(prev => ({ ...prev, [key]: value }))
    // Clear error when field is updated
    if (errors[key]) {
      setErrors(prev => ({ ...prev, [key]: undefined }))
    }
  }

  // Handle color selection
  const handleColorUsed = (color: ColorPreset) => {
    setRecentColors(prev => saveRecentColor(color, prev))
  }

  // Get configured printer models for filtering
  const configuredPrinterModels = useMemo(() => {
    const models = new Set<string>()
    printersWithCalibrations.forEach(({ printer }) => {
      if (printer.model) {
        const model = printer.model.toUpperCase()
        models.add(model)
        if (model.includes('X1')) models.add('X1C')
        if (model.includes('P1S')) models.add('P1S')
        if (model.includes('P1P')) models.add('P1P')
        if (model.includes('A1')) models.add('A1')
        if (model.includes('A1 MINI')) models.add('A1 MINI')
      }
    })
    return models
  }, [printersWithCalibrations])

  // Build filament options
  const filamentOptions = useMemo(() =>
    buildFilamentOptions(cloudPresets, configuredPrinterModels),
    [cloudPresets, configuredPrinterModels]
  )

  // Get brands from presets
  const availableBrands = useMemo(() =>
    extractBrandsFromPresets(cloudPresets),
    [cloudPresets]
  )

  // Find selected preset
  const selectedPresetOption = useMemo(() =>
    findPresetOption(formData.slicer_filament, filamentOptions),
    [filamentOptions, formData.slicer_filament]
  )

  // Sync preset input value
  useEffect(() => {
    if (selectedPresetOption) {
      setPresetInputValue(selectedPresetOption.displayName)
    } else if (formData.slicer_filament) {
      setPresetInputValue(formData.slicer_filament)
    }
  }, [selectedPresetOption, formData.slicer_filament])

  // Handle form submission
  const handleSubmit = async () => {
    const validation = validateForm(formData)
    if (!validation.isValid) {
      setErrors(validation.errors)
      if (validation.errors.slicer_filament) {
        setActiveTab('filament')
      }
      return
    }

    setIsSubmitting(true)
    const toastId = showToast('loading', 'Saving spool...')

    try {
      const input: SpoolInput = {
        material: formData.material,
        subtype: formData.subtype || null,
        brand: formData.brand || null,
        color_name: formData.color_name || null,
        rgba: formData.rgba.replace('#', '') || null,
        label_weight: formData.label_weight,
        core_weight: formData.core_weight,
        slicer_filament: formData.slicer_filament || null,
        slicer_filament_name: selectedPresetOption?.displayName || presetInputValue || null,
        location: formData.location || null,
        note: formData.note || null,
        ext_has_k: selectedProfiles.size > 0,
        data_origin: isEditing ? undefined : 'web',
        tag_id: isEditing ? editSpool?.tag_id : initialTagId || null,
        weight_current: isEditing ? editSpool?.weight_current : initialWeight || null,
      }

      const savedSpool = await onSave(input)

      // Save K-profiles
      if (selectedProfiles.size > 0) {
        updateToast(toastId, 'loading', 'Saving K-profiles...')
        const profiles: Omit<SpoolKProfile, 'id' | 'spool_id' | 'created_at'>[] = []

        selectedProfiles.forEach(key => {
          const parts = key.split(':')
          const serial = parts[0]
          const caliIdx = parseInt(parts[1], 10)
          const extruderStr = parts[2]
          const extruderId = extruderStr === 'null' ? null : parseInt(extruderStr, 10)

          const printerData = printersWithCalibrations.find(p => p.printer.serial === serial)
          const calProfile = printerData?.calibrations.find(c =>
            c.cali_idx === caliIdx && (c.extruder_id ?? null) === extruderId
          )

          if (calProfile) {
            profiles.push({
              printer_serial: serial,
              extruder: extruderId,
              nozzle_diameter: calProfile.nozzle_diameter,
              nozzle_type: null,
              k_value: calProfile.k_value.toString(),
              name: calProfile.name || calProfile.filament_id,
              cali_idx: caliIdx,
              setting_id: null,
            })
          }
        })

        if (profiles.length > 0) {
          await api.saveSpoolKProfiles(savedSpool.id, profiles)
        }
      } else {
        await api.saveSpoolKProfiles(savedSpool.id, [])
      }

      updateToast(toastId, 'success', 'Spool saved successfully')
      onClose()
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to save spool'
      setErrors({ slicer_filament: errorMsg })
      updateToast(toastId, 'error', errorMsg)
    } finally {
      setIsSubmitting(false)
    }
  }

  // Handle tag removal
  const handleRemoveTag = async () => {
    if (!editSpool) return

    setIsRemovingTag(true)
    const toastId = showToast('loading', 'Removing tag...')

    try {
      await api.updateSpool(editSpool.id, {
        ...editSpool,
        tag_id: null,
        tag_type: null,
      } as SpoolInput)

      updateToast(toastId, 'success', 'Tag removed from spool')
      setShowRemoveTagConfirm(false)
      onTagRemoved?.()
      onClose()
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to remove tag'
      updateToast(toastId, 'error', errorMsg)
    } finally {
      setIsRemovingTag(false)
    }
  }

  // Check if there are secondary actions available
  const hasSecondaryActions = isEditing && (
    onDelete || editSpool?.tag_id || onArchive || onRestore
  )

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={isEditing ? 'Edit Spool' : 'Add New Spool'}
      size="xl"
      footer={
        <div class="flex items-center justify-between w-full">
          {/* Left side - Secondary actions dropdown */}
          <div class="modal-footer-left">
            {hasSecondaryActions && (
              <div class="actions-dropdown" ref={actionsMenuRef}>
                <button
                  type="button"
                  class="btn btn-ghost"
                  onClick={() => setShowActionsMenu(!showActionsMenu)}
                  disabled={isSubmitting}
                  title="More actions"
                >
                  <MoreHorizontal class="w-5 h-5" />
                </button>
                {showActionsMenu && (
                  <div class="actions-dropdown-menu">
                    {editSpool?.tag_id && (
                      <button
                        type="button"
                        class="actions-dropdown-item"
                        onClick={() => {
                          setShowActionsMenu(false)
                          setShowRemoveTagConfirm(true)
                        }}
                      >
                        <Unlink class="w-4 h-4" />
                        Remove Tag
                      </button>
                    )}
                    {!editSpool?.archived_at && onArchive && (
                      <button
                        type="button"
                        class="actions-dropdown-item"
                        onClick={() => {
                          setShowActionsMenu(false)
                          onArchive(editSpool!)
                          onClose()
                        }}
                      >
                        <Archive class="w-4 h-4" />
                        Archive Spool
                      </button>
                    )}
                    {editSpool?.archived_at && onRestore && (
                      <button
                        type="button"
                        class="actions-dropdown-item"
                        onClick={() => {
                          setShowActionsMenu(false)
                          onRestore(editSpool!)
                          onClose()
                        }}
                      >
                        <ArchiveRestore class="w-4 h-4" />
                        Restore Spool
                      </button>
                    )}
                    {onDelete && (
                      <button
                        type="button"
                        class="actions-dropdown-item danger"
                        onClick={() => {
                          setShowActionsMenu(false)
                          onDelete(editSpool!)
                          onClose()
                        }}
                      >
                        <Trash2 class="w-4 h-4" />
                        Delete Spool
                      </button>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Right side - Primary actions */}
          <div class="modal-footer-right">
            <button class="btn btn-ghost" onClick={onClose} disabled={isSubmitting}>
              Cancel
            </button>
            <button class="btn btn-primary" onClick={handleSubmit} disabled={isSubmitting}>
              {isSubmitting ? 'Saving...' : isEditing ? 'Save Changes' : 'Add Spool'}
            </button>
          </div>
        </div>
      }
    >
      {/* Global error */}
      {Object.keys(errors).length > 0 && errors.slicer_filament && (
        <div class="mb-4 p-3 bg-[var(--error-color)]/10 border border-[var(--error-color)]/30 rounded-lg text-[var(--error-color)] text-sm flex items-center justify-between">
          <span>{errors.slicer_filament}</span>
          <button onClick={() => setErrors({})}>
            <X class="w-4 h-4" />
          </button>
        </div>
      )}

      {/* Tabs */}
      <div class="tabs mb-3">
        <button
          class={`tab ${activeTab === 'filament' ? 'active' : ''}`}
          onClick={() => setActiveTab('filament')}
        >
          Filament Info
        </button>
        <button
          class={`tab ${activeTab === 'pa_profile' ? 'active' : ''}`}
          onClick={() => setActiveTab('pa_profile')}
        >
          PA Profile (K)
          {validSelectedCount > 0 && (
            <span class="ml-2 text-xs px-1.5 py-0.5 rounded-full bg-[var(--accent-color)] text-white">
              {validSelectedCount}
            </span>
          )}
        </button>
      </div>

      {/* Tab Content */}
      {activeTab === 'filament' ? (
        <div class="space-y-3">
          <FilamentSection
            formData={formData}
            updateField={updateField}
            cloudAuthenticated={cloudAuthenticated}
            loadingCloudPresets={loadingCloudPresets}
            presetInputValue={presetInputValue}
            setPresetInputValue={setPresetInputValue}
            selectedPresetOption={selectedPresetOption}
            filamentOptions={filamentOptions}
            availableBrands={availableBrands}
          />

          <ColorSection
            formData={formData}
            updateField={updateField}
            recentColors={recentColors}
            onColorUsed={handleColorUsed}
          />

          <AdditionalSection
            formData={formData}
            updateField={updateField}
            spoolCatalog={spoolCatalog}
          />
        </div>
      ) : (
        <PAProfileSection
          formData={formData}
          updateField={updateField}
          printersWithCalibrations={printersWithCalibrations}
          selectedProfiles={selectedProfiles}
          setSelectedProfiles={setSelectedProfiles}
          expandedPrinters={expandedPrinters}
          setExpandedPrinters={setExpandedPrinters}
        />
      )}

      {/* Remove Tag Confirmation Modal */}
      {showRemoveTagConfirm && (
        <div class="fixed inset-0 z-[60] flex items-center justify-center">
          <div class="absolute inset-0 bg-black/50" onClick={() => setShowRemoveTagConfirm(false)} />
          <div class="relative bg-[var(--bg-primary)] rounded-lg border border-[var(--border-color)] p-6 max-w-md shadow-xl">
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">
              Remove NFC Tag?
            </h3>
            <p class="text-sm text-[var(--text-secondary)] mb-4">
              This will unlink the NFC tag from this spool. The tag can then be assigned to a different spool.
            </p>
            {editSpool?.tag_id && (
              <p class="text-xs text-[var(--text-muted)] font-mono mb-4 p-2 bg-[var(--bg-tertiary)] rounded">
                Tag ID: {editSpool.tag_id}
              </p>
            )}
            <div class="flex justify-end gap-2">
              <button
                class="btn"
                onClick={() => setShowRemoveTagConfirm(false)}
                disabled={isRemovingTag}
              >
                Cancel
              </button>
              <button
                class="btn btn-danger"
                onClick={handleRemoveTag}
                disabled={isRemovingTag}
              >
                {isRemovingTag ? 'Removing...' : 'Remove Tag'}
              </button>
            </div>
          </div>
        </div>
      )}
    </Modal>
  )
}
