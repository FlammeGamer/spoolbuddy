# SpoolBuddy Cabling Plan

## Hardware Components

| Component | Model | Interface | Status |
|-----------|-------|-----------|--------|
| Display | ELECROW CrowPanel Advance 7.0" | ESP32-S3 built-in | ✓ |
| NFC Reader | PN5180 | SPI | Ready to wire |
| Scale ADC | NAU7802 (SparkFun Qwiic Scale) | I2C | Connected |
| Load Cell | 5kg Single-Point | NAU7802 | Connected |

---

## IMPORTANT: J9 Header Pin Conflict

**WARNING: J9 header pins (IO4, IO5, IO6) are used internally by the RGB LCD display!**
These pins appear shorted to GND when the display is powered on.
DO NOT use J9 for SPI - use UART0-OUT header instead.

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
│   │ SPI CLK  │  │  Scale   │  │          │  │  N/A!   │  │ Control │   │
│   │ SPI MISO │  │  I2C     │  │          │  │CONFLICT │  │  Pins   │   │
│   └──────────┘  └──────────┘  └──────────┘  └─────────┘  └─────────┘   │
│                                                                         │
│   [BOOT]  [RESET]                                           [USB-C]    │
│                        [DIP SWITCHES]                       [UART0-IN] │
│                           S1  S0                                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### UART0-OUT Header (4-pin) - For PN5180 SPI CLK and MISO

```
┌──────┬──────┬──────┬──────┐
│ Pin1 │ Pin2 │ Pin3 │ Pin4 │
│IO44  │IO43  │ 3V3  │ GND  │
│MISO  │ SCK  │ VCC  │ GND  │
└──────┴──────┴──────┴──────┘

BLUE   ---| IO44 : ← MISO (SPI data IN from PN5180)
YELLOW ---| IO43 : ← SCK  (SPI clock)
RED    ---| 3V3  : ← VCC  (power)
BLACK  ---| GND  : ← GND  (ground)
```

### UART1-OUT Header (4-pin) - For NAU7802 Scale I2C

```
┌──────┬──────┬──────┬──────┐
│ Pin1 │ Pin2 │ Pin3 │ Pin4 │
│IO19  │IO20  │ 3V3  │ GND  │
│ SDA  │ SCL  │ VCC  │ GND  │
└──────┴──────┴──────┴──────┘
```

### J11 Header (7-pin) - For PN5180 Control Pins

```
        J11 (Right)
        ┌────────┐
Pin 1   │  IO19  │   ← Used by Scale (I2C SDA)
Pin 2   │  IO16  │   ← MOSI (SPI data OUT to PN5180)
Pin 3   │  IO15  │   ← RST  (PN5180 reset)
Pin 4   │   NC   │
Pin 5   │  IO2   │   ← BUSY (PN5180 busy signal)
Pin 6   │  IO8   │   ← NSS  (SPI chip select)
Pin 7   │   NC   │
        └────────┘

GREEN  ---| IO16 : ← MOSI (SPI data OUT to PN5180)
BLUE   ---| IO15 : ← RST
GREEN  ---| IO2  : ← BUSY
BLACK  ---| IO8  : ← NSS (chip select)
```

### J9 Header - DO NOT USE (Conflicts with LCD Display)

```
        J9 (Left) - UNUSABLE!
        ┌────────┐
Pin 1   │  IO20  │   ← Conflicts with LCD
Pin 2   │  IO5   │   ← Conflicts with LCD
Pin 3   │  IO4   │   ← Conflicts with LCD
Pin 4   │  IO6   │   ← Conflicts with LCD
Pin 5   │  3V3   │
Pin 6   │  GND   │
Pin 7   │   5V   │
        └────────┘
```

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
     PN5180 NFC Module              │   UART0-OUT Header                      │
    ┌──────────────────┐            │   ┌───────────────────┐                 │
    │                  │            │   │                   │                 │
    │   ┌──────────┐   │            │   │  IO43 ●──────────┼─────SCK         │
    │   │PN5180    │   │            │   │  IO44 ●──────────┼─────MISO        │
    │   │  Chip    │   │            │   │  3V3  ●──────────┼─────VCC         │
    │   └──────────┘   │            │   │  GND  ●──────────┼─────GND         │
    │                  │            │   │                   │                 │
    │   ┌──────────┐   │            │   └───────────────────┘                 │
    │   │ Antenna  │   │            │                                         │
    │   │  Coil    │   │            │   J11 Header                            │
    │   └──────────┘   │            │   ┌───────────────────┐                 │
    │                  │            │   │                   │                 │
    └──────────────────┘            │   │  IO16 ●──────────┼─────MOSI        │
            │                       │   │  IO15 ●──────────┼─────RST         │
            │                       │   │  IO2  ●──────────┼─────BUSY        │
            └───────────────────────┼───│  IO8  ●──────────┼─────NSS (CS)    │
                                    │   │                   │                 │
                                    │   └───────────────────┘                 │
                                    │                                         │
     NAU7802 + Load Cell            │   UART1-OUT (or I2C-OUT)                │
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
| VCC | 3.3V | UART0-OUT | Pin 3 | Red |
| GND | GND | UART0-OUT | Pin 4 | Black |
| SCK | IO43 | UART0-OUT | Pin 2 | Yellow |
| MISO | IO44 | UART0-OUT | Pin 1 | Blue |
| MOSI | IO16 | J11 | Pin 2 | Green |
| NSS (CS) | IO8 | J11 | Pin 6 | Orange |
| BUSY | IO2 | J11 | Pin 5 | White |
| RST | IO15 | J11 | Pin 3 | Brown |

