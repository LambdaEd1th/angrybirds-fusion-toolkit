#!/bin/bash

# ==============================================================================
# Angry Birds Fusion Toolkit - Interactive Wrapper Script
# File: scripts/batch.sh
#
# This script is designed to be placed in the "scripts/" directory.
# It automatically searches for the binary in common locations.
# ==============================================================================

# Define the binary name
BIN_NAME="angrybirds-fusion-toolkit"

# Attempt to find the executable in the following order:
# 1. Current directory (if moved here or inside release folder)
# 2. Cargo release target directory (development path)
if [ -f "./$BIN_NAME" ]; then
    TOOL="./$BIN_NAME"
elif [ -f "../target/release/$BIN_NAME" ]; then
    TOOL="../target/release/$BIN_NAME"
    echo "Running in development mode (using target/release binary)"
else
    echo "Error: Could not find '$BIN_NAME'."
    echo "Please ensure the project is built (cargo build --release) or the binary is in the current directory."
    exit 1
fi

echo "=========================================="
echo "   Angry Birds Fusion Toolkit"
echo "=========================================="
echo "Please select an operation mode:"
echo "1) Decrypt (Game File -> Readable)"
echo "2) Encrypt (Readable -> Game File)"
echo "=========================================="
read -p "Enter number (1-2): " MODE_CHOICE

USE_AUTO=false

case $MODE_CHOICE in
    1) CMD="decrypt";;
    2) CMD="encrypt";;
    *)
        echo "Invalid choice. Exiting."
        exit 1
        ;;
esac

echo ""
read -e -p "Enter Input File Path (drag & drop allowed): " INPUT_FILE
# Remove quotes if present (common when dragging files in terminal)
INPUT_FILE=$(echo "$INPUT_FILE" | tr -d "'\"")

if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: File '$INPUT_FILE' does not exist."
    exit 1
fi

echo ""
read -e -p "Enter Output File Path (leave empty for auto-naming): " OUTPUT_FILE
OUTPUT_FILE=$(echo "$OUTPUT_FILE" | tr -d "'\"")

# Game & Category Selection Logic
if [ "$CMD" == "decrypt" ]; then
    echo ""
    echo "Decryption Mode:"
    echo "1) Auto-detect Game & Category (Recommended)"
    echo "2) Manual Selection"
    read -p "Enter number (1-2): " DETECT_CHOICE
    if [ "$DETECT_CHOICE" == "1" ]; then
        USE_AUTO=true
    fi
fi

if [ "$USE_AUTO" == "false" ]; then
    echo ""
    echo "Select Game:"
    echo "1) Classic"
    echo "2) Rio"
    echo "3) Seasons"
    echo "4) Space"
    echo "5) Friends"
    echo "6) Star Wars"
    echo "7) Star Wars II"
    echo "8) Stella"
    read -p "Enter number (1-8): " GAME_CHOICE

    case $GAME_CHOICE in
        1) GAME="classic";;
        2) GAME="rio";;
        3) GAME="seasons";;
        4) GAME="space";;
        5) GAME="friends";;
        6) GAME="starwars";;
        7) GAME="starwarsii";;
        8) GAME="stella";;
        *) echo "Invalid game. Exiting."; exit 1;;
    esac

    echo ""
    echo "Select File Category:"
    echo "1) Native (Game Data/Levels)"
    echo "2) Save (Progress/Highscores)"
    echo "3) Downloaded (DLC/Assets)"
    read -p "Enter number (1-3): " CAT_CHOICE

    case $CAT_CHOICE in
        1) CATEGORY="native";;
        2) CATEGORY="save";;
        3) CATEGORY="downloaded";;
        *) echo "Invalid category. Exiting."; exit 1;;
    esac
fi

# Construct arguments array
CMD_ARGS=("$CMD" "-i" "$INPUT_FILE")

if [ -n "$OUTPUT_FILE" ]; then
    CMD_ARGS+=("-o" "$OUTPUT_FILE")
fi

if [ "$USE_AUTO" == "true" ]; then
    CMD_ARGS+=("--auto")
else
    CMD_ARGS+=("-g" "$GAME" "-c" "$CATEGORY")
fi

echo ""
echo "Executing: $TOOL ${CMD_ARGS[*]}"
echo "------------------------------------------"

# Execute the tool
$TOOL "${CMD_ARGS[@]}"

# Check exit status
if [ $? -eq 0 ]; then
    echo "------------------------------------------"
    echo "Operation completed successfully!"
else
    echo "------------------------------------------"
    echo "Operation failed. Please check the file or logs."
fi