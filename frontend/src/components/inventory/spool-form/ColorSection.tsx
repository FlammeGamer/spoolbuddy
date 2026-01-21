import { useState, useMemo, useEffect } from 'preact/hooks'
import { ChevronDown, ChevronUp, Search, Clock, Palette } from 'lucide-preact'
import type { ColorSectionProps } from './types'
import { QUICK_COLORS, ALL_COLORS } from './constants'
import { api, ColorEntry } from '../../../lib/api'
import { CustomAutocomplete } from '../../ui/CustomAutocomplete'

export function ColorSection({
  formData,
  updateField,
  recentColors,
  onColorUsed,
}: ColorSectionProps) {
  const [showAllColors, setShowAllColors] = useState(false)
  const [colorSearch, setColorSearch] = useState('')
  const [catalogColors, setCatalogColors] = useState<ColorEntry[]>([])
  const [loadingCatalog, setLoadingCatalog] = useState(false)

  // Fetch matching colors from catalog when brand or material changes
  useEffect(() => {
    const fetchCatalogColors = async () => {
      if (!formData.brand && !formData.material) {
        setCatalogColors([])
        return
      }

      setLoadingCatalog(true)
      try {
        const colors = await api.searchColors(formData.brand || undefined, formData.material || undefined)
        setCatalogColors(colors)
      } catch (e) {
        console.error('Failed to fetch catalog colors:', e)
        setCatalogColors([])
      } finally {
        setLoadingCatalog(false)
      }
    }

    fetchCatalogColors()
  }, [formData.brand, formData.material])

  const selectColor = (hex: string, name: string) => {
    // Ensure hex doesn't have # prefix for storage
    const cleanHex = hex.replace('#', '')
    updateField('rgba', `#${cleanHex}`)
    updateField('color_name', name)
    onColorUsed({ hex: cleanHex, name })
  }

  // Filter colors based on search
  const filteredColors = useMemo(() => {
    if (!colorSearch) return showAllColors ? ALL_COLORS : QUICK_COLORS
    const search = colorSearch.toLowerCase()
    return ALL_COLORS.filter(c =>
      c.name.toLowerCase().includes(search) ||
      c.hex.toLowerCase().includes(search)
    )
  }, [colorSearch, showAllColors])

  // Check if current color is selected
  const isSelected = (hex: string) =>
    formData.rgba.replace('#', '').toUpperCase() === hex.replace('#', '').toUpperCase()

  const currentHex = formData.rgba.replace('#', '')

  return (
    <div class="form-section">
      <div class="form-section-header">
        <h3>Color</h3>
      </div>
      <div class="form-section-content">
        {/* Color Preview Banner */}
        <div
          class="h-10 rounded-lg flex items-center justify-between px-3 border border-[var(--border-color)] relative overflow-hidden transition-all"
          style={{ backgroundColor: formData.rgba }}
        >
          <span
            class="text-sm font-semibold px-2 py-0.5 rounded-full relative z-10 shadow-sm"
            style={{
              backgroundColor: 'rgba(255,255,255,0.95)',
              color: '#333'
            }}
          >
            {formData.color_name || 'Select a color'}
          </span>
          <span
            class="font-mono text-xs px-2 py-0.5 rounded-full relative z-10 shadow-sm"
            style={{
              backgroundColor: 'rgba(0,0,0,0.7)',
              color: '#fff'
            }}
          >
            #{currentHex.toUpperCase()}
          </span>
        </div>

        {/* Catalog Colors (matching brand/material) */}
        {(catalogColors.length > 0 || loadingCatalog) && (
          <div class="space-y-1.5">
            <div class="flex items-center gap-1.5 text-xs text-[var(--text-muted)]">
              <Palette class="w-3 h-3" />
              <span>
                {formData.brand || formData.material
                  ? `Colors for ${[formData.brand, formData.material].filter(Boolean).join(' ')}`
                  : 'Catalog colors'}
              </span>
              {loadingCatalog && <span class="animate-pulse">...</span>}
            </div>
            {catalogColors.length > 0 && (
              <div class="flex flex-wrap gap-1.5">
                {catalogColors.map(color => (
                  <button
                    key={color.id}
                    type="button"
                    onClick={() => selectColor(color.hex_color, color.color_name)}
                    class={`w-6 h-6 rounded border-2 transition-all hover:scale-110 hover:z-20 relative group ${
                      isSelected(color.hex_color)
                        ? 'border-[var(--accent-color)] ring-1 ring-[var(--accent-color)]/30 scale-110'
                        : 'border-[var(--border-color)]'
                    }`}
                    style={{ backgroundColor: color.hex_color }}
                    title={`${color.color_name}${color.material ? ` (${color.material})` : ''}`}
                  >
                    <span
                      class="absolute -bottom-7 left-1/2 -translate-x-1/2 px-2 py-0.5 bg-[var(--bg-secondary)] border border-[var(--border-color)] rounded text-xs whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-10 shadow-lg"
                    >
                      {color.color_name}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Recently Used Colors */}
        {recentColors.length > 0 && (
          <div class="flex items-center gap-2">
            <div class="flex items-center gap-1.5 text-xs text-[var(--text-muted)] shrink-0">
              <Clock class="w-3 h-3" />
              <span>Recent</span>
            </div>
            <div class="flex flex-wrap gap-1.5">
              {recentColors.map(color => (
                <button
                  key={color.hex}
                  type="button"
                  onClick={() => selectColor(color.hex, color.name)}
                  class={`w-6 h-6 rounded border-2 transition-all hover:scale-110 ${
                    isSelected(color.hex)
                      ? 'border-[var(--accent-color)] ring-1 ring-[var(--accent-color)]/30 scale-110'
                      : 'border-[var(--border-color)]'
                  }`}
                  style={{ backgroundColor: `#${color.hex}` }}
                  title={color.name}
                />
              ))}
            </div>
          </div>
        )}

        {/* Color Search and Default Swatches - shown when no catalog colors found */}
        {catalogColors.length === 0 && (
          <>
            {/* Color Search */}
            <div class="relative">
              <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-muted)] pointer-events-none" />
              <input
                type="text"
                class="input"
                style={{ paddingLeft: '40px' }}
                placeholder="Search colors..."
                value={colorSearch}
                onInput={(e) => setColorSearch((e.target as HTMLInputElement).value)}
              />
            </div>

            {/* Color Swatches Grid */}
            <div class="space-y-1.5">
              <div class="flex items-center justify-between text-xs text-[var(--text-muted)]">
                <span>{colorSearch ? 'Search results' : (showAllColors ? 'All colors' : 'Common colors')}</span>
                {!colorSearch && (
                  <button
                    type="button"
                    onClick={() => setShowAllColors(!showAllColors)}
                    class="flex items-center gap-1 hover:text-[var(--text-primary)] transition-colors"
                  >
                    {showAllColors ? (
                      <>Show less <ChevronUp class="w-3 h-3" /></>
                    ) : (
                      <>Show all <ChevronDown class="w-3 h-3" /></>
                    )}
                  </button>
                )}
          </div>
          <div class="flex flex-wrap gap-1.5">
            {filteredColors.map(color => (
              <button
                key={color.hex}
                type="button"
                onClick={() => selectColor(color.hex, color.name)}
                class={`w-6 h-6 rounded border-2 transition-all hover:scale-110 relative group ${
                  isSelected(color.hex)
                    ? 'border-[var(--accent-color)] ring-1 ring-[var(--accent-color)]/30 scale-110'
                    : 'border-[var(--border-color)]'
                }`}
                style={{ backgroundColor: `#${color.hex}` }}
                title={color.name}
              >
                {/* Tooltip on hover */}
                <span
                  class="absolute -bottom-7 left-1/2 -translate-x-1/2 px-2 py-0.5 bg-[var(--bg-secondary)] border border-[var(--border-color)] rounded text-xs whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-10 shadow-lg"
                >
                  {color.name}
                </span>
              </button>
            ))}
            {filteredColors.length === 0 && (
              <p class="text-sm text-[var(--text-muted)] py-1">No colors match your search</p>
            )}
          </div>
        </div>
          </>
        )}

        {/* Manual Color Input */}
        <div class="form-row">
          <div class="form-field">
            <label class="form-label">Color Name</label>
            <CustomAutocomplete
              options={ALL_COLORS.map(c => ({ value: c.name, label: c.name }))}
              value={formData.color_name}
              onChange={(val) => {
                updateField('color_name', val)
                const preset = ALL_COLORS.find(c => c.name.toLowerCase() === val.toLowerCase())
                if (preset) {
                  updateField('rgba', `#${preset.hex}`)
                  onColorUsed(preset)
                }
              }}
              onSelect={(opt) => {
                const preset = ALL_COLORS.find(c => c.name === opt.value)
                if (preset) {
                  updateField('color_name', preset.name)
                  updateField('rgba', `#${preset.hex}`)
                  onColorUsed(preset)
                }
              }}
              placeholder="Type or select..."
            />
          </div>
          <div class="form-field">
            <label class="form-label">Hex Color</label>
            <div class="flex gap-2">
              <div class="relative flex-1">
                <span class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)]">#</span>
                <input
                  type="text"
                  class="input font-mono uppercase"
                  style={{ paddingLeft: '28px' }}
                  placeholder="RRGGBB"
                  value={currentHex.toUpperCase()}
                  onInput={(e) => {
                    let val = (e.target as HTMLInputElement).value.replace('#', '').replace(/[^0-9A-Fa-f]/g, '')
                    if (val.length <= 8) updateField('rgba', `#${val}`)
                  }}
                />
              </div>
              <input
                type="color"
                class="w-11 h-[38px] rounded-lg cursor-pointer border border-[var(--border-color)] shrink-0"
                value={formData.rgba.substring(0, 7)}
                onInput={(e) => updateField('rgba', (e.target as HTMLInputElement).value)}
                title="Pick custom color"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
