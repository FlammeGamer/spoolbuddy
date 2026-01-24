import { http, HttpResponse } from 'msw'
import {
  mockCalibrations,
  mockSlicerPresets,
  mockSettingDetail,
  mockCloudStatus,
  mockVersionInfo,
  mockUpdateCheck,
} from './data'

// Re-export mock data for backward compatibility
export const mockSpools = [
  {
    id: 'spool-1',
    tag_id: null,
    material: 'PLA',
    subtype: 'Basic',
    color_name: 'Black',
    rgba: '#000000FF',
    brand: 'Bambu Lab',
    label_weight: 1000,
    core_weight: 250,
    weight_new: 1000,
    weight_current: 850,
    slicer_filament: 'GFPL01',
    note: null,
    added_time: null,
    encode_time: null,
    added_full: false,
    consumed_since_add: 150,
    consumed_since_weight: 50,
    weight_used: 0,
    data_origin: null,
    tag_type: null,
    ext_has_k: false,
    created_at: 1702000000,
    updated_at: 1702000000,
  },
  {
    id: 'spool-2',
    tag_id: null,
    material: 'PETG',
    subtype: 'Basic',
    color_name: 'Red',
    rgba: '#FF0000FF',
    brand: 'Generic',
    label_weight: 1000,
    core_weight: 250,
    weight_new: 1000,
    weight_current: 1000,
    slicer_filament: null,
    note: 'Test spool',
    added_time: null,
    encode_time: null,
    added_full: true,
    consumed_since_add: 0,
    consumed_since_weight: 0,
    weight_used: 0,
    data_origin: null,
    tag_type: null,
    ext_has_k: false,
    created_at: 1702100000,
    updated_at: 1702100000,
  },
]

export const mockPrinters = [
  {
    serial: '00M09A123456789',
    name: 'X1 Carbon',
    model: 'X1C',
    ip_address: '192.168.1.100',
    access_code: '12345678',
    last_seen: 1702000000,
    config: null,
    auto_connect: true,
    connected: true,
  },
]

// Re-export from data.ts for backward compatibility
export { mockVersionInfo, mockUpdateCheck, mockCloudStatus, mockCalibrations }

