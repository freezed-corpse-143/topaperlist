#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
INSTALL_ROOT=${INSTALL_ROOT:-"$HOME/.local/share/topaperlist"}
BIN_DIR=${BIN_DIR:-"$HOME/.local/bin"}
COMMAND_NAME=${COMMAND_NAME:-search}
CARGO=${CARGO:-cargo}

case "$INSTALL_ROOT" in
    ""|"/")
        echo "INSTALL_ROOT must not be empty or /" >&2
        exit 1
        ;;
esac

case "$BIN_DIR" in
    ""|"/")
        echo "BIN_DIR must not be empty or /" >&2
        exit 1
        ;;
esac

if [ ! -d "$ROOT/Paper" ]; then
    echo "Paper directory was not found at $ROOT/Paper" >&2
    exit 1
fi

mkdir -p "$INSTALL_ROOT" "$BIN_DIR"
INSTALL_ROOT=$(CDPATH= cd -- "$INSTALL_ROOT" && pwd)
BIN_DIR=$(CDPATH= cd -- "$BIN_DIR" && pwd)
SOURCE_PAPER=$(CDPATH= cd -- "$ROOT/Paper" && pwd)
DEST_PAPER="$INSTALL_ROOT/Paper"

is_same_or_inside() {
    case "$1" in
        "$2"|"$2"/*) return 0 ;;
        *) return 1 ;;
    esac
}

if is_same_or_inside "$DEST_PAPER" "$SOURCE_PAPER" || is_same_or_inside "$SOURCE_PAPER" "$DEST_PAPER"; then
    echo "INSTALL_ROOT must not place installed Paper data inside, above, or equal to the source Paper directory." >&2
    exit 1
fi

if ! command -v "$CARGO" >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        CARGO="$HOME/.cargo/bin/cargo"
    fi
fi

if ! command -v "$CARGO" >/dev/null 2>&1; then
    echo "cargo was not found. Install Rust from https://rustup.rs/ and try again." >&2
    exit 1
fi
"$CARGO" build --release --manifest-path "$ROOT/Cargo.toml"

BUILT_BINARY="$ROOT/target/release/search"
if [ ! -f "$BUILT_BINARY" ]; then
    echo "search binary was not found at $BUILT_BINARY after building." >&2
    exit 1
fi

mkdir -p "$INSTALL_ROOT/bin"
cp "$BUILT_BINARY" "$INSTALL_ROOT/bin/search"
rm -rf "$DEST_PAPER"
cp -R "$SOURCE_PAPER" "$DEST_PAPER"

WRAPPER="$BIN_DIR/$COMMAND_NAME"
cat > "$WRAPPER" <<EOF
#!/usr/bin/env sh
exec "$INSTALL_ROOT/bin/search" "\$@"
EOF
chmod +x "$WRAPPER"

EXPECTED_SMOKE_OUTPUT="B	EMNLP	2020	Attention Is All You Need for Chinese Word Segmentation."
if ! SMOKE_OUTPUT=$("$WRAPPER" --conference EMNLP --year 2020 attention is all you need); then
    echo "Install smoke test failed." >&2
    exit 1
fi

if [ "$SMOKE_OUTPUT" != "$EXPECTED_SMOKE_OUTPUT" ]; then
    echo "Install smoke test output mismatch." >&2
    echo "Expected: $EXPECTED_SMOKE_OUTPUT" >&2
    echo "Actual: $SMOKE_OUTPUT" >&2
    exit 1
fi

echo "Installed $COMMAND_NAME to $WRAPPER"
echo "Paper data installed to $INSTALL_ROOT/Paper"
echo "Install smoke test passed"
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Add $BIN_DIR to PATH if $COMMAND_NAME is not found." ;;
esac
echo "Try: $COMMAND_NAME --conference AAAI --year 2024 diffusion"
