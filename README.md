# NITRATE

NITRATE is a terminal console for video extraction. It uses yt-dlp to fetch streams. It uses ffmpeg to merge and trim.

Repository: <https://github.com/imjulianeral/nitrate>

## Requirements

These programs must be on `PATH`:

- `yt-dlp`
- `ffmpeg`
- `curl` (the installer and `nitrate update` use curl)

## Install

The installer writes the `nitrate` binary for your OS and CPU.

### Linux and macOS

Run:

```sh
curl -fsSL https://github.com/imjulianeral/nitrate/releases/latest/download/install.sh | sh
```

If `/usr/local/bin` is writable, the script writes the binary there. If it is not writable, the script writes to `~/.local/bin`.

If the install directory is not on `PATH`, add it.

If you want a different directory, set `NITRATE_INSTALL_DIR` before you run the script.

### Windows

In PowerShell, run:

```powershell
irm https://github.com/imjulianeral/nitrate/releases/latest/download/install.ps1 | iex
```

The script writes `nitrate.exe` to `%LOCALAPPDATA%\nitrate`. Then it adds that directory to the user `PATH`.

Open a new terminal after the install. Then run `nitrate`.

If you want a different directory, set `NITRATE_INSTALL_DIR` before you run the script.

### Release binaries

GitHub Releases include:

- `nitrate-linux-x64`
- `nitrate-linux-arm64`
- `nitrate-macos-arm64`
- `nitrate-macos-x64`
- `nitrate-windows-x64.exe`

### Install from source

You need Rust 1.88 or newer.

```sh
cargo install --git https://github.com/imjulianeral/nitrate --locked
```

To build from a clone:

1. Clone the repository.
2. Run `cargo build --release`.
3. Copy `target/release/nitrate` to a directory on `PATH`.

## Update

NITRATE reads the latest GitHub Release and replaces the current binary.

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

The update then writes the new binary over the current one.

### Installer

You can run the install command again. The installer overwrites the binary in the install directory.

`nitrate update` replaces the binary that is in use. The installer writes to the default install directory.

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

## License

MIT
