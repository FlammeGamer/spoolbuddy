# SpoolBuddy Cabling Plan

## Hardware Components

| Component | Model | Interface | Status |
|-----------|-------|-----------|--------|
| Display | ELECROW CrowPanel Advance 7.0" | ESP32-S3 built-in | ✓ |
| NFC Reader | PN5180 | SPI | Ready to wire |
| Scale ADC | NAU7802 (SparkFun Qwiic Scale) | I2C | Connected |
| Load Cell | 5kg Single-Point | NAU7802 | Connected |

---

## CrowPanel Advance 7.0" Connector Reference

### Back Panel Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     CrowPanel Advance 7.0" (Back)                       │
│                                                                         │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐  ┌─────────┐   │
│   │UART0-OUT │  │UART1-OUT │  │ I2C-OUT  │  │   J9    │  │   J11   │   │
│   │  4-pin   │  │  4-pin   │  │  4-pin   │  │  1x7    │  │  1x7    │   │
│   └──────────┘  └──────────┘  └──────────┘  └─────────┘  └─────────┘   │
│                                             └─────────────────────────┘ │
│                                               Wireless Module Headers   │
│                                                                         │
│   [BOOT]  [RESET]                                           [USB-C]    │
│                        [DIP SWITCHES]                       [UART0-IN] │
│                           S1  S0                                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### Wireless Module Headers (J9 + J11) - For PN5180 NFC

```
        J9 (Left)              J11 (Right)
        ┌────────┐             ┌────────┐
Pin 1   │  IO20  │             │  IO19  │   ← Used by Scale (I2C)
Pin 2   │  IO5   │  ← SCK      │  IO16  │
Pin 3   │  IO4   │  ← MISO     │  IO15  │   ← RST
Pin 4   │  IO6   │  ← MOSI     │   NC   │
Pin 5   │  3V3   │  ← VCC      │  IO2   │   ← BUSY
Pin 6   │  GND   │  ← GND      │  IO8   │   ← CS (NSS)
Pin 7   │   5V   │             │   NC   │
        └────────┘             └────────┘

       ---| IO20 : ------       YELLOW ---| IO19 : ← Used by Scale (I2C)
YELLOW ---| IO5  : ← SCK               ---| IO16 : ----------------------
BLUE   ---| IO4  : ← MISO       BLUE   ---| IO15 : ← RST
GREEN  ---| IO6  : ← MOSI              ---| NC   : ----------------------
RED    ---| 3V3  : ← VCC        GREEN  ---| IO2  : ← BUSY
BLACK  ---| GND  : ← GND        BLACK  ---| IO8  : ← CS (NSS)
       ---| 5V   : ------              ---| NC   : ----------------------
```

### I2C-OUT Connector (4-pin) - For NAU7802 Scale

```
┌──────┬──────┬──────┬──────┐
│ Pin1 │ Pin2 │ Pin3 │ Pin4 │
│ 3V3  │ SDA  │ SCL  │ GND  │
│      │ IO19 │ IO20 │      │
└──────┴──────┴──────┴──────┘
```

### DIP Switch Settings

For PN5180 (Wireless Module mode - frees SPI from SD card):

| S1 | S0 | Mode |
|----|----|----|
| 0 | 1 | **Wireless Module** ← Use this |
| 0 | 0 | MIC + SPK |
| 1 | 0 | MIC + TF Card |

---

## Wiring Diagram

