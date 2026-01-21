import { Cloud, CloudOff } from 'lucide-preact'
import type { FilamentSectionProps } from './types'
import { MATERIALS, WEIGHTS } from './constants'
import { parsePresetName } from './utils'
import { CustomSelect } from '../../ui/CustomSelect'
import { CustomAutocomplete } from '../../ui/CustomAutocomplete'

export function FilamentSection({
  formData,
  updateField,
  cloudAuthenticated,
  loadingCloudPresets,
  presetInputValue,
  setPresetInputValue,
  selectedPresetOption,
  filamentOptions,
  availableBrands,
}: FilamentSectionProps) {
  return (
    <div class="form-section">
      <div class="form-section-header">
        <h3>Filament</h3>
      </div>
      <div class="form-section-content">
        {/* Slicer Preset - REQUIRED */}
        <div class="form-field">
          <div class="flex items-center justify-between mb-1">
            <label class="form-label">
              Slicer Preset <span class="text-[var(--error-color)]">*</span>
            </label>
            <span class={`flex items-center gap-1 text-xs ${cloudAuthenticated ? 'text-green-500' : 'text-[var(--text-muted)]'}`}>
              {loadingCloudPresets ? (
                <span class="flex items-center gap-1">
                  <span class="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
                  Loading...
                </span>
              ) : cloudAuthenticated ? (
                <><Cloud class="w-3 h-3" /> Cloud</>
              ) : (
                <><CloudOff class="w-3 h-3" /> Local</>
              )}
            </span>
          </div>

          {loadingCloudPresets ? (
            <div class="input bg-[var(--bg-tertiary)] animate-pulse h-[38px]" />
          ) : (
            <CustomAutocomplete
              options={filamentOptions.map(o => ({ value: o.code, label: o.displayName }))}
              value={presetInputValue}
              onChange={(val) => {
                setPresetInputValue(val)
                // Look up option by code or displayName
                let option = filamentOptions.find(o => o.code === val)
                if (!option) {
                  option = filamentOptions.find(o => o.displayName === val)
                }
                if (!option) {
                  const valLower = val.toLowerCase()
                  option = filamentOptions.find(o => o.displayName.toLowerCase() === valLower)
                }
                if (option) {
                  updateField('slicer_filament', option.code)
                  const parsed = parsePresetName(option.name)
                  if (parsed.brand) updateField('brand', parsed.brand)
                  if (parsed.material) updateField('material', parsed.material)
                  if (parsed.variant) {
                    updateField('subtype', parsed.variant)
                  } else {
                    updateField('subtype', '')
                  }
                } else {
                  updateField('slicer_filament', val)
                }
              }}
              onSelect={(opt) => {
                const option = filamentOptions.find(o => o.code === opt.value)
                if (option) {
                  setPresetInputValue(option.displayName)
                  updateField('slicer_filament', option.code)
                  const parsed = parsePresetName(option.name)
                  if (parsed.brand) updateField('brand', parsed.brand)
                  if (parsed.material) updateField('material', parsed.material)
                  if (parsed.variant) {
                    updateField('subtype', parsed.variant)
                  } else {
                    updateField('subtype', '')
                  }
                }
              }}
              placeholder="Type to search presets..."
              className={!formData.slicer_filament ? '[&>input]:border-[var(--warning-color)]/50' : ''}
            />
          )}

          {selectedPresetOption && (
            <p class="text-xs text-[var(--accent-color)] mt-1 flex items-center gap-1">
              <span class="inline-block w-1.5 h-1.5 rounded-full bg-[var(--accent-color)]" />
              {selectedPresetOption.displayName}
            </p>
          )}
          {!cloudAuthenticated && !loadingCloudPresets && !selectedPresetOption && (
            <p class="text-xs text-[var(--text-muted)] mt-1">
              Login to Bambu Cloud in Settings for custom presets
            </p>
          )}
        </div>

        {/* Material + Subtype */}
        <div class="form-row">
          <div class="form-field">
            <label class="form-label">Material</label>
            <CustomSelect
              options={[
                { value: '', label: 'Select...' },
                ...MATERIALS.map(m => ({ value: m, label: m }))
              ]}
              value={formData.material}
              onChange={(val) => updateField('material', val as string)}
              placeholder="Select..."
            />
          </div>
          <div class="form-field">
            <label class="form-label">Variant</label>
            <input
              type="text"
              class="input"
              placeholder="e.g., Silk, Matte, HF"
              value={formData.subtype}
              onInput={(e) => updateField('subtype', (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>

        {/* Brand + Weight */}
        <div class="form-row">
          <div class="form-field">
            <label class="form-label">Brand</label>
            <CustomSelect
              options={[
                { value: '', label: 'Select...' },
                ...availableBrands.map(brand => ({ value: brand, label: brand }))
              ]}
              value={formData.brand}
              onChange={(val) => updateField('brand', val as string)}
              placeholder="Select..."
              searchable
            />
          </div>
          <div class="form-field">
            <label class="form-label">Spool Weight</label>
            <CustomSelect
              options={WEIGHTS.map(w => ({
                value: w,
                label: w >= 1000 ? `${w / 1000}kg` : `${w}g`
              }))}
              value={formData.label_weight}
              onChange={(val) => updateField('label_weight', val as number)}
            />
          </div>
        </div>
      </div>
    </div>
  )
}