// API handlers
export const handlers = [
  // Spools
  http.get('/api/spools', () => {
    return HttpResponse.json(mockSpools)
  }),

  http.get('/api/spools/:id', ({ params }) => {
    const spool = mockSpools.find(s => s.id === params.id)
    if (spool) {
      return HttpResponse.json(spool)
    }
    return new HttpResponse(null, { status: 404 })
  }),

  http.post('/api/spools', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    const newSpool = {
      id: `spool-${Date.now()}`,
      tag_id: null,
      subtype: null,
      color_name: null,
      rgba: null,
      brand: null,
      label_weight: 1000,
      core_weight: 250,
      weight_new: null,
      weight_current: null,
      slicer_filament: null,
      note: null,
      added_time: null,
      encode_time: null,
      added_full: false,
      consumed_since_add: 0,
      consumed_since_weight: 0,
      weight_used: 0,
      data_origin: null,
      tag_type: null,
      ext_has_k: false,
      created_at: Math.floor(Date.now() / 1000),
      updated_at: Math.floor(Date.now() / 1000),
      ...body,
    }
    return HttpResponse.json(newSpool)
  }),

  http.put('/api/spools/:id', async ({ params, request }) => {
    const body = await request.json() as Record<string, unknown>
    const spool = mockSpools.find(s => s.id === params.id)
    if (spool) {
      const updated = { ...spool, ...body, updated_at: Math.floor(Date.now() / 1000) }
      return HttpResponse.json(updated)
    }
    return new HttpResponse(null, { status: 404 })
  }),

  http.delete('/api/spools/:id', ({ params }) => {
    const index = mockSpools.findIndex(s => s.id === params.id)
    if (index !== -1) {
      return HttpResponse.json({ success: true })
    }
    return new HttpResponse(null, { status: 404 })
  }),

  // Printers
  http.get('/api/printers', () => {
    return HttpResponse.json(mockPrinters)
  }),

  http.get('/api/printers/:serial', ({ params }) => {
    const printer = mockPrinters.find(p => p.serial === params.serial)
    if (printer) {
      return HttpResponse.json(printer)
    }
    return new HttpResponse(null, { status: 404 })
  }),

  http.get('/api/printers/:serial/calibrations', () => {
    return HttpResponse.json(mockCalibrations)
  }),

  // Updates
  http.get('/api/updates/version', () => {
    return HttpResponse.json(mockVersionInfo)
  }),

  http.get('/api/updates/check', () => {
    return HttpResponse.json(mockUpdateCheck)
  }),

  http.get('/api/updates/status', () => {
    return HttpResponse.json({ status: 'idle', message: null, progress: null, error: null })
  }),

  http.post('/api/updates/apply', () => {
    return HttpResponse.json({ status: 'checking', message: 'Starting update...', progress: null, error: null })
  }),

  http.post('/api/updates/reset-status', () => {
    return HttpResponse.json({ status: 'idle', message: null, progress: null, error: null })
  }),

  // Cloud
  http.get('/api/cloud/status', () => {
    return HttpResponse.json(mockCloudStatus)
  }),

  http.post('/api/cloud/login', () => {
    return HttpResponse.json({
      success: false,
      needs_verification: true,
      message: 'Verification code sent',
    })
  }),

  http.post('/api/cloud/verify', () => {
    return HttpResponse.json({
      success: true,
      needs_verification: false,
      message: 'Login successful',
    })
  }),

  http.post('/api/cloud/logout', () => {
    return HttpResponse.json({ success: true })
  }),

  http.get('/api/cloud/settings', () => {
    return HttpResponse.json({
      filament: mockSlicerPresets,
      printer: [],
      process: [],
    })
  }),

  http.get('/api/cloud/settings/:settingId', ({ params }) => {
    const settingId = params.settingId as string
    if (settingId === mockSettingDetail.setting_id) {
      return HttpResponse.json(mockSettingDetail)
    }
    // For official presets, return minimal detail
    return HttpResponse.json({
      setting_id: settingId,
      name: settingId,
    })
  }),

  // AMS slot operations
  http.post('/api/printers/:serial/ams/:amsId/tray/:trayId/filament', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    // Simulate setting filament on a slot
    return HttpResponse.json({ status: 'ok', ...body })
  }),

  http.post('/api/printers/:serial/ams/:amsId/tray/:trayId/calibration', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    // Simulate setting calibration on a slot
    return HttpResponse.json({ status: 'ok', ...body })
  }),

  http.post('/api/printers/:serial/ams/:amsId/tray/:trayId/reset', () => {
    // Simulate RFID re-read request
    return HttpResponse.json({ status: 'ok' })
  }),

  http.post('/api/printers/:serial/ams/:amsId/tray/:trayId/assign', async () => {
    return HttpResponse.json({
      status: 'configured',
      message: 'Spool assigned',
      needs_replacement: false,
    })
  }),

  // Device
  http.get('/api/device/status', () => {
    return HttpResponse.json({
      connected: false,
      last_weight: null,
      weight_stable: false,
      current_tag_id: null,
    })
  }),

  http.post('/api/device/tare', () => {
    return HttpResponse.json({ success: true })
  }),

  // AMS thresholds
  http.get('/api/settings/ams/thresholds', () => {
    return HttpResponse.json({
      humidity_good: 40,
      humidity_fair: 60,
      temp_good: 28,
      temp_fair: 35,
      history_retention_days: 30,
    })
  }),

  http.put('/api/settings/ams/thresholds', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json(body)
  }),

  // API Keys
  http.get('/api/keys', () => {
    return HttpResponse.json([
      {
        id: 1,
        name: 'Test Key',
        prefix: 'sk_test_',
        can_read: true,
        can_write: false,
        can_control: false,
        created_at: 1702000000,
        last_used_at: null,
      },
    ])
  }),

  http.post('/api/keys', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({
      id: 2,
      name: body.name || 'New Key',
      prefix: 'sk_new_',
      key: 'sk_new_test123456',
      can_read: body.can_read ?? true,
      can_write: body.can_write ?? false,
      can_control: body.can_control ?? false,
      created_at: Math.floor(Date.now() / 1000),
      last_used_at: null,
    })
  }),

  http.delete('/api/keys/:id', () => {
    return HttpResponse.json({ success: true })
  }),

  // System info
  http.get('/api/system/info', () => {
    return HttpResponse.json({
      version: '0.1.0',
      uptime: '1 hour',
      uptime_seconds: 3600,
      hostname: 'spoolbuddy-test',
      spool_count: 10,
      printer_count: 2,
      connected_printers: 1,
      database_size: '1.2 MB',
      disk_total: '50 GB',
      disk_used: '10 GB',
      disk_free: '40 GB',
      disk_percent: 20,
      platform: 'Linux',
      platform_release: '5.10.0',
      python_version: '3.11.0',
      boot_time: '2024-01-15 10:00:00',
      memory_total: '2 GB',
      memory_used: '512 MB',
      memory_available: '1.5 GB',
      memory_percent: 25,
      cpu_count: 4,
      cpu_percent: 15.5,
    })
  }),

  // Debug logging
  http.get('/api/debug/logging', () => {
    return HttpResponse.json({
      enabled: false,
      auto_disable_at: null,
    })
  }),

  http.post('/api/debug/logging', async ({ request }) => {
    const body = await request.json() as { enabled: boolean }
    return HttpResponse.json({
      enabled: body.enabled,
      auto_disable_at: body.enabled ? Math.floor(Date.now() / 1000) + 3600 : null,
    })
  }),

  // Logs
  http.get('/api/logs', () => {
    return HttpResponse.json({
      logs: [],
      total: 0,
      filtered: 0,
    })
  }),

  // Display status (for firmware version)
  http.get('/api/display/status', () => {
    return HttpResponse.json({
      connected: false,
      firmware_version: null,
    })
  }),

  // Firmware
  http.get('/api/firmware/check', () => {
    return HttpResponse.json({
      current_version: '1.0.0',
      latest_version: '1.0.0',
      update_available: false,
      error: null,
    })
  }),

  // ESP32 Reboot
  http.post('/api/device/reboot', () => {
    return HttpResponse.json({ success: true })
  }),

  // Scale calibration
  http.post('/api/device/scale/calibrate', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({ success: true, ...body })
  }),

  http.post('/api/device/scale/reset', () => {
    return HttpResponse.json({ success: true })
  }),

  // OTA trigger
  http.post('/api/firmware/trigger', () => {
    return HttpResponse.json({ success: true })
  }),

  // Support - debug logging
  http.get('/api/support/debug-logging', () => {
    return HttpResponse.json({
      enabled: false,
      enabled_at: null,
      duration_seconds: null,
    })
  }),

  http.post('/api/support/debug-logging', async ({ request }) => {
    const body = await request.json() as { enabled: boolean }
    return HttpResponse.json({
      enabled: body.enabled,
      enabled_at: body.enabled ? new Date().toISOString() : null,
      duration_seconds: body.enabled ? 300 : null,
    })
  }),

  // Cloud filaments
  http.get('/api/cloud/filaments', () => {
    return HttpResponse.json(mockSlicerPresets)
  }),

  // Color catalog
  http.get('/api/colors', () => {
    return HttpResponse.json([
      { id: 1, manufacturer: 'Bambu Lab', color_name: 'Black', hex_color: '#000000', material: 'PLA', is_default: true },
      { id: 2, manufacturer: 'Bambu Lab', color_name: 'White', hex_color: '#FFFFFF', material: 'PLA', is_default: true },
      { id: 3, manufacturer: 'Generic', color_name: 'Red', hex_color: '#FF0000', material: null, is_default: false },
    ])
  }),

  http.get('/api/colors/lookup', ({ request }) => {
    const url = new URL(request.url)
    const manufacturer = url.searchParams.get('manufacturer')
    const colorName = url.searchParams.get('color_name')
    // Return a simple lookup result
    if (manufacturer === 'Bambu Lab' && colorName === 'Black') {
      return HttpResponse.json({ found: true, hex_color: '#000000', material: 'PLA' })
    }
    return HttpResponse.json({ found: false, hex_color: null })
  }),

  http.get('/api/colors/search', () => {
    return HttpResponse.json([
      { id: 1, manufacturer: 'Bambu Lab', color_name: 'Black', hex_color: '#000000', material: 'PLA', is_default: true },
    ])
  }),

  // Spool catalog
  http.get('/api/catalog', () => {
    return HttpResponse.json([
      { id: 1, name: 'Standard 1kg', weight: 250, is_default: true, created_at: null },
      { id: 2, name: 'Mini 500g', weight: 200, is_default: false, created_at: null },
    ])
  }),

  http.post('/api/catalog', async ({ request }) => {
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({
      id: Date.now(),
      is_default: false,
      created_at: Math.floor(Date.now() / 1000),
      ...body,
    })
  }),

  http.put('/api/catalog/:id', async ({ params, request }) => {
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({
      id: Number(params.id),
      is_default: false,
      created_at: null,
      ...body,
    })
  }),

  http.delete('/api/catalog/:id', () => {
    return HttpResponse.json({ success: true })
  }),

  // Spool K-profiles
  http.get('/api/spools/:id/k-profiles', () => {
    return HttpResponse.json([])
  }),

  http.put('/api/spools/:id/k-profiles', async ({ request }) => {
    const body = await request.json() as { profiles: unknown[] }
    return HttpResponse.json({ status: 'ok', count: body.profiles?.length || 0 })
  }),

  // Spool archive/restore
  http.post('/api/spools/:id/archive', ({ params }) => {
    return HttpResponse.json({
      id: params.id,
      archived_at: Math.floor(Date.now() / 1000),
    })
  }),

  http.post('/api/spools/:id/restore', ({ params }) => {
    return HttpResponse.json({
      id: params.id,
      archived_at: null,
    })
  }),
]
