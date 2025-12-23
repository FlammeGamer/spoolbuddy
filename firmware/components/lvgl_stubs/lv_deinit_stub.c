/**
 * Stub for lv_deinit() which is only available when LV_MEM_CUSTOM=0
 * The Rust lvgl binding expects this function to exist, but we use
 * custom memory allocation (LV_MEM_CUSTOM=1) to leverage PSRAM.
 *
 * This stub does nothing - we never actually deinitialize LVGL.
 */

void lv_deinit(void) {
    // No-op stub - LVGL is never deinitialized in this firmware
}
