import { describe, it, expect } from 'vitest'
import { api } from '../../lib/api'
import { server } from '../setup'
import { http, HttpResponse } from 'msw'

describe('API Client', () => {
  describe('Spools', () => {
    describe('listSpools', () => {
      it('should list spools', async () => {
        const spools = await api.listSpools()
        expect(spools).toBeInstanceOf(Array)
        expect(spools.length).toBe(2)
        expect(spools[0].material).toBe('PLA')
      })

      it('should filter spools by material', async () => {
        server.use(
          http.get('/api/spools', ({ request }) => {
            const url = new URL(request.url)
            const material = url.searchParams.get('material')
            if (material === 'PLA') {
              return HttpResponse.json([
                { id: 'spool-1', material: 'PLA', color_name: 'Black' },
              ])
            }
            return HttpResponse.json([])
          })
        )

        const spools = await api.listSpools({ material: 'PLA' })
        expect(spools.length).toBe(1)
        expect(spools[0].material).toBe('PLA')
      })

      it('should filter spools by brand', async () => {
        server.use(
          http.get('/api/spools', ({ request }) => {
            const url = new URL(request.url)
            const brand = url.searchParams.get('brand')
            if (brand === 'Bambu Lab') {
              return HttpResponse.json([
                { id: 'spool-1', material: 'PLA', brand: 'Bambu Lab' },
              ])
            }
            return HttpResponse.json([])
          })
        )

        const spools = await api.listSpools({ brand: 'Bambu Lab' })
        expect(spools.length).toBe(1)
        expect(spools[0].brand).toBe('Bambu Lab')
      })

      it('should search spools', async () => {
        server.use(
          http.get('/api/spools', ({ request }) => {
            const url = new URL(request.url)
            const search = url.searchParams.get('search')
            if (search === 'black') {
              return HttpResponse.json([
                { id: 'spool-1', material: 'PLA', color_name: 'Black' },
              ])
            }
            return HttpResponse.json([])
          })
        )

        const spools = await api.listSpools({ search: 'black' })
        expect(spools.length).toBe(1)
      })
    })

    describe('getSpool', () => {
      it('should get a single spool', async () => {
        const spool = await api.getSpool('spool-1')
        expect(spool.id).toBe('spool-1')
        expect(spool.material).toBe('PLA')
      })

      it('should throw error for non-existent spool', async () => {
        await expect(api.getSpool('non-existent')).rejects.toThrow()
      })
    })

    describe('createSpool', () => {
      it('should create a spool', async () => {
        const newSpool = await api.createSpool({
          material: 'ABS',
          color_name: 'Blue',
          brand: 'Test Brand',
        })
        expect(newSpool.material).toBe('ABS')
        expect(newSpool.color_name).toBe('Blue')
        expect(newSpool.id).toBeDefined()
      })

      it('should create spool with minimal data', async () => {
        const newSpool = await api.createSpool({
          material: 'TPU',
        })
        expect(newSpool.material).toBe('TPU')
        expect(newSpool.id).toBeDefined()
      })
    })

    describe('updateSpool', () => {
      it('should update a spool', async () => {
        const updatedSpool = await api.updateSpool('spool-1', {
          material: 'PLA',
          color_name: 'White',
        })
        expect(updatedSpool.color_name).toBe('White')
      })

      it('should throw error for non-existent spool', async () => {
        await expect(api.updateSpool('non-existent', { material: 'PLA' })).rejects.toThrow()
      })
    })

    describe('deleteSpool', () => {
      it('should delete a spool', async () => {
        await expect(api.deleteSpool('spool-1')).resolves.not.toThrow()
      })

      it('should throw error for non-existent spool', async () => {
        await expect(api.deleteSpool('non-existent')).rejects.toThrow()
      })
    })

    describe('archiveSpool', () => {
      it('should archive a spool', async () => {
        server.use(
          http.post('/api/spools/:id/archive', () => {
            return HttpResponse.json({
              id: 'spool-1',
              material: 'PLA',
              archived_at: Date.now(),
            })
          })
        )

        const spool = await api.archiveSpool('spool-1')
        expect(spool.archived_at).toBeDefined()
      })
    })

    describe('restoreSpool', () => {
      it('should restore a spool', async () => {
        server.use(
          http.post('/api/spools/:id/restore', () => {
            return HttpResponse.json({
              id: 'spool-1',
              material: 'PLA',
              archived_at: null,
            })
          })
        )

        const spool = await api.restoreSpool('spool-1')
        expect(spool.archived_at).toBeNull()
      })
    })

    describe('setSpoolWeight', () => {
      it('should set spool weight', async () => {
        server.use(
          http.post('/api/spools/:id/weight', async ({ request }) => {
            const body = await request.json() as { weight: number }
            return HttpResponse.json({
              id: 'spool-1',
              material: 'PLA',
              weight_current: body.weight,
              consumed_since_weight: 0,
            })
          })
        )

        const spool = await api.setSpoolWeight('spool-1', 850)
        expect(spool.weight_current).toBe(850)
        expect(spool.consumed_since_weight).toBe(0)
      })
    })

    describe('getSpoolKProfiles', () => {
      it('should get K-profiles for a spool', async () => {
        server.use(
          http.get('/api/spools/:id/k-profiles', () => {
            return HttpResponse.json([
              { printer_serial: '00M09A123456789', k_value: '0.025', nozzle_diameter: '0.4' },
            ])
          })
        )

        const profiles = await api.getSpoolKProfiles('spool-1')
        expect(profiles).toBeInstanceOf(Array)
        expect(profiles[0].k_value).toBe('0.025')
      })
    })

    describe('saveSpoolKProfiles', () => {
      it('should save K-profiles for a spool', async () => {
        server.use(
          http.put('/api/spools/:id/k-profiles', () => {
            return HttpResponse.json({ status: 'ok', count: 1 })
          })
        )

        const result = await api.saveSpoolKProfiles('spool-1', [
          { printer_serial: '00M09A123456789', k_value: '0.025', extruder: 0, nozzle_diameter: '0.4', nozzle_type: 'hardened_steel', name: 'Test', cali_idx: 42, setting_id: 'GFSL05_07' },
        ])
        expect(result.status).toBe('ok')
        expect(result.count).toBe(1)
      })
    })
  })

  describe('Printers', () => {
    describe('listPrinters', () => {
      it('should list printers', async () => {
        const printers = await api.listPrinters()
        expect(printers).toBeInstanceOf(Array)
        expect(printers.length).toBe(1)
        expect(printers[0].name).toBe('X1 Carbon')
      })
    })

    describe('getPrinter', () => {
      it('should get a single printer', async () => {
        const printer = await api.getPrinter('00M09A123456789')
        expect(printer.serial).toBe('00M09A123456789')
        expect(printer.model).toBe('X1C')
      })

      it('should throw error for non-existent printer', async () => {
        await expect(api.getPrinter('non-existent')).rejects.toThrow()
      })
    })

    describe('createPrinter', () => {
      it('should create a printer', async () => {
        server.use(
          http.post('/api/printers', async ({ request }) => {
            const body = await request.json() as { serial: string; name: string }
            return HttpResponse.json({
              serial: body.serial,
              name: body.name,
              model: 'P1S',
              ip_address: null,
              access_code: null,
              last_seen: null,
              config: null,
              auto_connect: false,
              connected: false,
            })
          })
        )

        const printer = await api.createPrinter({
          serial: 'NEW123',
          name: 'New Printer',
        })
        expect(printer.serial).toBe('NEW123')
        expect(printer.name).toBe('New Printer')
      })
    })

    describe('updatePrinter', () => {
      it('should update a printer', async () => {
        server.use(
          http.put('/api/printers/:serial', async ({ params, request }) => {
            const body = await request.json() as { name: string }
            return HttpResponse.json({
              serial: params.serial,
              name: body.name,
              model: 'X1C',
              ip_address: '192.168.1.100',
              access_code: '12345678',
              last_seen: Date.now(),
              config: null,
              auto_connect: true,
              connected: true,
            })
          })
        )

        const printer = await api.updatePrinter('00M09A123456789', { name: 'Updated Name' })
        expect(printer.name).toBe('Updated Name')
      })
    })

    describe('deletePrinter', () => {
      it('should delete a printer', async () => {
        server.use(
          http.delete('/api/printers/:serial', () => {
            return HttpResponse.json({ success: true })
          })
        )

        await expect(api.deletePrinter('00M09A123456789')).resolves.not.toThrow()
      })
    })

    describe('connectPrinter', () => {
      it('should connect to a printer', async () => {
        server.use(
          http.post('/api/printers/:serial/connect', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.connectPrinter('00M09A123456789')).resolves.not.toThrow()
      })
    })

    describe('disconnectPrinter', () => {
      it('should disconnect from a printer', async () => {
        server.use(
          http.post('/api/printers/:serial/disconnect', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.disconnectPrinter('00M09A123456789')).resolves.not.toThrow()
      })
    })

    describe('setAutoConnect', () => {
      it('should set auto-connect for a printer', async () => {
        server.use(
          http.post('/api/printers/:serial/auto-connect', async ({ request }) => {
            const body = await request.json() as { auto_connect: boolean }
            return HttpResponse.json({
              serial: '00M09A123456789',
              name: 'X1 Carbon',
              auto_connect: body.auto_connect,
            })
          })
        )

        const printer = await api.setAutoConnect('00M09A123456789', true)
        expect(printer.auto_connect).toBe(true)
      })
    })

    describe('getCalibrations', () => {
      it('should get calibrations for a printer', async () => {
        const calibrations = await api.getCalibrations('00M09A123456789')
        expect(calibrations).toBeInstanceOf(Array)
      })
    })

    describe('AMS Operations', () => {
      it('should set slot filament', async () => {
        await expect(api.setSlotFilament('00M09A123456789', {
          ams_id: 0,
          tray_id: 0,
          tray_info_idx: 'GFSL05',
          tray_type: 'PLA',
          tray_sub_brands: 'Bambu PLA Basic',
          tray_color: 'FF0000FF',
          nozzle_temp_min: 190,
          nozzle_temp_max: 230,
          setting_id: 'GFSL05_07',
        })).resolves.not.toThrow()
      })

      it('should set calibration for a slot', async () => {
        await expect(api.setCalibration('00M09A123456789', 0, 0, {
          cali_idx: 42,
          filament_id: 'GFSL05',
          k_value: 0.025,
        })).resolves.not.toThrow()
      })

      it('should re-read slot RFID', async () => {
        await expect(api.rereadSlot('00M09A123456789', 0, 0)).resolves.not.toThrow()
      })

      it('should clear slot', async () => {
        server.use(
          http.post('/api/printers/:serial/ams/:amsId/tray/:trayId/filament', async ({ request }) => {
            const body = await request.json() as { tray_info_idx: string; tray_type: string }
            // Verify it sends empty values
            expect(body.tray_info_idx).toBe('')
            expect(body.tray_type).toBe('')
            return HttpResponse.json({ status: 'ok' })
          })
        )

        await expect(api.clearSlot('00M09A123456789', 0, 0)).resolves.not.toThrow()
      })

      it('should assign spool to slot', async () => {
        const result = await api.assignSpoolToSlot('00M09A123456789', 0, 0, 'spool-1')
        expect(result.status).toBe('configured')
        expect(result.needs_replacement).toBe(false)
      })
    })
  })

  describe('Updates', () => {
    describe('getVersion', () => {
      it('should get version info', async () => {
        const version = await api.getVersion()
        expect(version.version).toBe('0.1.0')
        expect(version.git_commit).toBe('abc1234')
        expect(version.git_branch).toBe('main')
      })
    })

    describe('checkForUpdates', () => {
      it('should check for updates', async () => {
        const check = await api.checkForUpdates()
        expect(check.current_version).toBe('0.1.0')
        expect(check.update_available).toBe(false)
      })

      it('should force check for updates', async () => {
        server.use(
          http.get('/api/updates/check', ({ request }) => {
            const url = new URL(request.url)
            const force = url.searchParams.get('force')
            return HttpResponse.json({
              current_version: '0.1.0',
              latest_version: force === 'true' ? '0.2.0' : '0.1.0',
              update_available: force === 'true',
              release_notes: null,
              release_url: null,
              published_at: null,
              error: null,
            })
          })
        )

        const check = await api.checkForUpdates(true)
        expect(check.update_available).toBe(true)
        expect(check.latest_version).toBe('0.2.0')
      })
    })

    describe('applyUpdate', () => {
      it('should apply update', async () => {
        const status = await api.applyUpdate()
        expect(status.status).toBe('checking')
      })

      it('should apply specific version', async () => {
        server.use(
          http.post('/api/updates/apply', async ({ request }) => {
            const body = await request.json() as { version: string }
            return HttpResponse.json({
              status: 'downloading',
              message: `Downloading ${body.version}...`,
              progress: 0,
              error: null,
            })
          })
        )

        const status = await api.applyUpdate('0.2.0')
        expect(status.status).toBe('downloading')
      })
    })

    describe('getUpdateStatus', () => {
      it('should get update status', async () => {
        const status = await api.getUpdateStatus()
        expect(status.status).toBe('idle')
      })
    })

    describe('resetUpdateStatus', () => {
      it('should reset update status', async () => {
        const status = await api.resetUpdateStatus()
        expect(status.status).toBe('idle')
      })
    })
  })

  describe('Cloud', () => {
    describe('getCloudStatus', () => {
      it('should get cloud status', async () => {
        const status = await api.getCloudStatus()
        expect(status.is_authenticated).toBe(false)
        expect(status.email).toBeNull()
      })
    })

    describe('cloudLogin', () => {
      it('should initiate cloud login', async () => {
        const result = await api.cloudLogin('test@example.com', 'password')
        expect(result.needs_verification).toBe(true)
      })
    })

    describe('cloudVerify', () => {
      it('should verify cloud login', async () => {
        const result = await api.cloudVerify('test@example.com', '123456')
        expect(result.success).toBe(true)
      })
    })

    describe('cloudSetToken', () => {
      it('should set cloud token', async () => {
        server.use(
          http.post('/api/cloud/token', () => {
            return HttpResponse.json({
              is_authenticated: true,
              email: 'test@example.com',
            })
          })
        )

        const status = await api.cloudSetToken('test-token')
        expect(status.is_authenticated).toBe(true)
        expect(status.email).toBe('test@example.com')
      })
    })

    describe('cloudLogout', () => {
      it('should logout from cloud', async () => {
        await expect(api.cloudLogout()).resolves.not.toThrow()
      })
    })

    describe('getSlicerSettings', () => {
      it('should get slicer settings', async () => {
        const settings = await api.getSlicerSettings()
        expect(settings.filament).toBeInstanceOf(Array)
        expect(settings.printer).toBeInstanceOf(Array)
        expect(settings.process).toBeInstanceOf(Array)
      })
    })

    describe('getFilamentPresets', () => {
      it('should get filament presets', async () => {
        server.use(
          http.get('/api/cloud/filaments', () => {
            return HttpResponse.json([
              { setting_id: 'GFSL05_07', name: 'Bambu PLA Basic', type: 'filament' },
            ])
          })
        )

        const presets = await api.getFilamentPresets()
        expect(presets).toBeInstanceOf(Array)
        expect(presets[0].setting_id).toBe('GFSL05_07')
      })
    })

    describe('getSettingDetail', () => {
      it('should get setting detail', async () => {
        const detail = await api.getSettingDetail('custom-pla-001')
        expect(detail.setting_id).toBe('custom-pla-001')
        expect(detail.filament_id).toBe('GFSL05')
      })
    })
  })

  describe('Discovery', () => {
    describe('getDiscoveryStatus', () => {
      it('should get discovery status', async () => {
        server.use(
          http.get('/api/discovery/status', () => {
            return HttpResponse.json({ running: false })
          })
        )

        const status = await api.getDiscoveryStatus()
        expect(status.running).toBe(false)
      })
    })

    describe('startDiscovery', () => {
      it('should start discovery', async () => {
        server.use(
          http.post('/api/discovery/start', () => {
            return HttpResponse.json({ running: true })
          })
        )

        const status = await api.startDiscovery()
        expect(status.running).toBe(true)
      })
    })

    describe('stopDiscovery', () => {
      it('should stop discovery', async () => {
        server.use(
          http.post('/api/discovery/stop', () => {
            return HttpResponse.json({ running: false })
          })
        )

        const status = await api.stopDiscovery()
        expect(status.running).toBe(false)
      })
    })

    describe('getDiscoveredPrinters', () => {
      it('should get discovered printers', async () => {
        server.use(
          http.get('/api/discovery/printers', () => {
            return HttpResponse.json([
              { serial: 'NEW123', name: 'New Printer', ip_address: '192.168.1.200', model: 'P1S' },
            ])
          })
        )

        const printers = await api.getDiscoveredPrinters()
        expect(printers).toBeInstanceOf(Array)
        expect(printers[0].serial).toBe('NEW123')
      })
    })
  })

  describe('Device', () => {
    describe('getDeviceStatus', () => {
      it('should get device status', async () => {
        const status = await api.getDeviceStatus()
        expect(status.connected).toBe(false)
      })
    })

    describe('tareScale', () => {
      it('should tare scale', async () => {
        server.use(
          http.post('/api/device/scale/tare', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.tareScale()).resolves.not.toThrow()
      })
    })

    describe('calibrateScale', () => {
      it('should calibrate scale', async () => {
        server.use(
          http.post('/api/device/scale/calibrate', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.calibrateScale(100)).resolves.not.toThrow()
      })
    })

    describe('resetScaleCalibration', () => {
      it('should reset scale calibration', async () => {
        server.use(
          http.post('/api/device/scale/reset', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.resetScaleCalibration()).resolves.not.toThrow()
      })
    })

    describe('writeTag', () => {
      it('should write tag', async () => {
        server.use(
          http.post('/api/device/write-tag', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.writeTag('spool-1')).resolves.not.toThrow()
      })
    })
  })

  describe('ESP32 Device', () => {
    describe('ESP32 operations', () => {
      it('should get ESP32 config', async () => {
        server.use(
          http.get('/api/device/config', () => {
            return HttpResponse.json({ ip: '192.168.1.50', port: 80, name: 'SpoolBuddy' })
          })
        )

        const config = await api.getESP32Config()
        expect(config?.ip).toBe('192.168.1.50')
      })

      it('should save ESP32 config', async () => {
        server.use(
          http.post('/api/device/config', async ({ request }) => {
            const body = await request.json() as { ip: string; port: number; name: string }
            return HttpResponse.json(body)
          })
        )

        const config = await api.saveESP32Config({ ip: '192.168.1.50', port: 80, name: 'SpoolBuddy' })
        expect(config.ip).toBe('192.168.1.50')
      })

      it('should connect to ESP32', async () => {
        server.use(
          http.post('/api/device/connect', () => {
            return HttpResponse.json({
              connected: true,
              device: { ip: '192.168.1.50', hostname: 'spoolbuddy', mac_address: null, firmware_version: '1.0.0', nfc_status: true, scale_status: true, uptime: 3600, last_seen: null },
              last_error: null,
              reconnect_attempts: 0,
            })
          })
        )

        const status = await api.connectESP32()
        expect(status.connected).toBe(true)
      })

      it('should disconnect from ESP32', async () => {
        server.use(
          http.post('/api/device/disconnect', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.disconnectESP32()).resolves.not.toThrow()
      })

      it('should discover ESP32 devices', async () => {
        server.use(
          http.post('/api/device/discover', () => {
            return HttpResponse.json({
              devices: [{ ip: '192.168.1.50', hostname: 'spoolbuddy', mac_address: null, firmware_version: '1.0.0', nfc_status: true, scale_status: true, uptime: 3600, last_seen: null }],
              scan_duration_ms: 3000,
            })
          })
        )

        const result = await api.discoverESP32Devices()
        expect(result.devices).toBeInstanceOf(Array)
        expect(result.devices[0].ip).toBe('192.168.1.50')
      })

      it('should ping ESP32', async () => {
        server.use(
          http.post('/api/device/ping', () => {
            return HttpResponse.json({
              reachable: true,
              device: { ip: '192.168.1.50', hostname: 'spoolbuddy', mac_address: null, firmware_version: '1.0.0', nfc_status: true, scale_status: true, uptime: 3600, last_seen: null },
            })
          })
        )

        const result = await api.pingESP32('192.168.1.50')
        expect(result.reachable).toBe(true)
      })

      it('should reboot ESP32', async () => {
        server.use(
          http.post('/api/device/reboot', () => {
            return HttpResponse.json({ success: true, message: 'Rebooting...' })
          })
        )

        const result = await api.rebootESP32()
        expect(result.success).toBe(true)
      })
    })
  })

  describe('Catalogs', () => {
    describe('Spool Catalog', () => {
      it('should get spool catalog', async () => {
        server.use(
          http.get('/api/catalog', () => {
            return HttpResponse.json([
              { id: 1, name: 'Standard 1kg', weight: 250, is_default: true, created_at: null },
            ])
          })
        )

        const catalog = await api.getSpoolCatalog()
        expect(catalog).toBeInstanceOf(Array)
        expect(catalog[0].name).toBe('Standard 1kg')
      })

      it('should add catalog entry', async () => {
        server.use(
          http.post('/api/catalog', async ({ request }) => {
            const body = await request.json() as { name: string; weight: number }
            return HttpResponse.json({
              id: 99,
              name: body.name,
              weight: body.weight,
              is_default: false,
              created_at: Date.now(),
            })
          })
        )

        const entry = await api.addCatalogEntry({ name: 'Custom Spool', weight: 200 })
        expect(entry.name).toBe('Custom Spool')
      })

      it('should delete catalog entry', async () => {
        server.use(
          http.delete('/api/catalog/:id', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.deleteCatalogEntry(1)).resolves.not.toThrow()
      })

      it('should reset spool catalog', async () => {
        server.use(
          http.post('/api/catalog/reset', () => {
            return new HttpResponse(null, { status: 204 })
          })
        )

        await expect(api.resetSpoolCatalog()).resolves.not.toThrow()
      })
    })

    describe('Color Catalog', () => {
      it('should get color catalog', async () => {
        server.use(
          http.get('/api/colors', () => {
            return HttpResponse.json([
              { id: 1, manufacturer: 'Bambu Lab', color_name: 'Black', hex_color: '000000', material: 'PLA', is_default: true, created_at: null },
            ])
          })
        )

        const catalog = await api.getColorCatalog()
        expect(catalog).toBeInstanceOf(Array)
        expect(catalog[0].color_name).toBe('Black')
      })

      it('should add color entry', async () => {
        server.use(
          http.post('/api/colors', async ({ request }) => {
            const body = await request.json() as { manufacturer: string; color_name: string; hex_color: string }
            return HttpResponse.json({
              id: 99,
              manufacturer: body.manufacturer,
              color_name: body.color_name,
              hex_color: body.hex_color,
              material: null,
              is_default: false,
              created_at: Date.now(),
            })
          })
        )

        const entry = await api.addColorEntry({ manufacturer: 'Generic', color_name: 'Red', hex_color: 'FF0000' })
        expect(entry.color_name).toBe('Red')
      })

      it('should lookup color', async () => {
        server.use(
          http.get('/api/colors/lookup', ({ request }) => {
            const url = new URL(request.url)
            const manufacturer = url.searchParams.get('manufacturer')
            const colorName = url.searchParams.get('color_name')
            if (manufacturer === 'Bambu Lab' && colorName === 'Black') {
              return HttpResponse.json({ found: true, hex_color: '000000', material: 'PLA' })
            }
            return HttpResponse.json({ found: false, hex_color: null, material: null })
          })
        )

        const result = await api.lookupColor('Bambu Lab', 'Black')
        expect(result.found).toBe(true)
        expect(result.hex_color).toBe('000000')
      })

      it('should search colors', async () => {
        server.use(
          http.get('/api/colors/search', ({ request }) => {
            const url = new URL(request.url)
            const manufacturer = url.searchParams.get('manufacturer')
            if (manufacturer === 'Bambu Lab') {
              return HttpResponse.json([
                { id: 1, manufacturer: 'Bambu Lab', color_name: 'Black', hex_color: '000000', material: 'PLA', is_default: true, created_at: null },
              ])
            }
            return HttpResponse.json([])
          })
        )

        const colors = await api.searchColors('Bambu Lab')
        expect(colors).toBeInstanceOf(Array)
        expect(colors.length).toBe(1)
      })
    })
  })

  describe('AMS History', () => {
    it('should get AMS history', async () => {
      server.use(
        http.get('/api/printers/:serial/ams/:amsId/history', () => {
          return HttpResponse.json({
            printer_serial: '00M09A123456789',
            ams_id: 0,
            data: [
              { recorded_at: Date.now(), humidity: 35, humidity_raw: 35, temperature: 25.5 },
            ],
            min_humidity: 30,
            max_humidity: 40,
            avg_humidity: 35,
            min_temperature: 24,
            max_temperature: 27,
            avg_temperature: 25.5,
          })
        })
      )

      const history = await api.getAMSHistory('00M09A123456789', 0, 24)
      expect(history.printer_serial).toBe('00M09A123456789')
      expect(history.data).toBeInstanceOf(Array)
    })
  })

  describe('API Keys', () => {
    it('should get API keys', async () => {
      server.use(
        http.get('/api/api-keys/', () => {
          return HttpResponse.json([
            { id: 1, name: 'Test Key', key_prefix: 'sk_test_', can_read: true, can_write: false, can_control: false, enabled: true, last_used: null, created_at: Date.now() },
          ])
        })
      )

      const keys = await api.getAPIKeys()
      expect(keys).toBeInstanceOf(Array)
      expect(keys[0].name).toBe('Test Key')
    })

    it('should create API key', async () => {
      server.use(
        http.post('/api/api-keys/', async ({ request }) => {
          const body = await request.json() as { name: string }
          return HttpResponse.json({
            id: 99,
            name: body.name,
            key_prefix: 'sk_new_',
            key: 'sk_new_abc123xyz',
            can_read: true,
            can_write: false,
            can_control: false,
            enabled: true,
            last_used: null,
            created_at: Date.now(),
          })
        })
      )

      const key = await api.createAPIKey({ name: 'New Key' })
      expect(key.name).toBe('New Key')
      expect(key.key).toBeDefined()
    })

    it('should update API key', async () => {
      server.use(
        http.patch('/api/api-keys/:id', async ({ params, request }) => {
          const body = await request.json() as { enabled: boolean }
          return HttpResponse.json({
            id: Number(params.id),
            name: 'Test Key',
            key_prefix: 'sk_test_',
            can_read: true,
            can_write: false,
            can_control: false,
            enabled: body.enabled,
            last_used: null,
            created_at: Date.now(),
          })
        })
      )

      const key = await api.updateAPIKey(1, { enabled: false })
      expect(key.enabled).toBe(false)
    })

    it('should delete API key', async () => {
      server.use(
        http.delete('/api/api-keys/:id', () => {
          return HttpResponse.json({ message: 'Key deleted' })
        })
      )

      const result = await api.deleteAPIKey(1)
      expect(result.message).toBe('Key deleted')
    })
  })

  describe('Support', () => {
    it('should get debug logging state', async () => {
      server.use(
        http.get('/api/support/debug-logging', () => {
          return HttpResponse.json({ enabled: false, enabled_at: null, duration_seconds: null })
        })
      )

      const state = await api.getDebugLogging()
      expect(state.enabled).toBe(false)
    })

    it('should set debug logging', async () => {
      server.use(
        http.post('/api/support/debug-logging', async ({ request }) => {
          const body = await request.json() as { enabled: boolean }
          return HttpResponse.json({
            enabled: body.enabled,
            enabled_at: body.enabled ? new Date().toISOString() : null,
            duration_seconds: body.enabled ? 300 : null,
          })
        })
      )

      const state = await api.setDebugLogging(true)
      expect(state.enabled).toBe(true)
    })

    it('should get logs', async () => {
      server.use(
        http.get('/api/support/logs', () => {
          return HttpResponse.json({
            entries: [
              { timestamp: new Date().toISOString(), level: 'INFO', logger_name: 'main', message: 'Test log' },
            ],
            total_count: 1,
            filtered_count: 1,
          })
        })
      )

      const logs = await api.getLogs()
      expect(logs.entries).toBeInstanceOf(Array)
      expect(logs.entries[0].message).toBe('Test log')
    })

    it('should clear logs', async () => {
      server.use(
        http.delete('/api/support/logs', () => {
          return HttpResponse.json({ message: 'Logs cleared' })
        })
      )

      const result = await api.clearLogs()
      expect(result.message).toBe('Logs cleared')
    })

    it('should get system info', async () => {
      server.use(
        http.get('/api/support/system-info', () => {
          return HttpResponse.json({
            version: '0.1.0',
            uptime: '1h 30m',
            uptime_seconds: 5400,
            hostname: 'spoolbuddy',
            spool_count: 10,
            printer_count: 2,
            connected_printers: 1,
            database_size: '1.5 MB',
            disk_total: '100 GB',
            disk_used: '50 GB',
            disk_free: '50 GB',
            disk_percent: 50,
            platform: 'Linux',
            platform_release: '5.15.0',
            python_version: '3.11.0',
            boot_time: new Date().toISOString(),
            memory_total: '8 GB',
            memory_used: '4 GB',
            memory_available: '4 GB',
            memory_percent: 50,
            cpu_count: 4,
            cpu_percent: 25,
          })
        })
      )

      const info = await api.getSystemInfo()
      expect(info.version).toBe('0.1.0')
      expect(info.spool_count).toBe(10)
    })
  })

  describe('Error Handling', () => {
    it('should throw error on network failure', async () => {
      server.use(
        http.get('/api/spools', () => {
          return HttpResponse.error()
        })
      )

      await expect(api.listSpools()).rejects.toThrow()
    })

    it('should throw error on 500 response', async () => {
      server.use(
        http.get('/api/spools', () => {
          return new HttpResponse('Internal Server Error', { status: 500 })
        })
      )

      await expect(api.listSpools()).rejects.toThrow('Internal Server Error')
    })

    it('should throw error on 404 response', async () => {
      server.use(
        http.get('/api/spools/:id', () => {
          return new HttpResponse('Not Found', { status: 404 })
        })
      )

      await expect(api.getSpool('non-existent')).rejects.toThrow()
    })

    it('should throw error on 401 response', async () => {
      server.use(
        http.get('/api/cloud/settings', () => {
          return new HttpResponse('Unauthorized', { status: 401 })
        })
      )

      await expect(api.getSlicerSettings()).rejects.toThrow('Unauthorized')
    })

    it('should handle empty response body', async () => {
      server.use(
        http.delete('/api/spools/:id', () => {
          return new HttpResponse(null, { status: 204 })
        })
      )

      // Should not throw
      await expect(api.deleteSpool('spool-1')).resolves.not.toThrow()
    })
  })
})
