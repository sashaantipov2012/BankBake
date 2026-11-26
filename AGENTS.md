# BankBake

BankBake is a Tauri desktop app that turns audio files into FMOD sound banks. It can also save and reopen `.bankbake` project archives with their audio assets and event settings.

## Repository layout

- `src/App.svelte` coordinates audio import, project open/save, settings, and baking. `src/lib/components/EventCard.svelte` handles event editing, audio preview, and waveforms.
- `src/lib/components/ui/` contains shared Svelte controls. `src/app.css` and `tailwind.config.js` define shared styles and theme tokens.
- `src-tauri/src/lib.rs` registers Tauri commands and manages app configuration. `src-tauri/src/fmod.rs` handles compiler discovery, bank generation, and project archives.
- `fmodstudio/` contains the bundled Windows FMOD Studio CLI and its runtime files. `samples/` contains audio for checking the import and bake workflows.

## Development commands

Run these from the repository root. The desktop build uses Rust, Node.js, and pnpm. Use `pnpm install --frozen-lockfile` to install JavaScript dependencies from the checked-in lockfile.

| Command | Purpose |
| --- | --- |
| `pnpm dev` | Preview the frontend in a browser at `http://localhost:1420`. |
| `pnpm check` | Run Svelte and TypeScript diagnostics. |
| `pnpm build` | Build the frontend. |
| `pnpm tauri dev` | Run the desktop app with native dialogs, drag and drop, and Tauri commands. |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run Rust tests on Windows with the bundled FMOD CLI. The bake test launches `fmodstudiocl.exe`. |
| `pnpm portable` | Build and package the Windows portable release under `release/`. |

## Working on BankBake

- Follow the existing Svelte 5 runes and reuse shared UI controls where they fit.
- When changing a Tauri command or payload, check the `invoke` call, `src/lib/types.ts`, and the Rust signature or Serde fields together. Rust payload fields generally serialize as camelCase.
- Preserve compatibility with existing `.bankbake` archives and saved settings when changing their formats.
- Use `pnpm check` and `pnpm build` for frontend changes. For changes to native behavior, use the desktop app and exercise the affected workflow with files in `samples/`. Use a scratch output folder for bake checks.
- Keep generated banks, release files, and local configuration out of commits.
