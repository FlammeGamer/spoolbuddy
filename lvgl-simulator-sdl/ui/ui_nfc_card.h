/**
 * NFC Card UI - Main screen NFC/Scale card management
 */

#ifndef UI_NFC_CARD_H
#define UI_NFC_CARD_H

#include <stdbool.h>

/**
 * Initialize the NFC card state (call when main screen loads)
 */
void ui_nfc_card_init(void);

/**
 * Clean up NFC card dynamic elements (call when leaving main screen)
 */
void ui_nfc_card_cleanup(void);

/**
 * Update NFC card UI based on tag/scale state
 * Call this periodically when main screen is active
 */
void ui_nfc_card_update(void);

/**
 * Mark a tag as "just configured" to suppress popup when returning to main screen.
 * The suppression is cleared when the tag is removed or a different tag is detected.
 */
void ui_nfc_card_set_configured_tag(const char *tag_id);

/**
 * Show tag details modal (read-only view with just Close button).
 * Called from encode button on main screen and scan results.
 */
void ui_nfc_card_show_details(void);

#endif // UI_NFC_CARD_H
