import { useState, useMemo, useEffect, useCallback } from 'preact/hooks'
import { X, Loader2, Settings, ChevronDown, CheckCircle2, RotateCcw, Palette } from 'lucide-preact'
import { api, CalibrationProfile, SlicerPreset, ColorEntry } from '../lib/api'

interface SlotInfo {
  amsId: number
  trayId: number
  trayCount: number
  trayType?: string
  trayColor?: string
  traySubBrands?: string
  trayInfoIdx?: string
}

// Get proper AMS label (handles HT AMS with ID 128+)
function getAmsLabel(amsId: number, trayCount: number): string {
  if (amsId === 255) return 'External'
  if (amsId === 254) return 'External L'

  let normalizedId: number
  let isHt = false

  if (amsId >= 128 && amsId <= 135) {
    normalizedId = amsId - 128
    isHt = true
  } else if (amsId >= 0 && amsId <= 3) {
    normalizedId = amsId
    isHt = trayCount === 1
  } else {
    normalizedId = 0
  }

  normalizedId = Math.max(0, Math.min(normalizedId, 7))
  const letter = String.fromCharCode(65 + normalizedId)

  return isHt ? `HT-${letter}` : `AMS-${letter}`
}

// Convert setting_id to tray_info_idx (filament_id format)
function convertToTrayInfoIdx(settingId: string): string {
  const baseId = settingId.includes('_') ? settingId.split('_')[0] : settingId
  if (baseId.startsWith('GFS')) {
    return 'GF' + baseId.slice(3)
  }
  if (baseId.startsWith('PFUS') || baseId.startsWith('PFSP')) {
    return baseId
  }
  return baseId
}

interface ConfigureAmsSlotModalProps {
  isOpen: boolean
  onClose: () => void
  printerSerial: string
  slotInfo: SlotInfo
  calibrations: CalibrationProfile[]
  currentKValue: number | null
  onSuccess?: () => void
}

// Known filament material types
const MATERIAL_TYPES = ['PLA', 'PETG', 'ABS', 'ASA', 'TPU', 'PC', 'PA', 'NYLON', 'PVA', 'HIPS', 'PP', 'PET']

// Extract filament type from preset name
function parsePresetName(name: string): { material: string; brand: string; variant: string } {
  const withoutSuffix = name.replace(/@.+$/, '').trim()
  const upperName = withoutSuffix.toUpperCase()

  for (const mat of MATERIAL_TYPES) {
    const regex = new RegExp(`\\b${mat}\\b`, 'i')
    if (regex.test(upperName)) {
      const parts = withoutSuffix.split(regex)
      const brand = parts[0]?.trim() || ''
      const variant = parts[1]?.trim() || ''
      return { material: mat, brand, variant }
    }
  }

  const parts = withoutSuffix.split(/\s+/)
  if (parts.length >= 2) {
    return { material: parts[1], brand: parts[0], variant: parts.slice(2).join(' ') }
  }

  return { material: withoutSuffix, brand: '', variant: '' }
}

// Check if a preset is a user preset
function isUserPreset(settingId: string): boolean {
  return !settingId.startsWith('GF') && !settingId.startsWith('P1')
}

// Quick-select color presets (common filament colors)
const QUICK_COLORS_BASIC = [
  { name: 'White', hex: 'FFFFFF' },
  { name: 'Black', hex: '000000' },
  { name: 'Red', hex: 'FF0000' },
  { name: 'Blue', hex: '0000FF' },
  { name: 'Green', hex: '00AA00' },
  { name: 'Yellow', hex: 'FFFF00' },
  { name: 'Orange', hex: 'FFA500' },
  { name: 'Gray', hex: '808080' },
]

