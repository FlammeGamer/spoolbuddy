import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/preact'
import { AmsCard, ExternalSpool, SpoolIcon } from '../../components/AmsCard'
import {
  mockAmsUnit,
  mockHtAmsUnit,
  mockDualNozzleAmsUnits,
  mockCalibrations,
} from '../mocks/data'
import { AmsUnit } from '../../lib/websocket'

describe('AmsCard', () => {
  describe('Regular AMS (4-slot)', () => {
    it('renders 4 slots for regular AMS', () => {
      render(<AmsCard unit={mockAmsUnit} />)

      // Should show AMS A label
      expect(screen.getByText('AMS A')).toBeInTheDocument()

      // Should show material types for filled slots
      expect(screen.getByText('PLA')).toBeInTheDocument()
      expect(screen.getByText('PETG')).toBeInTheDocument()
      expect(screen.getByText('ABS')).toBeInTheDocument()

      // Empty slot should show dash
      expect(screen.getByText('-')).toBeInTheDocument()
    })

    it('renders AMS B, C, D labels correctly', () => {
      const amsB: AmsUnit = { ...mockAmsUnit, id: 1 }
      const amsC: AmsUnit = { ...mockAmsUnit, id: 2 }
      const amsD: AmsUnit = { ...mockAmsUnit, id: 3 }

      const { rerender } = render(<AmsCard unit={amsB} />)
      expect(screen.getByText('AMS B')).toBeInTheDocument()

      rerender(<AmsCard unit={amsC} />)
      expect(screen.getByText('AMS C')).toBeInTheDocument()

      rerender(<AmsCard unit={amsD} />)
      expect(screen.getByText('AMS D')).toBeInTheDocument()
    })

    it('displays humidity and temperature', () => {
      render(<AmsCard unit={mockAmsUnit} />)

      // Should show humidity
      expect(screen.getByText('35%')).toBeInTheDocument()

      // Should show temperature
      expect(screen.getByText('25.5°C')).toBeInTheDocument()
    })

    it('shows K values for slots with filament', () => {
      render(<AmsCard unit={mockAmsUnit} />)

      // Should show K values
      expect(screen.getByText('K 0.025')).toBeInTheDocument()
      expect(screen.getByText('K 0.035')).toBeInTheDocument()
      expect(screen.getByText('K 0.040')).toBeInTheDocument()
    })

    it('shows active tray indicator for single nozzle', () => {
      render(<AmsCard unit={mockAmsUnit} trayNow={0} />)

      // The first tray should have an active indicator
      // Check for the active tray dot
      const activeDot = document.querySelector('.bg-\\[var\\(--accent-color\\)\\]')
      expect(activeDot).toBeInTheDocument()
    })

    it('shows fill level bars', () => {
      render(<AmsCard unit={mockAmsUnit} />)

      // Should show fill level percentages
      expect(screen.getByText('80%')).toBeInTheDocument()
      expect(screen.getByText('50%')).toBeInTheDocument()
      expect(screen.getByText('20%')).toBeInTheDocument()
    })
  })

  describe('HT AMS (single-slot)', () => {
    it('renders single slot for HT AMS', () => {
      render(<AmsCard unit={mockHtAmsUnit} />)

      // Should show HT label
      expect(screen.getByText('HT A')).toBeInTheDocument()

      // Should show material type
      expect(screen.getByText('PLA-CF')).toBeInTheDocument()
    })

    it('renders correct HT AMS labels (A-H)', () => {
      const htB: AmsUnit = { ...mockHtAmsUnit, id: 129 }
      const htC: AmsUnit = { ...mockHtAmsUnit, id: 130 }
      const htH: AmsUnit = { ...mockHtAmsUnit, id: 135 }

      const { rerender } = render(<AmsCard unit={htB} />)
      expect(screen.getByText('HT B')).toBeInTheDocument()

      rerender(<AmsCard unit={htC} />)
      expect(screen.getByText('HT C')).toBeInTheDocument()

      rerender(<AmsCard unit={htH} />)
      expect(screen.getByText('HT H')).toBeInTheDocument()
    })

    it('shows K value and fill level', () => {
      render(<AmsCard unit={mockHtAmsUnit} />)

      expect(screen.getByText('K 0.028')).toBeInTheDocument()
      expect(screen.getByText('65%')).toBeInTheDocument()
    })
  })

  describe('Dual Nozzle Support', () => {
    it('shows nozzle labels for multi-nozzle printers', () => {
      render(
        <AmsCard
          unit={mockDualNozzleAmsUnits[0]}
          numExtruders={2}
        />
      )

      // Should show R (Right) label for extruder 0
      expect(screen.getByText('R')).toBeInTheDocument()
    })

    it('shows L label for left nozzle AMS', () => {
      render(
        <AmsCard
          unit={mockDualNozzleAmsUnits[1]}
          numExtruders={2}
        />
      )

      // Should show L (Left) label for extruder 1
      expect(screen.getByText('L')).toBeInTheDocument()
    })

    it('shows active tray for dual nozzle with tray_now_left/right', () => {
      render(
        <AmsCard
          unit={mockDualNozzleAmsUnits[0]}
          numExtruders={2}
          trayNowRight={0}
          activeExtruder={0}
        />
      )

      // The active tray indicator should be visible
      const activeDot = document.querySelector('.bg-\\[var\\(--accent-color\\)\\]')
      expect(activeDot).toBeInTheDocument()
    })

    it('does not show active indicator when different extruder is active', () => {
      render(
        <AmsCard
          unit={mockDualNozzleAmsUnits[0]}  // extruder 0 (right)
          numExtruders={2}
          trayNowRight={0}
          activeExtruder={1}  // Left extruder is active
        />
      )

      // Should not show active indicator on right nozzle AMS
      // when left nozzle is the active one
      const allDots = document.querySelectorAll('.w-1\\.5.h-1\\.5.rounded-full.bg-\\[var\\(--accent-color\\)\\]')
      expect(allDots.length).toBe(0)
    })
  })

  describe('Slot Menu Interactions', () => {
    it('renders slot menu button when printerSerial is provided', () => {
      render(
        <AmsCard
          unit={mockAmsUnit}
          printerSerial="00M09A123456789"
          calibrations={mockCalibrations}
        />
      )

      // Should have kebab menu buttons for slots
      const menuButtons = screen.getAllByTitle('Slot options')
      expect(menuButtons.length).toBeGreaterThan(0)
    })

    it('does not render slot menu without printerSerial', () => {
      render(<AmsCard unit={mockAmsUnit} />)

      expect(screen.queryByTitle('Slot options')).not.toBeInTheDocument()
    })

    it('opens configure modal when slot menu is clicked', async () => {
      render(
        <AmsCard
          unit={mockAmsUnit}
          printerSerial="00M09A123456789"
          calibrations={mockCalibrations}
        />
      )

      const menuButton = screen.getAllByTitle('Slot options')[0]
      fireEvent.click(menuButton)

      // Modal should open - check for modal title
      await waitFor(() => {
        expect(screen.getByText('Configure AMS Slot')).toBeInTheDocument()
      })
    })

    it('shows reading spinner when tray is being read', () => {
      // tray_reading_bits = 1 means tray 0 of AMS 0 is being read
      render(
        <AmsCard
          unit={mockAmsUnit}
          printerSerial="00M09A123456789"
          calibrations={mockCalibrations}
          trayReadingBits={1}
        />
      )

      // Should show spinner instead of menu button
      expect(screen.getByTitle('Reading RFID...')).toBeInTheDocument()
    })
  })

  describe('Empty Slots', () => {
    it('shows dashed circle for empty slot icon', () => {
      const emptyUnit: AmsUnit = {
        id: 0,
        humidity: 35,
        temperature: 25.5,
        extruder: null,
        trays: [{
          ams_id: 0,
          tray_id: 0,
          tray_type: null,
          tray_color: null,
          tray_info_idx: null,
          k_value: null,
          nozzle_temp_min: null,
          nozzle_temp_max: null,
          remain: null,
        }],
      }

      render(<AmsCard unit={emptyUnit} />)

      // Empty slots show dash for material (may have multiple dashes for empty K/fill)
      const dashes = screen.getAllByText('-')
      expect(dashes.length).toBeGreaterThanOrEqual(1)
    })
  })

  describe('AMS Thresholds', () => {
    it('applies color based on humidity thresholds', () => {
      const highHumidityUnit: AmsUnit = {
        ...mockAmsUnit,
        humidity: 75,
      }

      render(
        <AmsCard
          unit={highHumidityUnit}
          amsThresholds={{
            humidity_good: 40,
            humidity_fair: 60,
            temp_good: 28,
            temp_fair: 35,
            history_retention_days: 7,
          }}
        />
      )

      // High humidity should be colored red (#ef4444)
      const humidityText = screen.getByText('75%')
      expect(humidityText).toHaveStyle({ color: '#ef4444' })
    })

    it('history click handler is called when sensor clicked', () => {
      const onHistoryClick = vi.fn()

      render(
        <AmsCard
          unit={mockAmsUnit}
          onHistoryClick={onHistoryClick}
        />
      )

      // Click on humidity indicator
      const humidityButton = screen.getByTitle(/Humidity.*click for history/)
      fireEvent.click(humidityButton)

      expect(onHistoryClick).toHaveBeenCalledWith(0, 'AMS A', 'humidity')
    })
  })
})

