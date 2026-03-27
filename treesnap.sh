#!/bin/bash
# -----------------------------------------------------------------------------
# Author: Lee Jongyoung
# Date: January 28, 2026
# Description: This script generates a snapshot of a file system's tree
#              structure, handling OS-specific differences and dependencies.
# License: MIT License (See README.md for details)
# -----------------------------------------------------------------------------

# 1. Environment and Execution Check
if [ "$RUN_VIA_MAKE" != "true" ]; then
    echo "❌ This script should be run using 'make snapshot'." >&2
    echo "   Please use the Makefile for the best experience." >&2
    exit 1
fi

DATE=$(date +%Y%m%d)
START_TIME=$SECONDS
OS_TYPE=$(uname)
# Refine OS_TYPE for WSL
if [[ "$OS_TYPE" == "Linux" ]] && grep -qE "(Microsoft|WSL)" /proc/version &> /dev/null; then
    OS_TYPE="WSL"
fi
REAL_USER=${SUDO_USER:-$USER}
REAL_HOME=$(eval echo "~${REAL_USER}")
SNAPSHOT_DIR="${REAL_HOME}/TreeSnapshots/snapshots"

# 2. Dependency Check
check_and_install_tree() {
    if command -v tree &> /dev/null; then
        # tree is already installed
        return 0
    fi

    echo "⚠️ 'tree' command not found. Attempting to install..."

    # Check for internet connectivity.
    # curl and nc are preferred over ping because ICMP is often blocked in
    # WSL, containers, and corporate firewalls. Ping is kept as a last resort.
    if ! { curl -s --connect-timeout 3 -o /dev/null http://connectivitycheck.gstatic.com/generate_204 2>/dev/null || \
           nc -zw3 8.8.8.8 53 2>/dev/null || \
           ping -c 1 8.8.8.8 &>/dev/null; }; then
        echo "❌ ERROR: Internet connection not available."
        echo "Please install 'tree' manually and run the script again."
        exit 1
    fi

    echo "🔧 Installing 'tree'..."
    # On WSL, brew is not available, and it will fall through to the Linux part, which is correct.
    if [[ "$OS_TYPE" == "Darwin" ]]; then
        if command -v brew &> /dev/null; then
            # Brew install does not need sudo
            brew install tree
        else
            echo "❌ ERROR: Homebrew (brew) not found. Please install Homebrew or 'tree' manually."
            exit 1
        fi
    elif [[ "$OS_TYPE" == "Linux" || "$OS_TYPE" == "WSL" ]]; then
        # Package managers need to be run as root (which the script is)
        if command -v apt-get &> /dev/null; then
            apt-get update && apt-get install -y tree
        elif command -v dnf &> /dev/null; then
            dnf install -y tree
        elif command -v yum &> /dev/null; then
            yum install -y tree
        else
            echo "❌ ERROR: Could not find a supported package manager (apt, dnf, yum)."
            echo "Please install 'tree' manually."
            exit 1
        fi
    fi

    # Verify installation
    if ! command -v tree &> /dev/null; then
        echo "❌ ERROR: 'tree' installation failed. Please install it manually."
        exit 1
    fi
    echo "✅ 'tree' has been successfully installed."
}

# 3. Initial Setup
mkdir -p "$SNAPSHOT_DIR" # Create directory (as root)
chown "$REAL_USER" "${REAL_HOME}/TreeSnapshots" "$SNAPSHOT_DIR" # Restore ownership to the real user
check_and_install_tree

# 4. Function to populate available drives into the global options array
# Using a global array avoids word-splitting on paths that contain spaces
# (e.g. macOS volumes like "/Volumes/My Passport").
populate_drives() {
    options=()
    # Always add the root of the current filesystem as the first option
    options+=("/")

    if [[ "$OS_TYPE" == "Darwin" ]]; then
        # On macOS, find and exclude the root volume from the /Volumes list
        local root_vol_name=""
        for vol in "/Volumes/"*; do
            if [[ -L "$vol" && "$(readlink "$vol")" == "/" ]]; then
                root_vol_name=$(basename "$vol")
                break
            fi
        done
        local all_volumes
        all_volumes=$(ls -1 /Volumes)
        while IFS= read -r line; do
            [[ -n "$line" && "$line" != "$root_vol_name" ]] && options+=("/Volumes/$line")
        done <<< "$all_volumes"

    elif [[ "$OS_TYPE" == "WSL" ]]; then
        # On WSL, add the mounted Windows drives (e.g., /mnt/c, /mnt/d)
        local wsl_drives
        wsl_drives=$(ls -1 /mnt/ | grep -E '^[a-z]$')
        while IFS= read -r letter; do
            [[ -n "$letter" ]] && options+=("/mnt/$letter")
        done <<< "$wsl_drives"

    elif [[ "$OS_TYPE" == "Linux" ]]; then
        # On native Linux, use lsblk to find other mount points
        local linux_drives
        linux_drives=$(lsblk -o MOUNTPOINT -n | tr -d ' ' | grep -vE "^(/boot|/snap|/var|/swap|/)$|^$")
        while IFS= read -r mount_point; do
            [[ -n "$mount_point" ]] && options+=("$mount_point")
        done <<< "$linux_drives"
    fi
}

