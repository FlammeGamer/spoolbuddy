import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/preact'
import { Dashboard } from '../../pages/Dashboard'
import { renderWithProviders } from '../utils'
import { server } from '../setup'
import { http, HttpResponse } from 'msw'
import { mockAuthenticatedCloudStatus } from '../mocks/data'

describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  describe('Device Status', () => {
    it('renders device section', async () => {
      renderWithProviders(<Dashboard />)

      expect(screen.getByText('Device')).toBeInTheDocument()
    })

    it('shows disconnected state initially', async () => {
      renderWithProviders(<Dashboard />)

      expect(screen.getByText('Disconnected')).toBeInTheDocument()
    })

    it('shows scale reading area', async () => {
      renderWithProviders(<Dashboard />)

      expect(screen.getByText('Scale')).toBeInTheDocument()
    })

    it('shows NFC status area', async () => {
      renderWithProviders(<Dashboard />)

      expect(screen.getByText('NFC')).toBeInTheDocument()
      expect(screen.getByText('No tag')).toBeInTheDocument()
    })
  })

  describe('Current Spool Section', () => {
    it('renders current spool section header', async () => {
      renderWithProviders(<Dashboard />)

      expect(screen.getByText('Current Spool')).toBeInTheDocument()
    })

    it('shows empty state when no tag detected', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('Place a spool on the scale to identify it')).toBeInTheDocument()
      })
    })

    it('shows "Ready to scan" text in empty state', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('Ready to scan')).toBeInTheDocument()
      })
    })

    it('shows NFC hint in empty state', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('NFC tag will be read automatically')).toBeInTheDocument()
      })
    })
  })

  describe('Printers Section', () => {
    it('renders printers section when printers exist', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('Printers')).toBeInTheDocument()
      })
    })

    it('shows printer names in list', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('X1 Carbon')).toBeInTheDocument()
      })
    })

    it('shows View all link', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('View all')).toBeInTheDocument()
      })
    })
  })

  describe('Cloud Banner', () => {
    it('shows cloud banner when not authenticated', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('Connect to Bambu Cloud')).toBeInTheDocument()
      })
    })

    it('hides cloud banner when authenticated', async () => {
      server.use(
        http.get('/api/cloud/status', () => {
          return HttpResponse.json(mockAuthenticatedCloudStatus)
        })
      )

      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.queryByText('Connect to Bambu Cloud')).not.toBeInTheDocument()
      })
    })

    it('hides cloud banner when dismissed', async () => {
      localStorage.setItem('spoolbuddy-cloud-banner-dismissed', 'true')

      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.queryByText('Connect to Bambu Cloud')).not.toBeInTheDocument()
      })
    })

    it('shows Connect button in cloud banner', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByRole('link', { name: 'Connect' })).toBeInTheDocument()
      })
    })
  })

  describe('Cloud Status Indicator', () => {
    it('shows offline indicator when not authenticated', async () => {
      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('Offline')).toBeInTheDocument()
      })
    })

    it('shows cloud indicator when authenticated', async () => {
      server.use(
        http.get('/api/cloud/status', () => {
          return HttpResponse.json(mockAuthenticatedCloudStatus)
        })
      )

      renderWithProviders(<Dashboard />)

      await waitFor(() => {
        expect(screen.getByText('Cloud')).toBeInTheDocument()
      })
    })
  })

  describe('Add Spool Button', () => {
    it('renders Add Spool button', async () => {
      renderWithProviders(<Dashboard />)

      expect(screen.getByRole('link', { name: /Add Spool/ })).toBeInTheDocument()
    })

    it('links to inventory with add param', async () => {
      renderWithProviders(<Dashboard />)

      const addButton = screen.getByRole('link', { name: /Add Spool/ })
      expect(addButton).toHaveAttribute('href', '/inventory?add=true')
    })
  })
})