```
                                    ┌─────────────────────────────────────────┐
                                    │     ELECROW CrowPanel Advance 7.0"      │
                                    │                                         │
                                    │   ┌─────────────────────────────────┐   │
                                    │   │                                 │   │
                                    │   │      7.0" Touch Display         │   │
                                    │   │         (800 x 480)             │   │
                                    │   │                                 │   │
                                    │   │      [Built-in - no wiring]     │   │
                                    │   │                                 │   │
                                    │   └─────────────────────────────────┘   │
                                    │                                         │
     PN5180 NFC Module              │   J9 Header (Left)                      │
    ┌──────────────────┐            │   ┌───────────────────┐                 │
    │                  │            │   │                   │                 │
    │   ┌──────────┐   │            │   │  IO5  ●──────────┼─────SCK         │
    │   │PN5180    │   │            │   │  IO4  ●──────────┼─────MISO        │
    │   │  Chip    │   │            │   │  IO6  ●──────────┼─────MOSI        │
    │   └──────────┘   │            │   │  3V3  ●──────────┼─────VCC         │
    │                  │            │   │  GND  ●──────────┼─────GND         │
    │   ┌──────────┐   │            │   │                   │                 │
    │   │ Antenna  │   │            │   └───────────────────┘                 │
    │   │  Coil    │   │            │                                         │
    │   └──────────┘   │            │   J11 Header (Right)                    │
    │                  │            │   ┌───────────────────┐                 │
    └──────────────────┘            │   │                   │                 │
            │                       │   │  IO15 ●──────────┼─────RST         │
            │                       │   │  IO2  ●──────────┼─────BUSY        │
            └───────────────────────┼───│  IO8  ●──────────┼─────NSS (CS)    │
                                    │   │                   │                 │
                                    │   └───────────────────┘                 │
                                    │                                         │
     NAU7802 + Load Cell            │   I2C-OUT (4-pin)                       │
    ┌──────────────────┐            │   ┌───────────────────┐                 │
    │  ┌────────────┐  │            │   │                   │                 │
    │  │ SparkFun   │  │            │   │  IO19 ●──────────┼─────SDA         │
    │  │ Qwiic      │  │            │   │  IO20 ●──────────┼─────SCL         │
    │  │ Scale      │  │            │   │  3V3  ●──────────┼─────VCC         │
    │  └────────────┘  │            │   │  GND  ●──────────┼─────GND         │
    │        │         │            │   │                   │                 │
    │   ┌────┴────┐    │            │   └───────────────────┘                 │
    │   │Load Cell│    │            │                                         │
    │   │(4-wire) │    │            │   USB-C (Power & Debug)                 │
    │   └─────────┘    │            │   ┌───────────────────┐                 │
    │                  │            │   │    ○ USB-C        │                 │
    └──────────────────┘            │   └───────────────────┘                 │
                                    │                                         │
                                    └─────────────────────────────────────────┘
```

---

## Pin Assignments

### PN5180 NFC Reader (SPI)

| PN5180 Pin | ESP32-S3 GPIO | Header | Pin # | Wire Color |
|------------|---------------|--------|-------|------------|
| VCC | 3.3V | J9 | Pin 5 | Red |
| GND | GND | J9 | Pin 6 | Black |
| SCK | IO5 | J9 | Pin 2 | Blue |
| MISO | IO4 | J9 | Pin 3 | Green |
| MOSI | IO6 | J9 | Pin 4 | Yellow |
| NSS (CS) | IO8 | J11 | Pin 6 | Orange |
| BUSY | IO2 | J11 | Pin 5 | White |
| RST | IO15 | J11 | Pin 3 | Brown |

**SPI Configuration:**
- Mode: SPI Mode 0 (CPOL=0, CPHA=0)
- Speed: 2 MHz (max 10 MHz)
- Bit order: MSB first

### NAU7802 Scale (I2C)

| NAU7802 Pin | ESP32-S3 GPIO | Header | Pin # | Wire Color |
|-------------|---------------|--------|-------|------------|
| VCC | 3.3V | I2C-OUT | Pin 1 | Red |
| SDA | IO19 | I2C-OUT | Pin 2 | Yellow |
| SCL | IO20 | I2C-OUT | Pin 3 | White |
| GND | GND | I2C-OUT | Pin 4 | Black |

**I2C Configuration:**
- Address: 0x2A
- Speed: 400 kHz (Fast mode)

### Load Cell Wiring to NAU7802

```
   Load Cell (5kg)                SparkFun Qwiic Scale
  ┌─────────────────┐            ┌─────────────────┐
  │                 │            │                 │
  │  Red ───────────┼────────────┤► E+ (Red)       │
  │  Black ─────────┼────────────┤► E- (Black)     │
  │  White ─────────┼────────────┤► A- (White)     │
  │  Green ─────────┼────────────┤► A+ (Green)     │
  │                 │            │                 │
  │   ┌─────────┐   │            │  Qwiic to I2C   │
  │   │ Strain  │   │            │  connector      │
  │   │ Gauge   │   │            │                 │
  │   └─────────┘   │            │                 │
  │                 │            │                 │
  └─────────────────┘            └─────────────────┘
```

*Note: Wire colors vary by manufacturer. If readings are negative, swap A+ and A-.*

---

## Connection Checklist

### Before Powering On

- [ ] DIP switch set: S1=0, S0=1 (Wireless Module mode)
- [ ] Verify all connections are secure
- [ ] Confirm 3.3V (not 5V) for PN5180
- [ ] Check no shorts between adjacent pins
- [ ] Ensure GND connections are solid