# 5. Drive Selection Menu
echo "------------------------------------------------"
echo "📂 Select a drive or mount point to scan:"
populate_drives

if [ ${#options[@]} -eq 0 ]; then
    echo "⚠️ No drives found. Please check your system."
    exit 1
fi

# Custom menu with more descriptive text
while true; do
    i=1
    for opt in "${options[@]}"; do
        if [[ "$opt" == "/" ]]; then
            echo "   $i) Root Filesystem (/)"
        elif [[ "$opt" =~ ^/mnt/[a-z]$ ]]; then # WSL drive
            DRIVE_LETTER=$(basename "$opt" | tr 'a-z' 'A-Z')
            echo "   $i) Windows Drive ($DRIVE_LETTER:)"
        else # macOS external or other Linux mount
            DRIVE_LABEL=$(basename "$opt")
            echo "   $i) External Drive ($DRIVE_LABEL)"
        fi
        i=$((i + 1))
    done

    read -p "👉 Choose a number (or 'q' to quit): " REPLY
    
    if [[ "$REPLY" == "q" ]]; then
        echo "👋 Exiting."
        exit 0
    fi

    if [[ "$REPLY" =~ ^[0-9]+$ ]] && [ "$REPLY" -ge 1 ] && [ "$REPLY" -le ${#options[@]} ]; then
        TARGET_PATH=${options[$REPLY-1]}
        
        # Determine a clean name for the output file
        if [[ "$TARGET_PATH" == "/" ]]; then
            if [[ "$OS_TYPE" == "Darwin" ]]; then
                DRIVE_NAME="macOS_Root"
            else
                DRIVE_NAME="Linux_Root"
            fi
        elif [[ "$TARGET_PATH" =~ ^/mnt/[a-z]$ ]]; then
            DRIVE_NAME="Windows_$(basename "$TARGET_PATH" | tr 'a-z' 'A-Z')"
        else
            DRIVE_NAME=$(basename "$TARGET_PATH")
        fi

        OUTPUT_FILE="${SNAPSHOT_DIR}/snapshot_${DATE}_${DRIVE_NAME// /_}.txt"
        break
    else
        echo "⚠️ Invalid selection. Please try again."
    fi
done

echo -e "\n🔍 Starting scan on: $TARGET_PATH"
echo "------------------------------------------------"

# 6. Run tree command in the background
# stdbuf ensures line-by-line output for monitoring (Linux/WSL only; not available on macOS).
# The --du option is excluded for performance, but can be added if needed.
if command -v stdbuf &> /dev/null; then
    stdbuf -oL tree -apu -h "$TARGET_PATH" > "$OUTPUT_FILE" &
else
    tree -apu -h "$TARGET_PATH" > "$OUTPUT_FILE" &
fi
TREE_PID=$!

# 7. Monitoring loop (checks line count and file size)
while kill -0 $TREE_PID 2>/dev/null; do
    if [ -f "$OUTPUT_FILE" ]; then
        LINE_COUNT=$(wc -l < "$OUTPUT_FILE")
        CURRENT_SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)
        ELAPSED=$(( SECONDS - START_TIME ))
        echo -ne "⏳ Scanning... [${ELAPSED}s] | Lines: ${LINE_COUNT} | Size: ${CURRENT_SIZE}\r"
    fi
    sleep 0.5
done

# Wait for the tree command to finish, just in case
wait $TREE_PID

# 8. Restore File Ownership (Crucial Step)
# If run with sudo, chown the file back to the original user.
# If run without sudo, it defaults to the current user.
chown "$REAL_USER" "$OUTPUT_FILE"

# 9. Final Report
END_TIME=$SECONDS
DURATION=$(( END_TIME - START_TIME ))
FINAL_LINES=$(wc -l < "$OUTPUT_FILE")
FINAL_SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)

echo -e "\n------------------------------------------------"
echo "✅ Scan Complete!"
echo "⏱️ Total Time: ${DURATION} seconds"
echo "📝 Total Lines: ${FINAL_LINES}"
echo "💾 Final Size: ${FINAL_SIZE}"
echo "👤 File Owner: $REAL_USER"
echo "📄 Location: $OUTPUT_FILE"
echo "------------------------------------------------"