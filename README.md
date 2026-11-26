# BankBake

BankBake is a lightweight desktop tool that compiles audio files directly into FMOD `.bank` files without needing to set up or manage an FMOD Studio project.

## Quick Start

### Windows

1. Download the `BankBake-Portable-…-win-x64.zip` file under **Assets**. [Latest BankBake release](https://github.com/sashaantipov2012/BankBake/releases/latest)
2. Right-click the downloaded ZIP, choose **Extract All**, and open the extracted folder.
3. Double-click `BankBake.exe`. The folder already includes the FMOD tools BankBake uses to make banks.

### Linux

The public release currently offers a Windows download. For Linux, use a `BankBake-Portable-…-linux-x64.tar.gz` archive from someone who built it, or make one using the [portable build instructions](#portable-builds).

1. Open the archive in your file manager and choose **Extract Here** or **Extract To**.
2. Install Wine using your Linux distribution's software or package manager. BankBake uses it to run the included FMOD tools. The app also needs GTK 3 and WebKitGTK 4.1, which your distribution provides. If you already use FMOD Studio on Linux, you can [use its compiler](#linux-compiler-setup).
3. Open the extracted folder and run `bankbake`. If double-clicking it does not launch the app, open a terminal in that folder and enter `./bankbake`.

### Make your first bank

1. Click **Load FMOD Sample Audio** to add the sounds that come with BankBake.
2. Next to **Output Folder**, click **Change** and pick a folder you can find again, such as a new folder in Documents. Leave **Bank Name** and **Master Bank Name** at their defaults for this first run.
3. Click **Bake Bank**. The first bake on Linux may take longer while Wine sets itself up.
4. When the bake succeeds, click **Open Output Folder**. You'll find `SoundBank.bank`, `Master.bank`, `Master.strings.bank`, `GUIDs.txt`, and `SoundBank.bankbake`. Use **Open Project** to reopen the `.bankbake` file and edit the sounds or settings later.

### Use your own audio

Click **Select Audio Files** or drag your files into the window. BankBake accepts WAV, OGG, MP3, FLAC, AIFF, and WMA. Set **Bank Name** to the name you want on the finished `.bank` file, then click **Bake Bank** when you're ready.

Each sound has an event path, which is the name your game or mod uses to find it. You can keep the suggested path or change it if your game or mod expects a specific path, such as `event:/sfx/player/jump`. You can also choose **SFX** or **Music**, turn on **Loop** to repeat a sound, or open **Advanced** to adjust volume and choose where a loop starts and ends.

## Building from Source

Requires [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/), and `pnpm`.

On CachyOS and other Arch-based distributions, install the desktop build tools with the command below. The `rust` package includes Cargo. For other Linux distributions, follow the system dependency instructions. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#system-dependencies)

```sh
sudo pacman -S --needed rust nodejs pnpm base-devel webkit2gtk-4.1 librsvg
```

Run these commands from the BankBake folder.

```sh
# Install dependencies
pnpm install --frozen-lockfile

# Run in development mode
pnpm tauri dev
```

### Linux compiler setup

BankBake supports two compiler setups on Linux.

- Install FMOD Studio 2.02 for Linux. BankBake looks for `fmodstudiocl` on `PATH` and in `/opt/fmodstudio`. For another location, set **FMOD Studio CLI Path** in Settings to the executable or its containing folder. [FMOD downloads](https://www.fmod.com/download/design.html)
- On x86-64, install Wine with `wine` and `winepath` on `PATH`. If BankBake cannot find a native compiler, it uses the bundled `fmodstudio/fmodstudiocl.exe` through Wine. Wine may initialize its prefix on the first bake.

To try a bake, load the bundled samples, choose a writable output folder, and click **Bake Bank**. The Rust end-to-end bake test uses automatic compiler discovery. For a native compiler in a custom location, add its directory to `PATH` before running the test.

```sh
cargo test --manifest-path src-tauri/Cargo.toml test_discover_compiler_and_bake_bank -- --ignored
```

### Portable builds

Run `pnpm portable` to package the app in `release/`. Windows produces `BankBake-Portable-v<version>-win-x64.zip`. Linux uses `tar` to produce `BankBake-Portable-v<version>-linux-<arch>.tar.gz`. Extract the Linux archive and run `./bankbake` from the extracted folder. The target system needs GTK 3, WebKitGTK 4.1, and one of the compiler setups above.

## Tech Stack

- **Backend**: Rust, Tauri 2
- **Frontend**: Svelte 5, TypeScript, Tailwind CSS, WaveSurfer.js
- **Compiler**: FMOD Studio 2.02 CLI

## License

[MIT](LICENSE)
