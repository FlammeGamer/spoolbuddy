//! HTTP Screenshot Server for development
//!
//! Provides an HTTP endpoint to capture the display framebuffer.
//! Access via browser: http://<device-ip>/screenshot
//!
//! Also serves a simple viewer page at http://<device-ip>/

use esp_idf_hal::io::Write;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::http::Method;
use log::info;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

// Display dimensions
const WIDTH: usize = 800;
const HEIGHT: usize = 480;

// Global framebuffer pointer (set from main.rs)
pub static FRAMEBUFFER_PTR: AtomicPtr<u16> = AtomicPtr::new(ptr::null_mut());

/// Set the framebuffer pointer for screenshot capture
pub fn set_framebuffer(ptr: *mut u16) {
    FRAMEBUFFER_PTR.store(ptr, Ordering::SeqCst);
}

/// Start the HTTP screenshot server
/// Returns the server handle (must be kept alive)
pub fn start_server() -> anyhow::Result<EspHttpServer<'static>> {
    let config = Configuration {
        stack_size: 8192,
        ..Default::default()
    };

    let mut server = EspHttpServer::new(&config)?;

    // Root page - simple viewer with auto-refresh
    server.fn_handler("/", Method::Get, |req| {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>SpoolBuddy Display</title>
    <style>
        body { background: #1a1a2e; margin: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; font-family: system-ui; }
        h1 { color: #eee; margin-bottom: 20px; }
        img { border: 2px solid #333; border-radius: 8px; max-width: 100%; height: auto; }
        .controls { margin-top: 20px; }
        button { background: #4a4a6a; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; margin: 0 5px; }
        button:hover { background: #5a5a7a; }
        .info { color: #888; margin-top: 10px; font-size: 14px; }
    </style>
</head>
<body>
    <h1>SpoolBuddy Display</h1>
    <img id="display" src="/screenshot" alt="Display">
    <div class="controls">
        <button onclick="refresh()">Refresh</button>
        <button onclick="toggleAuto()">Auto-refresh: <span id="autoStatus">OFF</span></button>
    </div>
    <div class="info">800x480 RGB565 | Click refresh or enable auto-refresh</div>
    <script>
        let autoRefresh = false;
        let interval = null;
        function refresh() {
            document.getElementById('display').src = '/screenshot?' + Date.now();
        }
        function toggleAuto() {
            autoRefresh = !autoRefresh;
            document.getElementById('autoStatus').textContent = autoRefresh ? 'ON' : 'OFF';
            if (autoRefresh) {
                interval = setInterval(refresh, 1000);
            } else {
                clearInterval(interval);
            }
        }
    </script>
</body>
</html>"#;

        req.into_ok_response()?
            .write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Screenshot endpoint - returns BMP image
    server.fn_handler("/screenshot", Method::Get, |req| {
        let fb_ptr = FRAMEBUFFER_PTR.load(Ordering::SeqCst);

        if fb_ptr.is_null() {
            req.into_status_response(500)?
                .write_all(b"Framebuffer not available")?;
            return Ok::<(), anyhow::Error>(());
        }

        // Create BMP file in memory
        let bmp_data = create_bmp_from_rgb565(fb_ptr);

        let mut response = req.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "image/bmp"),
                ("Cache-Control", "no-cache"),
            ],
        )?;

        response.write_all(&bmp_data)?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Raw RGB565 endpoint (for tools that prefer raw data)
    server.fn_handler("/raw", Method::Get, |req| {
        let fb_ptr = FRAMEBUFFER_PTR.load(Ordering::SeqCst);

        if fb_ptr.is_null() {
            req.into_status_response(500)?
                .write_all(b"Framebuffer not available")?;
            return Ok::<(), anyhow::Error>(());
        }

        let mut response = req.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "application/octet-stream"),
                ("X-Width", "800"),
                ("X-Height", "480"),
                ("X-Format", "RGB565"),
            ],
        )?;

        // Write raw framebuffer data
        unsafe {
            let data = std::slice::from_raw_parts(fb_ptr as *const u8, WIDTH * HEIGHT * 2);
            response.write_all(data)?;
        }

        Ok::<(), anyhow::Error>(())
    })?;

    info!("HTTP screenshot server started");
    info!("  Viewer:     http://<ip>/");
    info!("  Screenshot: http://<ip>/screenshot");
    info!("  Raw data:   http://<ip>/raw");

    Ok(server)
}

/// Create a BMP file from RGB565 framebuffer
fn create_bmp_from_rgb565(fb_ptr: *mut u16) -> Vec<u8> {
    // BMP file format:
    // - 14 byte file header
    // - 40 byte DIB header (BITMAPINFOHEADER)
    // - Pixel data (bottom-up, RGB888)

    let pixel_data_size = WIDTH * HEIGHT * 3; // RGB888
    let file_size = 14 + 40 + pixel_data_size;

    let mut bmp = Vec::with_capacity(file_size);

    // BMP File Header (14 bytes)
    bmp.extend_from_slice(b"BM");                          // Signature
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes()); // File size
    bmp.extend_from_slice(&[0u8; 4]);                      // Reserved
    bmp.extend_from_slice(&54u32.to_le_bytes());           // Pixel data offset

    // DIB Header - BITMAPINFOHEADER (40 bytes)
    bmp.extend_from_slice(&40u32.to_le_bytes());           // Header size
    bmp.extend_from_slice(&(WIDTH as i32).to_le_bytes());  // Width
    bmp.extend_from_slice(&(-(HEIGHT as i32)).to_le_bytes()); // Height (negative = top-down)
    bmp.extend_from_slice(&1u16.to_le_bytes());            // Color planes
    bmp.extend_from_slice(&24u16.to_le_bytes());           // Bits per pixel
    bmp.extend_from_slice(&0u32.to_le_bytes());            // Compression (none)
    bmp.extend_from_slice(&(pixel_data_size as u32).to_le_bytes()); // Image size
    bmp.extend_from_slice(&2835u32.to_le_bytes());         // X pixels per meter
    bmp.extend_from_slice(&2835u32.to_le_bytes());         // Y pixels per meter
    bmp.extend_from_slice(&0u32.to_le_bytes());            // Colors in palette
    bmp.extend_from_slice(&0u32.to_le_bytes());            // Important colors

    // Pixel data (top-down due to negative height)
    unsafe {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let idx = y * WIDTH + x;
                let rgb565 = *fb_ptr.add(idx);

                // Convert RGB565 to RGB888 (BMP stores as BGR)
                let r = ((rgb565 >> 11) & 0x1F) as u8;
                let g = ((rgb565 >> 5) & 0x3F) as u8;
                let b = (rgb565 & 0x1F) as u8;

                // Expand to 8-bit
                let r8 = (r << 3) | (r >> 2);
                let g8 = (g << 2) | (g >> 4);
                let b8 = (b << 3) | (b >> 2);

                // BMP uses BGR order
                bmp.push(b8);
                bmp.push(g8);
                bmp.push(r8);
            }
        }
    }

    bmp
}
