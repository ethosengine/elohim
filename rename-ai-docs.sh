#!/bin/bash

# Script to rename AI documentation files (claude.md, agent.md, gemini.md)
# Usage: ./rename-ai-docs.sh [claude|agent|gemini]

if [ $# -eq 0 ]; then
    echo "Usage: $0 [claude|agent|gemini]"
    echo "This will rename all claude.md, agent.md, and gemini.md files to the specified name"
    exit 1
fi

TARGET="$1"

if [[ ! "$TARGET" =~ ^(claude|agent|gemini)$ ]]; then
    echo "Error: Argument must be 'claude', 'agent', or 'gemini'"
    exit 1
fi

TARGET_FILE="${TARGET}.md"

echo "Renaming all claude.md, agent.md, and gemini.md files to ${TARGET_FILE}..."
echo ""

# Counter for renamed files
count=0

# Find and rename all matching files
for file in $(find . -type f \( -name "claude.md" -o -name "agent.md" -o -name "gemini.md" \)); do
    # Get the directory of the file
    dir=$(dirname "$file")
    current_name=$(basename "$file")

    # Skip if already the target name
    if [ "$current_name" == "$TARGET_FILE" ]; then
        continue
    fi

    # Check if target file already exists in the same directory
    if [ -f "${dir}/${TARGET_FILE}" ]; then
        echo "⚠️  Skipping ${file} - ${dir}/${TARGET_FILE} already exists"
        continue
    fi

    # Rename the file
    mv "$file" "${dir}/${TARGET_FILE}"
    echo "✓ Renamed: ${file} → ${dir}/${TARGET_FILE}"
    ((count++))
done

echo ""
echo "Done! Renamed ${count} file(s) to ${TARGET_FILE}"
