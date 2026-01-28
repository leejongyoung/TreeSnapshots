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

REPO_ROOT=$(pwd)
SNAPSHOT_DIR="${REPO_ROOT}/snapshots"
DATE=$(date +%Y%m%d)
START_TIME=$SECONDS
OS_TYPE=$(uname)
REAL_USER=${SUDO_USER:-$USER}

# 2. Dependency Check
check_and_install_tree() {
    if command -v tree &> /dev/null; then
        # tree is already installed
        return 0
    fi

    echo "⚠️ 'tree' command not found. Attempting to install..."

    # Check for internet connectivity
    if ! ping -c 1 8.8.8.8 &> /dev/null; then
        echo "❌ ERROR: Internet connection not available."
        echo "Please install 'tree' manually and run the script again."
        exit 1
    fi

    echo "🔧 Installing 'tree'..."
    if [[ "$OS_TYPE" == "Darwin" ]]; then
        if command -v brew &> /dev/null; then
            # Brew install does not need sudo
            brew install tree
        else
            echo "❌ ERROR: Homebrew (brew) not found. Please install Homebrew or 'tree' manually."
            exit 1
        fi
    elif [[ "$OS_TYPE" == "Linux" ]]; then
        # Package managers need sudo
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
chown "$REAL_USER" "$SNAPSHOT_DIR" # Change ownership to the real user
check_and_install_tree

# 4. Function to get available drives
get_drives() {
    local drive_list=()
    # Always add the root directory as the first "Local Device" option
    drive_list+=("/")

    local drives_output
    if [[ "$OS_TYPE" == "Darwin" ]]; then
        # On macOS, find the volume name for the root directory to exclude it
        local root_vol_name=""
        for vol in "/Volumes/"*; do
            if [[ -L "$vol" && "$(readlink "$vol")" == "/" ]]; then
                root_vol_name=$(basename "$vol")
                break
            fi
        done
        
        # List all volumes and filter out the root volume name
        local all_volumes
        all_volumes=$(ls -1 /Volumes)
        
        local temp_drives=()
        while IFS= read -r line; do
            [[ -n "$line" && "$line" != "$root_vol_name" ]] && temp_drives+=("$line")
        done <<< "$all_volumes"
        
        # Join the arrays (bash 3 compatible way)
        drive_list=("${drive_list[@]}" "${temp_drives[@]}")

    elif [[ "$OS_TYPE" == "Linux" ]]; then
        # On Linux, use lsblk to find mount points, excluding some common/system ones.
        drives_output=$(lsblk -o MOUNTPOINT -n | grep -vE "^(/boot|/snap|/var|/swap|/)$")
        while IFS= read -r line; do
            [[ -n "$line" ]] && drive_list+=("$line")
        done <<< "$drives_output"
    else
        echo "❌ Unsupported Operating System: $OS_TYPE"
        exit 1
    fi
    
    # Return the array
    echo "${drive_list[@]}"
}

# 5. Drive Selection Menu
echo "------------------------------------------------"
echo "📂 Select a drive or mount point to scan:"
options=($(get_drives))

if [ ${#options[@]} -eq 0 ]; then
    echo "⚠️ No drives found. Please check your system."
    exit 1
fi

# Custom menu imitating `select` but with more descriptive text
while true; do
    i=1
    for opt in "${options[@]}"; do
        if [[ "$opt" == "/" ]]; then
            echo "   $i) Local Device (/)"
        else
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
        DRIVE_IDENTIFIER=${options[$REPLY-1]}
        
        if [[ "$DRIVE_IDENTIFIER" == "/" ]]; then
            DRIVE_NAME="Local_Device"
            TARGET_PATH="/"
        elif [[ "$OS_TYPE" == "Darwin" ]]; then
            DRIVE_NAME="${DRIVE_IDENTIFIER}"
            TARGET_PATH="/Volumes/${DRIVE_IDENTIFIER}"
        else # Linux
            DRIVE_NAME=$(basename "$DRIVE_IDENTIFIER")
            [[ -z "$DRIVE_NAME" ]] && DRIVE_NAME="root" # Fallback
            TARGET_PATH="${DRIVE_IDENTIFIER}"
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
# Using stdbuf to ensure line-by-line output for monitoring.
# The --du option is excluded for performance, but can be added if needed.
stdbuf -oL tree -apu -h "$TARGET_PATH" > "$OUTPUT_FILE" &
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