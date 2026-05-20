#!/usr/bin/env sh
# Install / upgrade topaperlist search tool from local source.
# Idempotent — safe to run repeatedly; re-builds and updates in place.
set -eu

# Ensure UTF-8 output (avoid garbled text on misconfigured terminals).
if command -v locale >/dev/null 2>&1; then
    export LC_ALL="${LC_ALL:-C.UTF-8}"
fi

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR="$ROOT/search"
INSTALL_ROOT="${INSTALL_ROOT:-$HOME/.local/share/topaperlist}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
COMMAND_NAME="${COMMAND_NAME:-search}"
CARGO="${CARGO:-cargo}"

# ── Safety guards ──────────────────────────────────────────────
case "$INSTALL_ROOT" in
    ""|"/") echo "INSTALL_ROOT must not be empty or /" >&2; exit 1 ;;
esac
case "$BIN_DIR" in
    ""|"/") echo "BIN_DIR must not be empty or /" >&2; exit 1 ;;
esac

if [ ! -d "$ROOT/PAPERS" ]; then
    echo "PAPERS directory was not found at $ROOT/PAPERS" >&2
    exit 1
fi

# ── Resolve real paths ─────────────────────────────────────────
mkdir -p "$INSTALL_ROOT" "$BIN_DIR"
INSTALL_ROOT=$(CDPATH= cd -- "$INSTALL_ROOT" && pwd)
BIN_DIR=$(CDPATH= cd -- "$BIN_DIR" && pwd)
SOURCE_PAPERS=$(CDPATH= cd -- "$ROOT/PAPERS" && pwd)
DEST_PAPERS="$INSTALL_ROOT/PAPERS"
DEST_DB="$INSTALL_ROOT/papers.db"
LEGACY_DATA="$INSTALL_ROOT/PaperJson"
WRAPPER="$BIN_DIR/$COMMAND_NAME"
BINARY="$INSTALL_ROOT/bin/$COMMAND_NAME"

# ── Detect install mode ────────────────────────────────────────
HAS_BINARY=false;  [ -f "$BINARY" ] && HAS_BINARY=true
HAS_DATA=false;    [ -d "$DEST_PAPERS" ] && HAS_DATA=true
HAS_DB=false;      [ -f "$DEST_DB" ] && HAS_DB=true
HAS_LEGACY=false;  [ -d "$LEGACY_DATA" ] && HAS_LEGACY=true

echo "Install root: $INSTALL_ROOT"
if $HAS_LEGACY; then
    echo "Install mode: upgrade from legacy (PaperJson -> PAPERS + papers.db)"
elif $HAS_BINARY || $HAS_DATA || $HAS_DB; then
    echo "Install mode: upgrade existing install"
else
    echo "Install mode: fresh install"
fi

