import { useState, useEffect, useRef } from 'preact/hooks'

export interface AutocompleteOption {
  value: string
  label: string
}

interface CustomAutocompleteProps {
  options: AutocompleteOption[]
  value: string
  onChange: (value: string) => void
  onSelect?: (option: AutocompleteOption) => void
  placeholder?: string
  className?: string
}

export function CustomAutocomplete({
  options,
  value,
  onChange,
  onSelect,
  placeholder = 'Type to search...',
  className = '',
}: CustomAutocompleteProps) {
  const [isOpen, setIsOpen] = useState(false)
  const [inputValue, setInputValue] = useState(value)
  const containerRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // Sync external value changes
  useEffect(() => {
    setInputValue(value)
  }, [value])

  const filteredOptions = inputValue
    ? options.filter(o =>
        o.label.toLowerCase().includes(inputValue.toLowerCase()) ||
        o.value.toLowerCase().includes(inputValue.toLowerCase())
      ).slice(0, 50) // Limit results for performance
    : options.slice(0, 50)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false)
      }
    }
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isOpen])

  const handleInputChange = (e: Event) => {
    const newValue = (e.target as HTMLInputElement).value
    setInputValue(newValue)
    onChange(newValue)
    setIsOpen(true)
  }

  const handleSelect = (opt: AutocompleteOption) => {
    setInputValue(opt.label)
    onChange(opt.value)
    onSelect?.(opt)
    setIsOpen(false)
  }

  return (
    <div class={`relative ${className}`} ref={containerRef}>
      <input
        ref={inputRef}
        type="text"
        class="input w-full"
        placeholder={placeholder}
        value={inputValue}
        onInput={handleInputChange}
        onFocus={() => setIsOpen(true)}
      />

      {isOpen && filteredOptions.length > 0 && (
        <div class="absolute z-50 w-full mt-1 bg-[var(--bg-secondary)] border border-[var(--border-color)] rounded-lg shadow-lg max-h-64 overflow-y-auto">
          {filteredOptions.map(opt => (
            <button
              key={opt.value}
              type="button"
              class={`w-full px-3 py-2 text-left text-sm hover:bg-[var(--bg-tertiary)] transition-colors ${
                opt.value === value
                  ? 'bg-[var(--accent-color)]/10 text-[var(--accent-color)] font-medium'
                  : 'text-[var(--text-primary)]'
              }`}
              onClick={() => handleSelect(opt)}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
