#!/usr/bin/env bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TABLETS_DIR="$SCRIPT_DIR/../tablets"
OUTPUT_FILE="$SCRIPT_DIR/99-nexttabletdriver.rules"

if ! command -v jq &> /dev/null; then
    echo "Error: 'jq' is required but not installed. Please install it." >&2
    exit 1
fi

# Write base header and core rules
cat << 'EOF' > "$OUTPUT_FILE"
# NextTabletDriver udev rules
# Allows non-root users in the "input" group to access /dev/uinput
# and prevents double input by telling libinput to ignore the original tablets.
#
# Installation:
#   sudo cp 99-nexttabletdriver.rules /etc/udev/rules.d/
#   sudo udevadm control --reload-rules
#   sudo udevadm trigger
#
# Then add your user to the input group:
#   sudo usermod -aG input $USER
#
# You will need to log out and back in for group changes to take effect.

# Grant read/write access to /dev/uinput for the "input" group
KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0660", GROUP="input", TAG+="uaccess"

# Remove virtual tablet joypad devices (prevents tablet acting as a controller in games)
KERNEL=="js[0-9]*", SUBSYSTEM=="input", ATTRS{name}=="NextTabletDriver Virtual Pen", RUN+="/usr/bin/env rm %E{DEVNAME}"
KERNEL=="js[0-9]*", SUBSYSTEM=="input", ATTRS{name}=="NextTabletDriver Virtual Mouse", RUN+="/usr/bin/env rm %E{DEVNAME}"

EOF

declare -A vids
declare -A seen_cache
ignore_rules=()

while IFS= read -r -d '' file; do
    if ! json_content=$(jq -e . "$file" 2>/dev/null); then
        echo "Warning: Failed to parse $file" >&2
        continue
    fi

    name=$(echo "$json_content" | jq -r '.Name // empty')
    libinput=$(echo "$json_content" | jq -r '.Attributes.libinputoverride // 0')
    
    has_identifiers=$(echo "$json_content" | jq -e '.DigitizerIdentifiers // empty' >/dev/null && echo "true" || echo "false")
    if [ "$has_identifiers" != "true" ]; then
        continue
    fi

    while IFS=$'\t' read -r vendor_id product_id; do
        if [ -z "$vendor_id" ] || [ "$vendor_id" = "null" ]; then continue; fi

        # Convert to 4-character lowercase hex format
        vid_hex=$(printf "%04x" "$vendor_id")
        pid_hex=$(printf "%04x" "$product_id")

        vids["$vid_hex"]=1

        cache_key="${vid_hex}-${pid_hex}"
        if [ -z "${seen_cache[$cache_key]+abc}" ] && [ "$libinput" -gt 0 ]; then
            seen_cache["$cache_key"]=1
            ignore_rules+=("# $name")
            ignore_rules+=("SUBSYSTEM==\"input\", ATTRS{idVendor}==\"$vid_hex\", ATTRS{idProduct}==\"$pid_hex\", ENV{LIBINPUT_IGNORE_DEVICE}=\"$libinput\"")
        fi
    done < <(echo "$json_content" | jq -r '.DigitizerIdentifiers[]? | "\(.VendorID)\t\(.ProductID)"')

done < <(find "$TABLETS_DIR" -type f -name "*.json" -print0 2>/dev/null)

# Append HIDraw section
echo "# Grant read access to tablet HID devices via hidraw" >> "$OUTPUT_FILE"
for vid in "${!vids[@]}"; do
    echo "SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"$vid\", MODE=\"0660\", GROUP=\"input\"" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"

# Append Libinput ignore section
echo "# Prevent double input by ignoring the raw devices in libinput/Wayland/X11" >> "$OUTPUT_FILE"
for rule in "${ignore_rules[@]}"; do
    echo "$rule" >> "$OUTPUT_FILE"
done

echo "Generated $OUTPUT_FILE"