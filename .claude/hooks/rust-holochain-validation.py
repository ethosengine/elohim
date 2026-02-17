#!/usr/bin/env python3
"""
Rust/Holochain Validation Hook

Validates and auto-fixes Rust cargo commands for Holochain development.
Ensures proper RUSTFLAGS for WASM compilation and warns about common issues.

Hook Type: PreToolUse
Matcher: Bash
"""
import json
import sys
import os
import re

# WASM target for Holochain zomes
WASM_TARGET = "wasm32-unknown-unknown"

# Required RUSTFLAGS for WASM compilation with getrandom
WASM_RUSTFLAGS = '--cfg getrandom_backend="custom"'

# Directories that contain WASM zomes (need special RUSTFLAGS)
WASM_ZOME_DIRS = [
    "holochain/dna/",
    "holochain/elohim-wasm/",
]

# Directories that should NOT have WASM RUSTFLAGS (native Rust)
NATIVE_DIRS = [
    "doorway/",
    "holochain/elohim-storage/",
    "holochain/rna/rust/",
    "holochain/crates/",
]

# Commands that trigger WASM builds
CARGO_BUILD_COMMANDS = [
    r'\bcargo\s+build\b',
    r'\bcargo\s+check\b',
    r'\bcargo\s+test\b',
    r'\bcargo\s+clippy\b',
]

# Commands that should NOT have WASM RUSTFLAGS
NATIVE_ONLY_COMMANDS = [
    r'\bcargo\s+install\b',
    r'\bcargo\s+run\b(?!.*--target)',
]


def get_working_directory(command: str) -> str:
    """Extract working directory from cd command or return empty."""
    # Check for "cd <dir> &&" pattern
    cd_match = re.search(r'\bcd\s+([^\s&;]+)', command)
    if cd_match:
        return cd_match.group(1)
    return ""


def is_wasm_context(command: str, cwd: str = "") -> bool:
    """Check if the command is in a WASM zome context."""
    # Check explicit target
    if WASM_TARGET in command:
        return True

    # Check if command changes to a WASM directory
    work_dir = get_working_directory(command)
    if work_dir:
        for zome_dir in WASM_ZOME_DIRS:
            if zome_dir in work_dir:
                return True

    # Check if current working directory context suggests WASM
    for zome_dir in WASM_ZOME_DIRS:
        if zome_dir in command:
            return True

    return False


def is_native_context(command: str) -> bool:
    """Check if the command is for native Rust (not WASM)."""
    # Check for native-only commands
    for pattern in NATIVE_ONLY_COMMANDS:
        if re.search(pattern, command):
            return True

    # Check if in native directories
    work_dir = get_working_directory(command)
    for native_dir in NATIVE_DIRS:
        if native_dir in command or native_dir in work_dir:
            return True

    return False


def has_rustflags(command: str) -> bool:
    """Check if command already sets RUSTFLAGS."""
    return 'RUSTFLAGS' in command


def needs_rustflags_cleared(command: str) -> bool:
    """Check if command needs RUSTFLAGS cleared (native tools)."""
    # wasm-pack, cargo install, etc. should not have WASM RUSTFLAGS
    if re.search(r'\bwasm-pack\b', command):
        return True
    if re.search(r'\bcargo\s+install\b', command):
        return True
    return False


def is_cargo_command(command: str) -> bool:
    """Check if this is a cargo build/check/test command."""
    for pattern in CARGO_BUILD_COMMANDS:
        if re.search(pattern, command):
            return True
    return False


def main():
    try:
        # Read hook input from stdin
        data = json.load(sys.stdin)

        tool_name = data.get('tool_name', '')
        if tool_name != 'Bash':
            sys.exit(0)

        tool_input = data.get('tool_input', {})
        command = tool_input.get('command', '')

        if not command:
            sys.exit(0)

        # Skip if not a cargo command
        if 'cargo' not in command and 'wasm-pack' not in command:
            sys.exit(0)

        messages = []
        updated_command = None

        # Case 1: Native tools that should NOT have WASM RUSTFLAGS
        if needs_rustflags_cleared(command):
            if has_rustflags(command) and 'getrandom' in command:
                messages.append("Detected WASM RUSTFLAGS on native tool command.")
                messages.append("Consider using RUSTFLAGS='' or removing RUSTFLAGS for cargo install/wasm-pack.")

        # Case 2: WASM zome builds that need RUSTFLAGS
        elif is_cargo_command(command) and is_wasm_context(command):
            if not has_rustflags(command):
                messages.append(f"WASM build detected but RUSTFLAGS not set.")
                messages.append(f"Recommend: RUSTFLAGS='{WASM_RUSTFLAGS}' {command}")
                messages.append(f"Or ensure --target {WASM_TARGET} is specified.")

        # Case 3: Cargo commands in native directories with WASM flags
        elif is_cargo_command(command) and is_native_context(command):
            if has_rustflags(command) and 'getrandom' in command:
                messages.append("Native Rust build with WASM RUSTFLAGS detected.")
                messages.append("The getrandom custom backend is only for WASM targets.")
                messages.append("Consider using RUSTFLAGS='' for native builds.")

        # Case 4: Generic cargo build without clear context
        elif is_cargo_command(command) and not has_rustflags(command):
            # Check if it looks like it might be a zome
            if any(zome_dir in command for zome_dir in WASM_ZOME_DIRS):
                messages.append(f"Possible WASM zome build. If targeting {WASM_TARGET}, add:")
                messages.append(f"RUSTFLAGS='{WASM_RUSTFLAGS}'")

        if messages:
            output = {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": "RUST/HOLOCHAIN BUILD HINT:\n" + "\n".join(messages)
                }
            }
            print(json.dumps(output))

        sys.exit(0)

    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f"rust-holochain-validation hook error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
