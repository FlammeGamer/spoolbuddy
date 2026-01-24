import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, act } from '@testing-library/preact'
import { useWebSocket, WebSocketProvider } from '../../lib/websocket'

// Capture the last created WebSocket instance
let mockWebSocketInstance: MockWebSocket | null = null

class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  readyState = MockWebSocket.OPEN
  url: string
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
  onmessage: ((event: { data: string }) => void) | null = null
  onerror: ((event: Event) => void) | null = null

  constructor(url: string) {
    this.url = url
    mockWebSocketInstance = this
    // Simulate connection after a tick
    setTimeout(() => {
      if (this.onopen) this.onopen()
    }, 0)
  }

  send = vi.fn()
  close = vi.fn(() => {
    this.readyState = MockWebSocket.CLOSED
    if (this.onclose) this.onclose()
  })

  // Helper to simulate receiving a message
  simulateMessage(data: object) {
    if (this.onmessage) {
      this.onmessage({ data: JSON.stringify(data) })
    }
  }

  // Helper to simulate connection close
  simulateClose() {
    this.readyState = MockWebSocket.CLOSED
    if (this.onclose) this.onclose()
  }
}

// Component that exposes hook state for testing
function TestConsumer({ onState }: { onState: (state: ReturnType<typeof useWebSocket>) => void }) {
  const state = useWebSocket()
  onState(state)
  return (
    <div>
      <span data-testid="deviceConnected">{String(state.deviceConnected)}</span>
      <span data-testid="currentWeight">{state.currentWeight ?? 'null'}</span>
      <span data-testid="currentTagId">{state.currentTagId ?? 'null'}</span>
      <span data-testid="weightStable">{String(state.weightStable)}</span>
    </div>
  )
}