**SPI Configuration:**
- Mode: SPI Mode 0 (CPOL=0, CPHA=0)
- Speed: 1 MHz (can go up to 7 MHz)
- Bit order: MSB first

### NAU7802 Scale (I2C)

| NAU7802 Pin | ESP32-S3 GPIO | Header | Pin # | Wire Color |
|-------------|---------------|--------|-------|------------|
| VCC | 3.3V | UART1-OUT | Pin 3 | Red |
| SDA | IO19 | UART1-OUT | Pin 1 | Yellow |
| SCL | IO20 | UART1-OUT | Pin 2 | White |
| GND | GND | UART1-OUT | Pin 4 | Black |

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

- [ ] Verify all connections are secure
- [ ] Confirm 3.3V (not 5V) for PN5180
- [ ] Check no shorts between adjacent pins
- [ ] Ensure GND connections are solid

### PN5180 Verification

1. [ ] Connect SCK → UART0-OUT Pin 2 (IO43)
2. [ ] Connect MISO → UART0-OUT Pin 1 (IO44)
3. [ ] Connect MOSI → J11 Pin 2 (IO16)
4. [ ] Connect NSS → J11 Pin 6 (IO8)
5. [ ] Connect BUSY → J11 Pin 5 (IO2)
6. [ ] Connect RST → J11 Pin 3 (IO15)
7. [ ] Connect VCC → UART0-OUT Pin 3 (3V3)
8. [ ] Connect GND → UART0-OUT Pin 4 (GND)

### NAU7802 Verification

1. [ ] Connect SDA → UART1-OUT Pin 1 (IO19)
2. [ ] Connect SCL → UART1-OUT Pin 2 (IO20)
3. [ ] Connect VCC → UART1-OUT Pin 3 (3V3)
4. [ ] Connect GND → UART1-OUT Pin 4 (GND)
5. [ ] Load cell wired to E+/E-/A+/A-

---

## Quick Reference Card

```
┌────────────────────────────────────────────────────────────┐
│           SPOOLBUDDY QUICK WIRING (CrowPanel 7.0")         │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  *** DO NOT USE J9 HEADER - CONFLICTS WITH LCD ***         │
│                                                            │
│  PN5180 (NFC)              NAU7802 (Scale)                 │
│  ───────────               ──────────────                  │
│  VCC  → UART0 Pin3 (3V3)   VCC → UART1 Pin3 (3V3)         │
│  GND  → UART0 Pin4 (GND)   GND → UART1 Pin4 (GND)         │
│  SCK  → UART0 Pin2 (IO43)  SDA → UART1 Pin1 (IO19)        │
│  MISO → UART0 Pin1 (IO44)  SCL → UART1 Pin2 (IO20)        │
│  MOSI → J11 Pin2 (IO16)                                    │
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
1. Verify wiring against checklist above
2. Check that MISO/MOSI are NOT connected to J9 (conflicts with LCD!)
3. Verify 3.3V power (measure with multimeter)
4. Check RST is high (IO15)
5. Reduce SPI speed to 500kHz for testing
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

## Migration from Old Wiring (J9)

If you previously wired PN5180 to J9 header, you need to move 3 wires:

| Signal | Old Location | New Location |
|--------|--------------|--------------|
| SCK | J9 Pin 2 (IO5) | UART0-OUT Pin 2 (IO43) |
| MISO | J9 Pin 3 (IO4) | UART0-OUT Pin 1 (IO44) |
| MOSI | J9 Pin 4 (IO6) | J11 Pin 2 (IO16) |

Keep VCC/GND at UART0-OUT header.
Control pins (NSS, BUSY, RST) on J11 stay the same.

---

## Next Steps After Wiring

1. **Flash firmware**: See `firmware/README.md`
2. **Test NFC**: Place tag on antenna, check serial output
3. **Calibrate scale**: Use known weight, run calibration
4. **Connect to server**: Configure WiFi, verify WebSocket connection
5. **Test full flow**: Read tag → update UI → log weight
