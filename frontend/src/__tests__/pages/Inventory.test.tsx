import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/preact'
import { Inventory } from '../../pages/Inventory'
import { renderWithProviders } from '../utils'
import { server } from '../setup'
import { http, HttpResponse } from 'msw'
import { mockSpools } from '../mocks/data'

// Mock the toast hook
vi.mock('../../lib/toast', () => ({
  useToast: () => ({
    showToast: vi.fn(),
  }),
  ToastProvider: ({ children }: { children: preact.ComponentChildren }) => children,
}))

// Mock the SpoolsTable component to avoid React-table compatibility issues
vi.mock('../../components/inventory/SpoolsTable', () => ({
  SpoolsTable: ({ spools, loading }: { spools: unknown[]; loading: boolean }) => (
    <div data-testid="spools-table">
      {loading ? (
        <div>Loading...</div>
      ) : spools.length === 0 ? (
        <div>No spools</div>
      ) : (
        <div>
          {(spools as { material: string; color_name: string; brand: string }[]).map((s, i) => (
            <div key={i}>
              <span>{s.material}</span>
              <span>{s.color_name}</span>
              <span>{s.brand}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  ),
}))

describe('Inventory', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  describe('Header', () => {
    it('renders page title', async () => {
      renderWithProviders(<Inventory />)

      expect(screen.getByText('Inventory')).toBeInTheDocument()
    })

    it('renders subtitle', async () => {
      renderWithProviders(<Inventory />)

      expect(screen.getByText('Manage your filament spools')).toBeInTheDocument()
    })

    it('renders Add Spool button', async () => {
      renderWithProviders(<Inventory />)

      expect(screen.getByRole('button', { name: /Add Spool/ })).toBeInTheDocument()
    })
  })

  describe('Loading State', () => {
    it('shows loading text while fetching', async () => {
      server.use(
        http.get('/api/spools', async () => {
          await new Promise(resolve => setTimeout(resolve, 100))
          return HttpResponse.json(mockSpools)
        })
      )

      renderWithProviders(<Inventory />)

      expect(screen.getByText('Loading...')).toBeInTheDocument()
    })
  })

  describe('Spool List', () => {
    it('renders spools after loading', async () => {
      renderWithProviders(<Inventory />)

      await waitFor(() => {
        // Check for material types from mockSpools
        expect(screen.getByText('PLA')).toBeInTheDocument()
        expect(screen.getByText('PETG')).toBeInTheDocument()
      })
    })

    it('displays spool colors', async () => {
      renderWithProviders(<Inventory />)

      await waitFor(() => {
        expect(screen.getByText('Black')).toBeInTheDocument()
        expect(screen.getByText('Red')).toBeInTheDocument()
      })
    })

    it('displays spool brands', async () => {
      renderWithProviders(<Inventory />)

      await waitFor(() => {
        expect(screen.getByText(/Bambu Lab/)).toBeInTheDocument()
        expect(screen.getByText(/Generic/)).toBeInTheDocument()
      })
    })
  })

  describe('Empty State', () => {
    it('shows empty state when no spools', async () => {
      server.use(
        http.get('/api/spools', () => {
          return HttpResponse.json([])
        })
      )

      renderWithProviders(<Inventory />)

      await waitFor(() => {
        // The SpoolsTable component shows an empty state
        expect(screen.queryByText('Loading...')).not.toBeInTheDocument()
      })
    })
  })

  describe('Error Handling', () => {
    it('shows error message on API failure', async () => {
      server.use(
        http.get('/api/spools', () => {
          return new HttpResponse('Server error', { status: 500 })
        })
      )

      renderWithProviders(<Inventory />)

      await waitFor(() => {
        expect(screen.getByText(/Server error/)).toBeInTheDocument()
      })
    })
  })

  describe('Add Spool Modal', () => {
    it('opens modal when Add Spool button is clicked', async () => {
      renderWithProviders(<Inventory />)

      await waitFor(() => {
        expect(screen.queryByText('Loading...')).not.toBeInTheDocument()
      })

      const addButton = screen.getByRole('button', { name: /Add Spool/ })
      fireEvent.click(addButton)

      await waitFor(() => {
        // Modal should be open with title "Add New Spool"
        expect(screen.getByText('Add New Spool')).toBeInTheDocument()
      })
    })
  })
})
