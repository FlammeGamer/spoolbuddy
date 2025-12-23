//! SpoolBuddy Firmware - Home Screen UI
//! ESP32-S3 with ELECROW CrowPanel 7.0" (800x480 RGB565)
//! Using ESP-IDF RGB LCD driver with bounce buffer for PSRAM support
//! LVGL for proper font rendering

// Allow static mut refs - required for LVGL C bindings (display buffers, drivers)
#![allow(static_mut_refs)]

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

// WiFi module
mod wifi_init;

use std::ptr;

// LVGL imports
use lvgl::style::Style;
use lvgl::widgets::{Bar, Label};
use lvgl::{Align, Color, Display, Part, Widget};

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

// Humidity mockup icon (10x10) - reserved for future use
#[allow(dead_code)]
const HUMIDITY_WIDTH: u32 = 10;
#[allow(dead_code)]
const HUMIDITY_HEIGHT: u32 = 10;
#[allow(dead_code)]
static HUMIDITY_DATA: &[u8] = include_bytes!("../assets/humidity_mockup.bin");

// Temperature mockup icon (10x10) - reserved for future use
#[allow(dead_code)]
const TEMP_WIDTH: u32 = 10;
#[allow(dead_code)]
const TEMP_HEIGHT: u32 = 10;
#[allow(dead_code)]
static TEMP_DATA: &[u8] = include_bytes!("../assets/temp_mockup.bin");

// Spool clean icon (32x42 for compact AMS view)
const SPOOL_WIDTH: u32 = 32;
const SPOOL_HEIGHT: u32 = 42;
static SPOOL_CLEAN_DATA: &[u8] = include_bytes!("../assets/spool_clean.bin");

// Image descriptors (initialized at runtime)
static mut LOGO_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut BELL_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut NFC_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut WEIGHT_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut POWER_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut SETTING_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut ENCODE_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
#[allow(dead_code)]
static mut HUMIDITY_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
#[allow(dead_code)]
static mut TEMP_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };
static mut SPOOL_CLEAN_IMG_DSC: lvgl_sys::lv_img_dsc_t = unsafe { core::mem::zeroed() };

// Use LVGL's built-in anti-aliased Montserrat fonts (4bpp = smooth text)
#[allow(unused_macros)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
const COLOR_CARD: u32 = 0x2D2D2D;
const COLOR_BORDER: u32 = 0x3D3D3D;
const COLOR_ACCENT: u32 = 0x00FF00;
const COLOR_WHITE: u32 = 0xFFFFFF;
const COLOR_GRAY: u32 = 0x808080;
const COLOR_TEXT_MUTED: u32 = 0x707070;
const COLOR_STATUS_BAR: u32 = 0x1F1F1F;

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

