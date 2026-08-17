/* Minimal C consumer for the NextTabletDriver SDK.
 *
 * Runs standalone -- no NextTabletDriver desktop app needs to be installed
 * or running. Build instructions: see README.md in this directory.
 */

#include <stdio.h>

#ifdef _WIN32
#include <windows.h>
#define ntd_sleep_ms(ms) Sleep(ms)
#else
#include <unistd.h>
#define ntd_sleep_ms(ms) usleep((ms) * 1000)
#endif

#include "ntd_sdk.h"

int main(void) {
    printf("ntd_sdk ABI version: %u\n", ntd_sdk_abi_version());

    int32_t rc = ntd_init();
    if (rc != NTD_OK) {
        fprintf(stderr, "ntd_init failed: %d\n", rc);
        return 1;
    }

    /* Absolute mode, a small active area in millimeters. Values are clamped
     * to the tablet's real physical surface by the engine. */
    ntd_set_mode(0);
    ntd_set_active_area(80.0f, 70.0f, 76.51f, 51.5f, 0.0f);

    for (int i = 0; i < 200; ++i) {
        struct NtdState state;
        rc = ntd_poll_state(&state);
        if (rc != NTD_OK) {
            fprintf(stderr, "ntd_poll_state failed: %d\n", rc);
            break;
        }

        /* is_connected means "a pen is currently detected", not "a tablet is
         * plugged in" -- a tablet can be open and streaming (status 1 = out
         * of range) with no pen anywhere near its surface. Status 0 means no
         * supported tablet has been found at all. */
        if (state.is_connected) {
            printf("u=%.3f v=%.3f pressure=%d status=%u\n",
                   (double)state.u, (double)state.v, state.pressure,
                   (unsigned)state.status);
        } else if (state.status == 0) {
            printf("(no tablet detected)\n");
        } else {
            printf("(tablet found, pen out of range, status=%u)\n",
                   (unsigned)state.status);
        }

        ntd_sleep_ms(5);
    }

    ntd_shutdown();
    return 0;
}
