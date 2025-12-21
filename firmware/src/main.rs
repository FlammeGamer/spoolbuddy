//! SpoolBuddy Firmware - Home Screen UI
//! ESP32-S3 with ELECROW CrowPanel 7.0" (800x480 RGB565)
//! Using ESP-IDF RGB LCD driver with bounce buffer for PSRAM support
//! LVGL for proper font rendering

use cstr_core::CString;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::Hertz;
use esp_idf_sys as sys;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::{info, warn};

// Scale module for NAU7802
mod scale;

// NFC module for PN5180 (SPI) - hardware not connected yet
mod nfc;

// WiFi and HTTP screenshot server for development
mod wifi_init;
mod http_screenshot;

use std::ptr;

// LVGL imports
use lvgl::style::Style;
use lvgl::widgets::{Bar, Label};
use lvgl::{Align, Color, Display, DrawBuffer, Part, Widget};

// SpoolBuddy logo image (compiled via ESP-IDF component)
extern "C" {
    pub static spoolbuddy_logo: lvgl_sys::lv_img_dsc_t;
}

// Icon assets (embedded binary data from simulator)
// Logo (97x24)
const LOGO_WIDTH: u32 = 97;
const LOGO_HEIGHT: u32 = 24;
static LOGO_DATA: &[u8] = include_bytes!("../assets/logo.bin");

// Bell icon (20x20)
const BELL_WIDTH: u32 = 20;
const BELL_HEIGHT: u32 = 20;
static BELL_DATA: &[u8] = include_bytes!("../assets/bell.bin");

// NFC icon (72x72)
const NFC_WIDTH: u32 = 72;
const NFC_HEIGHT: u32 = 72;
static NFC_DATA: &[u8] = include_bytes!("../assets/nfc.bin");

// Weight icon (64x64)
const WEIGHT_WIDTH: u32 = 64;
const WEIGHT_HEIGHT: u32 = 64;
static WEIGHT_DATA: &[u8] = include_bytes!("../assets/weight.bin");

// Power icon (12x12)
const POWER_WIDTH: u32 = 12;
const POWER_HEIGHT: u32 = 12;
static POWER_DATA: &[u8] = include_bytes!("../assets/power.bin");

// Setting icon (40x40)
const SETTING_WIDTH: u32 = 40;
const SETTING_HEIGHT: u32 = 40;
static SETTING_DATA: &[u8] = include_bytes!("../assets/setting.bin");

// Encode icon (40x40)
const ENCODE_WIDTH: u32 = 40;
const ENCODE_HEIGHT: u32 = 40;
static ENCODE_DATA: &[u8] = include_bytes!("../assets/encode.bin");

// Image descriptors (initialized at runtime)
static mut LOGO_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut BELL_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut NFC_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut WEIGHT_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut POWER_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut SETTING_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut ENCODE_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };

// Use LVGL's built-in anti-aliased Montserrat fonts (4bpp = smooth text)
macro_rules! font {
    (12) => { unsafe { &lvgl_sys::lv_font_montserrat_12 } };
    (14) => { unsafe { &lvgl_sys::lv_font_montserrat_14 } };
    (16) => { unsafe { &lvgl_sys::lv_font_montserrat_16 } };
    (20) => { unsafe { &lvgl_sys::lv_font_montserrat_20 } };
    (24) => { unsafe { &lvgl_sys::lv_font_montserrat_24 } };
    (28) => { unsafe { &lvgl_sys::lv_font_montserrat_24 } };
}

// Display dimensions
const WIDTH: usize = 800;
const HEIGHT: usize = 480;

// Bounce buffer: 10 lines * 800 pixels = reduced for WiFi memory headroom
const BOUNCE_BUFFER_LINES: usize = 10;
const BOUNCE_BUFFER_SIZE_PX: usize = BOUNCE_BUFFER_LINES * WIDTH;

// LVGL draw buffer size - 1/20 of screen for memory efficiency
// 800*480/20 = 19200 pixels * 2 bytes = 38400 bytes
const LVGL_BUFFER_SIZE: usize = WIDTH * HEIGHT / 20;

// CrowPanel Advance 7.0" pin definitions
const PIN_PCLK: i32 = 39;
const PIN_HSYNC: i32 = 40;
const PIN_VSYNC: i32 = 41;
const PIN_DE: i32 = 42;

// RGB565 data pins
const PIN_B0: i32 = 21;
const PIN_B1: i32 = 47;
const PIN_B2: i32 = 48;
const PIN_B3: i32 = 45;
const PIN_B4: i32 = 38;
const PIN_G0: i32 = 9;
const PIN_G1: i32 = 10;
const PIN_G2: i32 = 11;
const PIN_G3: i32 = 12;
const PIN_G4: i32 = 13;
const PIN_G5: i32 = 14;
const PIN_R0: i32 = 7;
const PIN_R1: i32 = 17;
const PIN_R2: i32 = 18;
const PIN_R3: i32 = 3;
const PIN_R4: i32 = 46;

// Color constants for polished UI
const COLOR_BG: u32 = 0x1A1A1A;
const COLOR_CARD: u32 = 0x2D2D2D;
const COLOR_BORDER: u32 = 0x3D3D3D;
const COLOR_ACCENT: u32 = 0x00FF00;
const COLOR_WHITE: u32 = 0xFFFFFF;
const COLOR_GRAY: u32 = 0x808080;
const COLOR_STATUS_BAR: u32 = 0x1A1A1A;

/// Helper to create color from hex - RGB888 to RGB565
fn lv_color_hex(hex: u32) -> lvgl_sys::lv_color_t {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    lvgl_sys::lv_color_t {
        full: (r5 << 11) | (g6 << 5) | b5,
    }
}

/// Helper to set all padding at once
unsafe fn set_style_pad_all(obj: *mut lvgl_sys::lv_obj_t, pad: i16) {
    lvgl_sys::lv_obj_set_style_pad_top(obj, pad, 0);
    lvgl_sys::lv_obj_set_style_pad_bottom(obj, pad, 0);
    lvgl_sys::lv_obj_set_style_pad_left(obj, pad, 0);
    lvgl_sys::lv_obj_set_style_pad_right(obj, pad, 0);
}

