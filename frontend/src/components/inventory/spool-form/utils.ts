import type { SlicerPreset } from '../../../lib/api'
import type { ColorPreset, FilamentOption } from './types'
import { KNOWN_VARIANTS, DEFAULT_BRANDS, RECENT_COLORS_KEY, MAX_RECENT_COLORS } from './constants'
import { getFilamentOptions } from '../utils'

// Parse a slicer preset name to extract brand, material, and variant
export function parsePresetName(name: string): { brand: string; material: string; variant: string } {
  // Remove @printer suffix (e.g., "@Bambu Lab H2D 0.4 nozzle")
  let cleanName = name.replace(/@.*$/, '').trim()
  // Remove (Custom) tag
  cleanName = cleanName.replace(/\(Custom\)/i, '').trim()
  // Remove leading # or * markers
  cleanName = cleanName.replace(/^[#*]+\s*/, '').trim()

  // Materials list - order matters (longer/more specific first)
  const materials = [
    'PLA-CF', 'PETG-CF', 'ABS-GF', 'ASA-CF', 'PA-CF', 'PAHT-CF', 'PA6-CF', 'PA6-GF',
    'PPA-CF', 'PPA-GF', 'PET-CF', 'PPS-CF', 'PC-CF', 'PC-ABS', 'ABS-GF',
    'PETG', 'PLA', 'ABS', 'ASA', 'PC', 'PA', 'TPU', 'PVA', 'HIPS', 'BVOH', 'PPS', 'PCTG', 'PEEK', 'PEI'
  ]

  // Find material in the name
  let material = ''
  let materialIdx = -1
  for (const m of materials) {
    const idx = cleanName.toUpperCase().indexOf(m.toUpperCase())
    if (idx !== -1) {
      material = m
      materialIdx = idx
      break
    }
  }

  // Brand is everything before the material
  let brand = ''
  if (materialIdx > 0) {
    brand = cleanName.substring(0, materialIdx).trim()
    // Remove trailing spaces/dashes
    brand = brand.replace(/[-_\s]+$/, '')
  }

  // Everything after material is potential variant
  let afterMaterial = ''
  if (materialIdx !== -1 && material) {
    afterMaterial = cleanName.substring(materialIdx + material.length).trim()
    // Remove leading spaces/dashes
    afterMaterial = afterMaterial.replace(/^[-_\s]+/, '')
  }

  // Check for known variant - could be before OR after material
  let variant = ''

  // First check after material (most common)
  for (const v of KNOWN_VARIANTS) {
    if (afterMaterial.toLowerCase().includes(v.toLowerCase())) {
      variant = v
      break
    }
  }

  // If no variant found after material, check if brand contains a known variant
  if (!variant && brand) {
    for (const v of KNOWN_VARIANTS) {
      const variantPattern = new RegExp(`\\s+${v}$`, 'i')
      if (variantPattern.test(brand)) {
        variant = v
        brand = brand.replace(variantPattern, '').trim()
        break
      }
    }
  }

  return { brand, material, variant }
}

// Extract unique brands from cloud presets
export function extractBrandsFromPresets(presets: SlicerPreset[]): string[] {
  const brandSet = new Set<string>(DEFAULT_BRANDS)

  for (const preset of presets) {
    const { brand } = parsePresetName(preset.name)
    if (brand && brand.length > 1) {
      brandSet.add(brand)
    }
  }

  return Array.from(brandSet).sort((a, b) => a.localeCompare(b))
}

// Build filament options from cloud presets
export function buildFilamentOptions(
  cloudPresets: SlicerPreset[],
  configuredPrinterModels: Set<string>
): FilamentOption[] {
  if (cloudPresets.length > 0) {
    const customPresets: FilamentOption[] = []
    const defaultPresetsMap = new Map<string, FilamentOption>()

    for (const preset of cloudPresets) {
      if (preset.is_custom) {
        // Custom presets: include if matches configured printers or no printer filter
        const presetNameUpper = preset.name.toUpperCase()
        const matchesPrinter = configuredPrinterModels.size === 0 ||
          Array.from(configuredPrinterModels).some(model => presetNameUpper.includes(model)) ||
          !presetNameUpper.includes('@')

        if (matchesPrinter) {
          customPresets.push({
            code: preset.setting_id,
            name: preset.name,
            displayName: `${preset.name} (Custom)`,
            isCustom: true,
            allCodes: [preset.setting_id],
          })
        }
      } else {
        // Default presets: deduplicate by base name
        const baseName = preset.name.replace(/@.*$/, '').trim()
        const existing = defaultPresetsMap.get(baseName)
        if (existing) {
          existing.allCodes.push(preset.setting_id)
        } else {
          defaultPresetsMap.set(baseName, {
            code: preset.setting_id,
            name: baseName,
            displayName: baseName,
            isCustom: false,
            allCodes: [preset.setting_id],
          })
        }
      }
    }

    return [
      ...customPresets,
      ...Array.from(defaultPresetsMap.values()),
    ].sort((a, b) => a.displayName.localeCompare(b.displayName))
  }

  // Fallback to hardcoded defaults
  return getFilamentOptions().map(o => ({
    ...o,
    displayName: o.name,
    isCustom: false,
    allCodes: [o.code]
  }))
}

// Find selected preset option
export function findPresetOption(
  slicerFilament: string,
  filamentOptions: FilamentOption[]
): FilamentOption | undefined {
  if (!slicerFilament) return undefined

  // First try exact match on primary code
  let option = filamentOptions.find(o => o.code === slicerFilament)
  if (!option) {
    // Try matching against any code in allCodes
    option = filamentOptions.find(o => o.allCodes.includes(slicerFilament))
  }
  if (!option) {
    // Try case-insensitive match
    const slicerLower = slicerFilament.toLowerCase()
    option = filamentOptions.find(o =>
      o.code.toLowerCase() === slicerLower ||
      o.allCodes.some(c => c.toLowerCase() === slicerLower)
    )
  }
  return option
}

// Recent colors management
export function loadRecentColors(): ColorPreset[] {
  try {
    const stored = localStorage.getItem(RECENT_COLORS_KEY)
    if (stored) {
      return JSON.parse(stored) as ColorPreset[]
    }
  } catch {
    // Ignore errors
  }
  return []
}

export function saveRecentColor(color: ColorPreset, currentRecent: ColorPreset[]): ColorPreset[] {
  // Remove duplicate if exists
  const filtered = currentRecent.filter(
    c => c.hex.toUpperCase() !== color.hex.toUpperCase()
  )
  // Add to front
  const updated = [color, ...filtered].slice(0, MAX_RECENT_COLORS)

  try {
    localStorage.setItem(RECENT_COLORS_KEY, JSON.stringify(updated))
  } catch {
    // Ignore errors
  }

  return updated
}

// Check if a calibration matches based on brand, material, and variant
export function isMatchingCalibration(
  cal: { name?: string; filament_id?: string },
  formData: { material: string; brand: string; subtype: string }
): boolean {
  if (!formData.material) return false

  const profileName = cal.name || ''

  // Remove flow type prefixes
  const cleanName = profileName
    .replace(/^High Flow[_\s]+/i, '')
    .replace(/^Standard[_\s]+/i, '')
    .replace(/^HF[_\s]+/i, '')
    .replace(/^S[_\s]+/i, '')
    .trim()

  const parsed = parsePresetName(cleanName)

  // Match material (required)
  const materialMatch = parsed.material.toUpperCase() === formData.material.toUpperCase()
  if (!materialMatch) return false

  // Match brand if specified in form
  if (formData.brand) {
    const brandMatch = parsed.brand.toLowerCase().includes(formData.brand.toLowerCase()) ||
      formData.brand.toLowerCase().includes(parsed.brand.toLowerCase())
    if (!brandMatch) return false
  }

  // Match variant/subtype if specified in form
  if (formData.subtype) {
    const variantMatch = parsed.variant.toLowerCase().includes(formData.subtype.toLowerCase()) ||
      formData.subtype.toLowerCase().includes(parsed.variant.toLowerCase()) ||
      cleanName.toLowerCase().includes(formData.subtype.toLowerCase())
    if (!variantMatch) return false
  }

  return true
}
