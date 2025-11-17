# Krep Usage Guide

## Install

```bash
sudo apt-get install -y libgtk-4-dev libadwaita-1-dev
cargo install cargo-deb # optional for packaging
# From repo root
cargo build --release
sudo install -Dm755 target/release/krep /usr/local/bin/krep
sudo install -Dm755 target/release/krep-tray /usr/local/bin/krep-tray
sudo install -Dm644 assets/krep.png /usr/share/icons/hicolor/48x48/apps/krep.png
```

## CLI

- Next microdose: `krep` or `krep now`
- Force category: `krep now --category vo2|gtg|mobility`
- Preview only: `krep now --dry-run`
- Auto-complete (tests/automation): `krep now --auto-complete`
- Auto-skip cycle (tests): `krep now --auto-complete-skip`
- Rollup WAL to CSV: `krep rollup --cleanup`
- Data directory override: `--data-dir <path>`

State/WAL live in `$DATA_DIR/wal`; defaults to `~/.local/share/krep`.

## Configuration

`~/.config/krep/config.toml` (created on first run):

```toml
[data]
data_dir = "~/.local/share/krep"

[equipment]
available = ["kettlebell", "pullup_bar", "bands"]

[progression]
burpee_rep_ceiling = 10
kb_swing_max_reps = 15
```

Strength signal (optional): `$DATA_DIR/strength/signal.json`

```json
{ "last_session_at": "2024-01-15T10:30:00Z", "session_type": "lower" }
```

## Tray App (GNOME/Ayatana)

Run `krep-tray`. A tray icon appears with a **Microdose Now** menu.

Popup window actions:
- **Do It**: logs a real session (WAL/state updated)
- **Skip**: rotates to another prescription without persisting
- **Harder Next Time**: bumps progression for the current definition
- **Cancel**: closes window without side effects

If data files are corrupted/unreadable you'll see a banner:
`⚠ Some data could not be loaded. Defaults used.`

## Systemd user service

```bash
mkdir -p ~/.config/systemd/user
cp krep_tray.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable krep_tray.service
systemctl --user start krep_tray.service
```

## Skips & Intensity

- Skipping inserts a temporary `ShownButSkipped` entry to influence round-robin.
- `Harder Next Time` uses progression rules (burpee style upgrades, swing reps, GTG reps).
- WAL only accepts real sessions; skips never persist.