describe('useWebSocket', () => {
  let originalWebSocket: typeof WebSocket

  beforeEach(() => {
    vi.useFakeTimers()
    originalWebSocket = window.WebSocket as typeof WebSocket
    // @ts-ignore - mock WebSocket
    window.WebSocket = MockWebSocket
    mockWebSocketInstance = null
  })

  afterEach(() => {
    vi.useRealTimers()
    window.WebSocket = originalWebSocket
    mockWebSocketInstance = null
  })

  describe('Connection', () => {
    it('connects on mount', async () => {
      render(
        <WebSocketProvider>
          <TestConsumer onState={() => {}} />
        </WebSocketProvider>
      )

      // Run timers to trigger connection
      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      expect(mockWebSocketInstance).not.toBeNull()
      expect(mockWebSocketInstance?.url).toContain('/ws/ui')
    })

    it('creates WebSocket with correct URL', async () => {
      render(
        <WebSocketProvider>
          <TestConsumer onState={() => {}} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // Should use ws:// for http:// and host from window.location
      expect(mockWebSocketInstance?.url).toMatch(/^wss?:\/\//)
      expect(mockWebSocketInstance?.url).toContain('/ws/ui')
    })

    it('reconnects after disconnect with 3s delay', async () => {
      render(
        <WebSocketProvider>
          <TestConsumer onState={() => {}} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      const firstInstance = mockWebSocketInstance

      // Simulate disconnect
      await act(async () => {
        mockWebSocketInstance?.simulateClose()
      })

      // Not reconnected yet
      expect(mockWebSocketInstance).toBe(firstInstance)

      // Advance time by 3 seconds
      await act(async () => {
        vi.advanceTimersByTime(3000)
      })

      // Should have new instance
      expect(mockWebSocketInstance).not.toBe(firstInstance)
    })

    it('cleans up on unmount', async () => {
      const { unmount } = render(
        <WebSocketProvider>
          <TestConsumer onState={() => {}} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      const ws = mockWebSocketInstance

      unmount()

      expect(ws?.close).toHaveBeenCalled()
    })
  })

  describe('Message Handling', () => {
    it('handles initial_state message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'initial_state',
          device: {
            connected: true,
            last_weight: 850.5,
            weight_stable: true,
            current_tag_id: 'ABC123==',
            update_available: false,
          },
          printers: {
            '00M09A123456789': true,
            '00M09A987654321': false,
          },
        })
      })

      expect(capturedState?.deviceConnected).toBe(true)
      expect(capturedState?.currentWeight).toBe(850.5)
      expect(capturedState?.weightStable).toBe(true)
      // currentTagId is intentionally not set from initial_state to avoid stale data
      expect(capturedState?.currentTagId).toBeNull()
      expect(capturedState?.printerStatuses.get('00M09A123456789')).toBe(true)
      expect(capturedState?.printerStatuses.get('00M09A987654321')).toBe(false)
    })

    it('handles device_connected message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'device_connected',
        })
      })

      expect(capturedState?.deviceConnected).toBe(true)
    })

    it('handles device_disconnected message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // First connect
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({ type: 'device_connected' })
      })

      expect(capturedState?.deviceConnected).toBe(true)

      // Then disconnect
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({ type: 'device_disconnected' })
      })

      expect(capturedState?.deviceConnected).toBe(false)
      expect(capturedState?.currentWeight).toBeNull()
      expect(capturedState?.currentTagId).toBeNull()
    })

    it('handles weight message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'weight',
          grams: 1234.5,
          stable: true,
        })
      })

      expect(capturedState?.currentWeight).toBe(1234.5)
      expect(capturedState?.weightStable).toBe(true)
    })

    it('handles device_state message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'device_state',
          weight: 999.9,
          stable: false,
          tag_id: 'XYZ789==',
        })
      })

      expect(capturedState?.currentWeight).toBe(999.9)
      expect(capturedState?.weightStable).toBe(false)
      expect(capturedState?.currentTagId).toBe('XYZ789==')
    })

    it('handles tag_detected message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'tag_detected',
          tag_id: 'NEW_TAG==',
        })
      })

      expect(capturedState?.currentTagId).toBe('NEW_TAG==')
    })

    it('handles tag_removed message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // First set a tag
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'tag_detected',
          tag_id: 'SOME_TAG==',
        })
      })

      expect(capturedState?.currentTagId).toBe('SOME_TAG==')

      // Then remove it
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'tag_removed',
        })
      })

      expect(capturedState?.currentTagId).toBeNull()
    })

    it('handles printer_connected message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'printer_connected',
          serial: '00M09A123456789',
        })
      })

      expect(capturedState?.printerStatuses.get('00M09A123456789')).toBe(true)
    })

    it('handles printer_disconnected message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // First connect
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'printer_connected',
          serial: '00M09A123456789',
        })
      })

      // Then disconnect
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'printer_disconnected',
          serial: '00M09A123456789',
        })
      })

      expect(capturedState?.printerStatuses.get('00M09A123456789')).toBe(false)
    })

    it('handles printer_state message', async () => {
      let capturedState: ReturnType<typeof useWebSocket> | null = null

      render(
        <WebSocketProvider>
          <TestConsumer onState={(s) => { capturedState = s }} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      const mockState = {
        gcode_state: 'RUNNING',
        print_progress: 45,
        layer_num: 50,
        total_layer_num: 200,
        subtask_name: 'test.gcode',
        mc_remaining_time: 30,
        gcode_file: '/sdcard/test.gcode',
        ams_units: [],
        vt_tray: null,
        tray_now: 0,
        tray_now_left: null,
        tray_now_right: null,
        active_extruder: null,
        tray_reading_bits: null,
        nozzle_count: 1,
      }

      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'printer_state',
          serial: '00M09A123456789',
          state: mockState,
        })
      })

      expect(capturedState?.printerStates.get('00M09A123456789')).toEqual(mockState)
    })

    it('handles malformed JSON gracefully', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      render(
        <WebSocketProvider>
          <TestConsumer onState={() => {}} />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // Send invalid JSON
      await act(async () => {
        if (mockWebSocketInstance?.onmessage) {
          mockWebSocketInstance.onmessage({ data: 'not valid json{' })
        }
      })

      // Should log error but not crash
      expect(consoleSpy).toHaveBeenCalledWith(
        'Failed to parse WebSocket message:',
        expect.any(Error)
      )

      consoleSpy.mockRestore()
    })
  })

  describe('Subscribe', () => {
    it('subscribe() registers custom message handler', async () => {
      const customHandler = vi.fn()

      function TestSubscriber() {
        const state = useWebSocket()

        // Subscribe to messages
        state.subscribe(customHandler)

        return null
      }

      render(
        <WebSocketProvider>
          <TestSubscriber />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // Send a custom message
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({
          type: 'custom_event',
          data: 'test',
        })
      })

      expect(customHandler).toHaveBeenCalledWith({
        type: 'custom_event',
        data: 'test',
      })
    })

    it('unsubscribe removes handler', async () => {
      const customHandler = vi.fn()
      let unsubscribe: (() => void) | null = null

      function TestSubscriber() {
        const state = useWebSocket()
        unsubscribe = state.subscribe(customHandler)
        return null
      }

      render(
        <WebSocketProvider>
          <TestSubscriber />
        </WebSocketProvider>
      )

      await act(async () => {
        vi.advanceTimersByTime(10)
      })

      // First message should be received
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({ type: 'test1' })
      })

      expect(customHandler).toHaveBeenCalledTimes(1)

      // Unsubscribe
      unsubscribe?.()

      // Second message should not be received
      await act(async () => {
        mockWebSocketInstance?.simulateMessage({ type: 'test2' })
      })

      expect(customHandler).toHaveBeenCalledTimes(1)
    })
  })

  describe('Context Errors', () => {
    it('throws error when used outside provider', () => {
      // Suppress console.error for this test
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      expect(() => {
        render(<TestConsumer onState={() => {}} />)
      }).toThrow('useWebSocket must be used within a WebSocketProvider')

      consoleSpy.mockRestore()
    })
  })
})