// Extended colors shown when expanded
const QUICK_COLORS_EXTENDED = [
  { name: 'Cyan', hex: '00FFFF' },
  { name: 'Magenta', hex: 'FF00FF' },
  { name: 'Purple', hex: '800080' },
  { name: 'Pink', hex: 'FFC0CB' },
  { name: 'Brown', hex: '8B4513' },
  { name: 'Beige', hex: 'F5F5DC' },
  { name: 'Navy', hex: '000080' },
  { name: 'Teal', hex: '008080' },
  { name: 'Lime', hex: '32CD32' },
  { name: 'Gold', hex: 'FFD700' },
  { name: 'Silver', hex: 'C0C0C0' },
  { name: 'Maroon', hex: '800000' },
  { name: 'Olive', hex: '808000' },
  { name: 'Coral', hex: 'FF7F50' },
  { name: 'Salmon', hex: 'FA8072' },
  { name: 'Turquoise', hex: '40E0D0' },
  { name: 'Violet', hex: 'EE82EE' },
  { name: 'Indigo', hex: '4B0082' },
  { name: 'Chocolate', hex: 'D2691E' },
  { name: 'Tan', hex: 'D2B48C' },
  { name: 'Slate', hex: '708090' },
  { name: 'Charcoal', hex: '36454F' },
  { name: 'Ivory', hex: 'FFFFF0' },
  { name: 'Cream', hex: 'FFFDD0' },
]

