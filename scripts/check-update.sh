#!/usr/bin/env sh
# Check and update installed topaperlist data.
set -eu

usage() {
    cat <<EOF
Usage: check-update.sh [--yes] [--skip-this-version] [--quiet] [--install-root PATH] [--repo-url URL] [--branch NAME] [--binary PATH]
EOF
}

YES=false
SKIP_THIS_VERSION=false
QUIET=false
INSTALL_ROOT_EXPLICIT=false
INSTALL_ROOT="${TOPAPERLIST_INSTALL_ROOT:-${INSTALL_ROOT:-$HOME/.local/share/topaperlist}}"
REPO_URL="${TOPAPERLIST_REPO_URL:-https://github.com/dududuguo/topaperlist.git}"
BRANCH="${TOPAPERLIST_UPDATE_BRANCH:-main}"
BINARY="${TOPAPERLIST_BINARY:-}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --yes|-y) YES=true ;;
        --skip-this-version) SKIP_THIS_VERSION=true ;;
        --quiet|-q) QUIET=true ;;
        --install-root)
            shift
            [ "$#" -gt 0 ] || { usage >&2; exit 2; }
            INSTALL_ROOT="$1"
            INSTALL_ROOT_EXPLICIT=true
            ;;
        --repo-url)
            shift
            [ "$#" -gt 0 ] || { usage >&2; exit 2; }
            REPO_URL="$1"
            ;;
        --branch)
            shift
            [ "$#" -gt 0 ] || { usage >&2; exit 2; }
            BRANCH="$1"
            ;;
        --binary)
            shift
            [ "$#" -gt 0 ] || { usage >&2; exit 2; }
            BINARY="$1"
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo "Unsupported option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

info() {
    if [ "$QUIET" != "true" ]; then
        printf '%s\n' "$*"
    fi
}

short_sha() {
    value="$1"
    if [ -z "$value" ]; then
        printf '%s' "none"
    else
        printf '%s' "$value" | cut -c 1-12
    fi
}

if ! command -v git >/dev/null 2>&1; then
    info "topaperlist update check skipped: git was not found."
    exit 0
fi

if [ "$INSTALL_ROOT_EXPLICIT" = "true" ]; then
    PAPERS_DIR="$INSTALL_ROOT/PAPERS"
    PAPERS_DB_PATH="$INSTALL_ROOT/papers.db"
else
    PAPERS_DIR="${PAPERS_DIR:-$INSTALL_ROOT/PAPERS}"
    PAPERS_DB_PATH="${PAPERS_DB_PATH:-$INSTALL_ROOT/papers.db}"
fi
REPO_DIR="${TOPAPERLIST_REPO_DIR:-$INSTALL_ROOT/repo}"
VERSION_FILE="$INSTALL_ROOT/db.version"
SKIPPED_VERSION_FILE="$INSTALL_ROOT/skipped.version"
MANAGED_MARKER="$REPO_DIR/.topaperlist-managed"

if [ -z "$BINARY" ]; then
    BINARY="$INSTALL_ROOT/search"
fi

remote_line=$(git ls-remote "$REPO_URL" "refs/heads/$BRANCH" 2>/dev/null || true)
if [ -z "$remote_line" ]; then
    info "topaperlist update check skipped: unable to reach $REPO_URL."
    exit 0
fi

remote_version=$(printf '%s\n' "$remote_line" | awk 'NR == 1 { print $1 }')
if [ -z "$remote_version" ]; then
    info "topaperlist update check skipped: remote version was empty."
    exit 0
fi

local_version=""
if [ -f "$VERSION_FILE" ]; then
    local_version=$(tr -d '[:space:]' < "$VERSION_FILE")
elif [ -d "$REPO_DIR/.git" ]; then
    local_version=$(git -C "$REPO_DIR" rev-parse HEAD 2>/dev/null || true)
fi

if [ "$local_version" = "$remote_version" ]; then
    exit 0
fi

