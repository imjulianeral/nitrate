#!/bin/sh
# ./release.sh [patch|minor|major] [--push]
set -eu

usage() {
  echo "usage: $0 [patch|minor|major] [--push]" >&2
}

bump=minor
push=0
for arg in "$@"; do
  case "$arg" in
    patch | minor | major) bump=$arg ;;
    --push) push=1 ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 1
      ;;
  esac
done

root=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
cd "$root"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "not a git repository" >&2
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "git tree is not clean" >&2
  exit 1
fi

ver=$(sed -n 's/^version = "\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)"/\1/p' Cargo.toml | head -n 1)
if [ -z "$ver" ]; then
  echo "no version in Cargo.toml" >&2
  exit 1
fi

old_ifs=$IFS
IFS=.
# shellcheck disable=SC2086
set -- $ver
IFS=$old_ifs
major=$1
minor=$2
patch=$3

case "$bump" in
  patch) patch=$((patch + 1)) ;;
  minor)
    minor=$((minor + 1))
    patch=0
    ;;
  major)
    major=$((major + 1))
    minor=0
    patch=0
    ;;
esac

new="$major.$minor.$patch"
tag="v$new"

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "tag $tag already exists" >&2
  exit 1
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT INT HUP
sed "s/^version = \"$ver\"/version = \"$new\"/" Cargo.toml >"$tmp"
mv "$tmp" Cargo.toml
trap - EXIT INT HUP

if ! grep -q "^version = \"$new\"$" Cargo.toml; then
  echo "failed to write version $new" >&2
  exit 1
fi

cargo generate-lockfile

git add Cargo.toml Cargo.lock
git commit -m "$tag"
git tag "$tag"

echo "TAG  $tag"
if [ "$push" -eq 1 ]; then
  git push origin HEAD
  git push origin "$tag"
  echo "PUSH  $tag"
else
  echo "NEXT  git push origin HEAD --tags"
fi
