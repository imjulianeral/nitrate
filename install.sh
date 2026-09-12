#!/bin/sh
# curl -fsSL https://github.com/imjulianeral/nitrate/releases/latest/download/install.sh | sh
set -eu

REPO="${NITRATE_REPO:-imjulianeral/nitrate}"
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

if [ "$os" = windows ]; then
  asset="${BIN}-${os}-${arch}.zip"
else
  asset="${BIN}-${os}-${arch}.tar.gz"
fi
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
if ! command -v tar >/dev/null 2>&1; then
  echo "tar is required" >&2
  exit 1
fi

mkdir -p "$dest_dir"
tmp=$(mktemp)
stage=$(mktemp -d)
trap 'rm -rf "$tmp" "$stage"' EXIT INT HUP

echo "GET  $url"
curl -fsSL --retry 3 --connect-timeout 10 -A nitrate "$url" -o "$tmp"
tar -xf "$tmp" -C "$stage"

if [ ! -f "$stage/${BIN}" ] && [ ! -f "$stage/${BIN}.exe" ]; then
  echo "archive is missing ${BIN}" >&2
  exit 1
fi

if [ -f "$stage/${BIN}.exe" ]; then
  mv "$stage/${BIN}.exe" "${dest_dir}/${BIN}.exe"
  bin_name="${BIN}.exe"
else
  chmod +x "$stage/${BIN}"
  mv "$stage/${BIN}" "${dest_dir}/${BIN}"
  bin_name="${BIN}"
fi

if [ -d "$stage/tools" ]; then
  mkdir -p "${dest_dir}/tools"
  # shellcheck disable=SC2035
  mv "$stage/tools/"* "${dest_dir}/tools/"
  chmod +x "${dest_dir}/tools/"* 2>/dev/null || true
fi

trap - EXIT INT HUP
rm -rf "$tmp" "$stage"

echo "WRITE  ${dest_dir}/${bin_name}"
echo "WRITE  ${dest_dir}/tools"
echo "NITRATE  installed"
case ":$PATH:" in
  *":${dest_dir}:"*) ;;
  *)
    echo "ADD    ${dest_dir} to PATH"
    ;;
esac
