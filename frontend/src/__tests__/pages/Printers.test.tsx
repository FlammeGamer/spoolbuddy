import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, fireEvent, waitFor } from '@testing-library/preact'
import { Printers } from '../../pages/Printers'
import { renderWithProviders } from '../utils'
import { server } from '../setup'
import { http, HttpResponse } from 'msw'
import { mockPrinters } from '../mocks/data'

// Mock the toast hook
vi.mock('../../lib/toast', () => ({
  useToast: () => ({
    showToast: vi.fn(),
  }),
  ToastProvider: ({ children }: { children: preact.ComponentChildren }) => children,
}))

describe('Printers', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  describe('Header', () => {
    it('renders page title', async () => {
      renderWithProviders(<Printers />)

      expect(screen.getByText('Printers')).toBeInTheDocument()
    })

    it('renders subtitle', async () => {
      renderWithProviders(<Printers />)

      expect(screen.getByText('Manage your Bambu Lab printers')).toBeInTheDocument()
    })

    it('renders Discover button', async () => {
      renderWithProviders(<Printers />)

      expect(screen.getByRole('button', { name: /Discover/ })).toBeInTheDocument()
    })

    it('renders Add Printer button', async () => {
      renderWithProviders(<Printers />)

      expect(screen.getByRole('button', { name: /Add Printer/ })).toBeInTheDocument()
    })
  })

  describe('Loading State', () => {
    it('shows loading while fetching', async () => {
      server.use(
        http.get('/api/printers', async () => {
          await new Promise(resolve => setTimeout(resolve, 100))
          return HttpResponse.json(mockPrinters)
        })
      )

      renderWithProviders(<Printers />)

      expect(screen.getByText('Loading printers...')).toBeInTheDocument()
    })
  })

  describe('Printer List', () => {
    it('renders printers after loading', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        // At least one printer should be visible after loading
        expect(screen.getByText('X1 Carbon')).toBeInTheDocument()
      })
    })

    it('displays printer serial numbers', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        // At least one printer serial should be visible
        expect(screen.getByText('00M09A123456789')).toBeInTheDocument()
      })
    })

    it('displays printer IP addresses', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        // At least one printer IP should be visible
        expect(screen.getByText(/192.168.1.100/)).toBeInTheDocument()
      })
    })

    it('displays printer models', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByText(/X1C/)).toBeInTheDocument()
        // P1S model is shown for the second printer
      })
    })
  })

  describe('Connection Status', () => {
    it('shows Connected badge for connected printer', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByText('Connected')).toBeInTheDocument()
      })
    })

    it('shows Disconnect button for connected printers', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByRole('button', { name: 'Disconnect' })).toBeInTheDocument()
      })
    })

    it('shows connection status indicator', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        // Should show some connection status (either Connected or Offline)
        const connected = screen.queryByText('Connected')
        const offline = screen.queryByText('Offline')
        expect(connected || offline).toBeTruthy()
      })
    })
  })

  describe('Empty State', () => {
    it('shows empty state when no printers', async () => {
      server.use(
        http.get('/api/printers', () => {
          return HttpResponse.json([])
        })
      )

      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByText('No printers yet')).toBeInTheDocument()
      })
    })

    it('shows Add Your First Printer button in empty state', async () => {
      server.use(
        http.get('/api/printers', () => {
          return HttpResponse.json([])
        })
      )

      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /Add Your First Printer/ })).toBeInTheDocument()
      })
    })
  })

  describe('Auto-Connect Toggle', () => {
    it('shows auto-connect toggle for each printer', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        // "Auto" labels for the toggles - at least one should be visible
        const autoLabels = screen.getAllByText('Auto')
        expect(autoLabels.length).toBeGreaterThanOrEqual(1)
      })
    })
  })

  describe('Delete Printer', () => {
    it('shows delete button for each printer', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        const deleteButtons = screen.getAllByTitle('Delete printer')
        // At least one delete button should be visible for printers in list
        expect(deleteButtons.length).toBeGreaterThanOrEqual(1)
      })
    })

    it('opens delete confirmation modal when delete button clicked', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByText('X1 Carbon')).toBeInTheDocument()
      })

      const deleteButtons = screen.getAllByTitle('Delete printer')
      fireEvent.click(deleteButtons[0])

      await waitFor(() => {
        expect(screen.getByText('Delete Printer')).toBeInTheDocument()
        expect(screen.getByText(/Are you sure you want to delete this printer/)).toBeInTheDocument()
      })
    })

    it('closes confirmation modal when Cancel clicked', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.getByText('X1 Carbon')).toBeInTheDocument()
      })

      const deleteButtons = screen.getAllByTitle('Delete printer')
      fireEvent.click(deleteButtons[0])

      await waitFor(() => {
        expect(screen.getByText('Delete Printer')).toBeInTheDocument()
      })

      fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))

      await waitFor(() => {
        expect(screen.queryByText(/Are you sure you want to delete this printer/)).not.toBeInTheDocument()
      })
    })
  })

  describe('Add Printer Modal', () => {
    it('opens modal when Add Printer button is clicked', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.queryByText('Loading printers...')).not.toBeInTheDocument()
      })

      fireEvent.click(screen.getByRole('button', { name: /Add Printer/ }))

      await waitFor(() => {
        // Modal title should appear
        const addPrinterHeadings = screen.getAllByText('Add Printer')
        expect(addPrinterHeadings.length).toBeGreaterThan(1) // Title + button
      })
    })

    it('shows required fields in add modal', async () => {
      renderWithProviders(<Printers />)

      await waitFor(() => {
        expect(screen.queryByText('Loading printers...')).not.toBeInTheDocument()
      })

      fireEvent.click(screen.getByRole('button', { name: /Add Printer/ }))

      await waitFor(() => {
        expect(screen.getByText(/Serial Number/)).toBeInTheDocument()
        expect(screen.getByText(/IP Address/)).toBeInTheDocument()
        expect(screen.getByText(/Access Code/)).toBeInTheDocument()
      })
    })
  })

  describe('Info Card', () => {
    it('shows printer connection info', async () => {
      renderWithProviders(<Printers />)

      expect(screen.getByText('Printer Connection')).toBeInTheDocument()
      expect(screen.getByText(/MQTT over your local network/)).toBeInTheDocument()
    })
  })
})