# Prevent recursive nesting of PAPERS inside itself.
is_same_or_inside() {
    case "$1" in
        "$2"|"$2"/*) return 0 ;;
        *) return 1 ;;
    esac
}
if is_same_or_inside "$DEST_PAPERS" "$SOURCE_PAPERS" || is_same_or_inside "$SOURCE_PAPERS" "$DEST_PAPERS"; then
    echo "INSTALL_ROOT must not nest PAPERS inside the source PAPERS directory." >&2
    exit 1
fi

# ── Cargo detection (warn, do not auto-install) ────────────────
if ! command -v "$CARGO" >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        CARGO="$HOME/.cargo/bin/cargo"
    fi
fi
if ! command -v "$CARGO" >/dev/null 2>&1; then
    echo "cargo was not found on this system." >&2
    echo "Install Rust and cargo from https://rustup.rs/, then re-run this script." >&2
    exit 1
fi

# ── Build ──────────────────────────────────────────────────────
echo "Building $COMMAND_NAME from source..."
"$CARGO" build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

BUILT_BINARY="$PROJECT_DIR/target/release/$COMMAND_NAME"
if [ ! -f "$BUILT_BINARY" ]; then
    echo "Binary not found at $BUILT_BINARY after build." >&2
    exit 1
fi

# ── Install binary ─────────────────────────────────────────────
mkdir -p "$INSTALL_ROOT/bin"
cp "$BUILT_BINARY" "$BINARY"
echo "Installed binary to $BINARY"

# ── Install PAPERS data ────────────────────────────────────────
if [ -d "$DEST_PAPERS" ]; then
    rm -rf "$DEST_PAPERS"
fi
cp -R "$SOURCE_PAPERS" "$DEST_PAPERS"
echo "PAPERS data installed to $DEST_PAPERS"

# Remove legacy data if present.
if [ -d "$LEGACY_DATA" ]; then
    rm -rf "$LEGACY_DATA"
    echo "Removed legacy PaperJson data at $LEGACY_DATA"
fi

# ── Create wrapper script ──────────────────────────────────────
cat > "$WRAPPER" <<WRAPPEREOF
#!/usr/bin/env sh
export PAPERS_DIR="$DEST_PAPERS"
export PAPERS_DB_PATH="$DEST_DB"
exec "$BINARY" "\$@"
WRAPPEREOF
chmod +x "$WRAPPER"
echo "Wrapper installed to $WRAPPER"

# ── Inject env vars into shell RC files (idempotent) ───────────
SENTINEL_START="# >>> topaperlist install >>>"
SENTINEL_END="# <<< topaperlist install <<<"

ENV_BLOCK=$(cat <<EOF
$SENTINEL_START
export PAPERS_DIR="$DEST_PAPERS"
export PAPERS_DB_PATH="$DEST_DB"
export PATH="$BIN_DIR:\$PATH"
$SENTINEL_END
EOF
)

inject_rc() {
    rc_file="$1"
    [ ! -f "$rc_file" ] && return
    # Remove any previous topaperlist block, then append the new one.
    if grep -qF "$SENTINEL_START" "$rc_file" 2>/dev/null; then
        if sed --version 2>/dev/null | grep -q GNU; then
            sed -i "/$SENTINEL_START/,/$SENTINEL_END/d" "$rc_file"
        else
            sed -i '' "/$SENTINEL_START/,/$SENTINEL_END/d" "$rc_file"
        fi
    fi
    echo "$ENV_BLOCK" >> "$rc_file"
    echo "Updated $rc_file"
}

RC_FILES=""
for candidate in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$candidate" ]; then
        RC_FILES="$RC_FILES $candidate"
    fi
done

if [ -z "$RC_FILES" ]; then
    echo "$ENV_BLOCK" >> "$HOME/.profile"
    echo "Created $HOME/.profile with topaperlist environment"
else
    for rc in $RC_FILES; do
        inject_rc "$rc"
    done
fi

# Export for the current shell so the smoke test uses the right paths.
export PAPERS_DIR="$DEST_PAPERS"
export PAPERS_DB_PATH="$DEST_DB"

# ── Build database ─────────────────────────────────────────────
echo "Building paper database..."
"$BINARY" build-db

# ── Smoke test ─────────────────────────────────────────────────
EXPECTED="Attention Is All You Need for {C}hinese Word Segmentation"
SMOKE_OUTPUT=$("$BINARY" query --conference EMNLP --year 2020 attention is all you need) || {
    echo "Install smoke test failed." >&2
    exit 1
}
if ! echo "$SMOKE_OUTPUT" | grep -qF "$EXPECTED"; then
    echo "Install smoke test output mismatch." >&2
    echo "Expected to contain: $EXPECTED" >&2
    echo "Actual: $SMOKE_OUTPUT" >&2
    exit 1
fi

# ── Summary ────────────────────────────────────────────────────
echo ""
echo "topaperlist installed successfully."
echo "  Command  : $COMMAND_NAME"
echo "  Binary   : $BINARY"
echo "  Wrapper  : $WRAPPER"
echo "  Data     : $DEST_PAPERS"
echo "  Database : $DEST_DB"
echo ""

case ":$PATH:" in
    *":$BIN_DIR:"*)
        echo "Ready — run '$COMMAND_NAME query --conference AAAI --year 2024 diffusion'"
        ;;
    *)
        echo "Open a new terminal, or run: export PATH=\"$BIN_DIR:\$PATH\""
        echo "Then try: $COMMAND_NAME query --conference AAAI --year 2024 diffusion"
        ;;
esac
