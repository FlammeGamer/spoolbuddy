import { useState, useEffect, useCallback } from 'preact/hooks'
import { X } from 'lucide-preact'
import type { Spool } from '../lib/api'
import { SpoolIcon } from './AmsCard'

interface LinkSpoolModalProps {
  isOpen: boolean
  onClose: () => void
  tagId: string
  untaggedSpools: Spool[]
  onLink: (spool: Spool) => void
}

export function LinkSpoolModal({
  isOpen,
  onClose,
  tagId,
  untaggedSpools,
  onLink,
}: LinkSpoolModalProps) {
  const [selectedSpool, setSelectedSpool] = useState<Spool | null>(null)

  // Handle escape key
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      handleClose()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown)
      document.body.style.overflow = 'hidden'
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
      document.body.style.overflow = ''
    }
  }, [isOpen, handleKeyDown])

  if (!isOpen) return null

  const handleConfirm = () => {
    if (selectedSpool) {
      onLink(selectedSpool)
      setSelectedSpool(null)
    }
  }

  const handleClose = () => {
    setSelectedSpool(null)
    onClose()
  }

  return (
    <div class="modal-overlay animate-fade-in" onClick={handleClose}>
      <div
        class="modal max-w-lg animate-slide-up"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div class="modal-header">
          <div>
            <h2 class="modal-title">Link Tag to Spool</h2>
            <p class="text-sm text-[var(--text-muted)] font-mono">{tagId}</p>
          </div>
          <button
            onClick={handleClose}
            class="btn btn-icon"
            title="Close"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div class="modal-body space-y-3 max-h-[400px] overflow-y-auto">
          <p class="text-sm text-[var(--text-secondary)]">
            Select a spool to link this tag to:
          </p>

          {untaggedSpools.length === 0 ? (
            <div class="text-center py-8 text-[var(--text-muted)]">
              No spools without tags found
            </div>
          ) : (
            <div class="space-y-2">
              {untaggedSpools.map((spool) => (
                <button
                  key={spool.id}
                  type="button"
                  onClick={() => setSelectedSpool(spool)}
                  class={`w-full flex items-center gap-3 p-3 rounded-lg border-2 transition-all text-left ${
                    selectedSpool?.id === spool.id
                      ? 'border-[var(--accent-color)] bg-[var(--accent-color)]/10'
                      : 'border-[var(--border-color)] hover:border-[var(--accent-color)]/50 hover:bg-[var(--bg-tertiary)]'
                  }`}
                >
                  <SpoolIcon
                    color={spool.rgba ? `#${spool.rgba.slice(0, 6)}` : '#808080'}
                    isEmpty={false}
                    size={40}
                  />
                  <div class="flex-1 min-w-0">
                    <div class="font-medium text-[var(--text-primary)] truncate">
                      {spool.color_name || 'Unknown color'}
                    </div>
                    <div class="text-sm text-[var(--text-secondary)] truncate">
                      {spool.brand} • {spool.material}
                      {spool.subtype && ` ${spool.subtype}`}
                    </div>
                  </div>
                  {spool.weight_current !== null && (
                    <div class="text-sm font-mono text-[var(--text-muted)]">
                      {spool.weight_current}g
                    </div>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div class="modal-footer">
          <button onClick={handleClose} class="btn btn-secondary">
            Cancel
          </button>
          <button
            onClick={handleConfirm}
            disabled={!selectedSpool}
            class="btn btn-primary"
          >
            Link Tag
          </button>
        </div>
      </div>
    </div>
  )
}
