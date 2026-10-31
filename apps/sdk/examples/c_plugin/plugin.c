/* Example NextTabletDriver plugin written in C: it mirrors the horizontal axis.
 *
 * A plugin is a dynamic library (.dll on Windows, .so on Linux) that exports seven C functions.
 * You can write one in any language that can export C functions. The full contract is in
 * docs/plugin-development.md (section "Writing a plugin in another language").
 *
 * Build instructions: see README.md in this directory.
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#define NTD_EXPORT __declspec(dllexport)
#else
#define NTD_EXPORT __attribute__((visibility("default")))
#endif

/* ---- The plugin interface (ABI v3) ---------------------------------------------------- */

#define NTD_PLUGIN_MAGIC 0x4E540000u /* "NT" in the upper 16 bits */
#define NTD_PLUGIN_ABI_VERSION 3u

/* The target triple the library is built for. The engine refuses a plugin built for another
 * target. Add your own targets here; the strings are the Rust target triples. */
#if defined(_WIN32) && defined(_M_ARM64)
#define NTD_TARGET "aarch64-pc-windows-msvc"
#elif defined(_WIN32)
#define NTD_TARGET "x86_64-pc-windows-msvc"
#elif defined(__linux__) && defined(__aarch64__)
#define NTD_TARGET "aarch64-unknown-linux-gnu"
#elif defined(__linux__)
#define NTD_TARGET "x86_64-unknown-linux-gnu"
#else
#error "Unsupported target: add its triple to plugin.c"
#endif

/* One pen packet. The plugin may change it in place. The layout must match exactly:
 * 32 bytes, 8-byte alignment. */
typedef struct {
    float u;               /* position inside the active area, 0.0 to 1.0 */
    float v;
    float pressure;        /* 0.0 to 1.0 */
    int32_t tilt_x;        /* degrees */
    int32_t tilt_y;
    uint8_t is_down;       /* 1 while the tip touches the surface, else 0 (never a bool) */
    uint64_t timestamp_ns; /* reserved, always 0 for now */
} NtdPluginPacket;

/* Read-only information about the current mapping. 16 bytes. */
typedef struct {
    float active_area_width_mm;
    float active_area_height_mm;
    float target_area_width_px;
    float target_area_height_px;
} NtdPluginContext;

/* The result of a call, a plain 32-bit integer. A plugin that returns an error from process is
 * disabled by the engine, and the pipeline goes on without it. */
typedef int32_t NtdPluginStatus;
#define NTD_PLUGIN_OK 0
#define NTD_PLUGIN_ERROR 1

/* ---- The plugin ------------------------------------------------------------------------- */

/* The state of one plugin instance. */
typedef struct {
    bool flip;
} FlipX;

/* The manifest is JSON text. It describes the plugin and its settings. */
static const char MANIFEST[] =
    "{"
    "\"id\":\"c_flip_x\","
    "\"name\":\"Flip X (C)\","
    "\"version\":\"1.0.0\","
    "\"target\":\"" NTD_TARGET "\","
    "\"author\":\"You\","
    "\"description\":\"Mirrors the horizontal axis. Written in C.\","
    "\"icon\":null,"
    "\"properties\":[{"
    "\"id\":\"flip\","
    "\"name\":\"Flip horizontally\","
    "\"tooltip\":\"Turn the horizontal axis around.\","
    "\"kind\":{\"type\":\"Bool\",\"config\":{\"default\":true}}"
    "}]"
    "}";

/* 1. The handshake: lets the engine check that this library speaks its interface. */
NTD_EXPORT uint32_t ntd_plugin_handshake(void) {
    return (NTD_PLUGIN_MAGIC & 0xFFFF0000u) | (NTD_PLUGIN_ABI_VERSION & 0x0000FFFFu);
}

/* 2. The manifest. Return a null-terminated UTF-8 string that stays valid. */
NTD_EXPORT const char *ntd_plugin_manifest(void) {
    return MANIFEST;
}

/* 3. Create an instance. Its settings must start at the defaults of the manifest. */
NTD_EXPORT void *ntd_plugin_create(void) {
    FlipX *state = calloc(1, sizeof *state);
    if (state != NULL) {
        state->flip = true;
    }
    return state; /* NULL means "failed" */
}

/* 4. Process one packet. This runs for every packet: keep it short, and never allocate. */
NTD_EXPORT NtdPluginStatus ntd_plugin_process(void *instance, NtdPluginPacket *packet,
                                              const NtdPluginContext *context) {
    (void)context;
    if (instance == NULL || packet == NULL) {
        return NTD_PLUGIN_ERROR;
    }
    const FlipX *state = instance;
    if (state->flip) {
        packet->u = 1.0f - packet->u;
    }
    return NTD_PLUGIN_OK;
}

/* 5. Change a setting. `value_json` is JSON text: "true", "0.5", "3", "\"text\"" or "null".
 * This can be called often, with the same value. Ignore what you do not understand. */
NTD_EXPORT NtdPluginStatus ntd_plugin_set_property(void *instance, const char *key,
                                                   const char *value_json) {
    if (instance == NULL || key == NULL || value_json == NULL) {
        return NTD_PLUGIN_ERROR;
    }
    FlipX *state = instance;
    if (strcmp(key, "flip") == 0) {
        if (strcmp(value_json, "true") == 0) {
            state->flip = true;
        } else if (strcmp(value_json, "false") == 0) {
            state->flip = false;
        }
    }
    return NTD_PLUGIN_OK;
}

/* 6. Forget the running state. This plugin has none. */
NTD_EXPORT NtdPluginStatus ntd_plugin_reset(void *instance) {
    (void)instance;
    return NTD_PLUGIN_OK;
}

/* 7. Free the instance. */
NTD_EXPORT NtdPluginStatus ntd_plugin_destroy(void *instance) {
    free(instance);
    return NTD_PLUGIN_OK;
}