/// Create a card with premium styling - shadows, solid background (no gradient to avoid banding)
unsafe fn create_card(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, w: i16, h: i16) -> *mut lvgl_sys::lv_obj_t {
    let card = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(card, w, h);
    lvgl_sys::lv_obj_set_pos(card, x, y);
    lvgl_sys::lv_obj_set_style_bg_color(card, lv_color_hex(0x232323), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(card, 255, 0);
    lvgl_sys::lv_obj_set_style_border_color(card, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_border_width(card, 1, 0);
    lvgl_sys::lv_obj_set_style_radius(card, 16, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(card, 20, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_x(card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(card, 4, 0);
    lvgl_sys::lv_obj_set_style_shadow_spread(card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(card, 80, 0);
    set_style_pad_all(card, 0);
    lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    card
}

/// Create an AMS slot with 4 color squares
unsafe fn create_ams_slot_4color(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, selected: bool, colors: &[u32; 4]) {
    let slot = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(slot, 72, 42);
    lvgl_sys::lv_obj_set_pos(slot, x, y);
    lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(if selected { 0x1A2A1A } else { 0x2A2A2A }), 0);
    lvgl_sys::lv_obj_set_style_radius(slot, 8, 0);
    lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(if selected { COLOR_ACCENT } else { 0x3D3D3D }), 0);
    lvgl_sys::lv_obj_set_style_border_width(slot, if selected { 2 } else { 1 }, 0);
    if selected {
        lvgl_sys::lv_obj_set_style_shadow_color(slot, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_shadow_width(slot, 12, 0);
        lvgl_sys::lv_obj_set_style_shadow_spread(slot, 0, 0);
        lvgl_sys::lv_obj_set_style_shadow_opa(slot, 80, 0);
    }
    set_style_pad_all(slot, 0);
    lvgl_sys::lv_obj_clear_flag(slot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    let slot_label = lvgl_sys::lv_label_create(slot);
    let slot_text = cstr_core::CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(slot_label, slot_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(slot_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(slot_label, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(slot_label, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 2);

    let square_size: i16 = 14;
    let square_gap: i16 = 2;
    let total_width = square_size * 4 + square_gap * 3;
    let start_x = (72 - total_width) / 2;

    for (i, &color) in colors.iter().enumerate() {
        let sq = lvgl_sys::lv_obj_create(slot);
        lvgl_sys::lv_obj_set_size(sq, square_size, square_size);
        lvgl_sys::lv_obj_set_pos(sq, start_x + (i as i16) * (square_size + square_gap), 22);
        lvgl_sys::lv_obj_set_style_radius(sq, 2, 0);
        lvgl_sys::lv_obj_set_style_border_width(sq, 0, 0);
        set_style_pad_all(sq, 0);
        lvgl_sys::lv_obj_clear_flag(sq, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
        if color == 0 {
            lvgl_sys::lv_obj_set_style_bg_color(sq, lv_color_hex(0x505050), 0);
        } else {
            lvgl_sys::lv_obj_set_style_bg_color(sq, lv_color_hex(color), 0);
        }
    }
}

/// Create a single-color AMS slot for EXT and HT slots
unsafe fn create_ams_slot_single(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, color: u32) {
    let slot = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(slot, 72, 22);
    lvgl_sys::lv_obj_set_pos(slot, x, y);
    lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(COLOR_BORDER), 0);
    lvgl_sys::lv_obj_set_style_radius(slot, 6, 0);
    lvgl_sys::lv_obj_set_style_border_width(slot, 0, 0);
    set_style_pad_all(slot, 0);
    lvgl_sys::lv_obj_clear_flag(slot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    let slot_label = lvgl_sys::lv_label_create(slot);
    let slot_text = cstr_core::CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(slot_label, slot_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(slot_label, lv_color_hex(COLOR_GRAY), 0);
    lvgl_sys::lv_obj_align(slot_label, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 8, 0);

    let color_sq = lvgl_sys::lv_obj_create(slot);
    lvgl_sys::lv_obj_set_size(color_sq, 14, 14);
    lvgl_sys::lv_obj_align(color_sq, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -6, 0);
    lvgl_sys::lv_obj_set_style_radius(color_sq, 2, 0);
    lvgl_sys::lv_obj_set_style_border_width(color_sq, 0, 0);
    set_style_pad_all(color_sq, 0);
    lvgl_sys::lv_obj_clear_flag(color_sq, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    if color == 0 {
        lvgl_sys::lv_obj_set_style_bg_color(color_sq, lv_color_hex(0x505050), 0);
    } else {
        lvgl_sys::lv_obj_set_style_bg_color(color_sq, lv_color_hex(color), 0);
    }
}

/// Create an action button with specific icon type (standalone with card border)
unsafe fn create_action_button(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, w: i16, h: i16, title: &str, _subtitle: &str, icon_type: &str) {
    let btn = create_card(parent, x, y, w, h);
    create_action_button_content(btn, title, icon_type);
}

/// Common content for action buttons
unsafe fn create_action_button_content(btn: *mut lvgl_sys::lv_obj_t, title: &str, icon_type: &str) {
    // Icon container (transparent, for positioning) - centered vertically with offset for title
    let icon_container = lvgl_sys::lv_obj_create(btn);
    lvgl_sys::lv_obj_set_size(icon_container, 50, 50);
    lvgl_sys::lv_obj_align(icon_container, lvgl_sys::LV_ALIGN_CENTER as u8, 0, -15);
    lvgl_sys::lv_obj_set_style_bg_opa(icon_container, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon_container, 0, 0);
    set_style_pad_all(icon_container, 0);
    lvgl_sys::lv_obj_clear_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    match icon_type {
        "ams" => draw_ams_icon(icon_container),
        "encode" => draw_encode_icon(icon_container),
        "catalog" => draw_catalog_icon(icon_container),
        "settings" => draw_settings_icon(icon_container),
        _ => {}
    }

    // Title - positioned below center
    let title_label = lvgl_sys::lv_label_create(btn);
    let title_cstr = cstr_core::CString::new(title).unwrap();
    lvgl_sys::lv_label_set_text(title_label, title_cstr.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(title_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_align(title_label, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 35);
}

/// Draw AMS Setup icon (table/grid with rows, black background)
unsafe fn draw_ams_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 50, 50);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 10, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    lvgl_sys::lv_obj_clear_flag(bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Outer frame
    let frame = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(frame, 36, 36);
    lvgl_sys::lv_obj_align(frame, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(frame, 0, 0);
    lvgl_sys::lv_obj_set_style_border_color(frame, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(frame, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(frame, 4, 0);
    set_style_pad_all(frame, 0);
    lvgl_sys::lv_obj_clear_flag(frame, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Horizontal lines (3 rows)
    for i in 0..3 {
        let line = lvgl_sys::lv_obj_create(frame);
        lvgl_sys::lv_obj_set_size(line, 24, 2);
        lvgl_sys::lv_obj_set_pos(line, 4, 6 + i * 9);
        lvgl_sys::lv_obj_set_style_bg_color(line, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_border_width(line, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(line, 1, 0);
    }
}

/// Draw Encode Tag icon (PNG with black background)
unsafe fn draw_encode_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 50, 50);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 10, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    lvgl_sys::lv_obj_clear_flag(bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Initialize encode image descriptor
    ENCODE_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
        0, 0,
        ENCODE_WIDTH,
        ENCODE_HEIGHT,
    );
    ENCODE_IMG_DSC.data_size = (ENCODE_WIDTH * ENCODE_HEIGHT * 3) as u32;
    ENCODE_IMG_DSC.data = ENCODE_DATA.as_ptr();

    let icon = lvgl_sys::lv_img_create(bg);
    lvgl_sys::lv_img_set_src(icon, &raw const ENCODE_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(icon, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_img_recolor(icon, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(icon, 255, 0);
}

/// Draw Catalog icon (grid of squares, black background)
unsafe fn draw_catalog_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 50, 50);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 10, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    lvgl_sys::lv_obj_clear_flag(bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // 3x3 grid of small squares
    let size: i16 = 10;
    let gap: i16 = 3;
    let start_x: i16 = 7;
    let start_y: i16 = 7;

    for row in 0..3 {
        for col in 0..3 {
            let square = lvgl_sys::lv_obj_create(bg);
            lvgl_sys::lv_obj_set_size(square, size, size);
            lvgl_sys::lv_obj_set_pos(square, start_x + col * (size + gap), start_y + row * (size + gap));
            lvgl_sys::lv_obj_set_style_bg_color(square, lv_color_hex(COLOR_ACCENT), 0);
            lvgl_sys::lv_obj_set_style_border_width(square, 0, 0);
            lvgl_sys::lv_obj_set_style_radius(square, 2, 0);
        }
    }
}

/// Draw Settings icon (PNG with black background)
unsafe fn draw_settings_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 50, 50);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 10, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    lvgl_sys::lv_obj_clear_flag(bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Initialize setting image descriptor
    SETTING_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
        0, 0,
        SETTING_WIDTH,
        SETTING_HEIGHT,
    );
    SETTING_IMG_DSC.data_size = (SETTING_WIDTH * SETTING_HEIGHT * 3) as u32;
    SETTING_IMG_DSC.data = SETTING_DATA.as_ptr();

    let icon = lvgl_sys::lv_img_create(bg);
    lvgl_sys::lv_img_set_src(icon, &raw const SETTING_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(icon, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_img_recolor(icon, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(icon, 255, 0);
}

// Global pointer to panel framebuffer for LVGL flush callback
static mut PANEL_FB_PTR: *mut u16 = ptr::null_mut();
static mut FLUSH_COUNT: u32 = 0;

// Touch state for LVGL input device
static mut TOUCH_X: i16 = 0;
static mut TOUCH_Y: i16 = 0;
static mut TOUCH_PRESSED: bool = false;

// Scale state
static mut SCALE_WEIGHT: f32 = 0.0;
static mut SCALE_STABLE: bool = false;
static mut SCALE_INITIALIZED: bool = false;

// Screenshot state for development
static mut SCREENSHOT_PENDING: bool = false;
static mut SCREENSHOT_TAP_COUNT: u8 = 0;
static mut SCREENSHOT_LAST_TAP: u32 = 0;

// UI elements that need updating
static mut LBL_SCALE_WEIGHT: *mut lvgl_sys::lv_obj_t = ptr::null_mut();

/// LVGL flush callback - copies rendered pixels from draw buffer to panel framebuffer
/// Direct copy - pin mapping matches RGB565 format exactly
unsafe extern "C" fn flush_cb(
    disp_drv: *mut lvgl_sys::lv_disp_drv_t,
    area: *const lvgl_sys::lv_area_t,
    color_p: *mut lvgl_sys::lv_color_t,
) {
    FLUSH_COUNT += 1;

    let x1 = (*area).x1 as usize;
    let y1 = (*area).y1 as usize;
    let x2 = (*area).x2 as usize;
    let y2 = (*area).y2 as usize;

    if FLUSH_COUNT <= 10 {
        info!("flush_cb #{}: ({},{}) to ({},{})", FLUSH_COUNT, x1, y1, x2, y2);
    }

    if !PANEL_FB_PTR.is_null() && !color_p.is_null() {
        let width = x2 - x1 + 1;
        let height = y2 - y1 + 1;

        // Direct row-by-row copy - RGB565 format matches panel pin mapping
        for row in 0..height {
            let src_offset = row * width;
            let dst_offset = (y1 + row) * WIDTH + x1;
            core::ptr::copy_nonoverlapping(
                color_p.add(src_offset) as *const u16,
                PANEL_FB_PTR.add(dst_offset),
                width
            );
        }
    }

    // CRITICAL: Signal to LVGL that flushing is done
    lvgl_sys::lv_disp_flush_ready(disp_drv);
}

/// LVGL touch input read callback
unsafe extern "C" fn touch_read_cb(
    _indev_drv: *mut lvgl_sys::lv_indev_drv_t,
    data: *mut lvgl_sys::lv_indev_data_t,
) {
    if TOUCH_PRESSED {
        (*data).state = lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED;
        (*data).point.x = TOUCH_X;
        (*data).point.y = TOUCH_Y;
    } else {
        (*data).state = lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED;
    }
}

/// Dump framebuffer as hex over serial for screenshot capture
/// Format: SCREENSHOT_BEGIN\n then hex lines, then SCREENSHOT_END\n
/// Triple-tap top-left corner (0-100, 0-100) to trigger
unsafe fn dump_screenshot() {
    if PANEL_FB_PTR.is_null() {
        info!("SCREENSHOT_ERROR: No framebuffer");
        return;
    }

    info!("SCREENSHOT_BEGIN:{}x{}:RGB565", WIDTH, HEIGHT);

    // Dump framebuffer as hex, one row at a time
    // Each row is 800 pixels * 2 bytes = 1600 bytes = 3200 hex chars
    for y in 0..HEIGHT {
        let row_start = y * WIDTH;
        let mut row_hex = String::new();

        for x in 0..WIDTH {
            let pixel = *PANEL_FB_PTR.add(row_start + x);
            // Output as 4 hex digits (big endian for easier parsing)
            use std::fmt::Write;
            let _ = write!(row_hex, "{:04X}", pixel);
        }

        // Print row with line number prefix for error detection
        info!("R{:03}:{}", y, row_hex);
    }

    info!("SCREENSHOT_END");
}

/// Check if touch is in screenshot trigger zone (top-left 100x100)
fn is_screenshot_zone(x: i16, y: i16) -> bool {
    x < 100 && y < 100
}

/// Current screen identifier
static mut CURRENT_SCREEN: u8 = 0; // 0=Home, 1=AMS, 2=Encode, 3=Catalog, 4=Settings

/// Global screen pointers for navigation
static mut SCREEN_HOME: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_AMS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_ENCODE: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_CATALOG: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_SETTINGS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();

/// Button event callbacks
unsafe extern "C" fn btn_ams_setup_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("AMS Setup button pressed!");
    CURRENT_SCREEN = 1;
    if !SCREEN_AMS.is_null() {
        lvgl_sys::lv_disp_load_scr(SCREEN_AMS);
    }
}

unsafe extern "C" fn btn_encode_tag_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Encode Tag button pressed!");
    CURRENT_SCREEN = 2;
    if !SCREEN_ENCODE.is_null() {
        lvgl_sys::lv_disp_load_scr(SCREEN_ENCODE);
    }
}

unsafe extern "C" fn btn_catalog_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Catalog button pressed!");
    CURRENT_SCREEN = 3;
    if !SCREEN_CATALOG.is_null() {
        lvgl_sys::lv_disp_load_scr(SCREEN_CATALOG);
    }
}

unsafe extern "C" fn btn_settings_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Settings button pressed!");
    CURRENT_SCREEN = 4;
    if !SCREEN_SETTINGS.is_null() {
        lvgl_sys::lv_disp_load_scr(SCREEN_SETTINGS);
    }
}

/// Back button callback - return to home screen
unsafe extern "C" fn btn_back_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Back button pressed!");
    CURRENT_SCREEN = 0;
    if !SCREEN_HOME.is_null() {
        lvgl_sys::lv_disp_load_scr(SCREEN_HOME);
    }
}

/// Direct framebuffer wrapper for panel's own memory
struct PanelFramebuffer;

impl DrawTarget for PanelFramebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x >= 0 && point.x < WIDTH as i32 && point.y >= 0 && point.y < HEIGHT as i32 {
                let idx = (point.y as usize) * WIDTH + (point.x as usize);
                unsafe {
                    if !PANEL_FB_PTR.is_null() {
                        *PANEL_FB_PTR.add(idx) = color.into_storage();
                    }
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for PanelFramebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

/// Initialize the RGB LCD panel with bounce buffer support
unsafe fn init_rgb_panel() -> Result<sys::esp_lcd_panel_handle_t, sys::esp_err_t> {
    info!("Initializing RGB LCD panel with bounce buffers...");

    let mut panel_config: sys::esp_lcd_rgb_panel_config_t = std::mem::zeroed();

    panel_config.data_width = 16;
    panel_config.clk_src = sys::soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT;
    panel_config.num_fbs = 1;
    panel_config.bounce_buffer_size_px = BOUNCE_BUFFER_SIZE_PX;

    panel_config.pclk_gpio_num = PIN_PCLK;
    panel_config.hsync_gpio_num = PIN_HSYNC;
    panel_config.vsync_gpio_num = PIN_VSYNC;
    panel_config.de_gpio_num = PIN_DE;
    panel_config.disp_gpio_num = -1;

    panel_config.data_gpio_nums[0] = PIN_B0;
    panel_config.data_gpio_nums[1] = PIN_B1;
    panel_config.data_gpio_nums[2] = PIN_B2;
    panel_config.data_gpio_nums[3] = PIN_B3;
    panel_config.data_gpio_nums[4] = PIN_B4;
    panel_config.data_gpio_nums[5] = PIN_G0;
    panel_config.data_gpio_nums[6] = PIN_G1;
    panel_config.data_gpio_nums[7] = PIN_G2;
    panel_config.data_gpio_nums[8] = PIN_G3;
    panel_config.data_gpio_nums[9] = PIN_G4;
    panel_config.data_gpio_nums[10] = PIN_G5;
    panel_config.data_gpio_nums[11] = PIN_R0;
    panel_config.data_gpio_nums[12] = PIN_R1;
    panel_config.data_gpio_nums[13] = PIN_R2;
    panel_config.data_gpio_nums[14] = PIN_R3;
    panel_config.data_gpio_nums[15] = PIN_R4;

    // 14MHz is the minimum stable pixel clock for this panel
    panel_config.timings.pclk_hz = 14_000_000;
    panel_config.timings.h_res = WIDTH as u32;
    panel_config.timings.v_res = HEIGHT as u32;
    // Timing values for CrowPanel 7" 800x480
    panel_config.timings.hsync_back_porch = 20;   // Reduced further - was shifting image right
    panel_config.timings.hsync_front_porch = 40;
    panel_config.timings.hsync_pulse_width = 48;
    panel_config.timings.vsync_back_porch = 20;
    panel_config.timings.vsync_front_porch = 20;
    panel_config.timings.vsync_pulse_width = 4;

    // Try sampling on negative edge - may fix color channel alignment
    panel_config.timings.flags.set_pclk_active_neg(1);  // Sample on falling edge
    panel_config.timings.flags.set_de_idle_high(0);
    panel_config.timings.flags.set_pclk_idle_high(0);

    panel_config.flags.set_fb_in_psram(1);
    panel_config.flags.set_refresh_on_demand(0);
    panel_config.flags.set_no_fb(0);  // We're providing our own FB
    panel_config.flags.set_bb_invalidate_cache(1);  // Invalidate cache for bounce buffer

    // DMA alignment for PSRAM transfers (64-byte cache line)
    panel_config.psram_trans_align = 64;
    panel_config.sram_trans_align = 4;

    let mut panel_handle: sys::esp_lcd_panel_handle_t = ptr::null_mut();
    let err = sys::esp_lcd_new_rgb_panel(&panel_config, &mut panel_handle);

    if err != sys::ESP_OK {
        info!("esp_lcd_new_rgb_panel failed: {}", err);
        return Err(err);
    }

    let err = sys::esp_lcd_panel_reset(panel_handle);
    if err != sys::ESP_OK {
        info!("esp_lcd_panel_reset failed: {}", err);
        return Err(err);
    }

    let err = sys::esp_lcd_panel_init(panel_handle);
    if err != sys::ESP_OK {
        info!("esp_lcd_panel_init failed: {}", err);
        return Err(err);
    }

    // Turn on the display
    let err = sys::esp_lcd_panel_disp_on_off(panel_handle, true);
    if err != sys::ESP_OK {
        info!("esp_lcd_panel_disp_on_off failed: {}", err);
        // Continue anyway, not all panels need this
    }

    info!("RGB LCD panel initialized successfully!");
    info!("  Resolution: {}x{}", WIDTH, HEIGHT);
    info!("  Bounce buffer: {} lines", BOUNCE_BUFFER_LINES);

    Ok(panel_handle)
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("========================================");
    info!("SpoolBuddy Firmware v0.1");
    info!("CrowPanel Advance 7.0\" (800x480)");
    info!("========================================");

    let peripherals = Peripherals::take().unwrap();

    // Save modem for WiFi initialization later (after LCD to preserve internal SRAM)
    let modem = peripherals.modem;

    // Backlight control - GPIO1 is the primary backlight enable
    info!("[1/4] Enabling backlight GPIO...");
    let mut backlight = PinDriver::output(peripherals.pins.gpio1).unwrap();
    backlight.set_high().unwrap();
    info!("  GPIO1 -> HIGH");

    // Some panels also use GPIO2
    let mut backlight2 = PinDriver::output(peripherals.pins.gpio2).unwrap();
    backlight2.set_high().unwrap();
    info!("  GPIO2 -> HIGH");

    FreeRtos::delay_ms(200);

    // Note: WiFi initialization moved to after LCD panel setup to preserve
    // internal SRAM for bounce buffers

    // I2C bus scan and backlight control
    info!("[2/4] Setting up I2C and scanning bus...");
    let i2c_config = I2cConfig::new().baudrate(Hertz(100_000));
    let i2c_result = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio15,  // SDA
        peripherals.pins.gpio16,  // SCL
        &i2c_config,
    );

    // Keep I2C driver for touch input and scale
    let mut i2c_driver: Option<I2cDriver<'_>> = None;
    let mut gt911_addr: u8 = 0x5D;  // Default GT911 address
    let mut scale_state = scale::Nau7802State::new();
    let mut nau7802_found = false;

    match i2c_result {
        Ok(mut i2c) => {
            info!("  I2C initialized on GPIO15(SDA)/GPIO16(SCL)");

            // Scan I2C bus to find devices
            info!("  Scanning I2C bus...");
            for addr in 0x08..0x78 {
                let mut buf = [0u8; 1];
                if i2c.read(addr, &mut buf, 10).is_ok() {
                    info!("    Found device at 0x{:02X}", addr);
                    // Check for GT911 touch controller
                    if addr == 0x5D || addr == 0x14 {
                        gt911_addr = addr;
                        info!("    -> GT911 touch controller detected!");
                    }
                    // Check for NAU7802 scale (SparkFun Qwiic Scale)
                    if addr == scale::NAU7802_ADDR {
                        nau7802_found = true;
                        info!("    -> NAU7802 scale detected!");
                    }
                }
            }

            // Initialize NAU7802 scale if found
            if nau7802_found {
                info!("  Initializing NAU7802 scale...");
                match scale::nau7802::init(&mut i2c, &mut scale_state) {
                    Ok(()) => {
                        unsafe { SCALE_INITIALIZED = true; }
                        info!("  NAU7802 scale initialized successfully");

                        // Auto-tare on startup (wait for stable readings first)
                        info!("  Auto-taring scale (ensure platform is empty)...");
                        FreeRtos::delay_ms(500); // Wait for readings to stabilize
                        match scale::nau7802::tare(&mut i2c, &mut scale_state) {
                            Ok(()) => {
                                info!("  Scale tared successfully, zero offset: {}",
                                      scale_state.calibration.zero_offset);
                            }
                            Err(e) => {
                                info!("  Tare failed: {:?} (continuing with default offset)", e);
                            }
                        }
                    }
                    Err(e) => {
                        info!("  NAU7802 init failed: {:?}", e);
                    }
                }
            }

            // Try multiple backlight approaches
            // Approach 1: Direct brightness at 0x30
            info!("  Trying backlight at 0x30...");
            let _ = i2c.write(0x30, &[0xFF], 100);

            // Approach 2: XL9535 GPIO expander at 0x20 (common in CrowPanel)
            info!("  Trying XL9535 at 0x20...");
            // XL9535: Output port 0 register is 0x02, config register is 0x06
            let _ = i2c.write(0x20, &[0x06, 0x00], 100); // Set port 0 as output
            let _ = i2c.write(0x20, &[0x02, 0xFF], 100); // Set all outputs high

            // Approach 3: Try 0x24 (another common address)
            info!("  Trying 0x24...");
            let _ = i2c.write(0x24, &[0xFF], 100);

            // Keep driver for touch and scale
            i2c_driver = Some(i2c);
        }
        Err(e) => {
            info!("  I2C init failed: {:?}", e);
        }
    }

    FreeRtos::delay_ms(100);

    // Initialize RGB LCD Panel
    info!("[3/4] Initializing RGB LCD panel...");
    let panel_handle = unsafe { init_rgb_panel() };

    match panel_handle {
        Ok(panel) => {
            info!("[4/4] Getting panel framebuffer...");

            // Get the panel's own framebuffer
            let mut panel_fb: *mut core::ffi::c_void = ptr::null_mut();
            unsafe {
                let err = sys::esp_lcd_rgb_panel_get_frame_buffer(panel, 1, &mut panel_fb);
                if err != sys::ESP_OK || panel_fb.is_null() {
                    info!("FATAL: Could not get panel framebuffer (err={})", err);
                    loop { FreeRtos::delay_ms(1000); }
                }
                info!("  Got panel framebuffer at 0x{:08X}", panel_fb as usize);
                PANEL_FB_PTR = panel_fb as *mut u16;

            }

            // Initialize LVGL using raw API for proper flush control
            info!("[5/6] Initializing LVGL (raw API)...");

            unsafe {
                lvgl_sys::lv_init();
                info!("  lv_init() done");
            }

            // Use intermediate draw buffer approach for maximum compatibility
            // The buffer is in internal SRAM, avoiding PSRAM cache coherence issues
            static mut DRAW_BUF: [lvgl_sys::lv_color_t; 800 * 24] = [lvgl_sys::lv_color_t { full: 0 }; 800 * 24];
            static mut DISP_DRAW_BUF: core::mem::MaybeUninit<lvgl_sys::lv_disp_draw_buf_t> = core::mem::MaybeUninit::uninit();
            static mut DISP_DRV: core::mem::MaybeUninit<lvgl_sys::lv_disp_drv_t> = core::mem::MaybeUninit::uninit();

            unsafe {
                // Initialize draw buffer with internal SRAM buffer
                // 800 * 24 rows = 19200 pixels * 2 bytes = 38400 bytes (fits in SRAM)
                lvgl_sys::lv_disp_draw_buf_init(
                    DISP_DRAW_BUF.as_mut_ptr(),
                    DRAW_BUF.as_mut_ptr() as *mut core::ffi::c_void,
                    core::ptr::null_mut(),
                    (800 * 24) as u32,
                );
                info!("  Draw buffer initialized (800x24 internal SRAM)");

                // Initialize display driver
                lvgl_sys::lv_disp_drv_init(DISP_DRV.as_mut_ptr());
                let drv = DISP_DRV.as_mut_ptr();
                (*drv).draw_buf = DISP_DRAW_BUF.as_mut_ptr();
                (*drv).hor_res = WIDTH as i16;
                (*drv).ver_res = HEIGHT as i16;
                (*drv).flush_cb = Some(flush_cb);
                // Don't use full_refresh - requires screen-sized buffer which we don't have

                // Register display
                let disp = lvgl_sys::lv_disp_drv_register(drv);
                if disp.is_null() {
                    info!("FATAL: Failed to register display");
                    loop { FreeRtos::delay_ms(1000); }
                }
                info!("  Display driver registered (full_refresh=1)");

                // Register touch input device
                static mut INDEV_DRV: core::mem::MaybeUninit<lvgl_sys::lv_indev_drv_t> = core::mem::MaybeUninit::uninit();
                lvgl_sys::lv_indev_drv_init(INDEV_DRV.as_mut_ptr());
                let indev_drv = INDEV_DRV.as_mut_ptr();
                (*indev_drv).type_ = lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER;
                (*indev_drv).read_cb = Some(touch_read_cb);
                let indev = lvgl_sys::lv_indev_drv_register(indev_drv);
                if indev.is_null() {
                    info!("  Warning: Failed to register input device");
                } else {
                    info!("  Touch input device registered");
                }

                // Create simple UI
                info!("[6/6] Creating UI...");

                // Helper to create color from RGB using LVGL's proper bitfield interface
                let make_color = |r: u8, g: u8, b: u8| -> lvgl_sys::lv_color_t {
                    let mut color: lvgl_sys::lv_color_t = std::mem::zeroed();
                    color.ch._bitfield_1 = lvgl_sys::lv_color16_t__bindgen_ty_1::new_bitfield_1(
                        (b >> 3) as u16,  // blue: 5 bits
                        (g >> 2) as u16,  // green: 6 bits
                        (r >> 3) as u16,  // red: 5 bits
                    );
                    color
                };

                // Create Home Screen - Polished UI matching simulator
                let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
                SCREEN_HOME = scr;

                // Background
                lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(scr, 255, 0);
                set_style_pad_all(scr, 0);

                // === STATUS BAR (44px) ===
                let status_bar = lvgl_sys::lv_obj_create(scr);
                lvgl_sys::lv_obj_set_size(status_bar, 800, 44);
                lvgl_sys::lv_obj_set_pos(status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(status_bar, lv_color_hex(COLOR_STATUS_BAR), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(status_bar, 255, 0);
                lvgl_sys::lv_obj_set_style_border_width(status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_radius(status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_pad_left(status_bar, 16, 0);
                lvgl_sys::lv_obj_set_style_pad_right(status_bar, 16, 0);
                lvgl_sys::lv_obj_clear_flag(status_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                // SpoolBuddy logo image
                LOGO_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
                    lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
                    0, 0,
                    LOGO_WIDTH,
                    LOGO_HEIGHT,
                );
                LOGO_IMG_DSC.data_size = (LOGO_WIDTH * LOGO_HEIGHT * 3) as u32;
                LOGO_IMG_DSC.data = LOGO_DATA.as_ptr();

                let logo_img = lvgl_sys::lv_img_create(status_bar);
                lvgl_sys::lv_img_set_src(logo_img, &raw const LOGO_IMG_DSC as *const _);
                lvgl_sys::lv_obj_align(logo_img, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 0, 0);

                // Printer selector (center) - solid color (no gradient to avoid banding)
                let printer_btn = lvgl_sys::lv_btn_create(status_bar);
                lvgl_sys::lv_obj_set_size(printer_btn, 180, 32);
                lvgl_sys::lv_obj_align(printer_btn, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(printer_btn, lv_color_hex(0x242424), 0);
                lvgl_sys::lv_obj_set_style_radius(printer_btn, 16, 0);
                lvgl_sys::lv_obj_set_style_border_color(printer_btn, lv_color_hex(0x3D3D3D), 0);
                lvgl_sys::lv_obj_set_style_border_width(printer_btn, 1, 0);
                lvgl_sys::lv_obj_clear_flag(printer_btn, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                // Left status dot (green = connected) with subtle glow
                let left_dot = lvgl_sys::lv_obj_create(printer_btn);
                lvgl_sys::lv_obj_set_size(left_dot, 8, 8);
                lvgl_sys::lv_obj_align(left_dot, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 12, 0);
                lvgl_sys::lv_obj_set_style_bg_color(left_dot, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_radius(left_dot, 4, 0);
                lvgl_sys::lv_obj_set_style_border_width(left_dot, 0, 0);
                lvgl_sys::lv_obj_set_style_shadow_color(left_dot, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(left_dot, 6, 0);
                lvgl_sys::lv_obj_set_style_shadow_spread(left_dot, 2, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(left_dot, 150, 0);

                let printer_label = lvgl_sys::lv_label_create(printer_btn);
                let printer_text = CString::new("X1C-Studio").unwrap();
                lvgl_sys::lv_label_set_text(printer_label, printer_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(printer_label, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(printer_label, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // Right status icon (power button, orange = printing)
                POWER_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
                    lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
                    0, 0,
                    POWER_WIDTH,
                    POWER_HEIGHT,
                );
                POWER_IMG_DSC.data_size = (POWER_WIDTH * POWER_HEIGHT * 3) as u32;
                POWER_IMG_DSC.data = POWER_DATA.as_ptr();

                let power_img = lvgl_sys::lv_img_create(printer_btn);
                lvgl_sys::lv_img_set_src(power_img, &raw const POWER_IMG_DSC as *const _);
                lvgl_sys::lv_obj_align(power_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -8, 0);
                lvgl_sys::lv_obj_set_style_img_recolor(power_img, lv_color_hex(0xFFA500), 0);
                lvgl_sys::lv_obj_set_style_img_recolor_opa(power_img, 255, 0);

                // Time (rightmost)
                let time_label = lvgl_sys::lv_label_create(status_bar);
                let time_text = CString::new("14:23").unwrap();
                lvgl_sys::lv_label_set_text(time_label, time_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(time_label, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(time_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, 0, 0);

                // WiFi icon - 3 bars, bottom-aligned
                let wifi_x = -50;
                let wifi_bottom = 8;
                let wifi_bar3 = lvgl_sys::lv_obj_create(status_bar);
                lvgl_sys::lv_obj_set_size(wifi_bar3, 4, 16);
                lvgl_sys::lv_obj_align(wifi_bar3, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, wifi_x, wifi_bottom - 8);
                lvgl_sys::lv_obj_set_style_bg_color(wifi_bar3, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(wifi_bar3, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(wifi_bar3, 1, 0);
                lvgl_sys::lv_obj_set_style_border_width(wifi_bar3, 0, 0);

                let wifi_bar2 = lvgl_sys::lv_obj_create(status_bar);
                lvgl_sys::lv_obj_set_size(wifi_bar2, 4, 12);
                lvgl_sys::lv_obj_align(wifi_bar2, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, wifi_x - 6, wifi_bottom - 6);
                lvgl_sys::lv_obj_set_style_bg_color(wifi_bar2, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(wifi_bar2, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(wifi_bar2, 1, 0);
                lvgl_sys::lv_obj_set_style_border_width(wifi_bar2, 0, 0);

                let wifi_bar1 = lvgl_sys::lv_obj_create(status_bar);
                lvgl_sys::lv_obj_set_size(wifi_bar1, 4, 8);
                lvgl_sys::lv_obj_align(wifi_bar1, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, wifi_x - 12, wifi_bottom - 4);
                lvgl_sys::lv_obj_set_style_bg_color(wifi_bar1, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(wifi_bar1, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(wifi_bar1, 1, 0);
                lvgl_sys::lv_obj_set_style_border_width(wifi_bar1, 0, 0);

                // Bell icon
                BELL_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
                    lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
                    0, 0,
                    BELL_WIDTH,
                    BELL_HEIGHT,
                );
                BELL_IMG_DSC.data_size = (BELL_WIDTH * BELL_HEIGHT * 3) as u32;
                BELL_IMG_DSC.data = BELL_DATA.as_ptr();

                let bell_img = lvgl_sys::lv_img_create(status_bar);
                lvgl_sys::lv_img_set_src(bell_img, &raw const BELL_IMG_DSC as *const _);
                lvgl_sys::lv_obj_align(bell_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -82, 0);

                // Notification badge
                let badge = lvgl_sys::lv_obj_create(status_bar);
                lvgl_sys::lv_obj_set_size(badge, 14, 14);
                lvgl_sys::lv_obj_align(badge, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -70, -8);
                lvgl_sys::lv_obj_set_style_bg_color(badge, lv_color_hex(0xFF4444), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(badge, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(badge, 7, 0);
                lvgl_sys::lv_obj_set_style_border_width(badge, 0, 0);
                lvgl_sys::lv_obj_clear_flag(badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                let badge_text = lvgl_sys::lv_label_create(badge);
                let badge_str = CString::new("3").unwrap();
                lvgl_sys::lv_label_set_text(badge_text, badge_str.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(badge_text, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(badge_text, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // === STATUS BAR SEPARATOR ===
                let separator = lvgl_sys::lv_obj_create(scr);
                lvgl_sys::lv_obj_set_size(separator, 800, 1);
                lvgl_sys::lv_obj_set_pos(separator, 0, 44);
                lvgl_sys::lv_obj_set_style_bg_color(separator, lv_color_hex(COLOR_BORDER), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(separator, 255, 0);
                lvgl_sys::lv_obj_set_style_border_width(separator, 0, 0);
                lvgl_sys::lv_obj_set_style_radius(separator, 0, 0);

                // === MAIN CONTENT AREA ===
                let content_y: i16 = 52;
                let content_height: i16 = 280;
                let card_gap: i16 = 8;

                // Button dimensions (defined first so we can calculate left card width)
                let btn_width: i16 = 130;
                let btn_gap: i16 = 8;
                let btn_start_x: i16 = 800 - 16 - btn_width - btn_gap - btn_width;

                // Left column - Printer Card (expanded)
                let left_card_width = btn_start_x - 16 - card_gap;
                let printer_card = create_card(scr, 16, content_y, left_card_width, 130);

                // Print cover image placeholder (left side)
                let cover_size: i16 = 70;
                let cover_img = lvgl_sys::lv_obj_create(printer_card);
                lvgl_sys::lv_obj_set_size(cover_img, cover_size, cover_size);
                lvgl_sys::lv_obj_set_pos(cover_img, 12, 12);
                lvgl_sys::lv_obj_set_style_bg_color(cover_img, lv_color_hex(0x404040), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(cover_img, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(cover_img, 8, 0);
                lvgl_sys::lv_obj_set_style_border_width(cover_img, 0, 0);
                lvgl_sys::lv_obj_clear_flag(cover_img, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
                let cube_label = lvgl_sys::lv_label_create(cover_img);
                let cube_text = CString::new("3D").unwrap();
                lvgl_sys::lv_label_set_text(cube_label, cube_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(cube_label, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_align(cube_label, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // Printer name (right of image)
                let text_x: i16 = 12 + cover_size + 12;
                let printer_name = lvgl_sys::lv_label_create(printer_card);
                let name_text = CString::new("X1C-Studio").unwrap();
                lvgl_sys::lv_label_set_text(printer_name, name_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(printer_name, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_set_pos(printer_name, text_x, 16);

                // Status (below printer name, green)
                let status_label = lvgl_sys::lv_label_create(printer_card);
                let status_text = CString::new("Printing").unwrap();
                lvgl_sys::lv_label_set_text(status_label, status_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(status_label, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_pos(status_label, text_x, 38);

                // Filename and time (above progress bar)
                let file_label = lvgl_sys::lv_label_create(printer_card);
                let file_text = CString::new("Benchy.3mf").unwrap();
                lvgl_sys::lv_label_set_text(file_label, file_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(file_label, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_set_pos(file_label, 12, 88);

                let time_left = lvgl_sys::lv_label_create(printer_card);
                let time_left_text = CString::new("1h 23m left").unwrap();
                lvgl_sys::lv_label_set_text(time_left, time_left_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(time_left, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_align(time_left, lvgl_sys::LV_ALIGN_TOP_RIGHT as u8, -12, 88);

                // Progress bar (full width at bottom) - vibrant gradient with glow
                let progress_width = left_card_width - 24;
                let progress_percent: f32 = 0.6;
                let fill_width = (progress_width as f32 * progress_percent) as i16;

                // Background track with inner shadow effect
                let progress_bg = lvgl_sys::lv_obj_create(printer_card);
                lvgl_sys::lv_obj_set_size(progress_bg, progress_width, 16);
                lvgl_sys::lv_obj_set_pos(progress_bg, 12, 104);
                lvgl_sys::lv_obj_set_style_bg_color(progress_bg, lv_color_hex(0x0A0A0A), 0);
                lvgl_sys::lv_obj_set_style_radius(progress_bg, 8, 0);
                lvgl_sys::lv_obj_set_style_border_color(progress_bg, lv_color_hex(0x2A2A2A), 0);
                lvgl_sys::lv_obj_set_style_border_width(progress_bg, 1, 0);
                set_style_pad_all(progress_bg, 0);

                // Solid fill with subtle glow (no gradient to avoid banding)
                let progress_fill = lvgl_sys::lv_obj_create(progress_bg);
                lvgl_sys::lv_obj_set_size(progress_fill, fill_width, 14);
                lvgl_sys::lv_obj_set_pos(progress_fill, 1, 1);
                lvgl_sys::lv_obj_set_style_bg_color(progress_fill, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_radius(progress_fill, 7, 0);
                lvgl_sys::lv_obj_set_style_border_width(progress_fill, 0, 0);
                // Subtle glow - just enough to make it pop
                lvgl_sys::lv_obj_set_style_shadow_color(progress_fill, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(progress_fill, 8, 0);
                lvgl_sys::lv_obj_set_style_shadow_spread(progress_fill, 0, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(progress_fill, 80, 0);
                set_style_pad_all(progress_fill, 0);

                // Left column - NFC/Weight scan area (expanded)
                let scan_card = create_card(scr, 16, content_y + 138, left_card_width, 125);

                // === LEFT SIDE: NFC Icon (64x64) ===
                NFC_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
                    lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
                    0, 0,
                    NFC_WIDTH,
                    NFC_HEIGHT,
                );
                NFC_IMG_DSC.data_size = (NFC_WIDTH * NFC_HEIGHT * 3) as u32;
                NFC_IMG_DSC.data = NFC_DATA.as_ptr();

                let nfc_img = lvgl_sys::lv_img_create(scan_card);
                lvgl_sys::lv_img_set_src(nfc_img, &raw const NFC_IMG_DSC as *const _);
                lvgl_sys::lv_obj_set_pos(nfc_img, 16, 10);
                // Green tint when ready - professional accent color
                lvgl_sys::lv_obj_set_style_img_recolor(nfc_img, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_img_recolor_opa(nfc_img, 255, 0);

                // "Ready" text below NFC icon
                let nfc_status = lvgl_sys::lv_label_create(scan_card);
                lvgl_sys::lv_label_set_text(nfc_status, b"Ready\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(nfc_status, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_pos(nfc_status, 32, 90);

                // === CENTER: Instruction text ===
                let scan_hint = lvgl_sys::lv_label_create(scan_card);
                lvgl_sys::lv_label_set_text(scan_hint, b"Place spool on scale\nto scan & weigh\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(scan_hint, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_set_style_text_align(scan_hint, lvgl_sys::LV_TEXT_ALIGN_CENTER as u8, 0);
                lvgl_sys::lv_obj_align(scan_hint, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // === RIGHT SIDE: Weight display (icon + value + fill bar) ===
                // Current weight value
                let current_weight: f32 = 0.85;  // kg
                let max_weight: f32 = 1.0;  // kg (full spool)
                let fill_percent = ((current_weight / max_weight) * 100.0).min(100.0) as i16;

                // Weight icon (64x64, white - no tint)
                WEIGHT_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
                    lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
                    0, 0,
                    WEIGHT_WIDTH,
                    WEIGHT_HEIGHT,
                );
                WEIGHT_IMG_DSC.data_size = (WEIGHT_WIDTH * WEIGHT_HEIGHT * 3) as u32;
                WEIGHT_IMG_DSC.data = WEIGHT_DATA.as_ptr();

                let weight_img = lvgl_sys::lv_img_create(scan_card);
                lvgl_sys::lv_img_set_src(weight_img, &raw const WEIGHT_IMG_DSC as *const _);
                lvgl_sys::lv_obj_set_pos(weight_img, left_card_width - 84, 8);  // Adjusted for smaller card
                // Apply gray-white tint to match NFC icon
                lvgl_sys::lv_obj_set_style_img_recolor(weight_img, lv_color_hex(0xBBBBBB), 0);
                lvgl_sys::lv_obj_set_style_img_recolor_opa(weight_img, 200, 0);

                // Weight value below icon (green)
                let weight_value = lvgl_sys::lv_label_create(scan_card);
                lvgl_sys::lv_label_set_text(weight_value, b"0.85 kg\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(weight_value, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_pos(weight_value, left_card_width - 80, 72);  // Adjusted for smaller card
                LBL_SCALE_WEIGHT = weight_value;

                // Horizontal fill bar below value - clean gradient with subtle glow
                let bar_width: i16 = 70;
                let bar_height: i16 = 14;
                let bar_x = left_card_width - 87;
                let bar_y: i16 = 95;
                let scale_fill_width = ((bar_width as f32) * (fill_percent as f32 / 100.0)) as i16;

                // Bar background
                let bar_bg = lvgl_sys::lv_obj_create(scan_card);
                lvgl_sys::lv_obj_set_size(bar_bg, bar_width, bar_height);
                lvgl_sys::lv_obj_set_pos(bar_bg, bar_x, bar_y);
                lvgl_sys::lv_obj_set_style_bg_color(bar_bg, lv_color_hex(0x0A0A0A), 0);
                lvgl_sys::lv_obj_set_style_radius(bar_bg, 7, 0);
                lvgl_sys::lv_obj_set_style_border_color(bar_bg, lv_color_hex(0x2A2A2A), 0);
                lvgl_sys::lv_obj_set_style_border_width(bar_bg, 1, 0);
                set_style_pad_all(bar_bg, 0);

                // Bar fill with gradient and subtle glow
                let bar_fill = lvgl_sys::lv_obj_create(bar_bg);
                lvgl_sys::lv_obj_set_size(bar_fill, scale_fill_width, bar_height - 2);
                lvgl_sys::lv_obj_set_pos(bar_fill, 1, 1);
                lvgl_sys::lv_obj_set_style_bg_color(bar_fill, lv_color_hex(0x00BB44), 0);
                lvgl_sys::lv_obj_set_style_bg_grad_color(bar_fill, lv_color_hex(0x00DD66), 0);
                lvgl_sys::lv_obj_set_style_bg_grad_dir(bar_fill, lvgl_sys::LV_GRAD_DIR_VER as u8, 0);
                lvgl_sys::lv_obj_set_style_radius(bar_fill, 6, 0);
                lvgl_sys::lv_obj_set_style_border_width(bar_fill, 0, 0);
                lvgl_sys::lv_obj_set_style_shadow_color(bar_fill, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(bar_fill, 6, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(bar_fill, 60, 0);
                set_style_pad_all(bar_fill, 0);

                // Action buttons (right side) - individual cards, aligned with left side cards
                // Top row aligns with printer card (height 130), bottom row aligns with scan card (height 125)
                let btn_width: i16 = 130;
                let btn_gap: i16 = 8;
                let btn_start_x: i16 = 800 - 16 - btn_width - btn_gap - btn_width;
                let top_btn_height: i16 = 130;   // Match printer card height
                let bottom_btn_height: i16 = 125; // Match scan card height

                create_action_button(scr, btn_start_x, content_y, btn_width, top_btn_height, "AMS Setup", "", "ams");
                create_action_button(scr, btn_start_x, content_y + 138, btn_width, bottom_btn_height, "Catalog", "", "catalog");
                create_action_button(scr, btn_start_x + btn_width + btn_gap, content_y, btn_width, top_btn_height, "Encode Tag", "", "encode");
                create_action_button(scr, btn_start_x + btn_width + btn_gap, content_y + 138, btn_width, bottom_btn_height, "Settings", "", "settings");

                // === AMS STRIP ===
                let card_gap = 8;
                let ams_y = content_y + 263 + card_gap;  // 52 + 263 + 8 = 323

                // Left Nozzle card
                let left_nozzle = create_card(scr, 16, ams_y, 380, 110);

                // "L" badge (green circle)
                let l_badge = lvgl_sys::lv_obj_create(left_nozzle);
                lvgl_sys::lv_obj_set_size(l_badge, 22, 22);
                lvgl_sys::lv_obj_set_pos(l_badge, 12, 10);
                lvgl_sys::lv_obj_set_style_bg_color(l_badge, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_radius(l_badge, 11, 0);
                lvgl_sys::lv_obj_set_style_border_width(l_badge, 0, 0);
                set_style_pad_all(l_badge, 0);
                let l_letter = lvgl_sys::lv_label_create(l_badge);
                lvgl_sys::lv_label_set_text(l_letter, b"L\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(l_letter, lv_color_hex(COLOR_BG), 0);
                lvgl_sys::lv_obj_align(l_letter, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                let left_label = lvgl_sys::lv_label_create(left_nozzle);
                lvgl_sys::lv_label_set_text(left_label, b"Left Nozzle\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(left_label, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_set_pos(left_label, 40, 13);

                // AMS slots for left nozzle - row 1 (A, B, D with 4 color squares each)
                // Slot A colors: red, yellow, green, salmon
                create_ams_slot_4color(left_nozzle, 12, 38, "A", true, &[0xFF6B6B, 0xFFD93D, 0x6BCB77, 0xFFB5A7]);
                // Slot B colors: blue, dark, light blue, empty (striped)
                create_ams_slot_4color(left_nozzle, 92, 38, "B", false, &[0x4D96FF, 0x404040, 0x9ED5FF, 0]);
                // Slot D colors: magenta, purple, light purple, empty
                create_ams_slot_4color(left_nozzle, 172, 38, "D", false, &[0xFF6BD6, 0xC77DFF, 0xE5B8F4, 0]);

                // AMS slots for left nozzle - row 2 (EXT and HT)
                create_ams_slot_single(left_nozzle, 12, 82, "EXT-1", 0xFF6B6B);
                create_ams_slot_single(left_nozzle, 92, 82, "HT-A", 0x9ED5FF);

                // Right Nozzle card
                let right_nozzle = create_card(scr, 404, ams_y, 380, 110);

                // "R" badge (green circle)
                let r_badge = lvgl_sys::lv_obj_create(right_nozzle);
                lvgl_sys::lv_obj_set_size(r_badge, 22, 22);
                lvgl_sys::lv_obj_set_pos(r_badge, 12, 10);
                lvgl_sys::lv_obj_set_style_bg_color(r_badge, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_radius(r_badge, 11, 0);
                lvgl_sys::lv_obj_set_style_border_width(r_badge, 0, 0);
                set_style_pad_all(r_badge, 0);
                let r_letter = lvgl_sys::lv_label_create(r_badge);
                lvgl_sys::lv_label_set_text(r_letter, b"R\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(r_letter, lv_color_hex(COLOR_BG), 0);
                lvgl_sys::lv_obj_align(r_letter, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                let right_label = lvgl_sys::lv_label_create(right_nozzle);
                lvgl_sys::lv_label_set_text(right_label, b"Right Nozzle\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(right_label, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_set_pos(right_label, 40, 13);

                // AMS slots for right nozzle - row 1
                // Slot C colors: yellow, green, cyan, teal
                create_ams_slot_4color(right_nozzle, 12, 38, "C", false, &[0xFFD93D, 0x6BCB77, 0x4ECDC4, 0x45B7AA]);

                // AMS slots for right nozzle - row 2 (HT and EXT)
                create_ams_slot_single(right_nozzle, 12, 82, "HT-B", 0xFFA500);
                create_ams_slot_single(right_nozzle, 92, 82, "EXT-2", 0);  // Empty (striped)

                // === NOTIFICATION BAR ===
                let notif_bar = create_card(scr, 16, ams_y + 110 + card_gap, 768, 30);  // Below AMS cards

                // Warning dot
                let dot = lvgl_sys::lv_obj_create(notif_bar);
                lvgl_sys::lv_obj_set_size(dot, 10, 10);
                lvgl_sys::lv_obj_set_pos(dot, 12, 10);
                lvgl_sys::lv_obj_set_style_bg_color(dot, lv_color_hex(0xFFA500), 0); // Orange
                lvgl_sys::lv_obj_set_style_radius(dot, 5, 0);
                lvgl_sys::lv_obj_set_style_border_width(dot, 0, 0);

                let notif_text = lvgl_sys::lv_label_create(notif_bar);
                lvgl_sys::lv_label_set_text(notif_text, b"Low filament: PLA Black (A2) - 15% remaining\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(notif_text, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_set_pos(notif_text, 30, 8);

                let view_log = lvgl_sys::lv_label_create(notif_bar);
                lvgl_sys::lv_label_set_text(view_log, b"View Log >\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(view_log, lv_color_hex(COLOR_GRAY), 0);
                lvgl_sys::lv_obj_set_pos(view_log, 680, 8);

                info!("  Home screen UI created");

                // ==================== AMS OVERVIEW SCREEN ====================
                let ams_scr = lvgl_sys::lv_obj_create(ptr::null_mut());
                SCREEN_AMS = ams_scr;
                lvgl_sys::lv_obj_set_style_bg_color(ams_scr, make_color(0x1A, 0x1A, 0x1A), 0);

                // Status bar with back button
                let ams_status_bar = lvgl_sys::lv_obj_create(ams_scr);
                lvgl_sys::lv_obj_set_size(ams_status_bar, 800, 40);
                lvgl_sys::lv_obj_set_pos(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(ams_status_bar, make_color(0x1A, 0x1A, 0x1A), 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_clear_flag(ams_status_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                // Back button
                let back_btn = lvgl_sys::lv_btn_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(back_btn, 36, 28);
                lvgl_sys::lv_obj_align(back_btn, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 8, 0);
                lvgl_sys::lv_obj_set_style_bg_color(back_btn, make_color(0x2D, 0x2D, 0x2D), 0);
                lvgl_sys::lv_obj_set_style_radius(back_btn, 4, 0);
                lvgl_sys::lv_obj_set_style_shadow_width(back_btn, 0, 0);
                lvgl_sys::lv_obj_add_event_cb(back_btn, Some(btn_back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED, ptr::null_mut());

                let back_lbl = lvgl_sys::lv_label_create(back_btn);
                lvgl_sys::lv_label_set_text(back_lbl, b"<\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(back_lbl, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_set_style_text_font(back_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
                lvgl_sys::lv_obj_align(back_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // Title
                let ams_title = lvgl_sys::lv_label_create(ams_status_bar);
                lvgl_sys::lv_label_set_text(ams_title, b"AMS Overview\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(ams_title, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_set_style_text_font(ams_title, &lvgl_sys::lv_font_montserrat_16, 0);
                lvgl_sys::lv_obj_align(ams_title, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 56, 0);

                // AMS Units header
                let ams_header = lvgl_sys::lv_label_create(ams_scr);
                lvgl_sys::lv_label_set_text(ams_header, b"AMS Units\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(ams_header, make_color(0xB0, 0xB0, 0xB0), 0);
                lvgl_sys::lv_obj_set_style_text_font(ams_header, &lvgl_sys::lv_font_montserrat_14, 0);
                lvgl_sys::lv_obj_set_pos(ams_header, 12, 52);

                // AMS grid (2x2 layout for A, B, C, D)
                let ams_units = [
                    ("AMS A", "L", [(0xF5, 0xC5, 0x18), (0x33, 0x33, 0x33), (0xFF, 0x98, 0x00), (0x9E, 0x9E, 0x9E)]),
                    ("AMS B", "L", [(0xE9, 0x1E, 0x63), (0x21, 0x96, 0xF3), (0x4C, 0xAF, 0x50), (0x2D, 0x2D, 0x2D)]),
                    ("AMS C", "R", [(0xFF, 0xFF, 0xFF), (0x21, 0x21, 0x21), (0x2D, 0x2D, 0x2D), (0x2D, 0x2D, 0x2D)]),
                    ("AMS D", "R", [(0x9C, 0x27, 0xB0), (0x00, 0xBC, 0xD4), (0xFF, 0x57, 0x22), (0x60, 0x7D, 0x8B)]),
                ];

                for (i, (name, nozzle, colors)) in ams_units.iter().enumerate() {
                    let col = (i % 2) as i16;
                    let row = (i / 2) as i16;
                    let unit_x = 12 + col * 392;
                    let unit_y = 72 + row * 200;

                    // Unit container
                    let unit = lvgl_sys::lv_obj_create(ams_scr);
                    lvgl_sys::lv_obj_set_size(unit, 380, 190);
                    lvgl_sys::lv_obj_set_pos(unit, unit_x, unit_y);
                    lvgl_sys::lv_obj_set_style_bg_color(unit, make_color(0x25, 0x25, 0x25), 0);
                    lvgl_sys::lv_obj_set_style_border_color(unit, if i == 0 { make_color(0x00, 0xFF, 0x00) } else { make_color(0x3D, 0x3D, 0x3D) }, 0);
                    lvgl_sys::lv_obj_set_style_border_width(unit, if i == 0 { 2 } else { 1 }, 0);
                    lvgl_sys::lv_obj_set_style_radius(unit, 10, 0);
                    lvgl_sys::lv_obj_clear_flag(unit, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                    // Unit name
                    let name_lbl = lvgl_sys::lv_label_create(unit);
                    let name_cstr = std::ffi::CString::new(*name).unwrap();
                    lvgl_sys::lv_label_set_text(name_lbl, name_cstr.as_ptr() as *const i8);
                    lvgl_sys::lv_obj_set_style_text_color(name_lbl, make_color(0xFF, 0xFF, 0xFF), 0);
                    lvgl_sys::lv_obj_set_style_text_font(name_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
                    lvgl_sys::lv_obj_set_pos(name_lbl, 12, 10);

                    // Nozzle badge
                    let nozzle_badge = lvgl_sys::lv_obj_create(unit);
                    lvgl_sys::lv_obj_set_size(nozzle_badge, 24, 20);
                    lvgl_sys::lv_obj_set_pos(nozzle_badge, 80, 10);
                    lvgl_sys::lv_obj_set_style_bg_color(nozzle_badge, if *nozzle == "L" { make_color(0x00, 0xFF, 0x00) } else { make_color(0x3a, 0x86, 0xff) }, 0);
                    lvgl_sys::lv_obj_set_style_radius(nozzle_badge, 4, 0);
                    lvgl_sys::lv_obj_set_style_border_width(nozzle_badge, 0, 0);

                    let nozzle_lbl = lvgl_sys::lv_label_create(nozzle_badge);
                    let nozzle_cstr = std::ffi::CString::new(*nozzle).unwrap();
                    lvgl_sys::lv_label_set_text(nozzle_lbl, nozzle_cstr.as_ptr() as *const i8);
                    lvgl_sys::lv_obj_set_style_text_color(nozzle_lbl, if *nozzle == "L" { make_color(0x00, 0x00, 0x00) } else { make_color(0xFF, 0xFF, 0xFF) }, 0);
                    lvgl_sys::lv_obj_set_style_text_font(nozzle_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
                    lvgl_sys::lv_obj_align(nozzle_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                    // Humidity & temp
                    let stats_lbl = lvgl_sys::lv_label_create(unit);
                    lvgl_sys::lv_label_set_text(stats_lbl, b"19% | 25\xc2\xb0C\0".as_ptr() as *const i8);
                    lvgl_sys::lv_obj_set_style_text_color(stats_lbl, make_color(0x80, 0x80, 0x80), 0);
                    lvgl_sys::lv_obj_set_style_text_font(stats_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
                    lvgl_sys::lv_obj_align(stats_lbl, lvgl_sys::LV_ALIGN_TOP_RIGHT as u8, -12, 12);

                    // 4 spool slots
                    let slot_labels = ["1", "2", "3", "4"];
                    for (j, ((r, g, b), slot_num)) in colors.iter().zip(slot_labels.iter()).enumerate() {
                        let slot_x = 12 + (j as i16) * 90;
                        let slot_y: i16 = 45;

                        // Slot container
                        let slot_bg = lvgl_sys::lv_obj_create(unit);
                        lvgl_sys::lv_obj_set_size(slot_bg, 80, 130);
                        lvgl_sys::lv_obj_set_pos(slot_bg, slot_x, slot_y);
                        lvgl_sys::lv_obj_set_style_bg_color(slot_bg, make_color(0x1A, 0x1A, 0x1A), 0);
                        lvgl_sys::lv_obj_set_style_border_width(slot_bg, 0, 0);
                        lvgl_sys::lv_obj_set_style_radius(slot_bg, 6, 0);
                        lvgl_sys::lv_obj_clear_flag(slot_bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                        // Spool color
                        let spool_color = lvgl_sys::lv_obj_create(slot_bg);
                        lvgl_sys::lv_obj_set_size(spool_color, 50, 60);
                        lvgl_sys::lv_obj_align(spool_color, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 8);
                        lvgl_sys::lv_obj_set_style_bg_color(spool_color, make_color(*r, *g, *b), 0);
                        lvgl_sys::lv_obj_set_style_radius(spool_color, 6, 0);
                        lvgl_sys::lv_obj_set_style_border_color(spool_color, make_color(0x4D, 0x4D, 0x4D), 0);
                        lvgl_sys::lv_obj_set_style_border_width(spool_color, 1, 0);

                        // Slot ID
                        let unit_letter = match i { 0 => "A", 1 => "B", 2 => "C", _ => "D" };
                        let slot_id_str = format!("{}{}", unit_letter, slot_num);
                        let slot_id_cstr = std::ffi::CString::new(slot_id_str).unwrap();
                        let slot_id = lvgl_sys::lv_label_create(slot_bg);
                        lvgl_sys::lv_label_set_text(slot_id, slot_id_cstr.as_ptr() as *const i8);
                        lvgl_sys::lv_obj_set_style_text_color(slot_id, make_color(0xB0, 0xB0, 0xB0), 0);
                        lvgl_sys::lv_obj_set_style_text_font(slot_id, &lvgl_sys::lv_font_montserrat_14, 0);
                        lvgl_sys::lv_obj_align(slot_id, lvgl_sys::LV_ALIGN_BOTTOM_MID as u8, 0, -28);

                        // Fill percentage
                        let pct_lbl = lvgl_sys::lv_label_create(slot_bg);
                        lvgl_sys::lv_label_set_text(pct_lbl, b"85%\0".as_ptr() as *const i8);
                        lvgl_sys::lv_obj_set_style_text_color(pct_lbl, make_color(0x80, 0x80, 0x80), 0);
                        lvgl_sys::lv_obj_set_style_text_font(pct_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
                        lvgl_sys::lv_obj_align(pct_lbl, lvgl_sys::LV_ALIGN_BOTTOM_MID as u8, 0, -8);
                    }
                }

                info!("  AMS Overview screen created");

                // ==================== PLACEHOLDER SCREENS ====================
                // Encode Tag screen
                let encode_scr = lvgl_sys::lv_obj_create(ptr::null_mut());
                SCREEN_ENCODE = encode_scr;
                lvgl_sys::lv_obj_set_style_bg_color(encode_scr, make_color(0x1A, 0x1A, 0x1A), 0);

                let encode_back_btn = lvgl_sys::lv_btn_create(encode_scr);
                lvgl_sys::lv_obj_set_size(encode_back_btn, 36, 28);
                lvgl_sys::lv_obj_set_pos(encode_back_btn, 8, 8);
                lvgl_sys::lv_obj_set_style_bg_color(encode_back_btn, make_color(0x2D, 0x2D, 0x2D), 0);
                lvgl_sys::lv_obj_set_style_radius(encode_back_btn, 4, 0);
                lvgl_sys::lv_obj_set_style_shadow_width(encode_back_btn, 0, 0);
                lvgl_sys::lv_obj_add_event_cb(encode_back_btn, Some(btn_back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED, ptr::null_mut());
                let encode_back_lbl = lvgl_sys::lv_label_create(encode_back_btn);
                lvgl_sys::lv_label_set_text(encode_back_lbl, b"<\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(encode_back_lbl, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_align(encode_back_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                let encode_title = lvgl_sys::lv_label_create(encode_scr);
                lvgl_sys::lv_label_set_text(encode_title, b"Encode NFC Tag\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(encode_title, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_set_style_text_font(encode_title, &lvgl_sys::lv_font_montserrat_20, 0);
                lvgl_sys::lv_obj_align(encode_title, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 100);

                let encode_hint = lvgl_sys::lv_label_create(encode_scr);
                lvgl_sys::lv_label_set_text(encode_hint, b"Place spool on scale and scan tag\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(encode_hint, make_color(0x80, 0x80, 0x80), 0);
                lvgl_sys::lv_obj_set_style_text_font(encode_hint, &lvgl_sys::lv_font_montserrat_14, 0);
                lvgl_sys::lv_obj_align(encode_hint, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 140);

                info!("  Encode Tag screen created");

                // Catalog screen
                let catalog_scr = lvgl_sys::lv_obj_create(ptr::null_mut());
                SCREEN_CATALOG = catalog_scr;
                lvgl_sys::lv_obj_set_style_bg_color(catalog_scr, make_color(0x1A, 0x1A, 0x1A), 0);

                let cat_back_btn = lvgl_sys::lv_btn_create(catalog_scr);
                lvgl_sys::lv_obj_set_size(cat_back_btn, 36, 28);
                lvgl_sys::lv_obj_set_pos(cat_back_btn, 8, 8);
                lvgl_sys::lv_obj_set_style_bg_color(cat_back_btn, make_color(0x2D, 0x2D, 0x2D), 0);
                lvgl_sys::lv_obj_set_style_radius(cat_back_btn, 4, 0);
                lvgl_sys::lv_obj_set_style_shadow_width(cat_back_btn, 0, 0);
                lvgl_sys::lv_obj_add_event_cb(cat_back_btn, Some(btn_back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED, ptr::null_mut());
                let cat_back_lbl = lvgl_sys::lv_label_create(cat_back_btn);
                lvgl_sys::lv_label_set_text(cat_back_lbl, b"<\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(cat_back_lbl, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_align(cat_back_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                let cat_title = lvgl_sys::lv_label_create(catalog_scr);
                lvgl_sys::lv_label_set_text(cat_title, b"Spool Catalog\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(cat_title, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_set_style_text_font(cat_title, &lvgl_sys::lv_font_montserrat_20, 0);
                lvgl_sys::lv_obj_align(cat_title, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 100);

                let cat_hint = lvgl_sys::lv_label_create(catalog_scr);
                lvgl_sys::lv_label_set_text(cat_hint, b"24 spools in database\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(cat_hint, make_color(0x80, 0x80, 0x80), 0);
                lvgl_sys::lv_obj_set_style_text_font(cat_hint, &lvgl_sys::lv_font_montserrat_14, 0);
                lvgl_sys::lv_obj_align(cat_hint, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 140);

                info!("  Catalog screen created");

                // Settings screen
                let settings_scr = lvgl_sys::lv_obj_create(ptr::null_mut());
                SCREEN_SETTINGS = settings_scr;
                lvgl_sys::lv_obj_set_style_bg_color(settings_scr, make_color(0x1A, 0x1A, 0x1A), 0);

                let set_back_btn = lvgl_sys::lv_btn_create(settings_scr);
                lvgl_sys::lv_obj_set_size(set_back_btn, 36, 28);
                lvgl_sys::lv_obj_set_pos(set_back_btn, 8, 8);
                lvgl_sys::lv_obj_set_style_bg_color(set_back_btn, make_color(0x2D, 0x2D, 0x2D), 0);
                lvgl_sys::lv_obj_set_style_radius(set_back_btn, 4, 0);
                lvgl_sys::lv_obj_set_style_shadow_width(set_back_btn, 0, 0);
                lvgl_sys::lv_obj_add_event_cb(set_back_btn, Some(btn_back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED, ptr::null_mut());
                let set_back_lbl = lvgl_sys::lv_label_create(set_back_btn);
                lvgl_sys::lv_label_set_text(set_back_lbl, b"<\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(set_back_lbl, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_align(set_back_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                let set_title = lvgl_sys::lv_label_create(settings_scr);
                lvgl_sys::lv_label_set_text(set_title, b"Settings\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(set_title, make_color(0xFF, 0xFF, 0xFF), 0);
                lvgl_sys::lv_obj_set_style_text_font(set_title, &lvgl_sys::lv_font_montserrat_20, 0);
                lvgl_sys::lv_obj_align(set_title, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 100);

                // Settings items
                let settings_items = ["WiFi Network", "Backend Server", "Scale Calibration", "NFC Reader", "Display Brightness", "About"];
                for (i, item) in settings_items.iter().enumerate() {
                    let item_y = 160 + (i as i16) * 48;

                    let item_btn = lvgl_sys::lv_btn_create(settings_scr);
                    lvgl_sys::lv_obj_set_size(item_btn, 760, 40);
                    lvgl_sys::lv_obj_set_pos(item_btn, 20, item_y);
                    lvgl_sys::lv_obj_set_style_bg_color(item_btn, make_color(0x2D, 0x2D, 0x2D), 0);
                    lvgl_sys::lv_obj_set_style_radius(item_btn, 8, 0);
                    lvgl_sys::lv_obj_set_style_shadow_width(item_btn, 0, 0);

                    let item_lbl = lvgl_sys::lv_label_create(item_btn);
                    let item_cstr = std::ffi::CString::new(*item).unwrap();
                    lvgl_sys::lv_label_set_text(item_lbl, item_cstr.as_ptr() as *const i8);
                    lvgl_sys::lv_obj_set_style_text_color(item_lbl, make_color(0xFF, 0xFF, 0xFF), 0);
                    lvgl_sys::lv_obj_set_style_text_font(item_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
                    lvgl_sys::lv_obj_align(item_lbl, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 16, 0);

                    let arrow_lbl = lvgl_sys::lv_label_create(item_btn);
                    lvgl_sys::lv_label_set_text(arrow_lbl, b">\0".as_ptr() as *const i8);
                    lvgl_sys::lv_obj_set_style_text_color(arrow_lbl, make_color(0x80, 0x80, 0x80), 0);
                    lvgl_sys::lv_obj_set_style_text_font(arrow_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
                    lvgl_sys::lv_obj_align(arrow_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -16, 0);
                }

                info!("  Settings screen created");

                // Load home screen as initial screen
                lvgl_sys::lv_disp_load_scr(SCREEN_HOME);
                info!("  Initial screen loaded");
            }

            info!("========================================");
            info!("=== LVGL DISPLAY RUNNING ===");
            info!("========================================");

            // Initialize WiFi (after LCD panel to preserve internal SRAM for bounce buffers)
            info!("[7/7] Initializing WiFi...");
            let sysloop = EspSystemEventLoop::take().unwrap();
            let nvs = EspDefaultNvsPartition::take().ok();

            let wifi_ip = match wifi_init::connect_wifi(modem, sysloop, nvs) {
                Ok(ip) => {
                    info!("WiFi connected! IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
                    Some(ip)
                }
                Err(e) => {
                    warn!("WiFi failed: {:?} - continuing without network", e);
                    None
                }
            };

            // Start HTTP screenshot server if WiFi is connected
            let _http_server = if wifi_ip.is_some() {
                // Set framebuffer pointer for screenshot capture
                unsafe { http_screenshot::set_framebuffer(PANEL_FB_PTR); }

                match http_screenshot::start_server() {
                    Ok(server) => {
                        let ip = wifi_ip.unwrap();
                        info!("========================================");
                        info!("Screenshot server running!");
                        info!("Open in browser: http://{}.{}.{}.{}/", ip[0], ip[1], ip[2], ip[3]);
                        info!("========================================");
                        Some(server)
                    }
                    Err(e) => {
                        warn!("HTTP server failed: {:?}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Main loop
            let mut loop_count = 0u32;
            let tick_period_ms = 10u32;

            loop {
                loop_count += 1;

                if loop_count <= 5 {
                    info!("Loop {}: tick + timer_handler", loop_count);
                }

                // Poll GT911 touch controller
                if let Some(ref mut i2c) = i2c_driver {
                    // Read point info register (0x814E)
                    let reg_addr: [u8; 2] = [0x81, 0x4E];
                    let mut status = [0u8; 1];
                    if i2c.write_read(gt911_addr, &reg_addr, &mut status, 10).is_ok() {
                        let buffer_ready = (status[0] & 0x80) != 0;
                        let num_points = status[0] & 0x0F;

                        if buffer_ready && num_points > 0 {
                            // Read first touch point (0x814F, 8 bytes)
                            let point_reg: [u8; 2] = [0x81, 0x4F];
                            let mut point_data = [0u8; 8];
                            if i2c.write_read(gt911_addr, &point_reg, &mut point_data, 10).is_ok() {
                                let x = u16::from_le_bytes([point_data[1], point_data[2]]);
                                let y = u16::from_le_bytes([point_data[3], point_data[4]]);
                                unsafe {
                                    // Screenshot trigger: triple-tap top-left corner
                                    if !TOUCH_PRESSED && is_screenshot_zone(x as i16, y as i16) {
                                        let now = loop_count;
                                        // Reset if too much time passed (100 loops = 1 second)
                                        if now - SCREENSHOT_LAST_TAP > 100 {
                                            SCREENSHOT_TAP_COUNT = 0;
                                        }
                                        SCREENSHOT_TAP_COUNT += 1;
                                        SCREENSHOT_LAST_TAP = now;
                                        info!("Screenshot tap {} in corner", SCREENSHOT_TAP_COUNT);

                                        if SCREENSHOT_TAP_COUNT >= 3 {
                                            SCREENSHOT_TAP_COUNT = 0;
                                            SCREENSHOT_PENDING = true;
                                            info!("Screenshot triggered!");
                                        }
                                    }

                                    TOUCH_X = x as i16;
                                    TOUCH_Y = y as i16;
                                    TOUCH_PRESSED = true;
                                }
                                if loop_count % 50 == 0 {
                                    info!("Touch: x={}, y={}", x, y);
                                }
                            }
                        } else {
                            unsafe { TOUCH_PRESSED = false; }
                        }

                        // Clear buffer ready flag
                        if buffer_ready {
                            let clear_reg: [u8; 3] = [0x81, 0x4E, 0x00];
                            let _ = i2c.write(gt911_addr, &clear_reg, 10);
                        }
                    }

                    // Read NAU7802 scale (every 5th loop = ~50ms)
                    if loop_count % 5 == 0 {
                        if scale_state.initialized {
                            match scale::nau7802::read_weight(i2c, &mut scale_state) {
                                Ok(weight) => {
                                    unsafe {
                                        SCALE_WEIGHT = weight;
                                        SCALE_STABLE = scale_state.stable;

                                        // Update UI weight label (every 10th read = ~500ms)
                                        if loop_count % 50 == 0 && !LBL_SCALE_WEIGHT.is_null() {
                                            // Format weight string
                                            let weight_str = if weight.abs() < 10.0 {
                                                format!("{:.1} g\0", weight)
                                            } else if weight.abs() < 1000.0 {
                                                format!("{:.0} g\0", weight)
                                            } else {
                                                format!("{:.2} kg\0", weight / 1000.0)
                                            };
                                            lvgl_sys::lv_label_set_text(
                                                LBL_SCALE_WEIGHT,
                                                weight_str.as_ptr() as *const i8
                                            );
                                        }
                                    }
                                    // Log weight every 50 readings (~2.5 seconds)
                                    if loop_count % 250 == 0 {
                                        info!("Scale: {:.1}g (raw: {}, stable: {})",
                                              weight, scale_state.last_raw, scale_state.stable);
                                    }
                                }
                                Err(e) => {
                                    if loop_count % 500 == 0 {
                                        info!("Scale read error: {:?}", e);
                                    }
                                }
                            }
                        } else if loop_count % 500 == 0 {
                            info!("Scale not initialized");
                        }
                    }
                }

                unsafe {
                    // Tell LVGL how much time has passed
                    lvgl_sys::lv_tick_inc(tick_period_ms);
                    // Process LVGL tasks
                    lvgl_sys::lv_timer_handler();

                    // Check for pending screenshot (triple-tap top-left corner)
                    if SCREENSHOT_PENDING {
                        SCREENSHOT_PENDING = false;
                        dump_screenshot();
                    }
                }

                if loop_count % 100 == 0 {
                    info!("LVGL running - loop {}", loop_count);
                }

                FreeRtos::delay_ms(tick_period_ms);
            }
        }
        Err(e) => {
            info!("========================================");
            info!("FATAL: RGB panel initialization failed!");
            info!("Error code: {}", e);
            info!("========================================");
            loop {
                FreeRtos::delay_ms(1000);
            }
        }
    }
}

/// Create the LVGL UI with proper fonts
fn create_lvgl_ui(display: &Display) -> Result<(), lvgl::LvError> {
    // Get the active screen
    let mut screen = display.get_scr_act()?;

    // Set dark background
    let mut screen_style = Style::default();
    screen_style.set_bg_color(Color::from_rgb((26, 26, 26))); // #1A1A1A
    screen_style.set_radius(0);
    screen.add_style(Part::Main, &mut screen_style);

    // Title label - uses default Montserrat font
    let mut title = Label::create(&mut screen)?;
    title.set_text(CString::new("SpoolBuddy").unwrap().as_c_str());
    title.set_align(Align::TopLeft, 16, 12);

    let mut title_style = Style::default();
    title_style.set_text_color(Color::from_rgb((255, 255, 255)));
    title.add_style(Part::Main, &mut title_style);

    // Printer name label
    let mut printer = Label::create(&mut screen)?;
    printer.set_text(CString::new("X1C-Studio").unwrap().as_c_str());
    printer.set_align(Align::TopMid, 0, 12);

    let mut printer_style = Style::default();
    printer_style.set_text_color(Color::from_rgb((255, 255, 255)));
    printer.add_style(Part::Main, &mut printer_style);

    // Status text
    let mut status = Label::create(&mut screen)?;
    status.set_text(CString::new("Scale Ready").unwrap().as_c_str());
    status.set_align(Align::TopLeft, 100, 96);

    let mut status_style = Style::default();
    status_style.set_text_color(Color::from_rgb((255, 255, 255)));
    status.add_style(Part::Main, &mut status_style);

    // Subtitle
    let mut subtitle = Label::create(&mut screen)?;
    subtitle.set_text(CString::new("Place a spool to scan").unwrap().as_c_str());
    subtitle.set_align(Align::TopLeft, 100, 126);

    let mut subtitle_style = Style::default();
    subtitle_style.set_text_color(Color::from_rgb((176, 176, 176))); // Dimmed
    subtitle.add_style(Part::Main, &mut subtitle_style);

    // Printing status
    let mut printing = Label::create(&mut screen)?;
    printing.set_text(CString::new("Printing - Benchy.3mf").unwrap().as_c_str());
    printing.set_align(Align::TopRight, -20, 96);

    let mut printing_style = Style::default();
    printing_style.set_text_color(Color::from_rgb((0, 255, 0))); // Green accent
    printing.add_style(Part::Main, &mut printing_style);

    // Progress bar
    let mut progress = Bar::create(&mut screen)?;
    progress.set_size(200, 16);
    progress.set_align(Align::TopRight, -20, 140);
    progress.set_range(0, 100);
    progress.set_value(67, lvgl::AnimationState::OFF);

    // Progress bar style
    let mut bar_style = Style::default();
    bar_style.set_bg_color(Color::from_rgb((45, 45, 45))); // Dark gray
    bar_style.set_radius(8);
    progress.add_style(Part::Main, &mut bar_style);

    // Progress indicator style
    let mut ind_style = Style::default();
    ind_style.set_bg_color(Color::from_rgb((0, 255, 0))); // Green
    ind_style.set_radius(8);
    progress.add_style(Part::Any, &mut ind_style);

    // Font test labels at different sizes
    let mut font_test1 = Label::create(&mut screen)?;
    font_test1.set_text(CString::new("Montserrat 14: SpoolBuddy ABCD 1234").unwrap().as_c_str());
    font_test1.set_align(Align::TopLeft, 20, 200);

    let mut ft1_style = Style::default();
    ft1_style.set_text_color(Color::from_rgb((255, 255, 255)));
    font_test1.add_style(Part::Main, &mut ft1_style);

    // AMS section title
    let mut ams_title = Label::create(&mut screen)?;
    ams_title.set_text(CString::new("AMS Status - Left Nozzle").unwrap().as_c_str());
    ams_title.set_align(Align::TopLeft, 20, 280);

    let mut ams_style = Style::default();
    ams_style.set_text_color(Color::from_rgb((0, 255, 0))); // Green
    ams_title.add_style(Part::Main, &mut ams_style);

    // AMS unit labels
    let ams_labels = ["Unit A", "Unit B", "Unit C", "Unit D"];
    for (i, label_text) in ams_labels.iter().enumerate() {
        let mut ams_unit = Label::create(&mut screen)?;
        ams_unit.set_text(CString::new(*label_text).unwrap().as_c_str());
        ams_unit.set_align(Align::TopLeft, 20 + (i as i32 * 190), 320);

        let mut unit_style = Style::default();
        unit_style.set_text_color(Color::from_rgb((200, 200, 200)));
        ams_unit.add_style(Part::Main, &mut unit_style);
    }

    // Time display
    let mut time_lbl = Label::create(&mut screen)?;
    time_lbl.set_text(CString::new("14:23").unwrap().as_c_str());
    time_lbl.set_align(Align::TopRight, -16, 12);

    let mut time_style = Style::default();
    time_style.set_text_color(Color::from_rgb((176, 176, 176)));
    time_lbl.add_style(Part::Main, &mut time_style);

    info!("LVGL UI created with Montserrat fonts!");
    Ok(())
}