describe('ExternalSpool', () => {
  const mockExternalTray = {
    ams_id: 255,
    tray_id: 0,
    tray_type: 'PLA',
    tray_color: 'FFFFFFFF',
    tray_info_idx: 'GFSL05',
    k_value: 0.020,
    nozzle_temp_min: 190,
    nozzle_temp_max: 230,
    remain: 50,
  }

  it('renders external spool for single nozzle', () => {
    render(<ExternalSpool tray={mockExternalTray} numExtruders={1} />)

    expect(screen.getByText('Ext')).toBeInTheDocument()
    expect(screen.getByText('PLA')).toBeInTheDocument()
  })

  it('renders Ext-L and Ext-R for dual nozzle', () => {
    const { rerender } = render(
      <ExternalSpool tray={mockExternalTray} numExtruders={2} position="left" />
    )
    expect(screen.getByText('Ext-L')).toBeInTheDocument()
    expect(screen.getByText('L')).toBeInTheDocument()

    rerender(
      <ExternalSpool tray={mockExternalTray} numExtruders={2} position="right" />
    )
    expect(screen.getByText('Ext-R')).toBeInTheDocument()
    expect(screen.getByText('R')).toBeInTheDocument()
  })

  it('shows empty state when tray is null', () => {
    render(<ExternalSpool tray={null} numExtruders={1} />)

    expect(screen.getByText('Empty')).toBeInTheDocument()
  })

  it('shows slot menu when printerSerial provided', () => {
    render(
      <ExternalSpool
        tray={mockExternalTray}
        numExtruders={1}
        printerSerial="00M09A123456789"
        calibrations={mockCalibrations}
      />
    )

    expect(screen.getByTitle('Slot options')).toBeInTheDocument()
  })
})

describe('SpoolIcon', () => {
  it('renders filled spool with color', () => {
    const { container } = render(<SpoolIcon color="#FF0000" isEmpty={false} />)

    const svg = container.querySelector('svg')
    expect(svg).toBeInTheDocument()

    const circle = svg?.querySelector('circle')
    expect(circle).toHaveAttribute('fill', '#FF0000')
  })

  it('renders empty spool with dashed border', () => {
    const { container } = render(<SpoolIcon color="#808080" isEmpty={true} />)

    // Empty spool uses div with dashed border, not SVG
    const emptyDiv = container.querySelector('.border-dashed')
    expect(emptyDiv).toBeInTheDocument()
  })

  it('respects size prop', () => {
    const { container } = render(<SpoolIcon color="#FF0000" isEmpty={false} size={48} />)

    const svg = container.querySelector('svg')
    expect(svg).toHaveAttribute('width', '48')
    expect(svg).toHaveAttribute('height', '48')
  })
})
