#!/bin/bash
# PowerCost Tracker - Linux Permissions Setup
# This script sets up proper permissions to read Intel RAPL power data

set -e

echo "PowerCost Tracker - Linux Permissions Setup"
echo "============================================"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "This script must be run as root (sudo)."
    echo "Usage: sudo $0"
    exit 1
fi

echo "This script will configure permissions to allow PowerCost Tracker"
echo "to read Intel RAPL power data without requiring root privileges."
echo ""

# Method 1: udev rule
echo "Creating udev rule for powercap access..."

UDEV_RULE='SUBSYSTEM=="powercap", ACTION=="add", RUN+="/bin/chmod -R a+r /sys/class/powercap/"'
UDEV_FILE="/etc/udev/rules.d/99-powercap-read.rules"

echo "$UDEV_RULE" > "$UDEV_FILE"
chmod 644 "$UDEV_FILE"

echo "Created: $UDEV_FILE"

# Reload udev rules
echo "Reloading udev rules..."
udevadm control --reload-rules
udevadm trigger

# Apply permissions immediately
echo "Applying permissions to current powercap entries..."
if [ -d /sys/class/powercap ]; then
    chmod -R a+r /sys/class/powercap/
    echo "Permissions applied successfully!"
else
    echo "Note: /sys/class/powercap not found (RAPL may not be supported)"
fi

echo ""
echo "Setup complete!"
echo ""
echo "You can now run PowerCost Tracker as a normal user."
echo "The permissions will persist across reboots."
echo ""
echo "To verify, run: cat /sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj"
