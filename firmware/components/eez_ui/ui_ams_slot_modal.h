/**
 * AMS Slot Configuration Modal
 * Matches frontend ConfigureAmsSlotModal functionality
 */

#ifndef UI_AMS_SLOT_MODAL_H
#define UI_AMS_SLOT_MODAL_H

#include <stdbool.h>

/**
 * Open the AMS slot configuration modal
 *
 * @param printer_serial Printer serial number
 * @param ams_id AMS unit ID (0-3 for regular, 128-135 for HT, 254/255 for external)
 * @param tray_id Tray ID within AMS (0-3 for regular, 0 for HT/external)
 * @param tray_count Number of trays in this AMS (1 for HT, 4 for regular)
 * @param extruder_id Extruder this AMS is connected to (-1=unknown/single, 0=right, 1=left)
 * @param tray_type Current material type in slot (optional, can be NULL)
 * @param tray_color Current color hex string (optional, can be NULL)
 * @param on_success Callback when configuration succeeds (optional, can be NULL)
 */
void ui_ams_slot_modal_open(const char *printer_serial, int ams_id, int tray_id,
                            int tray_count, int extruder_id,
                            const char *tray_type, const char *tray_color,
                            void (*on_success)(void));

/**
 * Close the AMS slot configuration modal
 */
void ui_ams_slot_modal_close(void);

/**
 * Check if the modal is currently open
 */
bool ui_ams_slot_modal_is_open(void);

#endif // UI_AMS_SLOT_MODAL_H