/// Make an object click-through (doesn't capture click events)
unsafe fn make_click_through(obj: *mut lvgl_sys::lv_obj_t) {
    lvgl_sys::lv_obj_clear_flag(obj, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
    lvgl_sys::lv_obj_add_flag(obj, lvgl_sys::LV_OBJ_FLAG_EVENT_BUBBLE);
}

/// Create a card with glossy styling - shiny highlights and depth
unsafe fn create_card(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, w: i16, h: i16) -> *mut lvgl_sys::lv_obj_t {
    let card = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(card, w, h);
    lvgl_sys::lv_obj_set_pos(card, x, y);

    // Dark polished background
    lvgl_sys::lv_obj_set_style_bg_color(card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(card, 255, 0);

    // Subtle border for glossy bevel effect
    lvgl_sys::lv_obj_set_style_border_color(card, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_border_width(card, 1, 0);
    lvgl_sys::lv_obj_set_style_radius(card, 14, 0);

    // Strong shadow for depth and polish
    lvgl_sys::lv_obj_set_style_shadow_color(card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(card, 12, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_x(card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(card, 4, 0);
    lvgl_sys::lv_obj_set_style_shadow_spread(card, 2, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(card, 180, 0);

    set_style_pad_all(card, 0);
    lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Bright glossy highlight at top edge
    let gloss_top = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(gloss_top, w - 28, 3);
    lvgl_sys::lv_obj_set_pos(gloss_top, 14, 1);
    lvgl_sys::lv_obj_set_style_bg_color(gloss_top, lv_color_hex(0xFFFFFF), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(gloss_top, 80, 0);
    lvgl_sys::lv_obj_set_style_radius(gloss_top, 2, 0);
    lvgl_sys::lv_obj_set_style_border_width(gloss_top, 0, 0);
    set_style_pad_all(gloss_top, 0);

    // Dark bottom edge for depth
    let dark_bottom = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(dark_bottom, w - 28, 2);
    lvgl_sys::lv_obj_set_pos(dark_bottom, 14, h - 3);
    lvgl_sys::lv_obj_set_style_bg_color(dark_bottom, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(dark_bottom, 60, 0);
    lvgl_sys::lv_obj_set_style_radius(dark_bottom, 2, 0);
    lvgl_sys::lv_obj_set_style_border_width(dark_bottom, 0, 0);
    set_style_pad_all(dark_bottom, 0);

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
unsafe fn create_action_button(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, w: i16, h: i16, title: &str, _subtitle: &str, icon_type: &str) -> *mut lvgl_sys::lv_obj_t {
    // Use lv_btn_create for proper button event handling
    let btn = lvgl_sys::lv_btn_create(parent);
    lvgl_sys::lv_obj_set_size(btn, w, h);
    lvgl_sys::lv_obj_set_pos(btn, x, y);

    // Style like a card
    lvgl_sys::lv_obj_set_style_bg_color(btn, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(btn, 255, 0);
    lvgl_sys::lv_obj_set_style_border_color(btn, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_border_width(btn, 1, 0);
    lvgl_sys::lv_obj_set_style_radius(btn, 14, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(btn, 12, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(btn, 4, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(btn, 100, 0);
    set_style_pad_all(btn, 0);
    lvgl_sys::lv_obj_clear_flag(btn, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    create_action_button_content(btn, title, icon_type);
    btn
}

/// Create a small action button for AMS sidebar
unsafe fn create_action_button_small(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, w: i16, h: i16, title: &str, _subtitle: &str, icon_type: &str) -> *mut lvgl_sys::lv_obj_t {
    let btn = lvgl_sys::lv_btn_create(parent);
    lvgl_sys::lv_obj_set_size(btn, w, h);
    lvgl_sys::lv_obj_set_pos(btn, x, y);

    // Style like a card
    lvgl_sys::lv_obj_set_style_bg_color(btn, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(btn, 255, 0);
    lvgl_sys::lv_obj_set_style_border_color(btn, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_border_width(btn, 1, 0);
    lvgl_sys::lv_obj_set_style_radius(btn, 14, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(btn, 12, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(btn, 4, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(btn, 100, 0);
    set_style_pad_all(btn, 0);
    lvgl_sys::lv_obj_clear_flag(btn, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    create_action_button_content_small(btn, title, icon_type);
    btn
}

/// Common content for action buttons - large version for Home screen
unsafe fn create_action_button_content(btn: *mut lvgl_sys::lv_obj_t, title: &str, icon_type: &str) {
    // Icon container (transparent, for positioning) - centered vertically with offset for title
    let icon_container = lvgl_sys::lv_obj_create(btn);
    lvgl_sys::lv_obj_set_size(icon_container, 50, 50);
    lvgl_sys::lv_obj_align(icon_container, lvgl_sys::LV_ALIGN_CENTER as u8, 0, -15);
    lvgl_sys::lv_obj_set_style_bg_opa(icon_container, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon_container, 0, 0);
    set_style_pad_all(icon_container, 0);
    lvgl_sys::lv_obj_clear_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    // Make icon container click-through so parent button receives clicks
    lvgl_sys::lv_obj_clear_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
    lvgl_sys::lv_obj_add_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_EVENT_BUBBLE);

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

/// Compact content for sidebar buttons - small version for AMS Overview
unsafe fn create_action_button_content_small(btn: *mut lvgl_sys::lv_obj_t, title: &str, icon_type: &str) {
    // Icon container - centered in upper portion of button (40x40)
    let icon_container = lvgl_sys::lv_obj_create(btn);
    lvgl_sys::lv_obj_set_size(icon_container, 40, 40);
    lvgl_sys::lv_obj_align(icon_container, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 12);
    lvgl_sys::lv_obj_set_style_bg_opa(icon_container, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon_container, 0, 0);
    set_style_pad_all(icon_container, 0);
    lvgl_sys::lv_obj_clear_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    // Make icon container click-through so parent button receives clicks
    lvgl_sys::lv_obj_clear_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
    lvgl_sys::lv_obj_add_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_EVENT_BUBBLE);

    match icon_type {
        "ams" => draw_ams_icon_small(icon_container),
        "encode" => draw_encode_icon_small(icon_container),
        "catalog" => draw_catalog_icon_small(icon_container),
        "settings" => draw_settings_icon_small(icon_container),
        "nfc" => draw_nfc_icon_small(icon_container),
        "calibrate" => draw_calibrate_icon(icon_container),
        _ => {}
    }

    // Title - smaller font, positioned at bottom
    let title_label = lvgl_sys::lv_label_create(btn);
    let title_cstr = cstr_core::CString::new(title).unwrap();
    lvgl_sys::lv_label_set_text(title_label, title_cstr.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(title_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(title_label, &lvgl_sys::lv_font_montserrat_10, 0);
    lvgl_sys::lv_obj_align(title_label, lvgl_sys::LV_ALIGN_BOTTOM_MID as u8, 0, -6);
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
    make_click_through(bg);

    // Outer frame
    let frame = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(frame, 36, 36);
    lvgl_sys::lv_obj_align(frame, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(frame, 0, 0);
    lvgl_sys::lv_obj_set_style_border_color(frame, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(frame, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(frame, 4, 0);
    set_style_pad_all(frame, 0);
    make_click_through(frame);

    // Horizontal lines (3 rows)
    for i in 0..3 {
        let line = lvgl_sys::lv_obj_create(frame);
        lvgl_sys::lv_obj_set_size(line, 24, 2);
        lvgl_sys::lv_obj_set_pos(line, 4, 6 + i * 9);
        lvgl_sys::lv_obj_set_style_bg_color(line, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_border_width(line, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(line, 1, 0);
        make_click_through(line);
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
    make_click_through(bg);

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
    make_click_through(icon);
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
    make_click_through(bg);

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
            make_click_through(square);
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
    make_click_through(bg);

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
    make_click_through(icon);
}

/// Draw AMS icon (small 40x40 version for sidebar)
#[allow(dead_code)]
unsafe fn draw_ams_icon_small(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 40, 40);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    make_click_through(bg);

    let frame = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(frame, 28, 28);
    lvgl_sys::lv_obj_align(frame, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(frame, 0, 0);
    lvgl_sys::lv_obj_set_style_border_color(frame, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(frame, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(frame, 3, 0);
    set_style_pad_all(frame, 0);
    make_click_through(frame);

    for i in 0..3 {
        let line = lvgl_sys::lv_obj_create(frame);
        lvgl_sys::lv_obj_set_size(line, 18, 2);
        lvgl_sys::lv_obj_set_pos(line, 3, 4 + i * 7);
        lvgl_sys::lv_obj_set_style_bg_color(line, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_border_width(line, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(line, 1, 0);
        make_click_through(line);
    }
}

/// Draw Encode icon (small 40x40 version for sidebar)
#[allow(dead_code)]
unsafe fn draw_encode_icon_small(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 40, 40);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    make_click_through(bg);

    ENCODE_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, ENCODE_WIDTH, ENCODE_HEIGHT,
    );
    ENCODE_IMG_DSC.data_size = (ENCODE_WIDTH * ENCODE_HEIGHT * 3) as u32;
    ENCODE_IMG_DSC.data = ENCODE_DATA.as_ptr();

    let icon = lvgl_sys::lv_img_create(bg);
    lvgl_sys::lv_img_set_src(icon, &raw const ENCODE_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(icon, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_img_set_zoom(icon, 179);
    lvgl_sys::lv_obj_set_style_img_recolor(icon, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(icon, 255, 0);
    make_click_through(icon);
}

/// Draw Catalog icon (small 40x40 version for sidebar)
#[allow(dead_code)]
unsafe fn draw_catalog_icon_small(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 40, 40);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    make_click_through(bg);

    let size: i16 = 8;
    let gap: i16 = 2;
    let start_x: i16 = 6;
    let start_y: i16 = 6;

    for row in 0..3 {
        for col in 0..3 {
            let square = lvgl_sys::lv_obj_create(bg);
            lvgl_sys::lv_obj_set_size(square, size, size);
            lvgl_sys::lv_obj_set_pos(square, start_x + col * (size + gap), start_y + row * (size + gap));
            lvgl_sys::lv_obj_set_style_bg_color(square, lv_color_hex(COLOR_ACCENT), 0);
            lvgl_sys::lv_obj_set_style_border_width(square, 0, 0);
            lvgl_sys::lv_obj_set_style_radius(square, 2, 0);
            make_click_through(square);
        }
    }
}

/// Draw Settings icon (small 40x40 version for sidebar)
#[allow(dead_code)]
unsafe fn draw_settings_icon_small(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 40, 40);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    make_click_through(bg);

    SETTING_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, SETTING_WIDTH, SETTING_HEIGHT,
    );
    SETTING_IMG_DSC.data_size = (SETTING_WIDTH * SETTING_HEIGHT * 3) as u32;
    SETTING_IMG_DSC.data = SETTING_DATA.as_ptr();

    let icon = lvgl_sys::lv_img_create(bg);
    lvgl_sys::lv_img_set_src(icon, &raw const SETTING_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(icon, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_img_set_zoom(icon, 179);
    lvgl_sys::lv_obj_set_style_img_recolor(icon, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(icon, 255, 0);
    make_click_through(icon);
}

/// Draw NFC/Scan icon (small 40x40 version for sidebar)
#[allow(dead_code)]
unsafe fn draw_nfc_icon_small(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 40, 40);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    make_click_through(bg);

    NFC_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, NFC_WIDTH, NFC_HEIGHT,
    );
    NFC_IMG_DSC.data_size = (NFC_WIDTH * NFC_HEIGHT * 3) as u32;
    NFC_IMG_DSC.data = NFC_DATA.as_ptr();

    let icon = lvgl_sys::lv_img_create(bg);
    lvgl_sys::lv_img_set_src(icon, &raw const NFC_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(icon, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_img_set_zoom(icon, 100);
    lvgl_sys::lv_obj_set_style_img_recolor(icon, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(icon, 255, 0);
    make_click_through(icon);
}

/// Draw Calibrate icon (target/crosshair)
#[allow(dead_code)]
unsafe fn draw_calibrate_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let bg = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bg, 40, 40);
    lvgl_sys::lv_obj_align(bg, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(bg, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_radius(bg, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(bg, 0, 0);
    set_style_pad_all(bg, 0);
    make_click_through(bg);

    let h_line = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(h_line, 24, 2);
    lvgl_sys::lv_obj_align(h_line, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(h_line, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(h_line, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(h_line, 1, 0);
    make_click_through(h_line);

    let v_line = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(v_line, 2, 24);
    lvgl_sys::lv_obj_align(v_line, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(v_line, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(v_line, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(v_line, 1, 0);
    make_click_through(v_line);

    let ring = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(ring, 28, 28);
    lvgl_sys::lv_obj_align(ring, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(ring, 0, 0);
    lvgl_sys::lv_obj_set_style_border_color(ring, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(ring, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(ring, 14, 0);
    set_style_pad_all(ring, 0);
    make_click_through(ring);

    let dot = lvgl_sys::lv_obj_create(bg);
    lvgl_sys::lv_obj_set_size(dot, 4, 4);
    lvgl_sys::lv_obj_align(dot, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(dot, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(dot, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(dot, 2, 0);
    make_click_through(dot);
}

/// Draw scale/weight icon for settings
unsafe fn draw_scale_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let bracket = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(bracket, 20, 20);
    lvgl_sys::lv_obj_set_pos(bracket, 6, 6);
    lvgl_sys::lv_obj_clear_flag(bracket, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(bracket, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(bracket, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(bracket, 2, 0);
    lvgl_sys::lv_obj_set_style_border_side(bracket, (lvgl_sys::LV_BORDER_SIDE_LEFT | lvgl_sys::LV_BORDER_SIDE_BOTTOM) as u8, 0);
    lvgl_sys::lv_obj_set_style_radius(bracket, 0, 0);
    set_style_pad_all(bracket, 0);
}

/// Draw NFC chip icon for settings
unsafe fn draw_nfc_settings_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let outer = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(outer, 22, 22);
    lvgl_sys::lv_obj_set_pos(outer, 5, 5);
    lvgl_sys::lv_obj_clear_flag(outer, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(outer, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(outer, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(outer, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(outer, 4, 0);
    set_style_pad_all(outer, 0);

    let inner = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(inner, 8, 8);
    lvgl_sys::lv_obj_set_pos(inner, 12, 12);
    lvgl_sys::lv_obj_clear_flag(inner, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(inner, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(inner, 2, 0);
    lvgl_sys::lv_obj_set_style_border_width(inner, 0, 0);
    set_style_pad_all(inner, 0);
}

/// Draw display/monitor icon for settings
unsafe fn draw_display_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let monitor = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(monitor, 22, 16);
    lvgl_sys::lv_obj_set_pos(monitor, 5, 4);
    lvgl_sys::lv_obj_clear_flag(monitor, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(monitor, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(monitor, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(monitor, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(monitor, 2, 0);
    set_style_pad_all(monitor, 0);

    let stand = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(stand, 12, 4);
    lvgl_sys::lv_obj_set_pos(stand, 10, 22);
    lvgl_sys::lv_obj_clear_flag(stand, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(stand, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(stand, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(stand, 0, 0);
    set_style_pad_all(stand, 0);
}

/// Draw update/refresh icon
unsafe fn draw_update_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let circle = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(circle, 20, 20);
    lvgl_sys::lv_obj_set_pos(circle, 6, 6);
    lvgl_sys::lv_obj_clear_flag(circle, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(circle, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(circle, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(circle, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(circle, 10, 0);
    set_style_pad_all(circle, 0);

    let arrow = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(arrow, 4, 4);
    lvgl_sys::lv_obj_set_pos(arrow, 20, 6);
    lvgl_sys::lv_obj_clear_flag(arrow, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(arrow, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(arrow, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(arrow, 0, 0);
    set_style_pad_all(arrow, 0);
}

/// Draw gear/settings icon
unsafe fn draw_gear_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let outer = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(outer, 22, 22);
    lvgl_sys::lv_obj_set_pos(outer, 5, 5);
    lvgl_sys::lv_obj_clear_flag(outer, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(outer, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(outer, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(outer, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(outer, 11, 0);
    set_style_pad_all(outer, 0);

    let inner = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(inner, 10, 10);
    lvgl_sys::lv_obj_set_pos(inner, 11, 11);
    lvgl_sys::lv_obj_clear_flag(inner, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(inner, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(inner, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(inner, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(inner, 5, 0);
    set_style_pad_all(inner, 0);
}

/// Draw info icon (i in circle)
unsafe fn draw_info_icon(parent: *mut lvgl_sys::lv_obj_t) {
    let circle = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(circle, 22, 22);
    lvgl_sys::lv_obj_set_pos(circle, 5, 5);
    lvgl_sys::lv_obj_clear_flag(circle, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(circle, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(circle, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(circle, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(circle, 11, 0);
    set_style_pad_all(circle, 0);

    let i_lbl = lvgl_sys::lv_label_create(parent);
    let i_txt = CString::new("i").unwrap();
    lvgl_sys::lv_label_set_text(i_lbl, i_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(i_lbl, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_text_font(i_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(i_lbl, 13, 8);
}

/// Helper: Create section header
unsafe fn create_section_header(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, text: &str) {
    let lbl = lvgl_sys::lv_label_create(parent);
    let lbl_txt = CString::new(text).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(lbl, x, y);
}

/// Helper: Create numbered step row (for calibration/setup wizards)
unsafe fn create_numbered_step(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, num: u8, text: &str) {
    let row = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(row, 768, 36);
    lvgl_sys::lv_obj_set_pos(row, x, y);
    lvgl_sys::lv_obj_clear_flag(row, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(row, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(row, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(row, 0, 0);
    set_style_pad_all(row, 0);

    // Number circle
    let num_circle = lvgl_sys::lv_obj_create(row);
    lvgl_sys::lv_obj_set_size(num_circle, 24, 24);
    lvgl_sys::lv_obj_set_pos(num_circle, 8, 6);
    lvgl_sys::lv_obj_clear_flag(num_circle, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(num_circle, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(num_circle, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(num_circle, 0, 0);
    set_style_pad_all(num_circle, 0);

    let num_lbl = lvgl_sys::lv_label_create(num_circle);
    let num_str = format!("{}", num);
    let num_txt = CString::new(num_str).unwrap();
    lvgl_sys::lv_label_set_text(num_lbl, num_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(num_lbl, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_text_font(num_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(num_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Text
    let text_lbl = lvgl_sys::lv_label_create(row);
    let text_txt = CString::new(text).unwrap();
    lvgl_sys::lv_label_set_text(text_lbl, text_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(text_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(text_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(text_lbl, 44, 10);
}

/// Helper: Create settings row
unsafe fn create_settings_row(parent: *mut lvgl_sys::lv_obj_t, _x: i16, y: i16, title: &str, value: &str, status_color: &str, has_arrow: bool) -> *mut lvgl_sys::lv_obj_t {
    let row = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(row, 760, 44);
    lvgl_sys::lv_obj_set_pos(row, 20, y);
    lvgl_sys::lv_obj_clear_flag(row, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_add_flag(row, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
    lvgl_sys::lv_obj_set_style_bg_color(row, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(row, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(row, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(row, 0, 0);
    set_style_pad_all(row, 0);

    // Green indicator bar on left
    let indicator = lvgl_sys::lv_obj_create(row);
    lvgl_sys::lv_obj_set_size(indicator, 4, 28);
    lvgl_sys::lv_obj_set_pos(indicator, 0, 8);
    lvgl_sys::lv_obj_clear_flag(indicator, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(indicator, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(indicator, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(indicator, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(indicator, 0, 0);
    set_style_pad_all(indicator, 0);

    // Title
    let title_lbl = lvgl_sys::lv_label_create(row);
    let title_txt = CString::new(title).unwrap();
    lvgl_sys::lv_label_set_text(title_lbl, title_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(title_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(title_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(title_lbl, 16, 13);

    // Value/status with optional colored dot
    if !value.is_empty() {
        let color = match status_color {
            "green" => COLOR_ACCENT,
            "gray" => COLOR_TEXT_MUTED,
            _ => COLOR_TEXT_MUTED,
        };

        if !status_color.is_empty() {
            let dot = lvgl_sys::lv_obj_create(row);
            lvgl_sys::lv_obj_set_size(dot, 8, 8);
            lvgl_sys::lv_obj_align(dot, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -115, 0);
            lvgl_sys::lv_obj_clear_flag(dot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
            lvgl_sys::lv_obj_set_style_bg_color(dot, lv_color_hex(color), 0);
            lvgl_sys::lv_obj_set_style_bg_opa(dot, 255, 0);
            lvgl_sys::lv_obj_set_style_radius(dot, 4, 0);
            lvgl_sys::lv_obj_set_style_border_width(dot, 0, 0);
            set_style_pad_all(dot, 0);
        }

        let value_lbl = lvgl_sys::lv_label_create(row);
        let value_txt = CString::new(value).unwrap();
        lvgl_sys::lv_label_set_text(value_lbl, value_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(value_lbl, lv_color_hex(color), 0);
        lvgl_sys::lv_obj_set_style_text_font(value_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_align(value_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -30, 0);
    }

    if has_arrow {
        let arrow_lbl = lvgl_sys::lv_label_create(row);
        let arrow_txt = CString::new(">").unwrap();
        lvgl_sys::lv_label_set_text(arrow_lbl, arrow_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(arrow_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
        lvgl_sys::lv_obj_set_style_text_font(arrow_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
        lvgl_sys::lv_obj_align(arrow_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -10, 0);
    }
    row
}

/// Helper: Create settings row with icon
unsafe fn create_settings_row_with_icon(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, icon_type: &str, title: &str, value: &str, status_color: &str, has_arrow: bool) -> *mut lvgl_sys::lv_obj_t {
    let row = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(row, 768, 44);
    lvgl_sys::lv_obj_set_pos(row, x, y);
    lvgl_sys::lv_obj_clear_flag(row, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_add_flag(row, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
    lvgl_sys::lv_obj_set_style_bg_color(row, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(row, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(row, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(row, 0, 0);
    set_style_pad_all(row, 0);

    // Icon container
    let icon_container = lvgl_sys::lv_obj_create(row);
    lvgl_sys::lv_obj_set_size(icon_container, 32, 32);
    lvgl_sys::lv_obj_set_pos(icon_container, 8, 6);
    lvgl_sys::lv_obj_clear_flag(icon_container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(icon_container, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon_container, 0, 0);
    set_style_pad_all(icon_container, 0);

    match icon_type {
        "scale" => draw_scale_icon(icon_container),
        "nfc" => draw_nfc_settings_icon(icon_container),
        "display" => draw_display_icon(icon_container),
        "update" => draw_update_icon(icon_container),
        "gear" => draw_gear_icon(icon_container),
        "info" => draw_info_icon(icon_container),
        _ => {}
    }

    // Title
    let title_lbl = lvgl_sys::lv_label_create(row);
    let title_txt = CString::new(title).unwrap();
    lvgl_sys::lv_label_set_text(title_lbl, title_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(title_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(title_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(title_lbl, 52, 14);

    if !value.is_empty() {
        let color = match status_color {
            "green" => COLOR_ACCENT,
            "gray" => COLOR_TEXT_MUTED,
            _ => COLOR_TEXT_MUTED,
        };

        if !status_color.is_empty() {
            let dot = lvgl_sys::lv_obj_create(row);
            lvgl_sys::lv_obj_set_size(dot, 8, 8);
            lvgl_sys::lv_obj_align(dot, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -115, 0);
            lvgl_sys::lv_obj_clear_flag(dot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
            lvgl_sys::lv_obj_set_style_bg_color(dot, lv_color_hex(color), 0);
            lvgl_sys::lv_obj_set_style_bg_opa(dot, 255, 0);
            lvgl_sys::lv_obj_set_style_radius(dot, 4, 0);
            lvgl_sys::lv_obj_set_style_border_width(dot, 0, 0);
            set_style_pad_all(dot, 0);
        }

        let value_lbl = lvgl_sys::lv_label_create(row);
        let value_txt = CString::new(value).unwrap();
        lvgl_sys::lv_label_set_text(value_lbl, value_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(value_lbl, lv_color_hex(color), 0);
        lvgl_sys::lv_obj_set_style_text_font(value_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_align(value_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -30, 0);
    }

    if has_arrow {
        let arrow_lbl = lvgl_sys::lv_label_create(row);
        let arrow_txt = CString::new(">").unwrap();
        lvgl_sys::lv_label_set_text(arrow_lbl, arrow_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(arrow_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
        lvgl_sys::lv_obj_set_style_text_font(arrow_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
        lvgl_sys::lv_obj_align(arrow_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -10, 0);
    }
    row
}

/// Create status bar (reusable for all screens)
#[allow(dead_code)]
unsafe fn create_status_bar(scr: *mut lvgl_sys::lv_obj_t) {
    let status_bar = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(status_bar, 800, 44);
    lvgl_sys::lv_obj_set_pos(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(status_bar, lv_color_hex(COLOR_STATUS_BAR), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(status_bar, 255, 0);
    lvgl_sys::lv_obj_set_style_border_width(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_pad_left(status_bar, 16, 0);
    lvgl_sys::lv_obj_set_style_pad_right(status_bar, 16, 0);
    // Visible bottom shadow for depth separation
    lvgl_sys::lv_obj_set_style_shadow_color(status_bar, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(status_bar, 25, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(status_bar, 8, 0);
    lvgl_sys::lv_obj_set_style_shadow_spread(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(status_bar, 200, 0);
    lvgl_sys::lv_obj_clear_flag(status_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // SpoolBuddy logo
    LOGO_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0,
        LOGO_WIDTH, LOGO_HEIGHT,
    );
    LOGO_IMG_DSC.data_size = (LOGO_WIDTH * LOGO_HEIGHT * 3) as u32;
    LOGO_IMG_DSC.data = LOGO_DATA.as_ptr();

    let logo_img = lvgl_sys::lv_img_create(status_bar);
    lvgl_sys::lv_img_set_src(logo_img, &raw const LOGO_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(logo_img, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 0, 0);

    // Printer selector (center)
    let printer_btn = lvgl_sys::lv_btn_create(status_bar);
    lvgl_sys::lv_obj_set_size(printer_btn, 200, 32);
    lvgl_sys::lv_obj_align(printer_btn, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(printer_btn, lv_color_hex(0x242424), 0);
    lvgl_sys::lv_obj_set_style_radius(printer_btn, 16, 0);
    lvgl_sys::lv_obj_set_style_border_color(printer_btn, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_border_width(printer_btn, 1, 0);

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
    lvgl_sys::lv_obj_align(printer_label, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 28, 0);

    // Power icon (orange = printing)
    POWER_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0,
        POWER_WIDTH, POWER_HEIGHT,
    );
    POWER_IMG_DSC.data_size = (POWER_WIDTH * POWER_HEIGHT * 3) as u32;
    POWER_IMG_DSC.data = POWER_DATA.as_ptr();

    let power_img = lvgl_sys::lv_img_create(printer_btn);
    lvgl_sys::lv_img_set_src(power_img, &raw const POWER_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(power_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -24, 0);
    lvgl_sys::lv_obj_set_style_img_recolor(power_img, lv_color_hex(0xFFA500), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(power_img, 255, 0);

    let arrow_label = lvgl_sys::lv_label_create(printer_btn);
    let arrow_text = CString::new("v").unwrap();
    lvgl_sys::lv_label_set_text(arrow_label, arrow_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(arrow_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_align(arrow_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -8, 2);

    // Time (rightmost)
    let time_label = lvgl_sys::lv_label_create(status_bar);
    let time_text = CString::new("14:23").unwrap();
    lvgl_sys::lv_label_set_text(time_label, time_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(time_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_align(time_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, 0, 0);

    // WiFi bars
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
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0,
        BELL_WIDTH, BELL_HEIGHT,
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
    lvgl_sys::lv_obj_set_style_radius(badge, 7, 0);
    lvgl_sys::lv_obj_set_style_border_width(badge, 0, 0);
    set_style_pad_all(badge, 0);
    lvgl_sys::lv_obj_clear_flag(badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    let badge_num = lvgl_sys::lv_label_create(badge);
    let badge_text = CString::new("3").unwrap();
    lvgl_sys::lv_label_set_text(badge_num, badge_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(badge_num, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(badge_num, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(badge_num, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
}

/// Create status bar with back button for sub-screens
/// Simplified version with essential elements: back button, logo, printer name, time
unsafe fn create_status_bar_with_back(scr: *mut lvgl_sys::lv_obj_t, _title: &str, back_cb: unsafe extern "C" fn(*mut lvgl_sys::lv_event_t)) {
    let status_bar = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(status_bar, 800, 44);
    lvgl_sys::lv_obj_set_pos(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(status_bar, lv_color_hex(COLOR_STATUS_BAR), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(status_bar, 255, 0);
    lvgl_sys::lv_obj_set_style_border_width(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(status_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_pad_left(status_bar, 16, 0);
    lvgl_sys::lv_obj_set_style_pad_right(status_bar, 16, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(status_bar, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(status_bar, 25, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(status_bar, 8, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(status_bar, 200, 0);
    lvgl_sys::lv_obj_clear_flag(status_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_clear_flag(status_bar, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);

    // Back button (leftmost)
    let back_btn = lvgl_sys::lv_btn_create(status_bar);
    lvgl_sys::lv_obj_set_size(back_btn, 36, 28);
    lvgl_sys::lv_obj_align(back_btn, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(back_btn, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(back_btn, 4, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(back_btn, 0, 0);
    lvgl_sys::lv_obj_add_flag(back_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
    lvgl_sys::lv_obj_add_event_cb(back_btn, Some(back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

    let back_lbl = lvgl_sys::lv_label_create(back_btn);
    lvgl_sys::lv_label_set_text(back_lbl, b"<\0".as_ptr() as *const i8);
    lvgl_sys::lv_obj_set_style_text_color(back_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(back_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
    lvgl_sys::lv_obj_align(back_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // SpoolBuddy logo image (after back button)
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
    lvgl_sys::lv_obj_align(logo_img, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 44, 0);

    // Printer selector button (center) - matching home screen style
    let printer_btn = lvgl_sys::lv_btn_create(status_bar);
    lvgl_sys::lv_obj_set_size(printer_btn, 200, 32);
    lvgl_sys::lv_obj_align(printer_btn, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_set_style_bg_color(printer_btn, lv_color_hex(0x242424), 0);
    lvgl_sys::lv_obj_set_style_radius(printer_btn, 16, 0);
    lvgl_sys::lv_obj_set_style_border_color(printer_btn, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_border_width(printer_btn, 1, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(printer_btn, 0, 0);

    // Status dot (green = connected)
    let status_dot = lvgl_sys::lv_obj_create(printer_btn);
    lvgl_sys::lv_obj_set_size(status_dot, 8, 8);
    lvgl_sys::lv_obj_align(status_dot, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 12, 0);
    lvgl_sys::lv_obj_set_style_bg_color(status_dot, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(status_dot, 4, 0);
    lvgl_sys::lv_obj_set_style_border_width(status_dot, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(status_dot, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(status_dot, 6, 0);
    lvgl_sys::lv_obj_set_style_shadow_spread(status_dot, 2, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(status_dot, 150, 0);

    // Printer name
    let printer_label = lvgl_sys::lv_label_create(printer_btn);
    let printer_text = CString::new("X1C-Studio").unwrap();
    lvgl_sys::lv_label_set_text(printer_label, printer_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(printer_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_align(printer_label, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 28, 0);

    // Power icon
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
    lvgl_sys::lv_obj_align(power_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -24, 0);
    lvgl_sys::lv_obj_set_style_img_recolor(power_img, lv_color_hex(0xFFA500), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(power_img, 255, 0);

    // Dropdown arrow
    let arrow_label = lvgl_sys::lv_label_create(printer_btn);
    let arrow_text = CString::new("v").unwrap();
    lvgl_sys::lv_label_set_text(arrow_label, arrow_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(arrow_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_align(arrow_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -8, 2);

    // Time (rightmost)
    let time_label = lvgl_sys::lv_label_create(status_bar);
    let time_text = CString::new("14:23").unwrap();
    lvgl_sys::lv_label_set_text(time_label, time_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(time_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_align(time_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, 0, 0);

    // WiFi bars
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
    let bell_img = lvgl_sys::lv_img_create(status_bar);
    lvgl_sys::lv_img_set_src(bell_img, &raw const BELL_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(bell_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -82, 0);

    // Notification badge
    let badge = lvgl_sys::lv_obj_create(status_bar);
    lvgl_sys::lv_obj_set_size(badge, 14, 14);
    lvgl_sys::lv_obj_align(badge, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -70, -8);
    lvgl_sys::lv_obj_set_style_bg_color(badge, lv_color_hex(0xFF4444), 0);
    lvgl_sys::lv_obj_set_style_radius(badge, 7, 0);
    lvgl_sys::lv_obj_set_style_border_width(badge, 0, 0);
    set_style_pad_all(badge, 0);
    lvgl_sys::lv_obj_clear_flag(badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    let badge_num = lvgl_sys::lv_label_create(badge);
    let badge_text = CString::new("3").unwrap();
    lvgl_sys::lv_label_set_text(badge_num, badge_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(badge_num, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(badge_num, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(badge_num, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
}

/// Create bottom status bar
#[allow(dead_code)]
unsafe fn create_bottom_status_bar(scr: *mut lvgl_sys::lv_obj_t) {
    let bar_y: i16 = 436;
    let bar_h: i16 = 44;

    // Horizontal separator line above status bar
    let separator = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(separator, 800, 1);
    lvgl_sys::lv_obj_set_pos(separator, 0, bar_y);
    lvgl_sys::lv_obj_set_style_bg_color(separator, lv_color_hex(0x404040), 0);
    lvgl_sys::lv_obj_set_style_border_width(separator, 0, 0);

    // Full-width dark background bar
    let bar = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(bar, 800, bar_h);
    lvgl_sys::lv_obj_set_pos(bar, 0, bar_y + 1);
    lvgl_sys::lv_obj_set_style_bg_color(bar, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(bar, 255, 0);
    lvgl_sys::lv_obj_set_style_border_width(bar, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(bar, 0, 0);
    set_style_pad_all(bar, 0);

    // Connection status (left side)
    let conn_dot = lvgl_sys::lv_obj_create(bar);
    lvgl_sys::lv_obj_set_size(conn_dot, 10, 10);
    lvgl_sys::lv_obj_set_pos(conn_dot, 20, 17);
    lvgl_sys::lv_obj_set_style_bg_color(conn_dot, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(conn_dot, 5, 0);
    lvgl_sys::lv_obj_set_style_border_width(conn_dot, 0, 0);

    let conn_label = lvgl_sys::lv_label_create(bar);
    let conn_text = CString::new("Connected").unwrap();
    lvgl_sys::lv_label_set_text(conn_label, conn_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(conn_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(conn_label, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(conn_label, 36, 12);

    // Print status (centered)
    let status_label = lvgl_sys::lv_label_create(bar);
    let status_text = CString::new("Printing").unwrap();
    lvgl_sys::lv_label_set_text(status_label, status_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(status_label, lv_color_hex(0xFFA500), 0);
    lvgl_sys::lv_obj_set_style_text_font(status_label, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(status_label, lvgl_sys::LV_ALIGN_CENTER as u8, -95, 0);

    let progress_label = lvgl_sys::lv_label_create(bar);
    let progress_text = CString::new("45% - 2h 15m left").unwrap();
    lvgl_sys::lv_label_set_text(progress_label, progress_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(progress_label, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(progress_label, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(progress_label, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Last sync time (far right)
    let sync_label = lvgl_sys::lv_label_create(bar);
    let sync_text = CString::new("Updated 5s ago").unwrap();
    lvgl_sys::lv_label_set_text(sync_label, sync_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(sync_label, lv_color_hex(COLOR_GRAY), 0);
    lvgl_sys::lv_obj_set_style_text_font(sync_label, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(sync_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -20, 0);
}

fn lighten_color(color: u32, amount: u8) -> u32 {
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;
    let r = r.saturating_add(amount);
    let g = g.saturating_add(amount);
    let b = b.saturating_add(amount);
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Create compact AMS unit (4-slot) for AMS Overview screen
#[allow(dead_code)]
unsafe fn create_ams_unit_compact(
    parent: *mut lvgl_sys::lv_obj_t,
    x: i16, y: i16, w: i16, h: i16,
    name: &str, nozzle: &str, humidity: &str, temp: &str, active: bool,
    slots: &[(&str, u32, &str, &str, bool)],
) {
    let unit = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(unit, w, h);
    lvgl_sys::lv_obj_set_pos(unit, x, y);
    lvgl_sys::lv_obj_clear_flag(unit, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(unit, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(unit, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(unit, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(unit, 2, 0);
    if active {
        lvgl_sys::lv_obj_set_style_border_color(unit, lv_color_hex(COLOR_ACCENT), 0);
    } else {
        lvgl_sys::lv_obj_set_style_border_color(unit, lv_color_hex(0x404040), 0);
    }
    set_style_pad_all(unit, 6);

    // Header row: name + badge on left, humidity/temp on right
    let name_lbl = lvgl_sys::lv_label_create(unit);
    let name_txt = CString::new(name).unwrap();
    lvgl_sys::lv_label_set_text(name_lbl, name_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(name_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(name_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(name_lbl, 4, 0);

    // Nozzle badge (L or R)
    if !nozzle.is_empty() {
        let name_width: i16 = (name.len() as i16) * 7 + 12;
        let badge_lbl = lvgl_sys::lv_label_create(unit);
        let badge_txt = CString::new(nozzle).unwrap();
        lvgl_sys::lv_label_set_text(badge_lbl, badge_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(badge_lbl, lv_color_hex(0x1A1A1A), 0);
        lvgl_sys::lv_obj_set_style_text_font(badge_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_set_style_bg_color(badge_lbl, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_bg_opa(badge_lbl, 255, 0);
        lvgl_sys::lv_obj_set_style_pad_left(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_style_pad_right(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_style_pad_top(badge_lbl, 0, 0);
        lvgl_sys::lv_obj_set_style_pad_bottom(badge_lbl, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_pos(badge_lbl, name_width, 3);
    }

    // Humidity icon + value
    let stats_x = w - 95;
    HUMIDITY_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, HUMIDITY_WIDTH, HUMIDITY_HEIGHT,
    );
    HUMIDITY_IMG_DSC.data_size = (HUMIDITY_WIDTH * HUMIDITY_HEIGHT * 3) as u32;
    HUMIDITY_IMG_DSC.data = HUMIDITY_DATA.as_ptr();

    let hum_icon = lvgl_sys::lv_img_create(unit);
    lvgl_sys::lv_img_set_src(hum_icon, &raw const HUMIDITY_IMG_DSC as *const _);
    lvgl_sys::lv_obj_set_pos(hum_icon, stats_x, 2);
    lvgl_sys::lv_obj_set_style_img_recolor(hum_icon, lv_color_hex(0x4FC3F7), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(hum_icon, 255, 0);

    let hum_lbl = lvgl_sys::lv_label_create(unit);
    let hum_txt = CString::new(humidity).unwrap();
    lvgl_sys::lv_label_set_text(hum_lbl, hum_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(hum_lbl, lv_color_hex(0xFFFFFF), 0);
    lvgl_sys::lv_obj_set_style_text_font(hum_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(hum_lbl, stats_x + 12, 0);

    // Temperature icon + value
    TEMP_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, TEMP_WIDTH, TEMP_HEIGHT,
    );
    TEMP_IMG_DSC.data_size = (TEMP_WIDTH * TEMP_HEIGHT * 3) as u32;
    TEMP_IMG_DSC.data = TEMP_DATA.as_ptr();

    let temp_icon = lvgl_sys::lv_img_create(unit);
    lvgl_sys::lv_img_set_src(temp_icon, &raw const TEMP_IMG_DSC as *const _);
    lvgl_sys::lv_obj_set_pos(temp_icon, stats_x + 38, 2);
    lvgl_sys::lv_obj_set_style_img_recolor(temp_icon, lv_color_hex(0xFFB74D), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(temp_icon, 255, 0);

    let temp_lbl = lvgl_sys::lv_label_create(unit);
    let temp_txt = CString::new(temp).unwrap();
    lvgl_sys::lv_label_set_text(temp_lbl, temp_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(temp_lbl, lv_color_hex(0xFFFFFF), 0);
    lvgl_sys::lv_obj_set_style_text_font(temp_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(temp_lbl, stats_x + 50, 0);

    // Inner housing container with gradient
    let housing_y: i16 = 18;
    let housing_h: i16 = h - 12 - housing_y;
    let housing_w: i16 = w - 12;

    let housing = lvgl_sys::lv_obj_create(unit);
    lvgl_sys::lv_obj_set_size(housing, housing_w, housing_h);
    lvgl_sys::lv_obj_set_pos(housing, 0, housing_y);
    lvgl_sys::lv_obj_clear_flag(housing, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(housing, lv_color_hex(0x2A2A2A), 0);
    lvgl_sys::lv_obj_set_style_bg_grad_color(housing, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_bg_grad_dir(housing, lvgl_sys::LV_GRAD_DIR_VER as u8, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(housing, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(housing, 5, 0);
    lvgl_sys::lv_obj_set_style_border_width(housing, 0, 0);
    set_style_pad_all(housing, 4);

    // Spools row
    let num_slots = slots.len().min(4) as i16;
    let spool_w: i16 = 40;
    let spool_step: i16 = 42;
    let spool_row_w = spool_w + (num_slots - 1) * spool_step;
    let start_x: i16 = (housing_w - spool_row_w) / 2 - 4;

    let mat_y: i16 = 8;
    let spool_y: i16 = 26;
    let badge_y: i16 = 82;
    let pct_y: i16 = 98;

    for (i, (material, color, slot_id, fill_pct, slot_active)) in slots.iter().enumerate() {
        let sx = start_x + (i as i16) * spool_step;
        let visual_spool_x = sx - 4;

        // Material label
        let mat_lbl = lvgl_sys::lv_label_create(housing);
        let mat_txt = if *color != 0 {
            CString::new(*material).unwrap()
        } else {
            CString::new("--").unwrap()
        };
        lvgl_sys::lv_label_set_text(mat_lbl, mat_txt.as_ptr());
        lvgl_sys::lv_obj_set_width(mat_lbl, spool_w);
        lvgl_sys::lv_obj_set_style_text_color(mat_lbl, lv_color_hex(COLOR_WHITE), 0);
        lvgl_sys::lv_obj_set_style_text_font(mat_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_set_style_text_align(mat_lbl, lvgl_sys::LV_TEXT_ALIGN_CENTER as u8, 0);
        lvgl_sys::lv_obj_set_pos(mat_lbl, visual_spool_x, mat_y);

        // Spool visual
        create_spool_large(housing, sx, spool_y, *color);

        // Slot badge
        let slot_badge = lvgl_sys::lv_obj_create(housing);
        lvgl_sys::lv_obj_set_size(slot_badge, 28, 14);
        lvgl_sys::lv_obj_set_pos(slot_badge, visual_spool_x + (spool_w - 28) / 2, badge_y);
        if *slot_active {
            lvgl_sys::lv_obj_set_style_bg_color(slot_badge, lv_color_hex(COLOR_ACCENT), 0);
        } else {
            lvgl_sys::lv_obj_set_style_bg_color(slot_badge, lv_color_hex(0x000000), 0);
            lvgl_sys::lv_obj_set_style_bg_opa(slot_badge, 153, 0);
        }
        lvgl_sys::lv_obj_set_style_radius(slot_badge, 7, 0);
        lvgl_sys::lv_obj_set_style_border_width(slot_badge, 0, 0);
        set_style_pad_all(slot_badge, 0);

        let slot_lbl = lvgl_sys::lv_label_create(slot_badge);
        let slot_txt = CString::new(*slot_id).unwrap();
        lvgl_sys::lv_label_set_text(slot_lbl, slot_txt.as_ptr());
        let txt_color = if *slot_active { 0x1A1A1A } else { COLOR_WHITE };
        lvgl_sys::lv_obj_set_style_text_color(slot_lbl, lv_color_hex(txt_color), 0);
        lvgl_sys::lv_obj_set_style_text_font(slot_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_align(slot_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

        // Fill percentage
        let pct_lbl = lvgl_sys::lv_label_create(housing);
        let pct_str = if *color != 0 && !fill_pct.is_empty() { *fill_pct } else { "--" };
        let pct_txt = CString::new(pct_str).unwrap();
        lvgl_sys::lv_label_set_text(pct_lbl, pct_txt.as_ptr());
        lvgl_sys::lv_obj_set_width(pct_lbl, spool_w);
        lvgl_sys::lv_obj_set_style_text_color(pct_lbl, lv_color_hex(COLOR_WHITE), 0);
        lvgl_sys::lv_obj_set_style_text_font(pct_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_set_style_text_align(pct_lbl, lvgl_sys::LV_TEXT_ALIGN_CENTER as u8, 0);
        lvgl_sys::lv_obj_set_pos(pct_lbl, visual_spool_x, pct_y);
    }
}

/// Create LARGER spool visual (40x52)
unsafe fn create_spool_large(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, color: u32) {
    let zoom_offset_x: i16 = -4;
    let zoom_offset_y: i16 = -5;

    let inner_left: i16 = 11;
    let inner_top: i16 = 6;
    let inner_w: i16 = 20;
    let inner_h: i16 = 40;

    SPOOL_CLEAN_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
        0, 0,
        SPOOL_WIDTH,
        SPOOL_HEIGHT,
    );
    SPOOL_CLEAN_IMG_DSC.data_size = (SPOOL_WIDTH * SPOOL_HEIGHT * 3) as u32;
    SPOOL_CLEAN_IMG_DSC.data = SPOOL_CLEAN_DATA.as_ptr();

    let spool_img = lvgl_sys::lv_img_create(parent);
    lvgl_sys::lv_img_set_src(spool_img, &raw const SPOOL_CLEAN_IMG_DSC as *const _);
    lvgl_sys::lv_obj_set_pos(spool_img, x, y);
    lvgl_sys::lv_img_set_zoom(spool_img, 320);

    if color == 0 {
        lvgl_sys::lv_obj_set_style_img_opa(spool_img, 50, 0);

        let inner_x = x + zoom_offset_x + inner_left;
        let inner_y = y + zoom_offset_y + inner_top;

        let empty_bg = lvgl_sys::lv_obj_create(parent);
        lvgl_sys::lv_obj_set_size(empty_bg, inner_w, inner_h);
        lvgl_sys::lv_obj_set_pos(empty_bg, inner_x, inner_y);
        lvgl_sys::lv_obj_set_style_bg_color(empty_bg, lv_color_hex(0x1A1A1A), 0);
        lvgl_sys::lv_obj_set_style_bg_opa(empty_bg, 255, 0);
        lvgl_sys::lv_obj_set_style_radius(empty_bg, 2, 0);
        lvgl_sys::lv_obj_set_style_border_width(empty_bg, 1, 0);
        lvgl_sys::lv_obj_set_style_border_color(empty_bg, lv_color_hex(0x3A3A3A), 0);
        set_style_pad_all(empty_bg, 0);
        lvgl_sys::lv_obj_clear_flag(empty_bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

        // Add "+" indicator centered
        let plus_lbl = lvgl_sys::lv_label_create(empty_bg);
        let plus_txt = CString::new("+").unwrap();
        lvgl_sys::lv_label_set_text(plus_lbl, plus_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(plus_lbl, lv_color_hex(0x505050), 0);
        lvgl_sys::lv_obj_set_style_text_font(plus_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
        lvgl_sys::lv_obj_align(plus_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 1);
    }

    if color != 0 {
        let tint = lvgl_sys::lv_obj_create(parent);
        lvgl_sys::lv_obj_set_size(tint, inner_w, inner_h);
        let tint_x = x + zoom_offset_x + inner_left;
        let tint_y = y + zoom_offset_y + inner_top;
        lvgl_sys::lv_obj_set_pos(tint, tint_x, tint_y);
        lvgl_sys::lv_obj_set_style_bg_color(tint, lv_color_hex(color), 0);
        lvgl_sys::lv_obj_set_style_bg_opa(tint, 217, 0);
        lvgl_sys::lv_obj_set_style_radius(tint, 3, 0);
        lvgl_sys::lv_obj_set_style_border_width(tint, 0, 0);
        set_style_pad_all(tint, 0);
    }
}

/// Create compact single-slot unit (HT-A, HT-B)
#[allow(dead_code)]
unsafe fn create_single_unit_compact(
    parent: *mut lvgl_sys::lv_obj_t,
    x: i16, y: i16, w: i16, h: i16,
    name: &str, nozzle: &str, humidity: &str, temp: &str,
    material: &str, color: u32, fill_pct: &str,
) {
    let unit = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(unit, w, h);
    lvgl_sys::lv_obj_set_pos(unit, x, y);
    lvgl_sys::lv_obj_clear_flag(unit, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(unit, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(unit, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(unit, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(unit, 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(unit, lv_color_hex(0x404040), 0);
    set_style_pad_all(unit, 6);

    let name_lbl = lvgl_sys::lv_label_create(unit);
    let name_txt = CString::new(name).unwrap();
    lvgl_sys::lv_label_set_text(name_lbl, name_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(name_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(name_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(name_lbl, 4, 0);

    if !nozzle.is_empty() {
        let name_width: i16 = (name.len() as i16) * 7 + 12;
        let badge_lbl = lvgl_sys::lv_label_create(unit);
        let badge_txt = CString::new(nozzle).unwrap();
        lvgl_sys::lv_label_set_text(badge_lbl, badge_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(badge_lbl, lv_color_hex(0x1A1A1A), 0);
        lvgl_sys::lv_obj_set_style_text_font(badge_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_set_style_bg_color(badge_lbl, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_bg_opa(badge_lbl, 255, 0);
        lvgl_sys::lv_obj_set_style_pad_left(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_style_pad_right(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_style_pad_top(badge_lbl, 0, 0);
        lvgl_sys::lv_obj_set_style_pad_bottom(badge_lbl, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_pos(badge_lbl, name_width, 3);
    }

    let housing_y: i16 = 18;
    let housing_h: i16 = h - 12 - housing_y;
    let housing_w: i16 = w - 12;

    let housing = lvgl_sys::lv_obj_create(unit);
    lvgl_sys::lv_obj_set_size(housing, housing_w, housing_h);
    lvgl_sys::lv_obj_set_pos(housing, 0, housing_y);
    lvgl_sys::lv_obj_clear_flag(housing, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(housing, lv_color_hex(0x2A2A2A), 0);
    lvgl_sys::lv_obj_set_style_bg_grad_color(housing, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_bg_grad_dir(housing, lvgl_sys::LV_GRAD_DIR_VER as u8, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(housing, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(housing, 5, 0);
    lvgl_sys::lv_obj_set_style_border_width(housing, 0, 0);
    set_style_pad_all(housing, 4);

    let stats_y: i16 = 2;
    HUMIDITY_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, HUMIDITY_WIDTH, HUMIDITY_HEIGHT,
    );
    HUMIDITY_IMG_DSC.data_size = (HUMIDITY_WIDTH * HUMIDITY_HEIGHT * 3) as u32;
    HUMIDITY_IMG_DSC.data = HUMIDITY_DATA.as_ptr();

    let hum_icon = lvgl_sys::lv_img_create(housing);
    lvgl_sys::lv_img_set_src(hum_icon, &raw const HUMIDITY_IMG_DSC as *const _);
    lvgl_sys::lv_obj_set_pos(hum_icon, 0, stats_y);
    lvgl_sys::lv_obj_set_style_img_recolor(hum_icon, lv_color_hex(0x4FC3F7), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(hum_icon, 255, 0);

    let hum_lbl = lvgl_sys::lv_label_create(housing);
    let hum_txt = CString::new(humidity).unwrap();
    lvgl_sys::lv_label_set_text(hum_lbl, hum_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(hum_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(hum_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(hum_lbl, 11, stats_y - 2);

    TEMP_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32, 0, 0, TEMP_WIDTH, TEMP_HEIGHT,
    );
    TEMP_IMG_DSC.data_size = (TEMP_WIDTH * TEMP_HEIGHT * 3) as u32;
    TEMP_IMG_DSC.data = TEMP_DATA.as_ptr();

    let temp_icon = lvgl_sys::lv_img_create(housing);
    lvgl_sys::lv_img_set_src(temp_icon, &raw const TEMP_IMG_DSC as *const _);
    lvgl_sys::lv_obj_set_pos(temp_icon, 40, stats_y);
    lvgl_sys::lv_obj_set_style_img_recolor(temp_icon, lv_color_hex(0xFFB74D), 0);
    lvgl_sys::lv_obj_set_style_img_recolor_opa(temp_icon, 255, 0);

    let temp_lbl = lvgl_sys::lv_label_create(housing);
    let temp_txt = CString::new(temp).unwrap();
    lvgl_sys::lv_label_set_text(temp_lbl, temp_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(temp_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(temp_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(temp_lbl, 52, stats_y - 2);

    let spool_x = (housing_w - 40) / 2;
    let mat_y: i16 = 24;
    let spool_y: i16 = 42;
    let badge_y: i16 = 98;
    let pct_y: i16 = 114;

    let mat_lbl = lvgl_sys::lv_label_create(housing);
    let mat_txt = CString::new(material).unwrap();
    lvgl_sys::lv_label_set_text(mat_lbl, mat_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(mat_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(mat_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(mat_lbl, spool_x + 4, mat_y);

    create_spool_large(housing, spool_x, spool_y, color);

    let slot_badge = lvgl_sys::lv_obj_create(housing);
    lvgl_sys::lv_obj_set_size(slot_badge, 36, 14);
    lvgl_sys::lv_obj_set_pos(slot_badge, (housing_w - 36) / 2, badge_y);
    lvgl_sys::lv_obj_set_style_bg_color(slot_badge, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(slot_badge, 153, 0);
    lvgl_sys::lv_obj_set_style_radius(slot_badge, 7, 0);
    lvgl_sys::lv_obj_set_style_border_width(slot_badge, 0, 0);
    set_style_pad_all(slot_badge, 0);

    let slot_lbl = lvgl_sys::lv_label_create(slot_badge);
    let slot_txt = CString::new(name).unwrap();
    lvgl_sys::lv_label_set_text(slot_lbl, slot_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(slot_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(slot_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(slot_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    let pct_lbl = lvgl_sys::lv_label_create(housing);
    let pct_txt = CString::new(fill_pct).unwrap();
    lvgl_sys::lv_label_set_text(pct_lbl, pct_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(pct_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(pct_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(pct_lbl, (housing_w - 20) / 2, pct_y);
}

/// Create compact external spool unit
#[allow(dead_code)]
unsafe fn create_ext_unit_compact(
    parent: *mut lvgl_sys::lv_obj_t,
    x: i16, y: i16, w: i16, h: i16,
    name: &str, nozzle: &str, material: &str, color: u32,
) {
    let unit = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(unit, w, h);
    lvgl_sys::lv_obj_set_pos(unit, x, y);
    lvgl_sys::lv_obj_clear_flag(unit, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(unit, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(unit, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(unit, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(unit, 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(unit, lv_color_hex(0x404040), 0);
    set_style_pad_all(unit, 6);

    let name_lbl = lvgl_sys::lv_label_create(unit);
    let name_txt = CString::new(name).unwrap();
    lvgl_sys::lv_label_set_text(name_lbl, name_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(name_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(name_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(name_lbl, 4, 0);

    if !nozzle.is_empty() {
        let name_width: i16 = (name.len() as i16) * 6 + 4;
        let badge_gap: i16 = 4;
        let badge_lbl = lvgl_sys::lv_label_create(unit);
        let badge_txt = CString::new(nozzle).unwrap();
        lvgl_sys::lv_label_set_text(badge_lbl, badge_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(badge_lbl, lv_color_hex(0x1A1A1A), 0);
        lvgl_sys::lv_obj_set_style_text_font(badge_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_set_style_bg_color(badge_lbl, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_bg_opa(badge_lbl, 255, 0);
        lvgl_sys::lv_obj_set_style_pad_left(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_style_pad_right(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_style_pad_top(badge_lbl, 0, 0);
        lvgl_sys::lv_obj_set_style_pad_bottom(badge_lbl, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(badge_lbl, 2, 0);
        lvgl_sys::lv_obj_set_pos(badge_lbl, name_width + badge_gap, 3);
    }

    let housing_y: i16 = 18;
    let housing_h: i16 = h - 12 - housing_y;
    let housing_w: i16 = w - 12;

    let housing = lvgl_sys::lv_obj_create(unit);
    lvgl_sys::lv_obj_set_size(housing, housing_w, housing_h);
    lvgl_sys::lv_obj_set_pos(housing, 0, housing_y);
    lvgl_sys::lv_obj_clear_flag(housing, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(housing, lv_color_hex(0x2A2A2A), 0);
    lvgl_sys::lv_obj_set_style_bg_grad_color(housing, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_bg_grad_dir(housing, lvgl_sys::LV_GRAD_DIR_VER as u8, 0);
    lvgl_sys::lv_obj_set_style_bg_opa(housing, 255, 0);
    lvgl_sys::lv_obj_set_style_radius(housing, 5, 0);
    lvgl_sys::lv_obj_set_style_border_width(housing, 0, 0);
    set_style_pad_all(housing, 4);

    let mat_y: i16 = 16;
    let spool_size: i16 = 70;
    let spool_y: i16 = 34;
    let badge_y: i16 = 110;

    let mat_lbl = lvgl_sys::lv_label_create(housing);
    let mat_txt = CString::new(material).unwrap();
    lvgl_sys::lv_label_set_text(mat_lbl, mat_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(mat_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(mat_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(mat_lbl, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, mat_y);

    let outer = lvgl_sys::lv_obj_create(housing);
    lvgl_sys::lv_obj_set_size(outer, spool_size, spool_size);
    lvgl_sys::lv_obj_align(outer, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, spool_y);
    lvgl_sys::lv_obj_clear_flag(outer, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(outer, lv_color_hex(color), 0);
    lvgl_sys::lv_obj_set_style_radius(outer, spool_size / 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(outer, lv_color_hex(lighten_color(color, 20)), 0);
    lvgl_sys::lv_obj_set_style_border_width(outer, 2, 0);
    set_style_pad_all(outer, 0);

    let inner_size: i16 = 20;
    let inner = lvgl_sys::lv_obj_create(outer);
    lvgl_sys::lv_obj_set_size(inner, inner_size, inner_size);
    lvgl_sys::lv_obj_align(inner, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_clear_flag(inner, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(inner, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(inner, inner_size / 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(inner, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_border_width(inner, 1, 0);
    set_style_pad_all(inner, 0);

    let badge_w: i16 = 32;
    let slot_badge = lvgl_sys::lv_obj_create(housing);
    lvgl_sys::lv_obj_set_size(slot_badge, badge_w, 16);
    lvgl_sys::lv_obj_align(slot_badge, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, badge_y);
    lvgl_sys::lv_obj_set_style_bg_color(slot_badge, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(slot_badge, 153, 0);
    lvgl_sys::lv_obj_set_style_radius(slot_badge, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(slot_badge, 0, 0);
    set_style_pad_all(slot_badge, 0);

    let slot_lbl = lvgl_sys::lv_label_create(slot_badge);
    let slot_txt = CString::new(name).unwrap();
    lvgl_sys::lv_label_set_text(slot_lbl, slot_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(slot_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(slot_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(slot_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
}

/// Create Settings Page 2 (Hardware/System)
unsafe fn create_settings_2_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Settings
    create_status_bar_with_back(scr, "Hardware & System", btn_back_cb);

    // Content area
    let content_y: i16 = 52;

    // Hardware section
    create_section_header(scr, 16, content_y, "HARDWARE");

    let scale_row = create_settings_row_with_icon(scr, 16, content_y + 28, "scale", "Scale Calibration", "Last: 2d ago", "", true);
    lvgl_sys::lv_obj_add_event_cb(scale_row, Some(btn_scale_calibration_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

    let nfc_row = create_settings_row_with_icon(scr, 16, content_y + 76, "nfc", "NFC Reader", "Ready", "green", true);
    lvgl_sys::lv_obj_add_event_cb(nfc_row, Some(btn_nfc_reader_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

    let display_row = create_settings_row_with_icon(scr, 16, content_y + 124, "display", "Display Brightness", "80%", "", true);
    lvgl_sys::lv_obj_add_event_cb(display_row, Some(btn_display_brightness_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

    // System section
    let system_y = content_y + 200;
    create_section_header(scr, 16, system_y, "SYSTEM");

    let _update_row = create_settings_row_with_icon(scr, 16, system_y + 28, "update", "Check for Updates", "v1.0.2", "", true);
    // No navigation for update row - just info

    let advanced_row = create_settings_row_with_icon(scr, 16, system_y + 76, "gear", "Advanced Settings", "", "", true);
    lvgl_sys::lv_obj_add_event_cb(advanced_row, Some(btn_advanced_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

    let about_row = create_settings_row_with_icon(scr, 16, system_y + 124, "info", "About SpoolBuddy", "", "", true);
    lvgl_sys::lv_obj_add_event_cb(about_row, Some(btn_about_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

    scr
}

// Settings tab state
static mut SETTINGS_TAB_PANELS: [*mut lvgl_sys::lv_obj_t; 4] = [ptr::null_mut(); 4];
static mut SETTINGS_TAB_BTNS: [*mut lvgl_sys::lv_obj_t; 4] = [ptr::null_mut(); 4];
static mut SETTINGS_ACTIVE_TAB: usize = 0;

/// Settings tab click callback
unsafe extern "C" fn settings_tab_cb(e: *mut lvgl_sys::lv_event_t) {
    let target = lvgl_sys::lv_event_get_target(e);

    // Find which tab was clicked
    for i in 0..4 {
        if SETTINGS_TAB_BTNS[i] == target {
            // Update active tab
            SETTINGS_ACTIVE_TAB = i;

            // Update tab button styles
            for j in 0..4 {
                if j == i {
                    // Active tab
                    lvgl_sys::lv_obj_set_style_bg_color(SETTINGS_TAB_BTNS[j], lv_color_hex(COLOR_ACCENT), 0);
                    lvgl_sys::lv_obj_clear_flag(SETTINGS_TAB_PANELS[j], lvgl_sys::LV_OBJ_FLAG_HIDDEN);
                } else {
                    // Inactive tab
                    lvgl_sys::lv_obj_set_style_bg_color(SETTINGS_TAB_BTNS[j], lv_color_hex(0x2D2D2D), 0);
                    lvgl_sys::lv_obj_add_flag(SETTINGS_TAB_PANELS[j], lvgl_sys::LV_OBJ_FLAG_HIDDEN);
                }
            }
            break;
        }
    }
}

/// Create Settings screen with tabbed layout
unsafe fn create_settings_screen_fn() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Home
    create_status_bar_with_back(scr, "Settings", btn_back_cb);

    // Tab bar container
    let tab_bar = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(tab_bar, 800, 44);
    lvgl_sys::lv_obj_set_pos(tab_bar, 0, 44);
    lvgl_sys::lv_obj_set_style_bg_color(tab_bar, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_border_width(tab_bar, 0, 0);
    lvgl_sys::lv_obj_set_style_radius(tab_bar, 0, 0);
    set_style_pad_all(tab_bar, 0);
    lvgl_sys::lv_obj_clear_flag(tab_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    let tab_names = ["Network", "Printers", "Hardware", "System"];
    let tab_width = 200i16;

    for (i, name) in tab_names.iter().enumerate() {
        let btn = lvgl_sys::lv_btn_create(tab_bar);
        lvgl_sys::lv_obj_set_size(btn, tab_width - 4, 36);
        lvgl_sys::lv_obj_set_pos(btn, (i as i16) * tab_width + 2, 4);
        lvgl_sys::lv_obj_set_style_radius(btn, 6, 0);
        lvgl_sys::lv_obj_set_style_shadow_width(btn, 0, 0);
        lvgl_sys::lv_obj_set_style_border_width(btn, 0, 0);

        if i == 0 {
            lvgl_sys::lv_obj_set_style_bg_color(btn, lv_color_hex(COLOR_ACCENT), 0);
        } else {
            lvgl_sys::lv_obj_set_style_bg_color(btn, lv_color_hex(0x2D2D2D), 0);
        }

        let lbl = lvgl_sys::lv_label_create(btn);
        let text = CString::new(*name).unwrap();
        lvgl_sys::lv_label_set_text(lbl, text.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_WHITE), 0);
        lvgl_sys::lv_obj_align(lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

        lvgl_sys::lv_obj_add_event_cb(btn, Some(settings_tab_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
        SETTINGS_TAB_BTNS[i] = btn;
    }

    // Content area (below tab bar)
    let content_y: i16 = 96; // 44 status + 44 tabs + 8 padding
    let content_h: i16 = 384; // 480 - 96

    // Panel 0: Network
    let panel0 = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(panel0, 800, content_h);
    lvgl_sys::lv_obj_set_pos(panel0, 0, content_y);
    lvgl_sys::lv_obj_set_style_bg_opa(panel0, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(panel0, 0, 0);
    set_style_pad_all(panel0, 0);
    lvgl_sys::lv_obj_clear_flag(panel0, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    let wifi_row = create_settings_row(panel0, 16, 8, "WiFi Network", "NYHC!", "green", true);
    lvgl_sys::lv_obj_add_event_cb(wifi_row, Some(btn_wifi_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    let backend_row = create_settings_row(panel0, 16, 60, "Backend Server", "192.168.1.100:3000", "", true);
    lvgl_sys::lv_obj_add_event_cb(backend_row, Some(btn_backend_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    let mqtt_row = create_settings_row(panel0, 16, 112, "MQTT Broker", "Auto-discover", "", true);
    lvgl_sys::lv_obj_add_event_cb(mqtt_row, Some(btn_backend_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    SETTINGS_TAB_PANELS[0] = panel0;

    // Panel 1: Printers
    let panel1 = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(panel1, 800, content_h);
    lvgl_sys::lv_obj_set_pos(panel1, 0, content_y);
    lvgl_sys::lv_obj_set_style_bg_opa(panel1, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(panel1, 0, 0);
    set_style_pad_all(panel1, 0);
    lvgl_sys::lv_obj_clear_flag(panel1, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_add_flag(panel1, lvgl_sys::LV_OBJ_FLAG_HIDDEN);

    create_settings_row(panel1, 16, 8, "X1C-Studio", "Connected", "green", true);
    create_settings_row(panel1, 16, 60, "P1S-Garage", "Offline", "gray", true);
    let add_printer_row = create_settings_row(panel1, 16, 112, "Add Printer...", "", "", true);
    lvgl_sys::lv_obj_add_event_cb(add_printer_row, Some(btn_add_printer_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    SETTINGS_TAB_PANELS[1] = panel1;

    // Panel 2: Hardware
    let panel2 = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(panel2, 800, content_h);
    lvgl_sys::lv_obj_set_pos(panel2, 0, content_y);
    lvgl_sys::lv_obj_set_style_bg_opa(panel2, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(panel2, 0, 0);
    set_style_pad_all(panel2, 0);
    lvgl_sys::lv_obj_clear_flag(panel2, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_add_flag(panel2, lvgl_sys::LV_OBJ_FLAG_HIDDEN);

    let scale_row = create_settings_row(panel2, 16, 8, "Scale Calibration", "Last: 2d ago", "", true);
    lvgl_sys::lv_obj_add_event_cb(scale_row, Some(btn_scale_calibration_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    let nfc_row = create_settings_row(panel2, 16, 60, "NFC Reader", "Ready", "green", true);
    lvgl_sys::lv_obj_add_event_cb(nfc_row, Some(btn_nfc_reader_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    let display_row = create_settings_row(panel2, 16, 112, "Display Brightness", "80%", "", true);
    lvgl_sys::lv_obj_add_event_cb(display_row, Some(btn_display_brightness_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    SETTINGS_TAB_PANELS[2] = panel2;

    // Panel 3: System
    let panel3 = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(panel3, 800, content_h);
    lvgl_sys::lv_obj_set_pos(panel3, 0, content_y);
    lvgl_sys::lv_obj_set_style_bg_opa(panel3, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(panel3, 0, 0);
    set_style_pad_all(panel3, 0);
    lvgl_sys::lv_obj_clear_flag(panel3, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_add_flag(panel3, lvgl_sys::LV_OBJ_FLAG_HIDDEN);

    let advanced_row = create_settings_row(panel3, 16, 8, "Advanced Settings", "", "", true);
    lvgl_sys::lv_obj_add_event_cb(advanced_row, Some(btn_advanced_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    let about_row = create_settings_row(panel3, 16, 60, "About SpoolBuddy", "v0.1", "", true);
    lvgl_sys::lv_obj_add_event_cb(about_row, Some(btn_about_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
    create_settings_row(panel3, 16, 112, "Check for Updates", "v1.0.2 (latest)", "", true);
    SETTINGS_TAB_PANELS[3] = panel3;

    scr
}

/// Helper: Create clean About page row with green accent
unsafe fn create_about_row(parent: *mut lvgl_sys::lv_obj_t, y: i16, label: &str, value: &str, show_separator: bool) {
    // Row container
    let row = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(row, 728, 40);
    lvgl_sys::lv_obj_set_pos(row, 16, y);
    lvgl_sys::lv_obj_clear_flag(row, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(row, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(row, 0, 0);
    set_style_pad_all(row, 0);

    // Green accent bar on left
    let accent = lvgl_sys::lv_obj_create(row);
    lvgl_sys::lv_obj_set_size(accent, 3, 24);
    lvgl_sys::lv_obj_align(accent, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 0, 0);
    lvgl_sys::lv_obj_clear_flag(accent, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(accent, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(accent, 2, 0);
    lvgl_sys::lv_obj_set_style_border_width(accent, 0, 0);
    set_style_pad_all(accent, 0);

    // Label on left
    let label_lbl = lvgl_sys::lv_label_create(row);
    let label_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(label_lbl, label_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(label_lbl, lv_color_hex(0x888888), 0);
    lvgl_sys::lv_obj_set_style_text_font(label_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(label_lbl, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 16, 0);

    // Value on right
    let value_lbl = lvgl_sys::lv_label_create(row);
    let value_txt = CString::new(value).unwrap();
    lvgl_sys::lv_label_set_text(value_lbl, value_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(value_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(value_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(value_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, 0, 0);

    // Separator line
    if show_separator {
        let line = lvgl_sys::lv_obj_create(parent);
        lvgl_sys::lv_obj_set_size(line, 712, 1);
        lvgl_sys::lv_obj_set_pos(line, 24, y + 42);
        lvgl_sys::lv_obj_clear_flag(line, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
        lvgl_sys::lv_obj_set_style_bg_color(line, lv_color_hex(0x333333), 0);
        lvgl_sys::lv_obj_set_style_border_width(line, 0, 0);
        lvgl_sys::lv_obj_set_style_radius(line, 0, 0);
        set_style_pad_all(line, 0);
    }
}

/// Create About screen
unsafe fn create_about_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Settings-2
    create_status_bar_with_back(scr, "About SpoolBuddy", btn_back_cb);

    // Large logo directly (3x scale)
    LOGO_IMG_DSC.header._bitfield_1 = lvgl_sys::lv_img_header_t::new_bitfield_1(
        lvgl_sys::LV_IMG_CF_TRUE_COLOR_ALPHA as u32,
        0, 0,
        LOGO_WIDTH,
        LOGO_HEIGHT,
    );
    LOGO_IMG_DSC.data_size = (LOGO_WIDTH * LOGO_HEIGHT * 3) as u32;
    LOGO_IMG_DSC.data = LOGO_DATA.as_ptr();

    let logo_img = lvgl_sys::lv_img_create(scr);
    lvgl_sys::lv_img_set_src(logo_img, &raw const LOGO_IMG_DSC as *const _);
    lvgl_sys::lv_obj_align(logo_img, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 65);
    lvgl_sys::lv_img_set_zoom(logo_img, 768); // 3x zoom (256 = 1x)

    // Main info card - clean design
    let info_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(info_card, 760, 230);
    lvgl_sys::lv_obj_align(info_card, lvgl_sys::LV_ALIGN_TOP_MID as u8, 0, 185);
    lvgl_sys::lv_obj_clear_flag(info_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(info_card, lv_color_hex(0x222222), 0);
    lvgl_sys::lv_obj_set_style_radius(info_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_color(info_card, lv_color_hex(0x333333), 0);
    lvgl_sys::lv_obj_set_style_border_width(info_card, 1, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(info_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(info_card, 20, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(info_card, 5, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(info_card, 80, 0);
    set_style_pad_all(info_card, 0);

    // Clean info rows with green left accent
    create_about_row(info_card, 8, "Version", "v1.0.2", true);
    create_about_row(info_card, 52, "Build", "2025.12.15.1423", true);
    create_about_row(info_card, 96, "Hardware", "ESP32-S3 + ELECROW 7\"", true);
    create_about_row(info_card, 140, "Memory", "4.2 MB / 8 MB", true);
    create_about_row(info_card, 184, "License", "MIT License", false);

    // Footer
    let footer_lbl = lvgl_sys::lv_label_create(scr);
    let footer_txt = CString::new("Made with care for the 3D printing community").unwrap();
    lvgl_sys::lv_label_set_text(footer_lbl, footer_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(footer_lbl, lv_color_hex(0x555555), 0);
    lvgl_sys::lv_obj_set_style_text_font(footer_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(footer_lbl, lvgl_sys::LV_ALIGN_BOTTOM_MID as u8, 0, -16);

    scr
}

/// Create Scale Calibration screen
unsafe fn create_scale_calibration_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Settings-2
    create_status_bar_with_back(scr, "Scale Calibration", btn_back_cb);

    // Status card with shadow
    let status_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(status_card, 768, 80);
    lvgl_sys::lv_obj_set_pos(status_card, 16, 52);
    lvgl_sys::lv_obj_clear_flag(status_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(status_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(status_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(status_card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(status_card, 20, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(status_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(status_card, 80, 0);
    set_style_pad_all(status_card, 16);

    // Scale icon with circular green background
    let icon_bg = lvgl_sys::lv_obj_create(status_card);
    lvgl_sys::lv_obj_set_size(icon_bg, 48, 48);
    lvgl_sys::lv_obj_set_pos(icon_bg, 8, 0);
    lvgl_sys::lv_obj_clear_flag(icon_bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(icon_bg, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(icon_bg, 40, 0);
    lvgl_sys::lv_obj_set_style_radius(icon_bg, 24, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon_bg, 0, 0);
    set_style_pad_all(icon_bg, 0);

    // Scale L-bracket icon inside circle
    let bracket = lvgl_sys::lv_obj_create(icon_bg);
    lvgl_sys::lv_obj_set_size(bracket, 22, 22);
    lvgl_sys::lv_obj_set_pos(bracket, 13, 10);
    lvgl_sys::lv_obj_clear_flag(bracket, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(bracket, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(bracket, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(bracket, 3, 0);
    lvgl_sys::lv_obj_set_style_border_side(bracket, (lvgl_sys::LV_BORDER_SIDE_LEFT | lvgl_sys::LV_BORDER_SIDE_BOTTOM) as u8, 0);
    lvgl_sys::lv_obj_set_style_radius(bracket, 0, 0);
    set_style_pad_all(bracket, 0);

    let status_title = lvgl_sys::lv_label_create(status_card);
    let status_title_txt = CString::new("Scale Calibrated").unwrap();
    lvgl_sys::lv_label_set_text(status_title, status_title_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(status_title, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(status_title, &lvgl_sys::lv_font_montserrat_16, 0);
    lvgl_sys::lv_obj_set_pos(status_title, 72, 6);

    let status_sub = lvgl_sys::lv_label_create(status_card);
    let status_sub_txt = CString::new("Last calibration: 2 days ago").unwrap();
    lvgl_sys::lv_label_set_text(status_sub, status_sub_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(status_sub, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(status_sub, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(status_sub, 72, 28);

    // Calibration steps section
    let steps_y: i16 = 148;
    create_section_header(scr, 16, steps_y, "CALIBRATION STEPS");

    create_numbered_step(scr, 16, steps_y + 24, 1, "Remove all items from the scale and press \"Tare\"");
    create_numbered_step(scr, 16, steps_y + 68, 2, "Place a known weight (500g recommended) on scale");
    create_numbered_step(scr, 16, steps_y + 112, 3, "Enter the exact weight and press \"Calibrate\"");

    // Calibration weight input
    let weight_y: i16 = 300;
    create_section_header(scr, 16, weight_y, "CALIBRATION WEIGHT (GRAMS)");

    let input_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(input_card, 768, 50);
    lvgl_sys::lv_obj_set_pos(input_card, 16, weight_y + 24);
    lvgl_sys::lv_obj_clear_flag(input_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(input_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(input_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(input_card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(input_card, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(input_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(input_card, 60, 0);
    set_style_pad_all(input_card, 12);

    let input_lbl = lvgl_sys::lv_label_create(input_card);
    let input_txt = CString::new("500").unwrap();
    lvgl_sys::lv_label_set_text(input_lbl, input_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(input_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(input_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
    lvgl_sys::lv_obj_set_pos(input_lbl, 4, 6);

    // Buttons
    let btn_y: i16 = 410;

    let tare_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(tare_btn, 370, 50);
    lvgl_sys::lv_obj_set_pos(tare_btn, 16, btn_y);
    lvgl_sys::lv_obj_set_style_bg_color(tare_btn, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_radius(tare_btn, 12, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(tare_btn, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(tare_btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(tare_btn, 60, 0);

    let tare_lbl = lvgl_sys::lv_label_create(tare_btn);
    let tare_txt = CString::new("Tare").unwrap();
    lvgl_sys::lv_label_set_text(tare_lbl, tare_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(tare_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(tare_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(tare_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    let calibrate_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(calibrate_btn, 370, 50);
    lvgl_sys::lv_obj_set_pos(calibrate_btn, 414, btn_y);
    lvgl_sys::lv_obj_set_style_bg_color(calibrate_btn, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(calibrate_btn, 12, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(calibrate_btn, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(calibrate_btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(calibrate_btn, 60, 0);

    let calibrate_lbl = lvgl_sys::lv_label_create(calibrate_btn);
    let calibrate_txt = CString::new("Calibrate").unwrap();
    lvgl_sys::lv_label_set_text(calibrate_lbl, calibrate_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(calibrate_lbl, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_text_font(calibrate_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(calibrate_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    scr
}

/// Helper: Create info row with optional separator
unsafe fn create_info_row_with_separator(parent: *mut lvgl_sys::lv_obj_t, y: i16, label: &str, value: &str, show_separator: bool) {
    let lbl = lvgl_sys::lv_label_create(parent);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(lbl, 20, y + 14);

    let val = lvgl_sys::lv_label_create(parent);
    let val_txt = CString::new(value).unwrap();
    lvgl_sys::lv_label_set_text(val, val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(val, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(val, lvgl_sys::LV_ALIGN_TOP_RIGHT as u8, -20, y + 14);

    if show_separator {
        let sep = lvgl_sys::lv_obj_create(parent);
        lvgl_sys::lv_obj_set_size(sep, 728, 1);
        lvgl_sys::lv_obj_set_pos(sep, 20, y + 43);
        lvgl_sys::lv_obj_clear_flag(sep, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
        lvgl_sys::lv_obj_set_style_bg_color(sep, lv_color_hex(0x404040), 0);
        lvgl_sys::lv_obj_set_style_radius(sep, 0, 0);
        lvgl_sys::lv_obj_set_style_border_width(sep, 0, 0);
        set_style_pad_all(sep, 0);
    }
}

/// Helper: Create toggle row
unsafe fn create_toggle_row(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, enabled: bool) {
    let row = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(row, 768, 40);
    lvgl_sys::lv_obj_set_pos(row, x, y);
    lvgl_sys::lv_obj_clear_flag(row, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(row, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(row, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(row, 0, 0);
    set_style_pad_all(row, 0);

    let lbl = lvgl_sys::lv_label_create(row);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(lbl, 16, 11);

    // Toggle switch
    let toggle_w: i16 = 50;
    let toggle_h: i16 = 26;
    let toggle = lvgl_sys::lv_obj_create(row);
    lvgl_sys::lv_obj_set_size(toggle, toggle_w, toggle_h);
    lvgl_sys::lv_obj_align(toggle, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -16, 0);
    lvgl_sys::lv_obj_clear_flag(toggle, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_radius(toggle, 13, 0);
    lvgl_sys::lv_obj_set_style_border_width(toggle, 0, 0);
    set_style_pad_all(toggle, 0);

    if enabled {
        lvgl_sys::lv_obj_set_style_bg_color(toggle, lv_color_hex(COLOR_ACCENT), 0);
    } else {
        lvgl_sys::lv_obj_set_style_bg_color(toggle, lv_color_hex(0x505050), 0);
    }

    // Knob
    let knob = lvgl_sys::lv_obj_create(toggle);
    lvgl_sys::lv_obj_set_size(knob, 22, 22);
    lvgl_sys::lv_obj_clear_flag(knob, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(knob, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_radius(knob, 11, 0);
    lvgl_sys::lv_obj_set_style_border_width(knob, 0, 0);
    set_style_pad_all(knob, 0);

    if enabled {
        lvgl_sys::lv_obj_set_pos(knob, 26, 2);
    } else {
        lvgl_sys::lv_obj_set_pos(knob, 2, 2);
    }
}

/// Helper: Create icon row (for data management)
unsafe fn create_icon_row(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, _value: &str, icon_type: &str) {
    let row = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(row, 768, 40);
    lvgl_sys::lv_obj_set_pos(row, x, y);
    lvgl_sys::lv_obj_clear_flag(row, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(row, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(row, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(row, 0, 0);
    set_style_pad_all(row, 0);

    // Icon placeholder
    let icon = lvgl_sys::lv_obj_create(row);
    lvgl_sys::lv_obj_set_size(icon, 20, 20);
    lvgl_sys::lv_obj_set_pos(icon, 16, 10);
    lvgl_sys::lv_obj_clear_flag(icon, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_radius(icon, 4, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon, 0, 0);
    set_style_pad_all(icon, 0);

    let icon_color = if icon_type == "danger" { 0xE57373 } else { COLOR_ACCENT };
    lvgl_sys::lv_obj_set_style_bg_color(icon, lv_color_hex(icon_color), 0);

    let lbl = lvgl_sys::lv_label_create(row);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());

    let text_color = if icon_type == "danger" { 0xE57373 } else { COLOR_WHITE };
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(text_color), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(lbl, 48, 11);

    // Arrow
    let arrow_lbl = lvgl_sys::lv_label_create(row);
    let arrow_txt = CString::new(">").unwrap();
    lvgl_sys::lv_label_set_text(arrow_lbl, arrow_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(arrow_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(arrow_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(arrow_lbl, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -16, 0);
}

/// Create NFC Reader screen
unsafe fn create_nfc_reader_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Settings-2
    create_status_bar_with_back(scr, "NFC Reader", btn_back_cb);

    // Status card with shadow
    let status_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(status_card, 768, 80);
    lvgl_sys::lv_obj_set_pos(status_card, 16, 52);
    lvgl_sys::lv_obj_clear_flag(status_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(status_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(status_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(status_card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(status_card, 20, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(status_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(status_card, 80, 0);
    set_style_pad_all(status_card, 16);

    // NFC icon with circular green background
    let icon_bg = lvgl_sys::lv_obj_create(status_card);
    lvgl_sys::lv_obj_set_size(icon_bg, 48, 48);
    lvgl_sys::lv_obj_set_pos(icon_bg, 8, 0);
    lvgl_sys::lv_obj_clear_flag(icon_bg, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(icon_bg, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_bg_opa(icon_bg, 40, 0);
    lvgl_sys::lv_obj_set_style_radius(icon_bg, 24, 0);
    lvgl_sys::lv_obj_set_style_border_width(icon_bg, 0, 0);
    set_style_pad_all(icon_bg, 0);

    // NFC chip icon inside the circle
    let nfc_outer = lvgl_sys::lv_obj_create(icon_bg);
    lvgl_sys::lv_obj_set_size(nfc_outer, 26, 26);
    lvgl_sys::lv_obj_set_pos(nfc_outer, 11, 11);
    lvgl_sys::lv_obj_clear_flag(nfc_outer, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(nfc_outer, lvgl_sys::LV_OPA_TRANSP as u8, 0);
    lvgl_sys::lv_obj_set_style_border_color(nfc_outer, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_border_width(nfc_outer, 2, 0);
    lvgl_sys::lv_obj_set_style_radius(nfc_outer, 4, 0);
    set_style_pad_all(nfc_outer, 0);

    // Inner chip dot
    let nfc_inner = lvgl_sys::lv_obj_create(icon_bg);
    lvgl_sys::lv_obj_set_size(nfc_inner, 10, 10);
    lvgl_sys::lv_obj_set_pos(nfc_inner, 19, 19);
    lvgl_sys::lv_obj_clear_flag(nfc_inner, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(nfc_inner, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(nfc_inner, 2, 0);
    lvgl_sys::lv_obj_set_style_border_width(nfc_inner, 0, 0);
    set_style_pad_all(nfc_inner, 0);

    let status_title = lvgl_sys::lv_label_create(status_card);
    let status_title_txt = CString::new("NFC Reader Ready").unwrap();
    lvgl_sys::lv_label_set_text(status_title, status_title_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(status_title, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(status_title, &lvgl_sys::lv_font_montserrat_16, 0);
    lvgl_sys::lv_obj_set_pos(status_title, 72, 6);

    let status_sub = lvgl_sys::lv_label_create(status_card);
    let status_sub_txt = CString::new("PN5180 - Firmware v1.6").unwrap();
    lvgl_sys::lv_label_set_text(status_sub, status_sub_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(status_sub, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(status_sub, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(status_sub, 72, 28);

    // Info card with shadow
    let info_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(info_card, 768, 180);
    lvgl_sys::lv_obj_set_pos(info_card, 16, 148);
    lvgl_sys::lv_obj_clear_flag(info_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(info_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(info_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(info_card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(info_card, 20, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(info_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(info_card, 80, 0);
    set_style_pad_all(info_card, 0);

    // Info rows with separators
    create_info_row_with_separator(info_card, 0, "Reader Type", "PN5180 NFC/RFID", true);
    create_info_row_with_separator(info_card, 44, "Connection", "SPI", true);
    create_info_row_with_separator(info_card, 88, "Tags Read", "147 total", true);
    create_info_row_with_separator(info_card, 132, "Last Read", "5 minutes ago", false);

    // Test button with shadow
    let test_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(test_btn, 768, 50);
    lvgl_sys::lv_obj_set_pos(test_btn, 16, 410);
    lvgl_sys::lv_obj_set_style_bg_color(test_btn, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_radius(test_btn, 12, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(test_btn, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(test_btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(test_btn, 60, 0);

    let test_lbl = lvgl_sys::lv_label_create(test_btn);
    let test_txt = CString::new("Test NFC Reader").unwrap();
    lvgl_sys::lv_label_set_text(test_lbl, test_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(test_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(test_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(test_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    scr
}

/// Create Display Brightness screen
unsafe fn create_display_brightness_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Settings-2
    create_status_bar_with_back(scr, "Display Brightness", btn_back_cb);

    // Brightness slider card
    let brightness_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(brightness_card, 768, 80);
    lvgl_sys::lv_obj_set_pos(brightness_card, 16, 52);
    lvgl_sys::lv_obj_clear_flag(brightness_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(brightness_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(brightness_card, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(brightness_card, 0, 0);
    set_style_pad_all(brightness_card, 16);

    let brightness_lbl = lvgl_sys::lv_label_create(brightness_card);
    let brightness_txt = CString::new("Brightness").unwrap();
    lvgl_sys::lv_label_set_text(brightness_lbl, brightness_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(brightness_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(brightness_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(brightness_lbl, 0, 0);

    let pct_lbl = lvgl_sys::lv_label_create(brightness_card);
    let pct_txt = CString::new("80%").unwrap();
    lvgl_sys::lv_label_set_text(pct_lbl, pct_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(pct_lbl, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_text_font(pct_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(pct_lbl, lvgl_sys::LV_ALIGN_TOP_RIGHT as u8, 0, 0);

    // Slider track
    let track = lvgl_sys::lv_obj_create(brightness_card);
    lvgl_sys::lv_obj_set_size(track, 700, 12);
    lvgl_sys::lv_obj_set_pos(track, 0, 32);
    lvgl_sys::lv_obj_clear_flag(track, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(track, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_radius(track, 6, 0);
    lvgl_sys::lv_obj_set_style_border_width(track, 0, 0);
    set_style_pad_all(track, 0);

    // Slider fill (80%)
    let fill = lvgl_sys::lv_obj_create(track);
    lvgl_sys::lv_obj_set_size(fill, 560, 12);
    lvgl_sys::lv_obj_set_pos(fill, 0, 0);
    lvgl_sys::lv_obj_clear_flag(fill, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(fill, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(fill, 6, 0);
    lvgl_sys::lv_obj_set_style_border_width(fill, 0, 0);
    set_style_pad_all(fill, 0);

    // Slider knob
    let knob = lvgl_sys::lv_obj_create(brightness_card);
    lvgl_sys::lv_obj_set_size(knob, 20, 20);
    lvgl_sys::lv_obj_set_pos(knob, 550, 28);
    lvgl_sys::lv_obj_clear_flag(knob, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(knob, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_radius(knob, 10, 0);
    lvgl_sys::lv_obj_set_style_border_width(knob, 0, 0);
    set_style_pad_all(knob, 0);

    // Options section
    let options_y: i16 = 148;
    create_section_header(scr, 16, options_y, "OPTIONS");

    create_toggle_row(scr, 16, options_y + 24, "Auto Brightness", false);
    create_toggle_row(scr, 16, options_y + 68, "Screen Timeout", true);

    // Timeout duration
    let timeout_y: i16 = 260;
    create_section_header(scr, 16, timeout_y, "TIMEOUT DURATION");

    let timeout_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(timeout_card, 768, 50);
    lvgl_sys::lv_obj_set_pos(timeout_card, 16, timeout_y + 24);
    lvgl_sys::lv_obj_clear_flag(timeout_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(timeout_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(timeout_card, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(timeout_card, 0, 0);
    set_style_pad_all(timeout_card, 12);

    let timeout_lbl = lvgl_sys::lv_label_create(timeout_card);
    let timeout_txt = CString::new("5 minutes").unwrap();
    lvgl_sys::lv_label_set_text(timeout_lbl, timeout_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(timeout_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(timeout_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(timeout_lbl, 4, 8);

    scr
}

/// Create Advanced Settings screen
unsafe fn create_advanced_settings_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button to Settings-2
    create_status_bar_with_back(scr, "Advanced Settings", btn_back_cb);

    // Developer Options section
    let dev_y: i16 = 52;
    create_section_header(scr, 16, dev_y, "DEVELOPER OPTIONS");

    create_toggle_row(scr, 16, dev_y + 24, "Debug Logging", false);
    create_toggle_row(scr, 16, dev_y + 68, "Show FPS Counter", false);
    create_toggle_row(scr, 16, dev_y + 112, "Serial Console", true);

    // Data Management section
    let data_y: i16 = 200;
    create_section_header(scr, 16, data_y, "DATA MANAGEMENT");

    create_icon_row(scr, 16, data_y + 24, "Export Data", "", "download");
    create_icon_row(scr, 16, data_y + 68, "Import Data", "", "upload");
    create_icon_row(scr, 16, data_y + 112, "Factory Reset", "", "danger");

    scr
}

/// Create WiFi Settings screen
unsafe fn create_wifi_settings_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    create_status_bar_with_back(scr, "WiFi Settings", btn_back_cb);

    // Current connection section
    let section_y: i16 = 60;
    create_section_header(scr, 16, section_y, "CURRENT CONNECTION");

    create_settings_row(scr, 16, section_y + 28, "Network", "NYHC!", "green", false);
    create_settings_row(scr, 16, section_y + 72, "IP Address", "192.168.255.123", "", false);
    create_settings_row(scr, 16, section_y + 116, "Signal Strength", "Excellent (-45 dBm)", "", false);

    // Available Networks section
    let networks_y: i16 = 220;
    create_section_header(scr, 16, networks_y, "AVAILABLE NETWORKS");

    create_settings_row(scr, 16, networks_y + 28, "NYHC!", "Connected", "green", true);
    create_settings_row(scr, 16, networks_y + 72, "Neighbor-5G", "Secured", "", true);
    create_settings_row(scr, 16, networks_y + 116, "Guest-Network", "Open", "", true);

    scr
}

/// Create Backend Settings screen
unsafe fn create_backend_settings_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    create_status_bar_with_back(scr, "Backend Server", btn_back_cb);

    // Server section
    let section_y: i16 = 60;
    create_section_header(scr, 16, section_y, "SERVER CONNECTION");

    create_settings_row(scr, 16, section_y + 28, "Server URL", "192.168.1.100", "", true);
    create_settings_row(scr, 16, section_y + 72, "Port", "3000", "", true);
    create_settings_row(scr, 16, section_y + 116, "Status", "Connected", "green", false);

    // MQTT section
    let mqtt_y: i16 = 220;
    create_section_header(scr, 16, mqtt_y, "MQTT BROKER");

    create_settings_row(scr, 16, mqtt_y + 28, "Broker", "Auto-discover", "", true);
    create_settings_row(scr, 16, mqtt_y + 72, "Port", "1883", "", true);
    create_toggle_row(scr, 16, mqtt_y + 116, "Use TLS", false);

    scr
}

/// Create Add Printer screen
unsafe fn create_add_printer_screen() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    create_status_bar_with_back(scr, "Add Printer", btn_back_cb);

    // Discovery section
    let section_y: i16 = 60;
    create_section_header(scr, 16, section_y, "DISCOVERED PRINTERS");

    create_settings_row(scr, 16, section_y + 28, "X1C-Workshop", "192.168.1.50", "", true);
    create_settings_row(scr, 16, section_y + 72, "P1P-Office", "192.168.1.51", "", true);

    // Manual section
    let manual_y: i16 = 180;
    create_section_header(scr, 16, manual_y, "MANUAL SETUP");

    create_settings_row(scr, 16, manual_y + 28, "Enter IP Address", "", "", true);
    create_settings_row(scr, 16, manual_y + 72, "Enter Serial Number", "", "", true);
    create_settings_row(scr, 16, manual_y + 116, "Enter Access Code", "", "", true);

    // Add button
    let add_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(add_btn, 200, 48);
    lvgl_sys::lv_obj_set_pos(add_btn, 300, 380);
    lvgl_sys::lv_obj_set_style_bg_color(add_btn, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(add_btn, 8, 0);

    let add_lbl = lvgl_sys::lv_label_create(add_btn);
    let add_text = CString::new("Add Printer").unwrap();
    lvgl_sys::lv_label_set_text(add_lbl, add_text.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(add_lbl, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_align(add_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    scr
}

/// Helper: Create filter pill for catalog
unsafe fn create_filter_pill(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, text: &str, active: bool) {
    let pill = lvgl_sys::lv_obj_create(parent);
    let w: i16 = if text.len() > 8 { 75 } else { 50 };
    lvgl_sys::lv_obj_set_size(pill, w, 36);
    lvgl_sys::lv_obj_set_pos(pill, x, y);
    lvgl_sys::lv_obj_clear_flag(pill, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_radius(pill, 18, 0);
    set_style_pad_all(pill, 0);

    if active {
        lvgl_sys::lv_obj_set_style_bg_color(pill, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_border_width(pill, 0, 0);
    } else {
        lvgl_sys::lv_obj_set_style_bg_color(pill, lv_color_hex(0x2D2D2D), 0);
        lvgl_sys::lv_obj_set_style_border_width(pill, 1, 0);
        lvgl_sys::lv_obj_set_style_border_color(pill, lv_color_hex(0x505050), 0);
    }

    let lbl = lvgl_sys::lv_label_create(pill);
    let lbl_txt = CString::new(text).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    let text_color = if active { 0x000000 } else { COLOR_WHITE };
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(text_color), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
}

/// Helper: Create catalog card for spool display
unsafe fn create_catalog_card(
    parent: *mut lvgl_sys::lv_obj_t,
    x: i16, y: i16, w: i16, h: i16,
    material: &str, color_name: &str, color_hex: u32, weight: &str, pct: &str, slot: &str,
) {
    let card = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(card, w, h);
    lvgl_sys::lv_obj_set_pos(card, x, y);
    lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(card, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(card, 0, 0);
    set_style_pad_all(card, 10);

    // Spool visual (simplified circle)
    let spool_size: i16 = 50;
    let spool = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(spool, spool_size, spool_size);
    lvgl_sys::lv_obj_set_pos(spool, 5, 10);
    lvgl_sys::lv_obj_clear_flag(spool, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(spool, lv_color_hex(color_hex), 0);
    lvgl_sys::lv_obj_set_style_radius(spool, spool_size / 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(spool, lv_color_hex(lighten_color(color_hex, 30)), 0);
    lvgl_sys::lv_obj_set_style_border_width(spool, 2, 0);
    set_style_pad_all(spool, 0);

    // Inner circle (spool hole)
    let inner_size: i16 = 16;
    let inner = lvgl_sys::lv_obj_create(spool);
    lvgl_sys::lv_obj_set_size(inner, inner_size, inner_size);
    lvgl_sys::lv_obj_align(inner, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    lvgl_sys::lv_obj_clear_flag(inner, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(inner, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(inner, inner_size / 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(inner, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_border_width(inner, 1, 0);
    set_style_pad_all(inner, 0);

    // Slot badge (if assigned)
    if !slot.is_empty() {
        let badge = lvgl_sys::lv_obj_create(card);
        lvgl_sys::lv_obj_set_size(badge, 26, 18);
        lvgl_sys::lv_obj_set_pos(badge, w - 40, 5);
        lvgl_sys::lv_obj_clear_flag(badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
        lvgl_sys::lv_obj_set_style_bg_color(badge, lv_color_hex(COLOR_ACCENT), 0);
        lvgl_sys::lv_obj_set_style_radius(badge, 9, 0);
        lvgl_sys::lv_obj_set_style_border_width(badge, 0, 0);
        set_style_pad_all(badge, 0);

        let badge_lbl = lvgl_sys::lv_label_create(badge);
        let badge_txt = CString::new(slot).unwrap();
        lvgl_sys::lv_label_set_text(badge_lbl, badge_txt.as_ptr());
        lvgl_sys::lv_obj_set_style_text_color(badge_lbl, lv_color_hex(0x000000), 0);
        lvgl_sys::lv_obj_set_style_text_font(badge_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
        lvgl_sys::lv_obj_align(badge_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
    }

    // Material name
    let mat_lbl = lvgl_sys::lv_label_create(card);
    let mat_txt = CString::new(material).unwrap();
    lvgl_sys::lv_label_set_text(mat_lbl, mat_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(mat_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(mat_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(mat_lbl, 65, 10);

    // Color dot + name
    let dot = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(dot, 10, 10);
    lvgl_sys::lv_obj_set_pos(dot, 65, 30);
    lvgl_sys::lv_obj_clear_flag(dot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(dot, lv_color_hex(color_hex), 0);
    lvgl_sys::lv_obj_set_style_radius(dot, 5, 0);
    lvgl_sys::lv_obj_set_style_border_width(dot, 0, 0);
    set_style_pad_all(dot, 0);

    let color_lbl = lvgl_sys::lv_label_create(card);
    let color_txt = CString::new(color_name).unwrap();
    lvgl_sys::lv_label_set_text(color_lbl, color_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(color_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(color_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(color_lbl, 80, 28);

    // Weight + percentage
    let weight_lbl = lvgl_sys::lv_label_create(card);
    let weight_txt = CString::new(format!("{} ({})", weight, pct)).unwrap();
    lvgl_sys::lv_label_set_text(weight_lbl, weight_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(weight_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(weight_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(weight_lbl, 65, 48);
}

/// Create Catalog screen with search, filters, and spool grid
unsafe fn create_catalog_screen_fn() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar with back button
    create_status_bar_with_back(scr, "Spool Catalog", btn_back_cb);

    // Search bar
    let search_bar = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(search_bar, 280, 36);
    lvgl_sys::lv_obj_set_pos(search_bar, 16, 52);
    lvgl_sys::lv_obj_clear_flag(search_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(search_bar, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(search_bar, 18, 0);
    lvgl_sys::lv_obj_set_style_border_width(search_bar, 0, 0);
    set_style_pad_all(search_bar, 0);

    // Search icon (Q)
    let search_icon = lvgl_sys::lv_label_create(search_bar);
    let search_icon_txt = CString::new("Q").unwrap();
    lvgl_sys::lv_label_set_text(search_icon, search_icon_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(search_icon, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(search_icon, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(search_icon, 14, 9);

    let search_txt_lbl = lvgl_sys::lv_label_create(search_bar);
    let search_txt = CString::new("Search spools...").unwrap();
    lvgl_sys::lv_label_set_text(search_txt_lbl, search_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(search_txt_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(search_txt_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(search_txt_lbl, 36, 10);

    // Filter pills
    let pills_x: i16 = 310;
    create_filter_pill(scr, pills_x, 52, "All (24)", true);
    create_filter_pill(scr, pills_x + 80, 52, "In AMS (6)", false);
    create_filter_pill(scr, pills_x + 170, 52, "PLA", false);
    create_filter_pill(scr, pills_x + 220, 52, "PETG", false);

    // Spool grid
    let grid_y: i16 = 100;
    let card_w: i16 = 180;
    let card_h: i16 = 110;
    let gap: i16 = 12;

    // Spool data: (material, color_name, color_hex, weight, pct, slot)
    let spools = [
        ("PLA Basic", "Yellow", 0xF5C518u32, "847g", "85%", "A1"),
        ("PETG HF", "Black", 0x333333u32, "620g", "62%", "A2"),
        ("PETG Basic", "Orange", 0xFF9800u32, "450g", "45%", "A3"),
        ("PLA Matte", "Gray", 0x9E9E9Eu32, "900g", "90%", "A4"),
        ("PLA Silk", "Pink", 0xE91E63u32, "720g", "72%", "B1"),
        ("PLA Basic", "Blue", 0x2196F3u32, "550g", "55%", "B2"),
        ("PLA Basic", "Red", 0xF44336u32, "1000g", "100%", ""),
        ("PETG HF", "Green", 0x4CAF50u32, "880g", "88%", ""),
        ("PLA Basic", "White", 0xFFFFFFu32, "750g", "75%", ""),
        ("ABS", "Purple", 0x673AB7u32, "780g", "78%", ""),
        ("TPU 95A", "Lime", 0xCDDC39u32, "920g", "92%", ""),
        ("PETG Basic", "Cyan", 0x00BCD4u32, "650g", "65%", ""),
    ];

    for (i, (material, color_name, color_hex, weight, pct, slot)) in spools.iter().enumerate() {
        let col = (i % 4) as i16;
        let row = (i / 4) as i16;
        let x = 16 + col * (card_w + gap);
        let y = grid_y + row * (card_h + gap);

        create_catalog_card(scr, x, y, card_w, card_h, material, color_name, *color_hex, weight, pct, slot);
    }

    scr
}

/// Helper: Create setting item (label + value pair)
unsafe fn create_setting_item(parent: *mut lvgl_sys::lv_obj_t, x: i16, label: &str, value: &str) {
    let lbl = lvgl_sys::lv_label_create(parent);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(lbl, x, 0);

    let val = lvgl_sys::lv_label_create(parent);
    let val_txt = CString::new(value).unwrap();
    lvgl_sys::lv_label_set_text(val, val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(val, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(val, x, 14);
}

/// Helper: Create setting item row 2 (for second row of info)
unsafe fn create_setting_item_row2(parent: *mut lvgl_sys::lv_obj_t, x: i16, label: &str, value: &str) {
    let lbl = lvgl_sys::lv_label_create(parent);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(lbl, x, 36);

    let val = lvgl_sys::lv_label_create(parent);
    let val_txt = CString::new(value).unwrap();
    lvgl_sys::lv_label_set_text(val, val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(val, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(val, x, 50);
}

/// Helper: Style empty slot with dashed border (simpler than stripes)
unsafe fn style_empty_slot(slot: *mut lvgl_sys::lv_obj_t) {
    // Use a darker background with a dashed-look border for empty slots
    lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_border_width(slot, 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(0x3A3A3A), 0);
    // Add a "?" label to indicate empty
    let q_lbl = lvgl_sys::lv_label_create(slot);
    let q_txt = CString::new("?").unwrap();
    lvgl_sys::lv_label_set_text(q_lbl, q_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(q_lbl, lv_color_hex(0x505050), 0);
    lvgl_sys::lv_obj_set_style_text_font(q_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(q_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
}

/// Helper: Style small empty slot (for HT/Ext)
unsafe fn style_empty_slot_small(slot: *mut lvgl_sys::lv_obj_t) {
    lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(0x1A1A1A), 0);
    lvgl_sys::lv_obj_set_style_border_width(slot, 2, 0);
    lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(0x3A3A3A), 0);
}

/// Helper: Create AMS unit visual (box with 4 spool slots)
unsafe fn create_ams_unit_visual(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, slots: &[(u32, bool); 4]) {
    let unit = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(unit, 180, 60);
    lvgl_sys::lv_obj_set_pos(unit, x, y);
    lvgl_sys::lv_obj_clear_flag(unit, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(unit, lv_color_hex(0x252525), 0);
    lvgl_sys::lv_obj_set_style_radius(unit, 8, 0);
    lvgl_sys::lv_obj_set_style_border_color(unit, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_border_width(unit, 1, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(unit, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_width(unit, 10, 0);
    lvgl_sys::lv_obj_set_style_shadow_ofs_y(unit, 3, 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(unit, 80, 0);
    set_style_pad_all(unit, 0);

    // AMS label badge
    let badge = lvgl_sys::lv_obj_create(unit);
    lvgl_sys::lv_obj_set_size(badge, 24, 18);
    lvgl_sys::lv_obj_set_pos(badge, 6, 4);
    lvgl_sys::lv_obj_clear_flag(badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(badge, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_radius(badge, 4, 0);
    lvgl_sys::lv_obj_set_style_border_width(badge, 0, 0);
    set_style_pad_all(badge, 0);

    let lbl = lvgl_sys::lv_label_create(badge);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // 4 spool slots in a row
    for (i, (color, selected)) in slots.iter().enumerate() {
        let sx = 8 + (i as i16) * 42;
        let slot = lvgl_sys::lv_obj_create(unit);
        lvgl_sys::lv_obj_set_size(slot, 36, 36);
        lvgl_sys::lv_obj_set_pos(slot, sx, 20);
        lvgl_sys::lv_obj_clear_flag(slot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
        lvgl_sys::lv_obj_set_style_radius(slot, 18, 0);
        set_style_pad_all(slot, 0);

        if *color != 0 {
            lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(*color), 0);
            if *selected {
                lvgl_sys::lv_obj_set_style_border_width(slot, 3, 0);
                lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_shadow_color(slot, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(slot, 8, 0);
                lvgl_sys::lv_obj_set_style_shadow_spread(slot, 2, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(slot, 150, 0);
            } else {
                lvgl_sys::lv_obj_set_style_border_width(slot, 2, 0);
                lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(0x505050), 0);
            }

            // Slot number for filled slots
            let num_lbl = lvgl_sys::lv_label_create(slot);
            let num_txt = CString::new(format!("{}", i + 1)).unwrap();
            lvgl_sys::lv_label_set_text(num_lbl, num_txt.as_ptr());
            let text_color = if *color != 0xFFFFFF && *color != 0xECEFF1 && *color != 0xCDDC39 {
                COLOR_WHITE
            } else {
                0x000000
            };
            lvgl_sys::lv_obj_set_style_text_color(num_lbl, lv_color_hex(text_color), 0);
            lvgl_sys::lv_obj_set_style_text_font(num_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
            lvgl_sys::lv_obj_align(num_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
        } else {
            // Empty slot styling
            style_empty_slot(slot);
        }
    }
}

/// Helper: Create HT (High Temp) slot visual
unsafe fn create_ht_slot_visual(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, color: u32, selected: bool) {
    let container = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(container, 86, 44);
    lvgl_sys::lv_obj_set_pos(container, x, y);
    lvgl_sys::lv_obj_clear_flag(container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(container, lv_color_hex(0x252525), 0);
    lvgl_sys::lv_obj_set_style_radius(container, 6, 0);
    lvgl_sys::lv_obj_set_style_border_color(container, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_border_width(container, 1, 0);
    set_style_pad_all(container, 0);

    let lbl = lvgl_sys::lv_label_create(container);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(lbl, 6, 4);

    let slot = lvgl_sys::lv_obj_create(container);
    lvgl_sys::lv_obj_set_size(slot, 30, 30);
    lvgl_sys::lv_obj_set_pos(slot, 50, 8);
    lvgl_sys::lv_obj_clear_flag(slot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_radius(slot, 15, 0);
    set_style_pad_all(slot, 0);

    if color != 0 {
        lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(color), 0);
        if selected {
            lvgl_sys::lv_obj_set_style_border_width(slot, 3, 0);
            lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(COLOR_ACCENT), 0);
        } else {
            lvgl_sys::lv_obj_set_style_border_width(slot, 2, 0);
            lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(0x505050), 0);
        }
    } else {
        style_empty_slot_small(slot);
    }
}

/// Helper: Create External slot visual
unsafe fn create_ext_slot_visual(parent: *mut lvgl_sys::lv_obj_t, x: i16, y: i16, label: &str, color: u32, selected: bool) {
    let container = lvgl_sys::lv_obj_create(parent);
    lvgl_sys::lv_obj_set_size(container, 86, 44);
    lvgl_sys::lv_obj_set_pos(container, x, y);
    lvgl_sys::lv_obj_clear_flag(container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(container, lv_color_hex(0x252525), 0);
    lvgl_sys::lv_obj_set_style_radius(container, 6, 0);
    lvgl_sys::lv_obj_set_style_border_color(container, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_border_width(container, 1, 0);
    set_style_pad_all(container, 0);

    let lbl = lvgl_sys::lv_label_create(container);
    let lbl_txt = CString::new(label).unwrap();
    lvgl_sys::lv_label_set_text(lbl, lbl_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(lbl, 6, 4);

    let slot = lvgl_sys::lv_obj_create(container);
    lvgl_sys::lv_obj_set_size(slot, 30, 30);
    lvgl_sys::lv_obj_set_pos(slot, 50, 8);
    lvgl_sys::lv_obj_clear_flag(slot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_radius(slot, 15, 0);
    set_style_pad_all(slot, 0);

    if color != 0 {
        lvgl_sys::lv_obj_set_style_bg_color(slot, lv_color_hex(color), 0);
        if selected {
            lvgl_sys::lv_obj_set_style_border_width(slot, 3, 0);
            lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(COLOR_ACCENT), 0);
        } else {
            lvgl_sys::lv_obj_set_style_border_width(slot, 2, 0);
            lvgl_sys::lv_obj_set_style_border_color(slot, lv_color_hex(0x505050), 0);
        }
    } else {
        style_empty_slot_small(slot);
    }
}

/// Create Scan Result screen - shows detected spool info and AMS assignment
unsafe fn create_scan_result_screen_fn() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar
    create_status_bar_with_back(scr, "Scan Result", btn_back_cb);

    // Success banner
    let banner = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(banner, 768, 50);
    lvgl_sys::lv_obj_set_pos(banner, 16, 52);
    lvgl_sys::lv_obj_clear_flag(banner, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(banner, lv_color_hex(0x1B5E20), 0);
    lvgl_sys::lv_obj_set_style_radius(banner, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(banner, 0, 0);
    set_style_pad_all(banner, 0);

    // OK circle
    let ok_circle = lvgl_sys::lv_obj_create(banner);
    lvgl_sys::lv_obj_set_size(ok_circle, 28, 28);
    lvgl_sys::lv_obj_set_pos(ok_circle, 12, 11);
    lvgl_sys::lv_obj_clear_flag(ok_circle, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(ok_circle, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(ok_circle, 14, 0);
    lvgl_sys::lv_obj_set_style_border_width(ok_circle, 0, 0);
    set_style_pad_all(ok_circle, 0);

    let ok_lbl = lvgl_sys::lv_label_create(ok_circle);
    let ok_txt = CString::new("OK").unwrap();
    lvgl_sys::lv_label_set_text(ok_lbl, ok_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(ok_lbl, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_text_font(ok_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(ok_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Banner text
    let banner_title = lvgl_sys::lv_label_create(banner);
    let banner_title_txt = CString::new("Spool Detected").unwrap();
    lvgl_sys::lv_label_set_text(banner_title, banner_title_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(banner_title, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(banner_title, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(banner_title, 52, 8);

    let banner_sub = lvgl_sys::lv_label_create(banner);
    let banner_sub_txt = CString::new("Bambu Lab NFC tag read successfully").unwrap();
    lvgl_sys::lv_label_set_text(banner_sub, banner_sub_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(banner_sub, lv_color_hex(0x81C784), 0);
    lvgl_sys::lv_obj_set_style_text_font(banner_sub, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(banner_sub, 52, 28);

    // Spool info card
    let card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(card, 768, 130);
    lvgl_sys::lv_obj_set_pos(card, 16, 110);
    lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(card, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(card, 0, 0);
    set_style_pad_all(card, 16);

    // Spool visual
    let spool_container = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(spool_container, 70, 90);
    lvgl_sys::lv_obj_set_pos(spool_container, 0, 4);
    lvgl_sys::lv_obj_clear_flag(spool_container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(spool_container, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(spool_container, 0, 0);
    set_style_pad_all(spool_container, 0);

    create_spool_large(spool_container, 15, 0, 0xF5C518);

    // Weight badge
    let weight_badge = lvgl_sys::lv_obj_create(spool_container);
    lvgl_sys::lv_obj_set_size(weight_badge, 40, 18);
    lvgl_sys::lv_obj_set_pos(weight_badge, 15, 60);
    lvgl_sys::lv_obj_clear_flag(weight_badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(weight_badge, lv_color_hex(0x424242), 0);
    lvgl_sys::lv_obj_set_style_radius(weight_badge, 9, 0);
    lvgl_sys::lv_obj_set_style_border_width(weight_badge, 0, 0);
    set_style_pad_all(weight_badge, 0);

    let weight_lbl = lvgl_sys::lv_label_create(weight_badge);
    let weight_txt = CString::new("847g").unwrap();
    lvgl_sys::lv_label_set_text(weight_lbl, weight_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(weight_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(weight_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(weight_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Spool info text
    let info_x: i16 = 90;

    let name_lbl = lvgl_sys::lv_label_create(card);
    let name_txt = CString::new("PLA Basic").unwrap();
    lvgl_sys::lv_label_set_text(name_lbl, name_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(name_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(name_lbl, &lvgl_sys::lv_font_montserrat_20, 0);
    lvgl_sys::lv_obj_set_pos(name_lbl, info_x, 4);

    // Color indicator
    let color_dot = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(color_dot, 12, 12);
    lvgl_sys::lv_obj_set_pos(color_dot, info_x, 32);
    lvgl_sys::lv_obj_clear_flag(color_dot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(color_dot, lv_color_hex(0xF5C518), 0);
    lvgl_sys::lv_obj_set_style_radius(color_dot, 6, 0);
    lvgl_sys::lv_obj_set_style_border_width(color_dot, 0, 0);
    set_style_pad_all(color_dot, 0);

    let color_lbl = lvgl_sys::lv_label_create(card);
    let color_txt = CString::new("Yellow").unwrap();
    lvgl_sys::lv_label_set_text(color_lbl, color_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(color_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(color_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(color_lbl, info_x + 18, 29);

    let brand_lbl = lvgl_sys::lv_label_create(card);
    let brand_txt = CString::new("Bambu Lab").unwrap();
    lvgl_sys::lv_label_set_text(brand_lbl, brand_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(brand_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(brand_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(brand_lbl, info_x, 50);

    // Specs row 1
    let specs_y1: i16 = 70;
    let specs_y2: i16 = 86;

    let nozzle_label = lvgl_sys::lv_label_create(card);
    let nozzle_label_txt = CString::new("Nozzle").unwrap();
    lvgl_sys::lv_label_set_text(nozzle_label, nozzle_label_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(nozzle_label, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(nozzle_label, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(nozzle_label, info_x, specs_y1);

    let nozzle_val = lvgl_sys::lv_label_create(card);
    let nozzle_val_txt = CString::new("190-220C").unwrap();
    lvgl_sys::lv_label_set_text(nozzle_val, nozzle_val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(nozzle_val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(nozzle_val, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(nozzle_val, info_x, specs_y2);

    let bed_label = lvgl_sys::lv_label_create(card);
    let bed_label_txt = CString::new("Bed").unwrap();
    lvgl_sys::lv_label_set_text(bed_label, bed_label_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(bed_label, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(bed_label, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(bed_label, info_x + 100, specs_y1);

    let bed_val = lvgl_sys::lv_label_create(card);
    let bed_val_txt = CString::new("45-65C").unwrap();
    lvgl_sys::lv_label_set_text(bed_val, bed_val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(bed_val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(bed_val, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(bed_val, info_x + 100, specs_y2);

    let k_label = lvgl_sys::lv_label_create(card);
    let k_label_txt = CString::new("K Factor").unwrap();
    lvgl_sys::lv_label_set_text(k_label, k_label_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(k_label, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(k_label, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(k_label, info_x + 200, specs_y1);

    let k_val = lvgl_sys::lv_label_create(card);
    let k_val_txt = CString::new("0.020").unwrap();
    lvgl_sys::lv_label_set_text(k_val, k_val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(k_val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(k_val, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(k_val, info_x + 200, specs_y2);

    let dia_label = lvgl_sys::lv_label_create(card);
    let dia_label_txt = CString::new("Diameter").unwrap();
    lvgl_sys::lv_label_set_text(dia_label, dia_label_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(dia_label, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(dia_label, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(dia_label, info_x + 300, specs_y1);

    let dia_val = lvgl_sys::lv_label_create(card);
    let dia_val_txt = CString::new("1.75mm").unwrap();
    lvgl_sys::lv_label_set_text(dia_val, dia_val_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(dia_val, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(dia_val, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(dia_val, info_x + 300, specs_y2);

    // Assign to AMS Slot section
    let assign_label = lvgl_sys::lv_label_create(scr);
    let assign_txt = CString::new("Assign to AMS Slot").unwrap();
    lvgl_sys::lv_label_set_text(assign_label, assign_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(assign_label, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(assign_label, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(assign_label, 16, 252);

    // AMS units row
    let ams_y: i16 = 272;
    create_ams_unit_visual(scr, 16, ams_y, "A", &[
        (0xF5C518, true), (0, false), (0x4CAF50, false), (0, false)
    ]);
    create_ams_unit_visual(scr, 210, ams_y, "B", &[
        (0xE91E63, false), (0x2196F3, false), (0x4CAF50, false), (0xF5C518, true)
    ]);
    create_ams_unit_visual(scr, 404, ams_y, "C", &[
        (0xFFFFFF, false), (0, false), (0, false), (0, false)
    ]);
    create_ams_unit_visual(scr, 598, ams_y, "D", &[
        (0x00BCD4, false), (0xFF5722, false), (0, false), (0, false)
    ]);

    // Row 2: HT units and External slots
    let row2_y = ams_y + 70;
    create_ht_slot_visual(scr, 16, row2_y, "HT-A", 0x673AB7, false);
    create_ht_slot_visual(scr, 110, row2_y, "HT-B", 0xECEFF1, false);
    create_ext_slot_visual(scr, 204, row2_y, "Ext 1", 0x607D8B, false);
    create_ext_slot_visual(scr, 298, row2_y, "Ext 2", 0xCDDC39, false);

    // Assign & Save button
    let btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(btn, 768, 50);
    lvgl_sys::lv_obj_set_pos(btn, 16, 418);
    lvgl_sys::lv_obj_set_style_bg_color(btn, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(btn, 8, 0);
    lvgl_sys::lv_obj_set_style_border_width(btn, 0, 0);

    let btn_lbl = lvgl_sys::lv_label_create(btn);
    let btn_txt = CString::new("Assign & Save").unwrap();
    lvgl_sys::lv_label_set_text(btn_lbl, btn_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(btn_lbl, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_text_font(btn_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
    lvgl_sys::lv_obj_align(btn_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    scr
}

/// Create Spool Detail screen - shows detailed spool info with settings and actions
unsafe fn create_spool_detail_screen_fn() -> *mut lvgl_sys::lv_obj_t {
    let scr = lvgl_sys::lv_obj_create(ptr::null_mut());
    lvgl_sys::lv_obj_set_style_bg_color(scr, lv_color_hex(COLOR_BG), 0);
    lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

    // Status bar
    create_status_bar_with_back(scr, "Spool Detail", btn_back_cb);

    // Main spool info card
    let card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(card, 768, 120);
    lvgl_sys::lv_obj_set_pos(card, 16, 52);
    lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(card, 20, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(card, 80, 0);
    set_style_pad_all(card, 16);

    // Spool image container
    let spool_container = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(spool_container, 70, 90);
    lvgl_sys::lv_obj_set_pos(spool_container, 0, 0);
    lvgl_sys::lv_obj_clear_flag(spool_container, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_opa(spool_container, 0, 0);
    lvgl_sys::lv_obj_set_style_border_width(spool_container, 0, 0);
    set_style_pad_all(spool_container, 0);

    create_spool_large(spool_container, 15, 0, 0xF5C518);

    // Slot badge
    let slot_badge = lvgl_sys::lv_obj_create(spool_container);
    lvgl_sys::lv_obj_set_size(slot_badge, 28, 18);
    lvgl_sys::lv_obj_set_pos(slot_badge, 20, 60);
    lvgl_sys::lv_obj_clear_flag(slot_badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(slot_badge, lv_color_hex(0x424242), 0);
    lvgl_sys::lv_obj_set_style_radius(slot_badge, 9, 0);
    lvgl_sys::lv_obj_set_style_border_width(slot_badge, 0, 0);
    set_style_pad_all(slot_badge, 0);

    let slot_lbl = lvgl_sys::lv_label_create(slot_badge);
    let slot_txt = CString::new("A1").unwrap();
    lvgl_sys::lv_label_set_text(slot_lbl, slot_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(slot_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(slot_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_align(slot_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Spool info
    let info_x: i16 = 90;

    let name_lbl = lvgl_sys::lv_label_create(card);
    let name_txt = CString::new("PLA Basic").unwrap();
    lvgl_sys::lv_label_set_text(name_lbl, name_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(name_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(name_lbl, &lvgl_sys::lv_font_montserrat_20, 0);
    lvgl_sys::lv_obj_set_pos(name_lbl, info_x, 0);

    // Color indicator
    let color_dot = lvgl_sys::lv_obj_create(card);
    lvgl_sys::lv_obj_set_size(color_dot, 12, 12);
    lvgl_sys::lv_obj_set_pos(color_dot, info_x, 28);
    lvgl_sys::lv_obj_clear_flag(color_dot, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(color_dot, lv_color_hex(0xF5C518), 0);
    lvgl_sys::lv_obj_set_style_radius(color_dot, 6, 0);
    lvgl_sys::lv_obj_set_style_border_width(color_dot, 0, 0);
    set_style_pad_all(color_dot, 0);

    let color_lbl = lvgl_sys::lv_label_create(card);
    let color_txt = CString::new("Yellow").unwrap();
    lvgl_sys::lv_label_set_text(color_lbl, color_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(color_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(color_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(color_lbl, info_x + 18, 25);

    let brand_lbl = lvgl_sys::lv_label_create(card);
    let brand_txt = CString::new("Bambu Lab - 1.75mm").unwrap();
    lvgl_sys::lv_label_set_text(brand_lbl, brand_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(brand_lbl, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(brand_lbl, &lvgl_sys::lv_font_montserrat_12, 0);
    lvgl_sys::lv_obj_set_pos(brand_lbl, info_x, 46);

    // Weight display (using font 24 instead of 28)
    let weight_lbl = lvgl_sys::lv_label_create(card);
    let weight_txt = CString::new("847").unwrap();
    lvgl_sys::lv_label_set_text(weight_lbl, weight_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(weight_lbl, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_text_font(weight_lbl, &lvgl_sys::lv_font_montserrat_24, 0);
    lvgl_sys::lv_obj_set_pos(weight_lbl, info_x, 62);

    let weight_unit = lvgl_sys::lv_label_create(card);
    let weight_unit_txt = CString::new("g").unwrap();
    lvgl_sys::lv_label_set_text(weight_unit, weight_unit_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(weight_unit, lv_color_hex(COLOR_TEXT_MUTED), 0);
    lvgl_sys::lv_obj_set_style_text_font(weight_unit, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(weight_unit, info_x + 55, 72);

    let pct_lbl = lvgl_sys::lv_label_create(card);
    let pct_txt = CString::new("(85%)").unwrap();
    lvgl_sys::lv_label_set_text(pct_lbl, pct_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(pct_lbl, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_text_font(pct_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(pct_lbl, info_x + 70, 72);

    // Print Settings section
    let section1_y: i16 = 180;
    let section1_lbl = lvgl_sys::lv_label_create(scr);
    let section1_txt = CString::new("Print Settings").unwrap();
    lvgl_sys::lv_label_set_text(section1_lbl, section1_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(section1_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(section1_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(section1_lbl, 16, section1_y);

    let settings_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(settings_card, 768, 60);
    lvgl_sys::lv_obj_set_pos(settings_card, 16, section1_y + 24);
    lvgl_sys::lv_obj_clear_flag(settings_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(settings_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(settings_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(settings_card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(settings_card, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(settings_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(settings_card, 60, 0);
    set_style_pad_all(settings_card, 12);

    // Settings grid
    create_setting_item(settings_card, 0, "Nozzle", "190-220C");
    create_setting_item(settings_card, 170, "Bed", "45-65C");
    create_setting_item(settings_card, 340, "K Factor", "0.020");
    create_setting_item(settings_card, 510, "Max Speed", "500mm/s");

    // Spool Information section
    let section2_y: i16 = 275;
    let section2_lbl = lvgl_sys::lv_label_create(scr);
    let section2_txt = CString::new("Spool Information").unwrap();
    lvgl_sys::lv_label_set_text(section2_lbl, section2_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(section2_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(section2_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_set_pos(section2_lbl, 16, section2_y);

    let info_card = lvgl_sys::lv_obj_create(scr);
    lvgl_sys::lv_obj_set_size(info_card, 768, 80);
    lvgl_sys::lv_obj_set_pos(info_card, 16, section2_y + 24);
    lvgl_sys::lv_obj_clear_flag(info_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
    lvgl_sys::lv_obj_set_style_bg_color(info_card, lv_color_hex(0x2D2D2D), 0);
    lvgl_sys::lv_obj_set_style_radius(info_card, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(info_card, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(info_card, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(info_card, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(info_card, 60, 0);
    set_style_pad_all(info_card, 12);

    // Info grid - row 1
    create_setting_item(info_card, 0, "Tag ID", "A4B7C912");
    create_setting_item(info_card, 170, "Initial Weight", "1000g");
    create_setting_item(info_card, 340, "Used", "153g");
    create_setting_item(info_card, 510, "Last Weighed", "2 min ago");

    // Info grid - row 2
    create_setting_item_row2(info_card, 0, "Added", "Dec 10, 2024");
    create_setting_item_row2(info_card, 170, "Uses", "12 prints");

    // Bottom buttons
    let btn_y: i16 = 410;
    let btn_h: i16 = 44;

    // Assign Slot button (green)
    let assign_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(assign_btn, 160, btn_h);
    lvgl_sys::lv_obj_set_pos(assign_btn, 180, btn_y);
    lvgl_sys::lv_obj_set_style_bg_color(assign_btn, lv_color_hex(COLOR_ACCENT), 0);
    lvgl_sys::lv_obj_set_style_radius(assign_btn, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(assign_btn, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(assign_btn, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(assign_btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(assign_btn, 60, 0);

    let assign_lbl = lvgl_sys::lv_label_create(assign_btn);
    let assign_txt = CString::new("Assign Slot").unwrap();
    lvgl_sys::lv_label_set_text(assign_lbl, assign_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(assign_lbl, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_text_font(assign_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(assign_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Edit Info button (gray)
    let edit_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(edit_btn, 130, btn_h);
    lvgl_sys::lv_obj_set_pos(edit_btn, 350, btn_y);
    lvgl_sys::lv_obj_set_style_bg_color(edit_btn, lv_color_hex(0x3D3D3D), 0);
    lvgl_sys::lv_obj_set_style_radius(edit_btn, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(edit_btn, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(edit_btn, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(edit_btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(edit_btn, 60, 0);

    let edit_lbl = lvgl_sys::lv_label_create(edit_btn);
    let edit_txt = CString::new("Edit Info").unwrap();
    lvgl_sys::lv_label_set_text(edit_lbl, edit_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(edit_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(edit_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(edit_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    // Delete button (red)
    let del_btn = lvgl_sys::lv_btn_create(scr);
    lvgl_sys::lv_obj_set_size(del_btn, 110, btn_h);
    lvgl_sys::lv_obj_set_pos(del_btn, 490, btn_y);
    lvgl_sys::lv_obj_set_style_bg_color(del_btn, lv_color_hex(0xD32F2F), 0);
    lvgl_sys::lv_obj_set_style_radius(del_btn, 12, 0);
    lvgl_sys::lv_obj_set_style_border_width(del_btn, 0, 0);
    lvgl_sys::lv_obj_set_style_shadow_width(del_btn, 15, 0);
    lvgl_sys::lv_obj_set_style_shadow_color(del_btn, lv_color_hex(0x000000), 0);
    lvgl_sys::lv_obj_set_style_shadow_opa(del_btn, 60, 0);

    let del_lbl = lvgl_sys::lv_label_create(del_btn);
    let del_txt = CString::new("Delete").unwrap();
    lvgl_sys::lv_label_set_text(del_lbl, del_txt.as_ptr());
    lvgl_sys::lv_obj_set_style_text_color(del_lbl, lv_color_hex(COLOR_WHITE), 0);
    lvgl_sys::lv_obj_set_style_text_font(del_lbl, &lvgl_sys::lv_font_montserrat_14, 0);
    lvgl_sys::lv_obj_align(del_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

    scr
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

/// Current screen identifier
static mut CURRENT_SCREEN: u8 = 0; // 0=Home, 1=AMS, 2=Encode, 3=Catalog, 4=Settings

/// Navigation history stack for back button
const NAV_STACK_SIZE: usize = 10;
static mut NAV_STACK: [*mut lvgl_sys::lv_obj_t; NAV_STACK_SIZE] = [ptr::null_mut(); NAV_STACK_SIZE];
static mut NAV_STACK_TOP: usize = 0;

/// Push current screen to navigation stack before switching
unsafe fn nav_push(screen: *mut lvgl_sys::lv_obj_t) {
    if NAV_STACK_TOP < NAV_STACK_SIZE {
        NAV_STACK[NAV_STACK_TOP] = screen;
        NAV_STACK_TOP += 1;
    }
}

/// Pop previous screen from navigation stack
unsafe fn nav_pop() -> *mut lvgl_sys::lv_obj_t {
    if NAV_STACK_TOP > 0 {
        NAV_STACK_TOP -= 1;
        NAV_STACK[NAV_STACK_TOP]
    } else {
        SCREEN_HOME // Default to home if stack is empty
    }
}

/// Reset all input devices to clear any stuck state
unsafe fn reset_all_indevs() {
    let mut indev = lvgl_sys::lv_indev_get_next(ptr::null_mut());
    while !indev.is_null() {
        lvgl_sys::lv_indev_reset(indev, ptr::null_mut());
        indev = lvgl_sys::lv_indev_get_next(indev);
    }
}

/// Navigate to a screen, pushing current screen to history
unsafe fn navigate_to(target: *mut lvgl_sys::lv_obj_t) {
    if target.is_null() {
        return;
    }

    // Push current screen to stack
    let current = lvgl_sys::lv_disp_get_scr_act(ptr::null_mut());
    if !current.is_null() && current != target {
        nav_push(current);
    }

    // Reset all input devices BEFORE loading new screen
    reset_all_indevs();

    // Load target screen
    lvgl_sys::lv_disp_load_scr(target);

    // Reset again AFTER loading to be safe
    reset_all_indevs();
}

/// Navigate back to previous screen
unsafe fn navigate_back() {
    let prev = nav_pop();
    if !prev.is_null() {
        // Reset all input devices BEFORE loading
        reset_all_indevs();

        lvgl_sys::lv_disp_load_scr(prev);

        // Reset again AFTER loading
        reset_all_indevs();
    }
}

/// Global screen pointers for navigation
static mut SCREEN_HOME: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_AMS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_ENCODE: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_CATALOG: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_SETTINGS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_SETTINGS_2: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_SCALE_CALIBRATION: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_NFC_READER: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_DISPLAY_BRIGHTNESS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_ABOUT: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_SCAN_RESULT: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_SPOOL_DETAIL: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_ADVANCED_SETTINGS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_WIFI_SETTINGS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_BACKEND_SETTINGS: *mut lvgl_sys::lv_obj_t = ptr::null_mut();
static mut SCREEN_ADD_PRINTER: *mut lvgl_sys::lv_obj_t = ptr::null_mut();

/// Button event callbacks
unsafe extern "C" fn btn_ams_setup_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("AMS Setup button pressed!");
    CURRENT_SCREEN = 1;
    navigate_to(SCREEN_AMS);
}

unsafe extern "C" fn btn_encode_tag_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Encode Tag button pressed!");
    CURRENT_SCREEN = 2;
    navigate_to(SCREEN_ENCODE);
}

unsafe extern "C" fn btn_catalog_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Catalog button pressed!");
    CURRENT_SCREEN = 3;
    navigate_to(SCREEN_CATALOG);
}

unsafe extern "C" fn btn_settings_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Settings button pressed!");
    CURRENT_SCREEN = 4;
    navigate_to(SCREEN_SETTINGS);
}

/// Back button callback - return to previous screen
unsafe extern "C" fn btn_back_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Back button pressed!");
    navigate_back();
}

/// Back to settings callback (deprecated - use navigate_back)
#[allow(dead_code)]
unsafe extern "C" fn btn_back_to_settings_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Back to settings pressed!");
    navigate_back();
}

/// Back to settings-2 callback (deprecated - use navigate_back)
#[allow(dead_code)]
unsafe extern "C" fn btn_back_to_settings_2_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Back to settings-2 pressed!");
    navigate_back();
}

/// Settings → Settings-2 callback
#[allow(dead_code)]
unsafe extern "C" fn btn_settings_2_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Hardware & System pressed!");
    navigate_to(SCREEN_SETTINGS_2);
}

/// Settings-2 → Scale Calibration callback
unsafe extern "C" fn btn_scale_calibration_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Scale Calibration pressed!");
    navigate_to(SCREEN_SCALE_CALIBRATION);
}

/// Settings-2 → NFC Reader callback
unsafe extern "C" fn btn_nfc_reader_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("NFC Reader pressed!");
    navigate_to(SCREEN_NFC_READER);
}

/// Settings-2 → Display Brightness callback
unsafe extern "C" fn btn_display_brightness_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Display Brightness pressed!");
    navigate_to(SCREEN_DISPLAY_BRIGHTNESS);
}

/// Settings-2 → Advanced Settings callback
unsafe extern "C" fn btn_advanced_settings_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Advanced Settings pressed!");
    navigate_to(SCREEN_ADVANCED_SETTINGS);
}

/// Settings-2 → About callback
unsafe extern "C" fn btn_about_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("About pressed!");
    navigate_to(SCREEN_ABOUT);
}

/// WiFi Settings callback
unsafe extern "C" fn btn_wifi_settings_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("WiFi Settings pressed!");
    navigate_to(SCREEN_WIFI_SETTINGS);
}

/// Backend Settings callback
unsafe extern "C" fn btn_backend_settings_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Backend Settings pressed!");
    navigate_to(SCREEN_BACKEND_SETTINGS);
}

/// Add Printer callback
unsafe extern "C" fn btn_add_printer_cb(
    _e: *mut lvgl_sys::lv_event_t,
) {
    info!("Add Printer pressed!");
    navigate_to(SCREEN_ADD_PRINTER);
}

/// Direct framebuffer wrapper for panel's own memory
#[allow(dead_code)]
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
                lvgl_sys::lv_obj_clear_flag(scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

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
                // Visible bottom shadow for depth separation
                lvgl_sys::lv_obj_set_style_shadow_color(status_bar, lv_color_hex(0x000000), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(status_bar, 25, 0);
                lvgl_sys::lv_obj_set_style_shadow_ofs_y(status_bar, 8, 0);
                lvgl_sys::lv_obj_set_style_shadow_spread(status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(status_bar, 200, 0);
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

                // Printer selector (center) - with dropdown indicator
                let printer_btn = lvgl_sys::lv_btn_create(status_bar);
                lvgl_sys::lv_obj_set_size(printer_btn, 200, 32);  // Wider to fit dropdown arrow
                lvgl_sys::lv_obj_align(printer_btn, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(printer_btn, lv_color_hex(0x242424), 0);
                lvgl_sys::lv_obj_set_style_radius(printer_btn, 16, 0);
                lvgl_sys::lv_obj_set_style_border_color(printer_btn, lv_color_hex(0x3D3D3D), 0);
                lvgl_sys::lv_obj_set_style_border_width(printer_btn, 1, 0);

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
                lvgl_sys::lv_obj_align(printer_label, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 28, 0);

                // Power icon (orange = printing)
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
                lvgl_sys::lv_obj_align(power_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -24, 0);
                lvgl_sys::lv_obj_set_style_img_recolor(power_img, lv_color_hex(0xFFA500), 0);
                lvgl_sys::lv_obj_set_style_img_recolor_opa(power_img, 255, 0);

                // Dropdown arrow
                let arrow_label = lvgl_sys::lv_label_create(printer_btn);
                let arrow_text = CString::new("v").unwrap();
                lvgl_sys::lv_label_set_text(arrow_label, arrow_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(arrow_label, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(arrow_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -8, 2);

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
                let _content_height: i16 = 280;
                let card_gap: i16 = 8;

                // Button dimensions (defined first so we can calculate left card width)
                let btn_width: i16 = 130;
                let btn_gap: i16 = 8;
                let btn_start_x: i16 = 800 - 16 - btn_width - btn_gap - btn_width;

                // Left column - Printer Card (expanded)
                let left_card_width = btn_start_x - 16 - card_gap;
                let printer_card = create_card(scr, 16, content_y, left_card_width, 130);

                // Print thumbnail frame - polished with inner shadow and 3D cube icon
                let cover_size: i16 = 70;
                let cover_img = lvgl_sys::lv_obj_create(printer_card);
                lvgl_sys::lv_obj_set_size(cover_img, cover_size, cover_size);
                lvgl_sys::lv_obj_set_pos(cover_img, 12, 12);
                lvgl_sys::lv_obj_set_style_bg_color(cover_img, lv_color_hex(0x1A1A1A), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(cover_img, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(cover_img, 10, 0);
                lvgl_sys::lv_obj_set_style_border_color(cover_img, lv_color_hex(0x3A3A3A), 0);
                lvgl_sys::lv_obj_set_style_border_width(cover_img, 1, 0);
                set_style_pad_all(cover_img, 0);

                // 3D cube icon - front face
                let cube_front = lvgl_sys::lv_obj_create(cover_img);
                lvgl_sys::lv_obj_set_size(cube_front, 24, 24);
                lvgl_sys::lv_obj_set_pos(cube_front, 18, 26);
                lvgl_sys::lv_obj_set_style_bg_opa(cube_front, 0, 0);
                lvgl_sys::lv_obj_set_style_border_color(cube_front, lv_color_hex(0x505050), 0);
                lvgl_sys::lv_obj_set_style_border_width(cube_front, 2, 0);
                lvgl_sys::lv_obj_set_style_radius(cube_front, 2, 0);
                set_style_pad_all(cube_front, 0);

                // 3D cube icon - top face (parallelogram effect with offset rectangle)
                let cube_top = lvgl_sys::lv_obj_create(cover_img);
                lvgl_sys::lv_obj_set_size(cube_top, 24, 10);
                lvgl_sys::lv_obj_set_pos(cube_top, 26, 18);
                lvgl_sys::lv_obj_set_style_bg_opa(cube_top, 0, 0);
                lvgl_sys::lv_obj_set_style_border_color(cube_top, lv_color_hex(0x505050), 0);
                lvgl_sys::lv_obj_set_style_border_width(cube_top, 2, 0);
                lvgl_sys::lv_obj_set_style_radius(cube_top, 2, 0);
                set_style_pad_all(cube_top, 0);

                // 3D cube icon - side face
                let cube_side = lvgl_sys::lv_obj_create(cover_img);
                lvgl_sys::lv_obj_set_size(cube_side, 10, 24);
                lvgl_sys::lv_obj_set_pos(cube_side, 40, 26);
                lvgl_sys::lv_obj_set_style_bg_opa(cube_side, 0, 0);
                lvgl_sys::lv_obj_set_style_border_color(cube_side, lv_color_hex(0x505050), 0);
                lvgl_sys::lv_obj_set_style_border_width(cube_side, 2, 0);
                lvgl_sys::lv_obj_set_style_radius(cube_side, 2, 0);
                set_style_pad_all(cube_side, 0);

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

                let ams_btn = create_action_button(scr, btn_start_x, content_y, btn_width, top_btn_height, "AMS Setup", "", "ams");
                lvgl_sys::lv_obj_add_flag(ams_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(ams_btn, Some(btn_ams_setup_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

                let catalog_btn = create_action_button(scr, btn_start_x, content_y + 138, btn_width, bottom_btn_height, "Catalog", "", "catalog");
                lvgl_sys::lv_obj_add_flag(catalog_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(catalog_btn, Some(btn_catalog_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

                let encode_btn = create_action_button(scr, btn_start_x + btn_width + btn_gap, content_y, btn_width, top_btn_height, "Encode Tag", "", "encode");
                lvgl_sys::lv_obj_add_flag(encode_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(encode_btn, Some(btn_encode_tag_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

                let settings_btn = create_action_button(scr, btn_start_x + btn_width + btn_gap, content_y + 138, btn_width, bottom_btn_height, "Settings", "", "settings");
                lvgl_sys::lv_obj_add_flag(settings_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(settings_btn, Some(btn_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

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

                // Background
                lvgl_sys::lv_obj_set_style_bg_color(ams_scr, lv_color_hex(COLOR_BG), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_scr, 255, 0);
                set_style_pad_all(ams_scr, 0);
                lvgl_sys::lv_obj_clear_flag(ams_scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                // === STATUS BAR (shared style with home) ===
                let ams_status_bar = lvgl_sys::lv_obj_create(ams_scr);
                lvgl_sys::lv_obj_set_size(ams_status_bar, 800, 44);
                lvgl_sys::lv_obj_set_pos(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(ams_status_bar, lv_color_hex(COLOR_STATUS_BAR), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_status_bar, 255, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_pad_left(ams_status_bar, 16, 0);
                lvgl_sys::lv_obj_set_style_pad_right(ams_status_bar, 16, 0);
                lvgl_sys::lv_obj_set_style_shadow_color(ams_status_bar, lv_color_hex(0x000000), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(ams_status_bar, 25, 0);
                lvgl_sys::lv_obj_set_style_shadow_ofs_y(ams_status_bar, 8, 0);
                lvgl_sys::lv_obj_set_style_shadow_spread(ams_status_bar, 0, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(ams_status_bar, 200, 0);
                lvgl_sys::lv_obj_clear_flag(ams_status_bar, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
                lvgl_sys::lv_obj_clear_flag(ams_status_bar, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_flag(ams_status_bar, lvgl_sys::LV_OBJ_FLAG_EVENT_BUBBLE);
                // SpoolBuddy logo
                let ams_logo_img = lvgl_sys::lv_img_create(ams_status_bar);
                lvgl_sys::lv_img_set_src(ams_logo_img, &raw const LOGO_IMG_DSC as *const _);
                lvgl_sys::lv_obj_align(ams_logo_img, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 48, 0);

                // Printer selector (center)
                let ams_printer_btn = lvgl_sys::lv_btn_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(ams_printer_btn, 200, 32);
                lvgl_sys::lv_obj_align(ams_printer_btn, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(ams_printer_btn, lv_color_hex(0x242424), 0);
                lvgl_sys::lv_obj_set_style_radius(ams_printer_btn, 16, 0);
                lvgl_sys::lv_obj_set_style_border_color(ams_printer_btn, lv_color_hex(0x3D3D3D), 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_printer_btn, 1, 0);

                let ams_left_dot = lvgl_sys::lv_obj_create(ams_printer_btn);
                lvgl_sys::lv_obj_set_size(ams_left_dot, 8, 8);
                lvgl_sys::lv_obj_align(ams_left_dot, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 12, 0);
                lvgl_sys::lv_obj_set_style_bg_color(ams_left_dot, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_radius(ams_left_dot, 4, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_left_dot, 0, 0);
                lvgl_sys::lv_obj_set_style_shadow_color(ams_left_dot, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_shadow_width(ams_left_dot, 6, 0);
                lvgl_sys::lv_obj_set_style_shadow_spread(ams_left_dot, 2, 0);
                lvgl_sys::lv_obj_set_style_shadow_opa(ams_left_dot, 150, 0);

                let ams_printer_label = lvgl_sys::lv_label_create(ams_printer_btn);
                let ams_printer_text = CString::new("X1C-Studio").unwrap();
                lvgl_sys::lv_label_set_text(ams_printer_label, ams_printer_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(ams_printer_label, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(ams_printer_label, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 28, 0);

                let ams_power_img = lvgl_sys::lv_img_create(ams_printer_btn);
                lvgl_sys::lv_img_set_src(ams_power_img, &raw const POWER_IMG_DSC as *const _);
                lvgl_sys::lv_obj_align(ams_power_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -24, 0);
                lvgl_sys::lv_obj_set_style_img_recolor(ams_power_img, lv_color_hex(0xFFA500), 0);
                lvgl_sys::lv_obj_set_style_img_recolor_opa(ams_power_img, 255, 0);

                let ams_arrow_label = lvgl_sys::lv_label_create(ams_printer_btn);
                let ams_arrow_text = CString::new("v").unwrap();
                lvgl_sys::lv_label_set_text(ams_arrow_label, ams_arrow_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(ams_arrow_label, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(ams_arrow_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -8, 2);

                // Time
                let ams_time_label = lvgl_sys::lv_label_create(ams_status_bar);
                let ams_time_text = CString::new("14:23").unwrap();
                lvgl_sys::lv_label_set_text(ams_time_label, ams_time_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(ams_time_label, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(ams_time_label, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, 0, 0);

                // WiFi bars
                let ams_wifi_x = -50;
                let ams_wifi_bottom = 8;
                let ams_wifi_bar3 = lvgl_sys::lv_obj_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(ams_wifi_bar3, 4, 16);
                lvgl_sys::lv_obj_align(ams_wifi_bar3, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, ams_wifi_x, ams_wifi_bottom - 8);
                lvgl_sys::lv_obj_set_style_bg_color(ams_wifi_bar3, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_wifi_bar3, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_wifi_bar3, 1, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_wifi_bar3, 0, 0);

                let ams_wifi_bar2 = lvgl_sys::lv_obj_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(ams_wifi_bar2, 4, 12);
                lvgl_sys::lv_obj_align(ams_wifi_bar2, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, ams_wifi_x - 6, ams_wifi_bottom - 6);
                lvgl_sys::lv_obj_set_style_bg_color(ams_wifi_bar2, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_wifi_bar2, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_wifi_bar2, 1, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_wifi_bar2, 0, 0);

                let ams_wifi_bar1 = lvgl_sys::lv_obj_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(ams_wifi_bar1, 4, 8);
                lvgl_sys::lv_obj_align(ams_wifi_bar1, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, ams_wifi_x - 12, ams_wifi_bottom - 4);
                lvgl_sys::lv_obj_set_style_bg_color(ams_wifi_bar1, lv_color_hex(COLOR_ACCENT), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_wifi_bar1, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_wifi_bar1, 1, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_wifi_bar1, 0, 0);

                // Bell icon
                let ams_bell_img = lvgl_sys::lv_img_create(ams_status_bar);
                lvgl_sys::lv_img_set_src(ams_bell_img, &raw const BELL_IMG_DSC as *const _);
                lvgl_sys::lv_obj_align(ams_bell_img, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -82, 0);

                let ams_badge = lvgl_sys::lv_obj_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(ams_badge, 14, 14);
                lvgl_sys::lv_obj_align(ams_badge, lvgl_sys::LV_ALIGN_RIGHT_MID as u8, -70, -8);
                lvgl_sys::lv_obj_set_style_bg_color(ams_badge, lv_color_hex(0xFF4444), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_badge, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_badge, 7, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_badge, 0, 0);
                lvgl_sys::lv_obj_clear_flag(ams_badge, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                let ams_badge_text = lvgl_sys::lv_label_create(ams_badge);
                let ams_badge_str = CString::new("3").unwrap();
                lvgl_sys::lv_label_set_text(ams_badge_text, ams_badge_str.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(ams_badge_text, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_align(ams_badge_text, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // === MAIN CONTENT AREA ===
                let ams_content_y: i16 = 48;
                let ams_panel_x: i16 = 8;
                let ams_sidebar_x: i16 = 616;
                let ams_panel_w: i16 = ams_sidebar_x - ams_panel_x - 8;
                let ams_panel_h: i16 = 388;

                // === AMS PANEL - ONE container card for all units ===
                let ams_panel = lvgl_sys::lv_obj_create(ams_scr);
                lvgl_sys::lv_obj_set_size(ams_panel, ams_panel_w, ams_panel_h);
                lvgl_sys::lv_obj_set_pos(ams_panel, ams_panel_x, ams_content_y);
                lvgl_sys::lv_obj_set_style_bg_color(ams_panel, lv_color_hex(0x2D2D2D), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(ams_panel, 255, 0);
                lvgl_sys::lv_obj_set_style_radius(ams_panel, 12, 0);
                lvgl_sys::lv_obj_set_style_border_width(ams_panel, 0, 0);
                set_style_pad_all(ams_panel, 10);
                lvgl_sys::lv_obj_clear_flag(ams_panel, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                // "AMS Units" title INSIDE the panel
                let ams_panel_title = lvgl_sys::lv_label_create(ams_panel);
                let ams_panel_title_text = CString::new("AMS Units").unwrap();
                lvgl_sys::lv_label_set_text(ams_panel_title, ams_panel_title_text.as_ptr());
                lvgl_sys::lv_obj_set_style_text_color(ams_panel_title, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_set_style_text_font(ams_panel_title, &lvgl_sys::lv_font_montserrat_14, 0);
                lvgl_sys::lv_obj_set_pos(ams_panel_title, 0, 0);

                // Grid layout inside panel
                let ams_unit_gap: i16 = 4;
                let ams_row1_y: i16 = 22;
                let ams_row1_h: i16 = 170;
                let ams_row2_y: i16 = ams_row1_y + ams_row1_h + ams_unit_gap;
                let ams_row2_h: i16 = 170;

                // Row 1: AMS A, AMS B, AMS C (3 equal width units)
                let ams_inner_w: i16 = ams_panel_w - 20;
                let ams_unit_w_4slot: i16 = (ams_inner_w - 2 * ams_unit_gap) / 3;

                create_ams_unit_compact(ams_panel, 0, ams_row1_y, ams_unit_w_4slot, ams_row1_h,
                    "AMS A", "L", "19%", "25°C", true, &[
                        ("PLA", 0xF5C518, "A1", "85%", true),
                        ("PETG", 0x333333, "A2", "62%", false),
                        ("PETG", 0xFF9800, "A3", "45%", false),
                        ("PLA", 0x9E9E9E, "A4", "90%", false),
                    ]);

                create_ams_unit_compact(ams_panel, ams_unit_w_4slot + ams_unit_gap, ams_row1_y, ams_unit_w_4slot, ams_row1_h,
                    "AMS B", "L", "24%", "24°C", false, &[
                        ("PLA", 0xE91E63, "B1", "72%", false),
                        ("PLA", 0x2196F3, "B2", "55%", false),
                        ("PETG", 0x4CAF50, "B3", "33%", false),
                        ("", 0, "B4", "", false),
                    ]);

                create_ams_unit_compact(ams_panel, 2 * (ams_unit_w_4slot + ams_unit_gap), ams_row1_y, ams_unit_w_4slot, ams_row1_h,
                    "AMS C", "R", "31%", "23°C", false, &[
                        ("ASA", 0xFFFFFF, "C1", "95%", false),
                        ("ASA", 0x212121, "C2", "88%", false),
                        ("", 0, "C3", "", false),
                        ("", 0, "C4", "", false),
                    ]);

                // Row 2: AMS D (4 slots), HT-A, HT-B, Ext 1, Ext 2
                let ams_d_w: i16 = ams_unit_w_4slot;
                let ams_single_w: i16 = (ams_inner_w - ams_d_w - 4 * ams_unit_gap) / 4;

                create_ams_unit_compact(ams_panel, 0, ams_row2_y, ams_d_w, ams_row2_h,
                    "AMS D", "R", "28%", "22°C", false, &[
                        ("PLA", 0x00BCD4, "D1", "100%", false),
                        ("PLA", 0xFF5722, "D2", "67%", false),
                        ("", 0, "D3", "", false),
                        ("", 0, "D4", "", false),
                    ]);

                let ams_sx = ams_d_w + ams_unit_gap;
                create_single_unit_compact(ams_panel, ams_sx, ams_row2_y, ams_single_w, ams_row2_h,
                    "HT-A", "L", "42%", "65°C", "ABS", 0x673AB7, "78%");
                create_single_unit_compact(ams_panel, ams_sx + ams_single_w + ams_unit_gap, ams_row2_y, ams_single_w, ams_row2_h,
                    "HT-B", "R", "38%", "58°C", "PC", 0xECEFF1, "52%");
                create_ext_unit_compact(ams_panel, ams_sx + 2 * (ams_single_w + ams_unit_gap), ams_row2_y, ams_single_w, ams_row2_h,
                    "Ext 1", "L", "TPU", 0x607D8B);
                create_ext_unit_compact(ams_panel, ams_sx + 3 * (ams_single_w + ams_unit_gap), ams_row2_y, ams_single_w, ams_row2_h,
                    "Ext 2", "R", "PVA", 0x8BC34A);

                // === RIGHT SIDEBAR - Action buttons (2x2 grid) ===
                let ams_btn_x: i16 = 620;
                let ams_btn_y: i16 = ams_content_y;
                let ams_btn_w: i16 = 82;
                let ams_btn_h: i16 = 82;
                let ams_btn_gap: i16 = 8;

                let ams_scan_btn = create_action_button_small(ams_scr, ams_btn_x, ams_btn_y, ams_btn_w, ams_btn_h, "Scan", "", "nfc");
                lvgl_sys::lv_obj_add_flag(ams_scan_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(ams_scan_btn, Some(btn_nfc_reader_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
                let ams_catalog_btn = create_action_button_small(ams_scr, ams_btn_x + ams_btn_w + ams_btn_gap, ams_btn_y, ams_btn_w, ams_btn_h, "Catalog", "", "catalog");
                lvgl_sys::lv_obj_add_flag(ams_catalog_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(ams_catalog_btn, Some(btn_catalog_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
                let ams_calibrate_btn = create_action_button_small(ams_scr, ams_btn_x, ams_btn_y + ams_btn_h + ams_btn_gap, ams_btn_w, ams_btn_h, "Calibrate", "", "calibrate");
                lvgl_sys::lv_obj_add_flag(ams_calibrate_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(ams_calibrate_btn, Some(btn_scale_calibration_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
                let ams_settings_btn = create_action_button_small(ams_scr, ams_btn_x + ams_btn_w + ams_btn_gap, ams_btn_y + ams_btn_h + ams_btn_gap, ams_btn_w, ams_btn_h, "Settings", "", "settings");
                lvgl_sys::lv_obj_add_flag(ams_settings_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(ams_settings_btn, Some(btn_settings_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());

                // === BACK BUTTON (child of status bar for proper click handling) ===
                let ams_back_btn = lvgl_sys::lv_btn_create(ams_status_bar);
                lvgl_sys::lv_obj_set_size(ams_back_btn, 36, 28);
                lvgl_sys::lv_obj_align(ams_back_btn, lvgl_sys::LV_ALIGN_LEFT_MID as u8, 0, 0);
                lvgl_sys::lv_obj_set_style_bg_color(ams_back_btn, lv_color_hex(0x2D2D2D), 0);
                lvgl_sys::lv_obj_set_style_radius(ams_back_btn, 4, 0);
                lvgl_sys::lv_obj_set_style_shadow_width(ams_back_btn, 0, 0);
                lvgl_sys::lv_obj_add_flag(ams_back_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(ams_back_btn, Some(btn_back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
                let ams_back_lbl = lvgl_sys::lv_label_create(ams_back_btn);
                lvgl_sys::lv_label_set_text(ams_back_lbl, b"<\0".as_ptr() as *const i8);
                lvgl_sys::lv_obj_set_style_text_color(ams_back_lbl, lv_color_hex(COLOR_WHITE), 0);
                lvgl_sys::lv_obj_set_style_text_font(ams_back_lbl, &lvgl_sys::lv_font_montserrat_16, 0);
                lvgl_sys::lv_obj_align(ams_back_lbl, lvgl_sys::LV_ALIGN_CENTER as u8, 0, 0);

                // === BOTTOM STATUS BAR ===
                create_bottom_status_bar(ams_scr);

                info!("  AMS Overview screen created");

                // ==================== PLACEHOLDER SCREENS ====================
                // Encode Tag screen
                let encode_scr = lvgl_sys::lv_obj_create(ptr::null_mut());
                SCREEN_ENCODE = encode_scr;
                lvgl_sys::lv_obj_set_style_bg_color(encode_scr, make_color(0x1A, 0x1A, 0x1A), 0);
                lvgl_sys::lv_obj_clear_flag(encode_scr, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                let encode_back_btn = lvgl_sys::lv_btn_create(encode_scr);
                lvgl_sys::lv_obj_set_size(encode_back_btn, 36, 28);
                lvgl_sys::lv_obj_set_pos(encode_back_btn, 8, 8);
                lvgl_sys::lv_obj_set_style_bg_color(encode_back_btn, make_color(0x2D, 0x2D, 0x2D), 0);
                lvgl_sys::lv_obj_set_style_radius(encode_back_btn, 4, 0);
                lvgl_sys::lv_obj_set_style_shadow_width(encode_back_btn, 0, 0);
                lvgl_sys::lv_obj_add_flag(encode_back_btn, lvgl_sys::LV_OBJ_FLAG_CLICKABLE);
                lvgl_sys::lv_obj_add_event_cb(encode_back_btn, Some(btn_back_cb), lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED, ptr::null_mut());
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

                // Catalog screen (full grid with search and filters)
                SCREEN_CATALOG = create_catalog_screen_fn();
                info!("  Catalog screen created");

                // Settings screen (polished version with sections)
                SCREEN_SETTINGS = create_settings_screen_fn();
                info!("  Settings screen created");

                // Settings page 2 (Hardware & System)
                SCREEN_SETTINGS_2 = create_settings_2_screen();
                info!("  Settings 2 screen created");

                // About screen
                SCREEN_ABOUT = create_about_screen();
                info!("  About screen created");

                // Scale Calibration screen
                SCREEN_SCALE_CALIBRATION = create_scale_calibration_screen();
                info!("  Scale Calibration screen created");

                // NFC Reader screen
                SCREEN_NFC_READER = create_nfc_reader_screen();
                info!("  NFC Reader screen created");

                // Display Brightness screen
                SCREEN_DISPLAY_BRIGHTNESS = create_display_brightness_screen();
                info!("  Display Brightness screen created");

                // Advanced Settings screen
                SCREEN_ADVANCED_SETTINGS = create_advanced_settings_screen();
                info!("  Advanced Settings screen created");

                // WiFi Settings screen
                SCREEN_WIFI_SETTINGS = create_wifi_settings_screen();
                info!("  WiFi Settings screen created");

                // Backend Settings screen
                SCREEN_BACKEND_SETTINGS = create_backend_settings_screen();
                info!("  Backend Settings screen created");

                // Add Printer screen
                SCREEN_ADD_PRINTER = create_add_printer_screen();
                info!("  Add Printer screen created");

                // Scan Result screen
                SCREEN_SCAN_RESULT = create_scan_result_screen_fn();
                info!("  Scan Result screen created");

                // Spool Detail screen
                SCREEN_SPOOL_DETAIL = create_spool_detail_screen_fn();
                info!("  Spool Detail screen created");

                // Load home screen as initial screen
                lvgl_sys::lv_disp_load_scr(SCREEN_HOME);
                info!("  Initial screen loaded");
            }

            info!("========================================");
            info!("=== LVGL DISPLAY RUNNING ===");
            info!("========================================");

            // Initialize WiFi (after LCD panel to preserve internal SRAM for bounce buffers)
            info!("[7/7] Initializing WiFi...");

            // Log heap status before WiFi init
            unsafe {
                let free_internal = esp_idf_sys::heap_caps_get_free_size(esp_idf_sys::MALLOC_CAP_INTERNAL);
                let free_8bit = esp_idf_sys::heap_caps_get_free_size(esp_idf_sys::MALLOC_CAP_8BIT);
                let free_dma = esp_idf_sys::heap_caps_get_free_size(esp_idf_sys::MALLOC_CAP_DMA);
                info!("Heap before WiFi: internal={}KB, 8bit={}KB, DMA={}KB",
                    free_internal / 1024, free_8bit / 1024, free_dma / 1024);
            }

            let sysloop = EspSystemEventLoop::take().unwrap();
            let nvs = EspDefaultNvsPartition::take().ok();

            let _wifi_ip = match wifi_init::connect_wifi(modem, sysloop, nvs) {
                Ok(ip) => {
                    info!("WiFi connected! IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
                    Some(ip)
                }
                Err(e) => {
                    warn!("WiFi failed: {:?} - continuing without network", e);
                    None
                }
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
                        }
                    }
                }

                unsafe {
                    // Tell LVGL how much time has passed
                    lvgl_sys::lv_tick_inc(tick_period_ms);
                    // Process LVGL tasks
                    lvgl_sys::lv_timer_handler();
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
#[allow(dead_code)]
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
