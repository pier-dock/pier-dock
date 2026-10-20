# Contributing to Pier

<p>
  <b>English</b> · <a href="CONTRIBUTING.pt-BR.md">Português</a>
</p>

Thanks for helping out with Pier. This guide covers how to build Pier from source, run the tests, and send a pull request.

To report a bug or suggest a feature, [open an issue](https://github.com/pier-dock/pier-dock/issues/new/choose) and pick the matching template.

## Project layout

- `src/`: the React frontend (the dock, the widget panels, and the settings window).
- `src-tauri/`: the Rust backend, built with [Tauri 2](https://tauri.app), with the platform code for Windows, macOS, and Linux.
- `gnome-extension/pier@pier.dev/`: the GNOME Shell extension Pier installs on GNOME.
- `scripts/`: helper scripts to regenerate the icons and prepare a release.

## Development setup

### Prerequisites

- [Node.js](https://nodejs.org) 22 or later
- [Rust](https://rustup.rs) (stable)
- Platform tools:
  - **Windows:** Visual Studio Build Tools with the C++ workload and the Windows SDK. Use the `stable-x86_64-pc-windows-msvc` Rust toolchain.
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).
  - **Linux:** `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, and `patchelf`.

### Run and build

```sh
npm install
npm run tauri dev     # run in development mode
npm run tauri build   # build the installers into src-tauri/target/release/bundle/
```

When you run `npm run tauri dev` on macOS, the system grants permissions to the terminal that ran the command, not to Pier.

### Preview in the browser

Outside Tauri, `npm run dev` answers IPC calls with sample data so you can review the UI in a browser. Open `http://localhost:1420` with one of these query strings:

- `?view=dock`
- `?view=panel&kind=media` (or `github-mine`, `github-review`, `github-contrib`, `volume`, `network`)
- `?view=settings`

### Tests

```sh
npm test                               # frontend tests
cd src-tauri
cargo test                             # Rust unit tests
cargo test -- --ignored --nocapture    # smoke tests against the real system
```

The smoke tests exercise real windows, apps, media sessions, and the GitHub API on the machine that runs them.

### Icons

All platform icons come from `app-icon.svg` and `src-tauri/icons/source/`. To regenerate them, run `./scripts/icons.sh` on macOS (it requires `iconutil`).

## Pull requests

- Keep each pull request small and focused on one change.
- Title it in the [Conventional Commits](https://www.conventionalcommits.org) style, `type(scope): summary`, for example `feat(dock): …` or `fix(windows): …`. Pull requests are squash-merged, so the title becomes the commit message.
- CI runs `npm run build` and `npm test`, and `cargo test --locked` on Windows, macOS, and Linux. Run them locally before you open the pull request.
- If the change is visible to users, update `README.md` and `README.pt-BR.md`.
- Don't change the version in `package.json` or `src-tauri/Cargo.toml`, and don't add version sections to the changelogs. The maintainer picks the version and writes the release notes when publishing a release, and CI rejects pull requests that change them.

## License

By contributing, you agree that your contributions are licensed under the [GNU General Public License v3.0 or later](LICENSE), the same license as Pier.
