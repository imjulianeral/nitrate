# NITRATE

NITRATE is a terminal console for video extraction. It uses yt-dlp to fetch streams. It uses ffmpeg to merge and trim.

Repository: <https://github.com/imjulianeral/nitrate>

## Requirements

The installer and the GitHub Release archives include `yt-dlp`, `ffmpeg`, `ffprobe`, and `qjs`.
You do not install these programs yourself.

`yt-dlp` uses `qjs` for YouTube. You do not install Node.js or Deno.

`curl` is required for the installer and for `nitrate update`.

If you build from source, a release build downloads the same programs into `target/release/tools`.


## Install

The installer writes the `nitrate` binary and a `tools` directory for your OS and CPU.

### Linux and macOS

Run:

```sh
curl -fsSL https://github.com/imjulianeral/nitrate/releases/latest/download/install.sh | sh
```

If `/usr/local/bin` is writable, the script writes the files there. If it is not writable, the script writes to `~/.local/bin`.

If the install directory is not on `PATH`, add it.

If you want a different directory, set `NITRATE_INSTALL_DIR` before you run the script.

### Windows

In PowerShell, run:

```powershell
irm https://github.com/imjulianeral/nitrate/releases/latest/download/install.ps1 | iex
```

The script writes `nitrate.exe` and `tools` to `%LOCALAPPDATA%\nitrate`. Then it adds that directory to the user `PATH`.

Open a new terminal after the install. Then run `nitrate`.

If you want a different directory, set `NITRATE_INSTALL_DIR` before you run the script.

### Release archives

GitHub Releases include:

- `nitrate-linux-x64.tar.gz`
- `nitrate-linux-arm64.tar.gz`
- `nitrate-macos-arm64.tar.gz`
- `nitrate-macos-x64.tar.gz`
- `nitrate-windows-x64.zip`

Each archive contains `nitrate` and a `tools` directory with `yt-dlp`, `ffmpeg`, `ffprobe`, and `qjs`.

### Install from source

You need Rust 1.88 or newer. You also need `curl` so the build can download `yt-dlp` and `ffmpeg`.

```sh
cargo install --git https://github.com/imjulianeral/nitrate --locked
```

`cargo install` writes only the `nitrate` binary. It does not write the `tools` directory. Use a GitHub Release if you want the bundled programs.

To build from a clone:

1. Clone the repository.
2. Run `cargo build --release`.
3. Copy `target/release/nitrate` and `target/release/tools` to one directory.
4. Put that directory on `PATH`, or run the binary from that directory.

The release build downloads `yt-dlp` and `ffmpeg` into `vendor/tools` and copies them to `target/release/tools`.

Set `NITRATE_SKIP_BUNDLE=1` if you do not want the download. Then you must have `yt-dlp` and `ffmpeg` on `PATH`.

## Update

NITRATE reads the latest GitHub Release and replaces the current binary and the `tools` directory.

### Command line

Run:

```sh
nitrate update
```

If the installed version is current, the command prints `NITRATE  <version>  already current`.

### Console

When a newer release exists, the console shows an update hint.

1. Press `U`.
2. Quit the console.

The update then writes the new files over the current ones.

### Installer

You can run the install command again. The installer overwrites the files in the install directory.

`nitrate update` replaces the files next to the binary that is in use. The installer writes to the default install directory.

If you installed to a custom directory, set `NITRATE_INSTALL_DIR`. Then run the installer again.

## Run

```sh
nitrate
```

Other commands:

```sh
nitrate version
nitrate help
```

You can set `YT_DLP` and `FFMPEG` to force a program path. If you do not set these, NITRATE uses the bundled programs in `tools` next to the binary. That directory includes `qjs` for YouTube.

## Cut a release

A GitHub Release starts when you push a tag that matches `v*.*.*`.

The tree must be clean. Then run:

```sh
./release.sh minor
git push origin HEAD --tags
```

If you want a patch bump or a major bump, pass `patch` or `major`.

The script writes the new version to `Cargo.toml` and `Cargo.lock`. Then it creates a commit and a tag. You do not need cargo-edit.

`./release.sh minor --push` also pushes the commit and the tag.

## License

MIT

The bundled `ffmpeg` and `ffprobe` binaries are GPL. The bundled `yt-dlp` binaries include third-party licenses from the yt-dlp project. The bundled `qjs` binary is QuickJS-NG (MIT).
