#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR="$ROOT/search"
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

if [ ! -d "$ROOT/PAPERS" ]; then
    echo "PAPERS directory was not found at $ROOT/PAPERS" >&2
    exit 1
fi

mkdir -p "$INSTALL_ROOT" "$BIN_DIR"
INSTALL_ROOT=$(CDPATH= cd -- "$INSTALL_ROOT" && pwd)
BIN_DIR=$(CDPATH= cd -- "$BIN_DIR" && pwd)
SOURCE_PAPERS=$(CDPATH= cd -- "$ROOT/PAPERS" && pwd)
DEST_PAPERS="$INSTALL_ROOT/PAPERS"
DEST_DB="$INSTALL_ROOT/papers.db"

is_same_or_inside() {
    case "$1" in
        "$2"|"$2"/*) return 0 ;;
        *) return 1 ;;
    esac
}

if is_same_or_inside "$DEST_PAPERS" "$SOURCE_PAPERS" || is_same_or_inside "$SOURCE_PAPERS" "$DEST_PAPERS"; then
    echo "INSTALL_ROOT must not place installed PAPERS data inside, above, or equal to the source PAPERS directory." >&2
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

echo "Building search from source..."
"$CARGO" build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

BUILT_BINARY="$PROJECT_DIR/target/release/search"
if [ ! -f "$BUILT_BINARY" ]; then
    echo "search binary was not found at $BUILT_BINARY after building." >&2
    exit 1
fi

# Install binary
mkdir -p "$INSTALL_ROOT/bin"
cp "$BUILT_BINARY" "$INSTALL_ROOT/bin/search"

# Install PAPERS data
rm -rf "$DEST_PAPERS"
cp -R "$SOURCE_PAPERS" "$DEST_PAPERS"

# Create wrapper script with env vars
WRAPPER="$BIN_DIR/$COMMAND_NAME"
cat > "$WRAPPER" <<WRAPPEREOF
#!/usr/bin/env sh
export PAPERS_DIR="$DEST_PAPERS"
export PAPERS_DB_PATH="$DEST_DB"
exec "$INSTALL_ROOT/bin/search" "\$@"
WRAPPEREOF
chmod +x "$WRAPPER"

# Build the database
echo "Building paper database..."
"$WRAPPER" build-db

# Smoke test
EXPECTED_SMOKE_OUTPUT="B	EMNLP	2020	Attention Is All You Need for Chinese Word Segmentation."
if ! SMOKE_OUTPUT=$("$WRAPPER" query --conference EMNLP --year 2020 attention is all you need); then
    echo "Install smoke test failed." >&2
    exit 1
fi

if ! echo "$SMOKE_OUTPUT" | grep -q "Attention Is All You Need for Chinese Word Segmentation"; then
    echo "Install smoke test output mismatch." >&2
    echo "Expected to contain: $EXPECTED_SMOKE_OUTPUT" >&2
    echo "Actual: $SMOKE_OUTPUT" >&2
    exit 1
fi

echo "Installed $COMMAND_NAME to $WRAPPER"
echo "PAPERS data installed to $DEST_PAPERS"
echo "Database at $DEST_DB"
echo "Install smoke test passed"
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Add $BIN_DIR to PATH if $COMMAND_NAME is not found." ;;
esac
echo "Try: $COMMAND_NAME query --conference AAAI --year 2024 diffusion"
