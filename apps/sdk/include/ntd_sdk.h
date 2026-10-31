/*
 * NextTabletDriver SDK -- generated C header. Do not edit by hand;
 * regenerated from apps/sdk/src/ffi.rs by cbindgen (see apps/sdk/build.rs).
 */

#ifndef NTD_SDK_H
#define NTD_SDK_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Bumped whenever this FFI surface changes in a backward-incompatible way.
 *
 * Independent from [`next_tablet_driver::engine::interop::shm::SDK_ABI_VERSION`],
 * which versions the separate inter-process shared-memory layout.
 */
#define NTD_SDK_ABI_VERSION 1

/**
 * Success: the call did what it was asked.
 */
#define NTD_OK 0

/**
 * `ntd_init` only: the HID API itself failed to initialise (no supported
 * backend, missing permissions, ...).
 */
#define NTD_ERR_HID_INIT_FAILED -1

/**
 * `ntd_init` was never called, or `ntd_shutdown` already ran.
 */
#define NTD_ERR_NOT_INITIALIZED -2

/**
 * A required output pointer was null.
 */
#define NTD_ERR_NULL_POINTER -3

/**
 * An argument was outside its valid range (e.g. an unknown mode byte).
 */
#define NTD_ERR_INVALID_ARGUMENT -4

/**
 * The current HID owner couldn't be reached to apply a forwarded command.
 */
#define NTD_ERR_COMMAND_FAILED -5

/**
 * The call panicked; caught at the FFI boundary and never propagated to
 * the host.
 */
#define NTD_ERR_PANIC -6

/**
 * Fixed capacity of [`NtdState::device_name`], in bytes.
 *
 * Kept as a local literal (rather than referencing
 * [`next_tablet_driver::engine::interop::shm::DEVICE_NAME_CAPACITY`]
 * directly) so it can be emitted as a `#define` in the generated C header.
 * [`NtdState::device_name`] itself uses the literal `64` rather than this
 * constant, since `csbindgen`, unlike `cbindgen`, can't resolve a named
 * constant into a fixed-size C# buffer length. The `const _` assertions
 * below keep all three (this constant, the real capacity, and the struct's
 * array literal) from silently drifting apart.
 */
#define NTD_DEVICE_NAME_CAPACITY 64

/**
 * Live tablet + config snapshot returned by [`ntd_poll_state`].
 *
 * Field-for-field mirror of [`next_tablet_driver::engine::interop::shm::SdkPublicState`].
 * The two layouts are asserted equal in size below, since both cross an
 * FFI/ABI boundary (this one to the host process, that one to other
 * processes).
 */
typedef struct NtdState {
  /**
   * A pen is detected. This is not the same as "a tablet is plugged in".
   */
  bool is_connected;
  /**
   * Pen status code: 0 no supported tablet, 1 out of range, 2 hover, 3 contact, 4 active, 5
   * eraser, 6 pen, 7 touch, 8 aux, 9 rotation, 10 tool, 11 mouse.
   */
  uint8_t status;
  /**
   * Horizontal position inside the active area, from 0 to 1.
   */
  float u;
  /**
   * Vertical position inside the active area, from 0 to 1.
   */
  float v;
  /**
   * Horizontal position on screen in pixels (absolute mode), or movement since the last packet
   * (relative mode).
   */
  float screen_x;
  /**
   * Vertical position on screen in pixels (absolute mode), or movement since the last packet
   * (relative mode).
   */
  float screen_y;
  /**
   * Pressure, from 0 to 8191.
   */
  int32_t pressure;
  /**
   * Horizontal tilt, in degrees.
   */
  int32_t tilt_x;
  /**
   * Vertical tilt, in degrees.
   */
  int32_t tilt_y;
  /**
   * Bits of the pen buttons that are pressed.
   */
  uint8_t buttons;
  /**
   * The tip counts as touching.
   */
  bool is_down;
  /**
   * The eraser end of the pen is in use.
   */
  bool eraser;
  /**
   * Name of the tablet as UTF-8 bytes; `device_name_len` is the number of valid bytes.
   */
  uint8_t device_name[64];
  /**
   * Number of valid bytes in `device_name`.
   */
  uint32_t device_name_len;
  /**
   * USB vendor ID of the tablet.
   */
  uint16_t vid;
  /**
   * USB product ID of the tablet.
   */
  uint16_t pid;
  /**
   * Current mode: 0 absolute, 1 relative.
   */
  uint8_t mode;
  /**
   * Left edge of the active area, in millimetres.
   */
  float active_area_x;
  /**
   * Top edge of the active area, in millimetres.
   */
  float active_area_y;
  /**
   * Width of the active area, in millimetres.
   */
  float active_area_w;
  /**
   * Height of the active area, in millimetres.
   */
  float active_area_h;
  /**
   * Rotation of the active area, in degrees.
   */
  float active_area_rotation;
  /**
   * Increases every time the settings change.
   */
  uint32_t config_version;
} NtdState;

/**
 * Returns this build's FFI ABI version, so a host can detect a mismatch
 * against the header/bindings it was compiled with.
 */
uint32_t ntd_sdk_abi_version(void);

/**
 * Starts the embedded engine: becomes the HID owner if no other process
 * currently is one, otherwise starts in reader mode and mirrors the real
 * owner's state. Idempotent: calling this again while already initialised
 * is a no-op that returns [`NTD_OK`].
 */
int32_t ntd_init(void);

/**
 * Stops the embedded engine and joins its background thread. Safe to call
 * even if [`ntd_init`] was never called, or was already shut down.
 */
void ntd_shutdown(void);

/**
 * Copies the current tablet + config snapshot into `*out_state`.
 *
 * # Safety
 *
 * `out_state` must be a non-null, valid, properly aligned pointer to a
 * writable `NtdState`, valid for the duration of this call.
 */
int32_t ntd_poll_state(struct NtdState *out_state);

/**
 * Sets the driver mode (`0` = absolute, `1` = relative). Writes directly if
 * this process is the current HID owner, otherwise forwards the change to
 * whichever process is.
 */
int32_t ntd_set_mode(uint8_t mode);

/**
 * Sets the active mapping area (millimeters, clamped to the current
 * device's physical surface). Writes directly if this process is the
 * current HID owner, otherwise forwards the change to whichever process is.
 */
int32_t ntd_set_active_area(float x, float y, float w, float h, float rotation);

#endif  /* NTD_SDK_H */
