#!/bin/sh
# curl -fsSL https://github.com/scrtx/video-tools/releases/latest/download/install.sh | sh
set -eu

REPO="${NITRATE_REPO:-scrtx/video-tools}"
BIN="nitrate"

os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)

case "$os" in
  linux) os=linux ;;
  darwin) os=macos ;;
  mingw*|msys*|cygwin*) os=windows ;;
  *)
    echo "unsupported os: $os" >&2
    exit 1
    ;;
esac

case "$arch" in
  x86_64|amd64) arch=x64 ;;
  aarch64|arm64) arch=arm64 ;;
  *)
    echo "unsupported arch: $arch" >&2
    exit 1
    ;;
esac

ext=""
if [ "$os" = windows ]; then
  ext=".exe"
fi
asset="${BIN}-${os}-${arch}${ext}"
url="https://github.com/${REPO}/releases/latest/download/${asset}"

if [ -n "${NITRATE_INSTALL_DIR:-}" ]; then
  dest_dir=$NITRATE_INSTALL_DIR
elif [ -w /usr/local/bin ] 2>/dev/null; then
  dest_dir=/usr/local/bin
else
  dest_dir="${HOME}/.local/bin"
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

mkdir -p "$dest_dir"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT INT HUP

echo "GET  $url"
curl -fsSL --retry 3 --connect-timeout 10 -A nitrate "$url" -o "$tmp"
chmod +x "$tmp"
mv "$tmp" "${dest_dir}/${BIN}${ext}"
trap - EXIT INT HUP

echo "WRITE  ${dest_dir}/${BIN}${ext}"
echo "NITRATE  installed"
case ":$PATH:" in
  *":${dest_dir}:"*) ;;
  *)
    echo "ADD    ${dest_dir} to PATH"
    ;;
esac
