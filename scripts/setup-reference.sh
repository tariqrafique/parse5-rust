#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REFERENCE_DIR="$ROOT_DIR/reference"

PARSE5_REF="${PARSE5_REF:-master}"
LWC_REF="${LWC_REF:-master}"
CLONE_DEPTH="${REFERENCE_CLONE_DEPTH:-1}"

mkdir -p "$REFERENCE_DIR"

clone_if_missing() {
    local name="$1"
    local url="$2"
    local ref="$3"
    local target="$REFERENCE_DIR/$name"

    if [[ -d "$target/.git" ]]; then
        echo "$name already exists:"
        git -C "$target" remote -v | sed 's/^/  /'
        echo "  branch: $(git -C "$target" rev-parse --abbrev-ref HEAD)"
        echo "  commit: $(git -C "$target" rev-parse HEAD)"
        return
    fi

    if [[ -e "$target" ]]; then
        echo "error: $target exists but is not a git checkout" >&2
        return 1
    fi

    echo "Cloning $name ($ref) into $target"
    git clone --depth "$CLONE_DEPTH" --branch "$ref" "$url" "$target"
}

clone_if_missing "parse5" "https://github.com/inikulin/parse5.git" "$PARSE5_REF"
clone_if_missing "lwc" "https://github.com/salesforce/lwc.git" "$LWC_REF"

if [[ -d "$REFERENCE_DIR/parse5/.git" ]]; then
    echo "Ensuring parse5 fixture submodules are available"
    git -C "$REFERENCE_DIR/parse5" submodule sync --recursive
    git \
        -C "$REFERENCE_DIR/parse5" \
        -c url.https://github.com/.insteadOf=git@github.com: \
        submodule update --init --recursive --depth "$CLONE_DEPTH"
fi

echo "Reference setup complete."