if [ "$SKIP_THIS_VERSION" = "true" ]; then
    mkdir -p "$INSTALL_ROOT"
    printf '%s' "$remote_version" > "$SKIPPED_VERSION_FILE"
    info "Skipped topaperlist data version $(short_sha "$remote_version")."
    exit 0
fi

if [ "$YES" != "true" ] && [ -f "$SKIPPED_VERSION_FILE" ]; then
    skipped_version=$(tr -d '[:space:]' < "$SKIPPED_VERSION_FILE")
    if [ "$skipped_version" = "$remote_version" ]; then
        exit 0
    fi
fi

if [ "$YES" != "true" ]; then
    if [ ! -t 0 ]; then
        exit 0
    fi

    printf 'topaperlist data update available: %s -> %s\n' \
        "$(short_sha "$local_version")" "$(short_sha "$remote_version")"
    while :; do
        printf 'Choose: [u]pdate / [s]kip this version / [c]ancel '
        IFS= read -r answer || answer=""
        case "$answer" in
            u|U|update|UPDATE|y|Y|yes|YES)
                QUIET=false
                break
                ;;
            s|S|skip|SKIP|skip\ this\ version|Skip\ this\ version)
                mkdir -p "$INSTALL_ROOT"
                printf '%s' "$remote_version" > "$SKIPPED_VERSION_FILE"
                QUIET=false
                info "Skipped topaperlist data version $(short_sha "$remote_version")."
                exit 0
                ;;
            ""|c|C|cancel|CANCEL|n|N|no|NO)
                exit 0
                ;;
            *)
                printf '%s\n' "Please choose u, s, or c."
                ;;
        esac
    done
fi

mkdir -p "$INSTALL_ROOT"

if [ -d "$REPO_DIR/.git" ]; then
    if [ ! -f "$MANAGED_MARKER" ]; then
        echo "Refusing to update unmanaged repo at $REPO_DIR. Remove it or set TOPAPERLIST_REPO_DIR to a managed directory." >&2
        exit 1
    fi
    info "Fetching topaperlist data from $REPO_URL..."
    git -C "$REPO_DIR" fetch --depth=1 origin "$BRANCH"
    git -C "$REPO_DIR" checkout -B "$BRANCH" FETCH_HEAD
else
    if [ -e "$REPO_DIR" ] && [ "$(find "$REPO_DIR" -mindepth 1 -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')" != "0" ]; then
        echo "Refusing to clone into non-empty directory: $REPO_DIR" >&2
        exit 1
    fi
    mkdir -p "$(dirname "$REPO_DIR")"
    info "Cloning topaperlist data from $REPO_URL..."
    git clone --depth=1 --branch "$BRANCH" "$REPO_URL" "$REPO_DIR"
    : > "$MANAGED_MARKER"
fi

SOURCE_PAPERS="$REPO_DIR/PAPERS"
if [ ! -d "$SOURCE_PAPERS" ]; then
    echo "PAPERS directory was not found in updated repo: $SOURCE_PAPERS" >&2
    exit 1
fi

case "$PAPERS_DIR" in
    ""|"/") echo "Refusing to replace unsafe PAPERS_DIR: $PAPERS_DIR" >&2; exit 1 ;;
esac
if [ -d "$PAPERS_DIR" ]; then
    rm -rf "$PAPERS_DIR"
fi
cp -R "$SOURCE_PAPERS" "$PAPERS_DIR"

if [ ! -x "$BINARY" ]; then
    echo "Search binary was not found or is not executable: $BINARY" >&2
    exit 1
fi

export PAPERS_DIR
export PAPERS_DB_PATH
export PAPERS_DB_VERSION="$remote_version"
export PAPERS_DB_SOURCE="$REPO_URL#$remote_version"

info "Rebuilding paper database..."
"$BINARY" build-db

printf '%s' "$remote_version" > "$VERSION_FILE"
rm -f "$SKIPPED_VERSION_FILE"
info "topaperlist data updated to $(short_sha "$remote_version")."
