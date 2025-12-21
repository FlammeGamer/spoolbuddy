//! SpoolBuddy PC Simulator
//!
//! Run the UI natively on desktop using SDL2, or in headless mode for remote servers.
//!
//! # Usage
//! ```bash
//! # Interactive mode (requires display)
//! cargo run --release
//!
//! # Headless mode - renders all screens to PNG files
//! cargo run --release -- --headless
//! ```
//!
//! # Controls (interactive mode)
//! - Click: Touch input
//! - 1-6: Switch screens (Home, SpoolInfo, AmsSelect, Settings, Calibration, WifiSetup)
//! - W/S: Increase/decrease weight
//! - Space: Toggle weight stable indicator
//! - Q: Quit

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use log::info;
use spoolbuddy_ui::{
    render, Screen, SpoolDisplay, SpoolSource, TouchEvent, UiManager, DISPLAY_HEIGHT, DISPLAY_WIDTH,
};

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    let headless = args.iter().any(|a| a == "--headless" || a == "-h");

    if headless {
        run_headless();
    } else {
        run_interactive();
    }
}

fn run_headless() {
    use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay};
    use std::fs::File;
    use std::io::Write;

    info!("SpoolBuddy Simulator - HEADLESS MODE");
    info!("Display: {}x{}", DISPLAY_WIDTH, DISPLAY_HEIGHT);

    // Create simulator display
    let mut display: SimulatorDisplay<Rgb565> =
        SimulatorDisplay::new(embedded_graphics::geometry::Size::new(
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
        ));

    // Create UI manager with demo state
    let mut ui = UiManager::new();
    ui.set_weight(1234.5, true);
    ui.set_wifi_status(true, Some("HomeNetwork"));
    ui.set_server_connected(true);

    let demo_spool = create_demo_spool();

    // Create output directory
    std::fs::create_dir_all("screenshots").unwrap();

    // Render each screen
    let screens = [
        (Screen::Home, "home"),
        (Screen::SpoolInfo, "spool_info"),
        (Screen::AmsSelect, "ams_select"),
        (Screen::Settings, "settings"),
        (Screen::Calibration, "calibration"),
    ];

    for (screen, name) in screens.iter() {
        ui.navigate(*screen);

        // For SpoolInfo, load the demo spool
        if *screen == Screen::SpoolInfo {
            ui.set_spool(Some(demo_spool.clone()));
        }

        // Render
        if let Err(e) = render(&mut display, &ui) {
            log::error!("Render error for {}: {:?}", name, e);
            continue;
        }
        ui.mark_clean();

        // Save as BMP (simpler than PNG, no extra deps)
        let filename = format!("screenshots/{}.bmp", name);
        save_display_as_bmp(&display, &filename);
        info!("Saved: {}", filename);
    }

    info!("All screenshots saved to screenshots/ directory");
}

fn save_display_as_bmp(display: &embedded_graphics_simulator::SimulatorDisplay<Rgb565>, filename: &str) {
    use std::fs::File;
    use std::io::Write;

    let width = DISPLAY_WIDTH as u32;
    let height = DISPLAY_HEIGHT as u32;

    // BMP file header (14 bytes) + DIB header (40 bytes) = 54 bytes
    let row_size = ((width * 3 + 3) / 4) * 4; // Rows padded to 4-byte boundary
    let pixel_data_size = row_size * height;
    let file_size = 54 + pixel_data_size;

    let mut file = File::create(filename).unwrap();

    // BMP File Header
    file.write_all(b"BM").unwrap(); // Signature
    file.write_all(&(file_size as u32).to_le_bytes()).unwrap(); // File size
    file.write_all(&[0u8; 4]).unwrap(); // Reserved
    file.write_all(&54u32.to_le_bytes()).unwrap(); // Pixel data offset

    // DIB Header (BITMAPINFOHEADER)
    file.write_all(&40u32.to_le_bytes()).unwrap(); // Header size
    file.write_all(&(width as i32).to_le_bytes()).unwrap(); // Width
    file.write_all(&(-(height as i32)).to_le_bytes()).unwrap(); // Height (negative = top-down)
    file.write_all(&1u16.to_le_bytes()).unwrap(); // Planes
    file.write_all(&24u16.to_le_bytes()).unwrap(); // Bits per pixel
    file.write_all(&0u32.to_le_bytes()).unwrap(); // Compression (none)
    file.write_all(&(pixel_data_size as u32).to_le_bytes()).unwrap(); // Image size
    file.write_all(&2835u32.to_le_bytes()).unwrap(); // X pixels per meter
    file.write_all(&2835u32.to_le_bytes()).unwrap(); // Y pixels per meter
    file.write_all(&0u32.to_le_bytes()).unwrap(); // Colors in table
    file.write_all(&0u32.to_le_bytes()).unwrap(); // Important colors

    // Pixel data (BGR format, rows padded)
    let padding = (row_size - width * 3) as usize;
    for y in 0..height {
        for x in 0..width {
            let point = embedded_graphics::geometry::Point::new(x as i32, y as i32);
            let color = display.get_pixel(point);

            // Convert RGB565 to BGR24
            let r = ((color.r() as u32 * 255) / 31) as u8;
            let g = ((color.g() as u32 * 255) / 63) as u8;
            let b = ((color.b() as u32 * 255) / 31) as u8;

            file.write_all(&[b, g, r]).unwrap();
        }
        // Row padding
        for _ in 0..padding {
            file.write_all(&[0u8]).unwrap();
        }
    }
}