export function ConfigureAmsSlotModal({
  isOpen,
  onClose,
  printerSerial,
  slotInfo,
  calibrations,
  currentKValue: _currentKValue,
  onSuccess,
}: ConfigureAmsSlotModalProps) {
  const [filamentPresets, setFilamentPresets] = useState<SlicerPreset[]>([])
  const [selectedPresetId, setSelectedPresetId] = useState<string>('')
  const [selectedCaliIdx, setSelectedCaliIdx] = useState<number>(-1)
  const [colorHex, setColorHex] = useState<string>('')
  const [colorInput, setColorInput] = useState<string>('')
  const [searchQuery, setSearchQuery] = useState('')
  const [showSuccess, setShowSuccess] = useState(false)
  const [showExtendedColors, setShowExtendedColors] = useState(false)
  const [loading, setLoading] = useState(false)
  const [rereadLoading, setRereadLoading] = useState(false)
  const [clearLoading, setClearLoading] = useState(false)
  const [presetsLoading, setPresetsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [catalogColors, setCatalogColors] = useState<ColorEntry[]>([])
  const [catalogColorsLoading, setCatalogColorsLoading] = useState(false)

  // Fetch filament presets when modal opens
  useEffect(() => {
    if (isOpen) {
      setPresetsLoading(true)
      api.getSlicerSettings()
        .then(settings => {
          setFilamentPresets(settings.filament || [])
        })
        .catch(err => {
          console.error('Failed to fetch presets:', err)
        })
        .finally(() => {
          setPresetsLoading(false)
        })
    }
  }, [isOpen])

  // Reset state when modal closes
  useEffect(() => {
    if (!isOpen) {
      setSelectedPresetId('')
      setSelectedCaliIdx(-1)
      setColorHex('')
      setColorInput('')
      setSearchQuery('')
      setShowSuccess(false)
      setShowExtendedColors(false)
      setError(null)
      setCatalogColors([])
    }
  }, [isOpen])

  // Escape key handler
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose()
    }
  }, [onClose])

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown)
      document.body.style.overflow = 'hidden'
      return () => {
        document.removeEventListener('keydown', handleKeyDown)
        document.body.style.overflow = ''
      }
    }
  }, [isOpen, handleKeyDown])

  // Filter presets based on search
  const filteredPresets = useMemo(() => {
    if (!filamentPresets) return []
    const query = searchQuery.toLowerCase()
    return filamentPresets
      .filter(p => !query || p.name.toLowerCase().includes(query))
      .sort((a, b) => {
        const aIsUser = isUserPreset(a.setting_id)
        const bIsUser = isUserPreset(b.setting_id)
        if (aIsUser && !bIsUser) return -1
        if (!aIsUser && bIsUser) return 1
        return a.name.localeCompare(b.name)
      })
  }, [filamentPresets, searchQuery])

  // Get selected preset info for K profile filtering
  const selectedPresetInfo = useMemo(() => {
    if (!selectedPresetId || !filamentPresets) return null
    const preset = filamentPresets.find(p => p.setting_id === selectedPresetId)
    if (!preset) return null

    let nameWithoutSuffix = preset.name.replace(/@.+$/, '').trim()
    if (nameWithoutSuffix.startsWith('# ')) {
      nameWithoutSuffix = nameWithoutSuffix.slice(2).trim()
    }
    const parsed = parsePresetName(nameWithoutSuffix)

    return {
      fullName: nameWithoutSuffix,
      material: parsed.material,
      brand: parsed.brand,
    }
  }, [selectedPresetId, filamentPresets])

  // Filter calibrations based on selected preset and deduplicate
  const matchingCalibrations = useMemo(() => {
    if (!calibrations || !selectedPresetInfo) return []

    const { fullName, material, brand } = selectedPresetInfo
    const upperMaterial = material.toUpperCase()
    const upperBrand = brand.toUpperCase()

    if (!upperMaterial || upperMaterial.length < 2) return []

    const filtered = calibrations.filter(cal => {
      const profileName = (cal.name || cal.filament_id || '').toUpperCase()

      if (upperBrand) {
        if (!profileName.includes(upperBrand)) return false
        if (!profileName.includes(upperMaterial)) return false
        return true
      }

      if (profileName.includes(fullName.toUpperCase())) return true
      if (profileName.includes(upperMaterial)) return true

      return false
    })

    // Deduplicate profiles with same name and k_value (multi-nozzle printers have duplicates)
    // Prefer extruder_id=1 (High Flow) profiles as they're more commonly used on H2D
    const seen = new Map<string, CalibrationProfile>()
    for (const profile of filtered) {
      const key = `${profile.name}|${profile.k_value}`
      const existing = seen.get(key)
      // Prefer extruder_id=1 (High Flow) over extruder_id=0
      if (!existing || (profile.extruder_id === 1 && existing.extruder_id !== 1)) {
        seen.set(key, profile)
      }
    }
    return Array.from(seen.values())
  }, [calibrations, selectedPresetInfo])

  // Auto-select first matching calibration when preset changes
  useEffect(() => {
    if (matchingCalibrations.length > 0) {
      setSelectedCaliIdx(matchingCalibrations[0].cali_idx)
    } else {
      setSelectedCaliIdx(-1)
    }
  }, [selectedPresetId, matchingCalibrations])

  // Fetch catalog colors when preset changes (has brand/material info)
  useEffect(() => {
    const fetchCatalogColors = async () => {
      if (!selectedPresetInfo?.brand && !selectedPresetInfo?.material) {
        setCatalogColors([])
        return
      }

      setCatalogColorsLoading(true)
      try {
        const colors = await api.searchColors(
          selectedPresetInfo.brand || undefined,
          selectedPresetInfo.material || undefined
        )
        setCatalogColors(colors)
      } catch (e) {
        console.error('Failed to fetch catalog colors:', e)
        setCatalogColors([])
      } finally {
        setCatalogColorsLoading(false)
      }
    }

    fetchCatalogColors()
  }, [selectedPresetInfo?.brand, selectedPresetInfo?.material])

  // Handle re-read slot (trigger RFID scan)
  const handleReread = async () => {
    setRereadLoading(true)
    setError(null)
    try {
      await api.rereadSlot(printerSerial, slotInfo.amsId, slotInfo.trayId)
      setShowSuccess(true)
      onSuccess?.()
      setTimeout(() => {
        setShowSuccess(false)
        onClose()
      }, 1500)
    } catch (err) {
      console.error('Failed to re-read slot:', err)
      setError(err instanceof Error ? err.message : 'Failed to re-read slot')
    } finally {
      setRereadLoading(false)
    }
  }

  // Handle clear slot (reset to empty state)
  const handleClear = async () => {
    setClearLoading(true)
    setError(null)
    try {
      await api.clearSlot(printerSerial, slotInfo.amsId, slotInfo.trayId)
      setShowSuccess(true)
      onSuccess?.()
      setTimeout(() => {
        setShowSuccess(false)
        onClose()
      }, 1500)
    } catch (err) {
      console.error('Failed to clear slot:', err)
      setError(err instanceof Error ? err.message : 'Failed to clear slot')
    } finally {
      setClearLoading(false)
    }
  }

  // Handle configure slot
  const handleConfigure = async () => {
    if (!selectedPresetId) {
      setError('Please select a filament profile')
      return
    }

    setLoading(true)
    setError(null)

    try {
      const selectedPreset = filamentPresets.find(p => p.setting_id === selectedPresetId)
      if (!selectedPreset) throw new Error('Selected preset not found')

      const parsed = parsePresetName(selectedPreset.name)
      const color = colorHex || slotInfo.trayColor?.slice(0, 6) || 'FFFFFF'

      // Get preset name for tray_sub_brands (strip @ suffix and # prefix)
      let traySubBrands = selectedPreset.name.replace(/@.+$/, '').trim()
      if (traySubBrands.startsWith('# ')) {
        traySubBrands = traySubBrands.slice(2).trim()
      }

      // Get tray_info_idx: for user presets, fetch detail to get filament_id or derive from base_id
      let trayInfoIdx = convertToTrayInfoIdx(selectedPresetId)

      // For user presets (not starting with GFS), fetch the detail to get the real filament_id
      if (!selectedPresetId.startsWith('GFS')) {
        try {
          const detail = await api.getSettingDetail(selectedPresetId)
          if (detail.filament_id) {
            trayInfoIdx = detail.filament_id
          } else if (detail.base_id) {
            // If no filament_id but has base_id (e.g., "GFSL05_09"), derive tray_info_idx from it
            trayInfoIdx = convertToTrayInfoIdx(detail.base_id)
            console.log(`Derived tray_info_idx from base_id: ${detail.base_id} -> ${trayInfoIdx}`)
          }
        } catch (e) {
          console.warn('Failed to fetch preset detail for filament_id:', e)
          // Fall back to derived tray_info_idx
        }
      }

      // Default temp range based on material
      let tempMin = 190, tempMax = 230
      const mat = parsed.material.toUpperCase()
      if (mat.includes('PLA')) { tempMin = 190; tempMax = 230 }
      else if (mat.includes('PETG')) { tempMin = 220; tempMax = 260 }
      else if (mat.includes('ABS')) { tempMin = 240; tempMax = 280 }
      else if (mat.includes('ASA')) { tempMin = 240; tempMax = 280 }
      else if (mat.includes('TPU')) { tempMin = 200; tempMax = 240 }
      else if (mat.includes('PC')) { tempMin = 260; tempMax = 300 }
      else if (mat.includes('PA') || mat.includes('NYLON')) { tempMin = 250; tempMax = 290 }

      // Configure the slot filament
      await api.setSlotFilament(printerSerial, {
        ams_id: slotInfo.amsId,
        tray_id: slotInfo.trayId,
        tray_info_idx: trayInfoIdx,
        tray_type: parsed.material || 'PLA',
        tray_sub_brands: traySubBrands,
        tray_color: color + 'FF',
        nozzle_temp_min: tempMin,
        nozzle_temp_max: tempMax,
        setting_id: selectedPresetId,
      })

      // Set calibration (K profile)
      const selectedCal = calibrations.find(c => c.cali_idx === selectedCaliIdx)
      const kValue = selectedCal?.k_value || 0
      await api.setCalibration(printerSerial, slotInfo.amsId, slotInfo.trayId, {
        cali_idx: selectedCaliIdx,
        filament_id: selectedCal?.filament_id || '',
        setting_id: selectedCal?.setting_id || '',
        k_value: kValue,
        nozzle_temp_max: tempMax,
      })

      setShowSuccess(true)
      onSuccess?.()
      setTimeout(() => {
        setShowSuccess(false)
        onClose()
      }, 1500)
    } catch (err) {
      console.error('Failed to configure slot:', err)
      setError(err instanceof Error ? err.message : 'Failed to configure slot')
    } finally {
      setLoading(false)
    }
  }

  if (!isOpen) return null

  const displayColor = colorHex || slotInfo.trayColor?.slice(0, 6) || 'FFFFFF'
  const canConfigure = selectedPresetId && !loading && !rereadLoading && !clearLoading

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        class="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div class="relative w-full max-w-lg mx-4 bg-[var(--bg-secondary)] border border-[var(--border-color)] rounded-xl shadow-2xl">
        {/* Success overlay - positioned to cover entire modal */}
        {showSuccess && (
          <div class="absolute inset-0 bg-[var(--bg-secondary)] z-20 flex items-center justify-center rounded-xl">
            <div class="text-center space-y-3">
              <CheckCircle2 class="w-16 h-16 text-[var(--accent-color)] mx-auto" />
              <p class="text-lg font-semibold text-[var(--text-primary)]">Slot Configured!</p>
              <p class="text-sm text-[var(--text-muted)]">Settings sent to printer</p>
            </div>
          </div>
        )}

        {/* Header */}
        <div class="flex items-center justify-between p-4 border-b border-[var(--border-color)]">
          <div class="flex items-center gap-2">
            <Settings class="w-5 h-5 text-[var(--accent-color)]" />
            <h2 class="text-lg font-semibold text-[var(--text-primary)]">Configure AMS Slot</h2>
          </div>
          <button
            onClick={onClose}
            class="p-1 text-[var(--text-muted)] hover:text-[var(--text-primary)] rounded transition-colors"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div class="p-4 space-y-4 max-h-[60vh] overflow-y-auto">

          {/* Slot info */}
          <div class="p-3 bg-[var(--bg-primary)] rounded-lg border border-[var(--border-color)]">
            <p class="text-xs text-[var(--text-muted)] mb-1">Configuring slot:</p>
            <div class="flex items-center gap-2">
              {slotInfo.trayColor && (
                <span
                  class="w-4 h-4 rounded-full border border-white/20"
                  style={{ backgroundColor: `#${slotInfo.trayColor.slice(0, 6)}` }}
                />
              )}
              <span class="text-[var(--text-primary)] font-medium">
                {getAmsLabel(slotInfo.amsId, slotInfo.trayCount)} Slot {slotInfo.trayId + 1}
              </span>
              {slotInfo.trayType && (
                <span class="text-[var(--text-muted)]">({slotInfo.trayType})</span>
              )}
            </div>
          </div>

          {presetsLoading ? (
            <div class="flex justify-center py-8">
              <Loader2 class="w-6 h-6 text-[var(--accent-color)] animate-spin" />
            </div>
          ) : (
            <>
              {/* Filament Profile Select */}
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2 font-medium">
                  Filament Profile <span class="text-red-400">*</span>
                </label>
                <input
                  type="text"
                  placeholder="Search presets..."
                  value={searchQuery}
                  onInput={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
                  class="w-full px-3 py-2 bg-[var(--bg-primary)] border border-[var(--border-color)] rounded-lg text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent-color)] focus:outline-none mb-2"
                />
                <div class="max-h-48 overflow-y-auto space-y-1">
                  {filteredPresets.length === 0 ? (
                    <p class="text-center py-4 text-[var(--text-muted)]">
                      {filamentPresets.length === 0
                        ? 'No cloud presets. Login to Bambu Cloud to sync.'
                        : 'No matching presets found.'}
                    </p>
                  ) : (
                    filteredPresets.map((preset) => (
                      <button
                        key={preset.setting_id}
                        onClick={() => setSelectedPresetId(preset.setting_id)}
                        class={`w-full p-2 rounded-lg border text-left transition-colors ${
                          selectedPresetId === preset.setting_id
                            ? 'bg-[var(--accent-color)]/20 border-[var(--accent-color)]'
                            : 'bg-[var(--bg-primary)] border-[var(--border-color)] hover:border-[var(--text-muted)]'
                        }`}
                      >
                        <div class="flex items-center justify-between">
                          <span class="text-[var(--text-primary)] text-sm truncate">{preset.name}</span>
                          {isUserPreset(preset.setting_id) && (
                            <span class="text-xs px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-400">
                              Custom
                            </span>
                          )}
                        </div>
                      </button>
                    ))
                  )}
                </div>
              </div>

              {/* K Profile Select */}
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2 font-medium">
                  K Profile (Pressure Advance)
                  {selectedPresetInfo && (
                    <span class="ml-2 text-xs text-[var(--accent-color)]">
                      Filtering for: {selectedPresetInfo.fullName}
                    </span>
                  )}
                </label>
                {matchingCalibrations.length > 0 ? (
                  <div class="relative">
                    <select
                      value={selectedCaliIdx}
                      onChange={(e) => setSelectedCaliIdx(parseInt((e.target as HTMLSelectElement).value))}
                      class="w-full px-3 py-2 bg-[var(--bg-primary)] border border-[var(--border-color)] rounded-lg text-[var(--text-primary)] focus:border-[var(--accent-color)] focus:outline-none appearance-none pr-10"
                    >
                      <option value={-1}>No K profile (use default 0.020)</option>
                      {matchingCalibrations.map((cal) => (
                        <option key={cal.cali_idx} value={cal.cali_idx}>
                          {cal.name || cal.filament_id || `Profile ${cal.cali_idx}`} (K={cal.k_value.toFixed(3)})
                        </option>
                      ))}
                    </select>
                    <ChevronDown class="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-muted)] pointer-events-none" />
                  </div>
                ) : selectedPresetId ? (
                  <p class="text-sm text-[var(--text-muted)] italic py-2">
                    No matching K profiles found. Default K=0.020 will be used.
                  </p>
                ) : (
                  <span class="inline-block text-xs px-2 py-1 rounded bg-amber-500/20 text-amber-400 border border-amber-500/30">
                    Select a filament profile first
                  </span>
                )}
                {selectedCaliIdx !== -1 && (
                  <p class="text-xs text-[var(--accent-color)] mt-1">
                    K={calibrations.find(c => c.cali_idx === selectedCaliIdx)?.k_value.toFixed(3)} from printer calibration
                  </p>
                )}
              </div>

              {/* Custom Color */}
              <div>
                <label class="block text-sm text-[var(--text-muted)] mb-2 font-medium">
                  Custom Color (optional)
                </label>

                {/* Catalog colors (matching brand/material) */}
                {(catalogColors.length > 0 || catalogColorsLoading) && (
                  <div class="mb-3">
                    <div class="flex items-center gap-1.5 text-xs text-[var(--text-muted)] mb-1.5">
                      <Palette class="w-3 h-3" />
                      <span>
                        {selectedPresetInfo?.brand || selectedPresetInfo?.material
                          ? `Colors for ${[selectedPresetInfo.brand, selectedPresetInfo.material].filter(Boolean).join(' ')}`
                          : 'Catalog colors'}
                      </span>
                      {catalogColorsLoading && <span class="animate-pulse">...</span>}
                    </div>
                    {catalogColors.length > 0 && (
                      <div class="flex flex-wrap gap-1.5 max-h-20 overflow-y-auto">
                        {catalogColors.map((color) => (
                          <button
                            key={color.id}
                            onClick={() => {
                              const hex = color.hex_color.replace('#', '').toUpperCase()
                              setColorHex(hex)
                              setColorInput(color.color_name)
                            }}
                            class={`w-7 h-7 rounded-md border-2 transition-all ${
                              colorHex === color.hex_color.replace('#', '').toUpperCase()
                                ? 'border-[var(--accent-color)] scale-110'
                                : 'border-white/20 hover:border-white/40'
                            }`}
                            style={{ backgroundColor: color.hex_color }}
                            title={`${color.color_name}${color.material ? ` (${color.material})` : ''}`}
                          />
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Basic colors */}
                <div class="flex flex-wrap gap-1.5 mb-2">
                  {QUICK_COLORS_BASIC.map((color) => (
                    <button
                      key={color.hex}
                      onClick={() => {
                        setColorHex(color.hex)
                        setColorInput(color.name)
                      }}
                      class={`w-7 h-7 rounded-md border-2 transition-all ${
                        colorHex === color.hex
                          ? 'border-[var(--accent-color)] scale-110'
                          : 'border-white/20 hover:border-white/40'
                      }`}
                      style={{ backgroundColor: `#${color.hex}` }}
                      title={color.name}
                    />
                  ))}
                  <button
                    onClick={() => setShowExtendedColors(!showExtendedColors)}
                    class="w-7 h-7 rounded-md border-2 border-white/20 hover:border-white/40 flex items-center justify-center text-white/60 hover:text-white/80 transition-all text-xs"
                    title={showExtendedColors ? 'Show less colors' : 'Show more colors'}
                  >
                    {showExtendedColors ? '−' : '+'}
                  </button>
                </div>
                {/* Extended colors (collapsible) */}
                {showExtendedColors && (
                  <div class="flex flex-wrap gap-1.5 mb-2">
                    {QUICK_COLORS_EXTENDED.map((color) => (
                      <button
                        key={color.hex}
                        onClick={() => {
                          setColorHex(color.hex)
                          setColorInput(color.name)
                        }}
                        class={`w-7 h-7 rounded-md border-2 transition-all ${
                          colorHex === color.hex
                            ? 'border-[var(--accent-color)] scale-110'
                            : 'border-white/20 hover:border-white/40'
                        }`}
                        style={{ backgroundColor: `#${color.hex}` }}
                        title={color.name}
                      />
                    ))}
                  </div>
                )}
                <div class="flex gap-2 items-center">
                  <div
                    class="w-10 h-10 rounded-lg border-2 border-white/20 flex-shrink-0"
                    style={{ backgroundColor: `#${displayColor}` }}
                  />
                  <input
                    type="text"
                    placeholder="Color name or hex (e.g., brown, FF8800)"
                    value={colorInput}
                    onInput={(e) => {
                      const input = (e.target as HTMLInputElement).value
                      setColorInput(input)
                      const cleaned = input.replace(/[^0-9A-Fa-f]/g, '').toUpperCase()
                      if (cleaned.length === 6) {
                        setColorHex(cleaned)
                      } else if (cleaned.length === 3) {
                        setColorHex(cleaned.split('').map(c => c + c).join(''))
                      }
                    }}
                    class="flex-1 px-3 py-2 bg-[var(--bg-primary)] border border-[var(--border-color)] rounded-lg text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent-color)] focus:outline-none text-sm"
                  />
                  {colorHex && (
                    <button
                      onClick={() => {
                        setColorHex('')
                        setColorInput('')
                      }}
                      class="px-2 py-1 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] bg-[var(--bg-tertiary)] rounded"
                      title="Clear custom color"
                    >
                      Clear
                    </button>
                  )}
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div class="flex justify-between p-4 border-t border-[var(--border-color)]">
          {/* Re-read and Reset buttons on the left */}
          <div class="flex gap-2">
            <button
              onClick={handleReread}
              disabled={loading || rereadLoading || clearLoading}
              class="btn flex items-center gap-2 text-amber-400 hover:text-amber-300 hover:bg-amber-500/10"
              title="Trigger RFID re-read without clearing slot"
            >
              {rereadLoading ? (
                <>
                  <Loader2 class="w-4 h-4 animate-spin" />
                  Re-reading...
                </>
              ) : (
                <>
                  <RotateCcw class="w-4 h-4" />
                  Re-read
                </>
              )}
            </button>
            <button
              onClick={handleClear}
              disabled={loading || rereadLoading || clearLoading}
              class="btn flex items-center gap-2 text-red-400 hover:text-red-300 hover:bg-red-500/10"
              title="Clear slot to empty state"
            >
              {clearLoading ? (
                <>
                  <Loader2 class="w-4 h-4 animate-spin" />
                  Clearing...
                </>
              ) : (
                <>
                  <X class="w-4 h-4" />
                  Reset
                </>
              )}
            </button>
          </div>

          {/* Cancel and Configure buttons on the right */}
          <div class="flex gap-2">
            <button onClick={onClose} disabled={loading || rereadLoading || clearLoading} class="btn">
              Cancel
            </button>
            <button
              onClick={handleConfigure}
              disabled={!canConfigure}
              class="btn btn-primary flex items-center gap-2"
            >
              {loading ? (
                <>
                  <Loader2 class="w-4 h-4 animate-spin" />
                  Configuring...
                </>
              ) : (
                <>
                  <Settings class="w-4 h-4" />
                  Configure Slot
                </>
              )}
            </button>
          </div>
        </div>

        {/* Error */}
        {error && (
          <div class="mx-4 mb-4 p-2 bg-red-500/20 border border-red-500/50 rounded text-sm text-red-400">
            {error}
          </div>
        )}
      </div>
    </div>
  )
}