### PN5180 Verification

1. [ ] Connect SCK → J9 Pin 2 (IO5)
2. [ ] Connect MISO → J9 Pin 3 (IO4)
3. [ ] Connect MOSI → J9 Pin 4 (IO6)
4. [ ] Connect NSS → J11 Pin 6 (IO8)
5. [ ] Connect BUSY → J11 Pin 5 (IO2)
6. [ ] Connect RST → J11 Pin 3 (IO15)
7. [ ] Connect VCC → J9 Pin 5 (3V3)
8. [ ] Connect GND → J9 Pin 6 (GND)

### NAU7802 Verification

1. [ ] Connect SDA → I2C-OUT Pin 2 (IO19)
2. [ ] Connect SCL → I2C-OUT Pin 3 (IO20)
3. [ ] Connect VCC → I2C-OUT Pin 1 (3V3)
4. [ ] Connect GND → I2C-OUT Pin 4 (GND)
5. [ ] Load cell wired to E+/E-/A+/A-

---

## Quick Reference Card

```
┌────────────────────────────────────────────────────────────┐
│           SPOOLBUDDY QUICK WIRING (CrowPanel 7.0")         │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  DIP Switch: S1=0, S0=1                                    │
│                                                            │
│  PN5180 (NFC)              NAU7802 (Scale)                 │
│  ───────────               ──────────────                  │
│  VCC  → J9 Pin5 (3V3)      VCC → I2C Pin1 (3V3)           │
│  GND  → J9 Pin6 (GND)      GND → I2C Pin4 (GND)           │
│  SCK  → J9 Pin2 (IO5)      SDA → I2C Pin2 (IO19)          │
│  MISO → J9 Pin3 (IO4)      SCL → I2C Pin3 (IO20)          │
│  MOSI → J9 Pin4 (IO6)                                      │
│  CS   → J11 Pin6 (IO8)     Load Cell → Qwiic terminal     │
│  BUSY → J11 Pin5 (IO2)       Red   → E+                   │
│  RST  → J11 Pin3 (IO15)      Black → E-                   │
│                              White → A-                    │
│  Power: USB-C 5V/2A          Green → A+                   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

---

## Physical Assembly Notes

### NFC Antenna Positioning
- Position PN5180 antenna coil **under** the scale platform
- Center the antenna with the spool's core hole
- PN5180 has ~20cm read range (suitable for Bambu Lab tags inside spool core)
- Keep antenna flat and parallel to scale surface

### Scale Platform
- Load cell mounting: single-point (bar type)
- Ensure stable, level mounting surface
- Protect load cell from overload (add mechanical stops if needed)
- Shield from drafts for stable readings

---

## Power Requirements

| Component | Voltage | Current (typical) | Current (peak) |
|-----------|---------|-------------------|----------------|
| CrowPanel 7.0" | 5V (via USB) | 300mA | 600mA |
| PN5180 | 3.3V | 80mA | 150mA |
| NAU7802 | 3.3V | 1mA | 2mA |
| **Total** | **5V USB** | **~400mA** | **~750mA** |

**Recommendation:** Use a quality USB-C cable and 5V/2A power adapter.

---

## Troubleshooting

### PN5180 Not Responding
1. Check DIP switch: S1=0, S0=1
2. Check SPI wiring (especially MISO/MOSI not swapped)
3. Verify 3.3V power (measure with multimeter)
4. Check RST is high (IO15)
5. Reduce SPI speed to 1MHz for testing
6. Check BUSY pin behavior during operations

### NAU7802 Erratic Readings
1. Check load cell wiring (swap A+/A- if readings inverted)
2. Ensure stable power supply
3. Add decoupling capacitor (100nF) near NAU7802
4. Shield from electrical noise
5. Allow warm-up time (~1 minute)

### Display Not Working
- Display is built-in; no wiring needed
- If blank: check USB power, try different cable
- If touch not working: GT911 touch controller is internal

---

## Next Steps After Wiring

1. **Set DIP switches**: S1=0, S0=1 for Wireless Module mode
2. **Flash firmware**: See `firmware/README.md`
3. **Test NFC**: Place tag on antenna, check serial output
4. **Calibrate scale**: Use known weight, run calibration
5. **Connect to server**: Configure WiFi, verify WebSocket connection
6. **Test full flow**: Read tag → update UI → log weight
