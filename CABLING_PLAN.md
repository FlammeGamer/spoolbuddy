# SpoolBuddy Cabling Plan

## Hardware Components

| Component | Model | Interface | Status |
|-----------|-------|-----------|--------|
| Display | ELECROW CrowPanel Advance 7.0" | ESP32-S3 built-in | ✓ |
| NFC Reader | PN5180 | SPI | Ready to wire |
| Raspberry Pi Pico | RP 2040 | SPI and I2C | Ready to wire |
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
│   │          │  │  Scale   │  │          │  │  N/A!   │  │ Control │   │
│   │          │  │  I2C     │  │          │  │CONFLICT │  │  Pins   │   │
│   └──────────┘  └──────────┘  └──────────┘  └─────────┘  └─────────┘   │
│                                                                         │
│   [BOOT]  [RESET]                                           [USB-C]    │
│                        [DIP SWITCHES]                       [UART0-IN] │
│                           S1  S0                                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### UART1-OUT Header (4-pin) - For NAU7802 Scale I2C

```
┌──────┬──────┬──────┬──────┐
│ Pin1 │ Pin2 │ Pin3 │ Pin4 │
│IO19  │IO20  │ 3V3  │ GND  │
│ SDA  │ SCL  │ VCC  │ GND  │
└──────┴──────┴──────┴──────┘
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
    │   ┌──────────┐   │            │   │  IO43 ●───────────┼                 │
    │   │PN5180    │   │            │   │  IO44 ●───────────┼                 │
    │   │  Chip    │   │            │   │  3V3  ●───────────┼                 │
    │   └──────────┘   │            │   │  GND  ●───────────┼                 │
    │                  │            │   │                   │                 │
    │   ┌──────────┐   │            │   └───────────────────┘                 │
    │   │ Antenna  │   │            │                                         │
    │   │  Coil    │   │            │   J11 Header                            │
    │   └──────────┘   │            │   ┌───────────────────┐                 │
    │                  │            │   │                   │                 │
    └──────────────────┘            │   │  IO19 ●──────────┼─────SDA          │
            │ via Pi Pico Bridge    │   │  IO20 ●──────────┼─────SCL          │
            │        I2C            │   │  3V3  ●──────────┼─────VCC          │
            └───────────────────────┼───│  GND  ●──────────┼─────GND          │
            │                       │   │                   │                 │
            │                       │   └───────────────────┘                 │
            │                       │                                         │
     NAU7802 + Load Cell            │   USB-C (Power & Debug)                 │
    ┌──────────────────┐            │   ┌───────────────────┐                 │
    │  ┌────────────┐  │            │   │    ○ USB-C        │                 │
    │  │ SparkFun   │  │            │   └───────────────────┘                 │
    │  │ Qwiic      │  │            └─────────────────────────────────────────┘
    │  │ Scale      │  │            
    │  └────────────┘  │
    │        │         │ 
    │   ┌────┴────┐    │ 
    │   │Load Cell│    │
    │   │(4-wire) │    │
    │   └─────────┘    │
    │                  │
    └──────────────────┘ 

```

---

## Pin Assignments

### PN5180 NFC Reader (SPI)

| PN5180 Pin | Raspberry Pi Pico GPIO | Wire Color |
|------------|------------------------|------------|
| VCC | 3.3V (or CrowPanel UART1-OUT Pin 3) | Red |
| GND | GND (or CrowPanel UART-1 OUT Pin 4) | Black |
| SCK | GP19 | Yellow |
| MISO | GP16 | Blue |
| MOSI | GP18 | Green |
| NSS (CS) | GP17 | Orange |
| BUSY | GP20 | White |
| RST | GP21 | Brown |

**SPI Configuration:**
- Mode: SPI Mode 0 (CPOL=0, CPHA=0)
- Speed: 1 MHz (can go up to 7 MHz)
- Bit order: MSB first

### Raspberry Pi Pico (I2C)

| Raspberry Pi Pico Pin | ESP32-S3 GPIO | Header | Pin # | Wire Color |
|-------------|---------------|--------|-------|------------|
| VSYS | 3.3V | UART1-OUT | Pin 3 | Red |
| GP4 | IO19 | UART1-OUT | Pin 1 | Yellow |
| GP5 | IO20 | UART1-OUT | Pin 2 | White |
| GND | GND | UART1-OUT | Pin 4 | Black |

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

1. [ ] Connect SCK → GP19 (Pin 25)
2. [ ] Connect MISO → GP16 (Pin 21)
3. [ ] Connect MOSI → GP18 (Pin 24)
4. [ ] Connect NSS → GP17 (Pin 22)
5. [ ] Connect BUSY → GP20 (Pin 26)
6. [ ] Connect RST → GP21 (Pin 27)
7. [ ] Connect VCC → UART1-OUT Pin 3 (3V3) or (Pico 3V3)
8. [ ] Connect GND → UART1-OUT Pin 4 (GND) or (Pico GND)

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
│  PN5180 (NFC)                NAU7802 (Scale)               │
│  ───────────                 ──────────────                │
│  VCC  → UART0 Pin3 (3V3)     VCC → UART1 Pin3 (3V3)        │
│  GND  → UART0 Pin4 (GND)     GND → UART1 Pin4 (GND)        │
│  SCK  → Pico GP19 (Pin 25)   SDA → UART1 Pin1 (IO19)       │
│  MISO → Pico GP16 (Pin 21)   SCL → UART1 Pin2 (IO20)       │
│  MOSI → Pico GP18 (Pin 24)                                 │
│  NSS  → Pico GP17 (Pin 22)   Load Cell → Squeeze terminal  │
│  BUSY → Pico GP20 (Pin 26)   Red   → E+                    │
│  RST  → Pico GP21 (Pin 27)   Black → E-                    │
│                              White → A-                    │
│  Power: USB-C 5V/2A          Green → A+                    │
│                                                            │
│  Raspberry Pi Pico (SPI to I2C bridge)                     │
│  3V3 → UART1 Pin3 (3V3)                                    │
│  GND → UART1 Pin4 (GND)                                    │
│  GP4 → UART1 Pin1 (IO19)                                   │
│  GP5 → UART1 Pin2 (IO20)                                   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

---

## Physical Assembly Notes

### NFC Antenna Positioning
- Position PN5180 antenna coil **under** the scale platform
- Center the antenna with the spool's core hole
- PN5180 has ~5cm read range with 3.3V supply (suitable for Bambu Lab tags inside spool core)
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
| Raspberry Pi Pico | 3.3V | 45mA | 100mA |
| NAU7802 | 3.3V | 1mA | 2mA |
| **Total** | **5V USB** | **~400mA** | **~750mA** |

**Recommendation:** Use a quality USB-C cable and 5V/2A power adapter.

---

## Troubleshooting

### PN5180 Not Responding
1. Verify wiring against checklist above
2. Verify 3.3V power (measure with multimeter)
3. Check RST is high (IO15)
4. Reduce SPI speed to 500kHz for testing
5. Check BUSY pin behavior during operations

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

1. **Flash firmware**: See `firmware/README.md`
2. **Test NFC**: Place tag on antenna, check serial output
3. **Calibrate scale**: Use known weight, run calibration
4. **Connect to server**: Configure WiFi, verify WebSocket connection
5. **Test full flow**: Read tag → update UI → log weight