fn run_interactive() {
    use embedded_graphics_simulator::{
        sdl2::Keycode, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
    };

    info!("SpoolBuddy Simulator starting...");
    info!("Display: {}x{}", DISPLAY_WIDTH, DISPLAY_HEIGHT);

    // Create simulator display
    let mut display: SimulatorDisplay<Rgb565> =
        SimulatorDisplay::new(embedded_graphics::geometry::Size::new(
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
        ));

    // Create window with output settings
    let output_settings = OutputSettingsBuilder::new()
        .scale(1)
        .pixel_spacing(0)
        .build();
    let mut window = Window::new("SpoolBuddy Simulator", &output_settings);

    // Create UI manager
    let mut ui = UiManager::new();

    // Set some demo state
    ui.set_weight(1234.5, true);
    ui.set_wifi_status(true, Some("HomeNetwork"));
    ui.set_server_connected(true);

    // Demo spool for testing SpoolInfo screen
    let demo_spool = create_demo_spool();

    info!("Controls:");
    info!("  Click: Touch input");
    info!("  1-6: Switch screens");
    info!("  W/S: Increase/decrease weight");
    info!("  Space: Toggle weight stable");
    info!("  P: Load demo spool");
    info!("  Q/Esc: Quit");

    // Main loop
    'running: loop {
        // Render UI
        if ui.is_dirty() {
            if let Err(e) = render(&mut display, &ui) {
                log::error!("Render error: {:?}", e);
            }
            ui.mark_clean();
        }

        // Update window
        window.update(&display);

        // Handle events
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,

                SimulatorEvent::KeyDown { keycode, .. } => {
                    match keycode {
                        // Quit
                        Keycode::Q | Keycode::Escape => {
                            break 'running;
                        }

                        // Screen navigation
                        Keycode::Num1 => {
                            ui.navigate(Screen::Home);
                        }
                        Keycode::Num2 => {
                            ui.navigate(Screen::SpoolInfo);
                        }
                        Keycode::Num3 => {
                            ui.navigate(Screen::AmsSelect);
                        }
                        Keycode::Num4 => {
                            ui.navigate(Screen::Settings);
                        }
                        Keycode::Num5 => {
                            ui.navigate(Screen::Calibration);
                        }
                        Keycode::Num6 => {
                            ui.navigate(Screen::WifiSetup);
                        }

                        // Weight control
                        Keycode::W => {
                            let current = ui.state().weight;
                            ui.set_weight(current + 10.0, ui.state().weight_stable);
                        }
                        Keycode::S => {
                            let current = ui.state().weight;
                            ui.set_weight((current - 10.0).max(0.0), ui.state().weight_stable);
                        }

                        // Toggle stable
                        Keycode::Space => {
                            let stable = !ui.state().weight_stable;
                            ui.set_weight(ui.state().weight, stable);
                        }

                        // Load demo spool
                        Keycode::P => {
                            ui.set_spool(Some(demo_spool.clone()));
                        }

                        _ => {}
                    }
                }

                SimulatorEvent::MouseButtonDown { point, .. } => {
                    info!("Touch at ({}, {})", point.x, point.y);
                    if let Some(action) = ui.handle_touch(TouchEvent::Press {
                        x: point.x as u16,
                        y: point.y as u16,
                    }) {
                        info!("UI Action: {:?}", action);
                    }
                }

                SimulatorEvent::MouseButtonUp { point, .. } => {
                    ui.handle_touch(TouchEvent::Release {
                        x: point.x as u16,
                        y: point.y as u16,
                    });
                }

                SimulatorEvent::MouseMove { point, .. } => {
                    // Could track drag for future features
                    let _ = point;
                }

                _ => {}
            }
        }

        // Small delay to prevent CPU spinning
        std::thread::sleep(std::time::Duration::from_millis(16)); // ~60fps
    }

    info!("Simulator closed.");
}

fn create_demo_spool() -> SpoolDisplay {
    let mut id = heapless::String::new();
    let _ = id.push_str("spool_demo_001");

    let mut material = heapless::String::new();
    let _ = material.push_str("PLA");

    let mut color_name = heapless::String::new();
    let _ = color_name.push_str("Ocean Blue");

    let mut brand = heapless::String::new();
    let _ = brand.push_str("Bambu Lab");

    SpoolDisplay {
        id,
        material,
        color_name,
        brand,
        color_rgba: 0x2196F3FF, // Material Design Blue
        weight_current: 850.0,
        weight_label: 1000.0,
        k_value: Some(0.98),
        source: SpoolSource::Bambu,
    }
}
