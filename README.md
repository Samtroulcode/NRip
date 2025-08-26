# NRip

> **neo rip** — a safe and modern replacement for `rm` that sends your files to the **graveyard**. Bury now, decide later. If you like living dangerously, there’s always the crematorium.

<p align="center">
  <img src="assets/img/tombstone.svg" width="120" alt="NRip tombstone"/>
</p>

---

NRip moves files/dirs to a **graveyard** instead of deleting them. You can then **list**, **cremate** (permanently delete), or **resurrect** them — interactively with **fzf** or non‑interactively by targeting a basename substring or an ID prefix.

> Inspired by [rip](https://github.com/nivekuil/rip). Binary name: **nrip** (neo rip — a wink to nvim and rip).

> **Default paths (XDG)**
>
> * **Graveyard**: `${XDG_DATA_HOME:-$HOME/.local/share}/nrip/graveyard`
> * **Index**: `${XDG_DATA_HOME:-$HOME/.local/share}/nrip/index.json`

## What you get (in the dead of night)

* 🪦 **Bury** (default action): timestamped, unique names — no collisions among the dearly departed.
* 🔎 **List**: readable output with age, kind, short IDs, and original path.
* ⚰️ **Cremate**: permanently erase from the graveyard (interactive or targeted).
* 🧟 **Resurrect**: bring files back to their old haunt; refuses to overwrite the living.
* 🔗 **Cross‑FS aware**: falls back to copy→swap when `EXDEV` strikes.
* ☠️ **Shell completion**: contextual suggestions for cremation and resurrection.

> **Short IDs** — list view prints a 7‑char ID derived from the unique graveyard name. You can target **cremate**/**resurrect** using a basename substring *or* that ID prefix.

---

## Gloomy tour (demo)

  ![Resurrect demo](assets/gifs/demo.gif)

---

## Installation

### Arch User Repository (AUR)

```bash
yay -S nrip
# or
paru -S nrip
```

### Cargo

```bash
cargo install nrip
```

### From source

> **Runtime dependency**: interactive **cremate/resurrect** requires [`fzf`](https://github.com/junegunn/fzf).
>
> * Arch: `pacman -S fzf`
> * Debian/Ubuntu: `sudo apt install fzf`
> * macOS (Homebrew): `brew install fzf`

```bash
git clone https://github.com/Samtroulcode/NRip
cd NRip
cargo install --path .
# binary will be in ~/.cargo/bin/nrip
```

### Local build

```bash
cargo build --release
./target/release/nrip --help
```

---

## Usage

```
Usage: nrip [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Files/dirs to remove (default action)

Options:
  -c, --cremate [<TARGET>]    Permanently remove from graveyard
  -r, --resurrect [<TARGET>]  Resurrect (restore) from graveyard
      --target <TARGET>       (optional) explicit target (used with --cremate/--resurrect)
  -f, --force                 (optional) force
  -l, --list                  List graveyard contents
      --dry-run               Dry run (no changes)
  -y, --yes                   (optional) skip confirmation prompts
  -h, --help                  Print help
  -V, --version               Print version
```

### Basic rites

**Bury (default action)**

```bash
nrip file1 dir2
```

The deceased are moved to the graveyard under a **unique name**:
`YYYYMMDDTHHMMSS__RANDOM__basename`.

**List the dearly departed**

```bash
nrip -l
```

Shows short **ID**, timestamp, age, type icon, basename, and original path.

**Cremate (permanent deletion)**

```bash
nrip -c               # FZF interactive menu
nrip -c foo           # target by basename substring or ID prefix
nrip -c --dry-run     # simulate
nrip -c -y            # no prompts (the quick burn)
```

> `--prune` remains available as a compatibility alias.

**Resurrect (restore)**

```bash
nrip -r               # FZF interactive menu
nrip -r foo           # target by basename substring or ID prefix
nrip -r --dry-run     # simulate
nrip -r -y            # raise without confirmation
```

> Restoration is **non‑destructive**: if the original destination already exists, NRip refuses to disturb the living.

> **Matching rules** — `TARGET` can be a **substring of the basename** or a **prefix of the short ID**. Without `TARGET`, an **interactive picker** (fzf) is displayed.

---

## Shell completion

Hidden completion endpoint: `nrip --__complete <context> <prefix>` where `<context>` is `cremate|prune` or `resurrect`.

### Zsh

```zsh
# ~/.zshrc
_nrip_complete() {
  local cur prev
  cur=${words[-1]}
  prev=${words[-2]}

  if [[ $prev == "-c" || $prev == "--cremate" || $prev == "--prune" ]]; then
    compadd -- ${(f)"$(nrip --__complete prune "$cur")"}
    return 0
  elif [[ $prev == "-r" || $prev == "--resurrect" ]]; then
    compadd -- ${(f)"$(nrip --__complete resurrect "$cur")"}
    return 0
  fi
  return 1
}
compdef _nrip_complete nrip
```

### Bash

```bash
# ~/.bashrc
_nrip_complete() {
  local cur prev
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"

  if [[ "$prev" == "-c" || "$prev" == "--cremate" || "$prev" == "--prune" ]]; then
    mapfile -t COMPREPLY < <(nrip --__complete prune "$cur")
  elif [[ "$prev" == "-r" || "$prev" == "--resurrect" ]]; then
    mapfile -t COMPREPLY < <(nrip --__complete resurrect "$cur")
  fi
}
complete -F _nrip_complete nrip
```

---

## Under the slab (how it works)

* **Atomic move first** — attempt `rename(2)`; on cross‑device (`EXDEV`), use copy → swap → remove.
* **Durability** — directory entries are synced to keep the graveyard from losing corpses on power loss.
* **Index** — `index.json` tracks original/trashed paths, timestamps, and kind; guarded by a lock to prevent concurrent corruption.
* **Journal** — `.journal` notes `PENDING/DONE` and `RESTORE_*` events for basic forensics.
* **Symlinks** — preserved during recursive operations when applicable.

> **Security note** — NRip is a user‑space trash bin. It does **not** perform secure shredding.

---

## Roadmap of horrors (configuration)

Planned `~/.config/nrip/config.toml` keys:

```toml
# Change the graveyard location
graveyard_dir = "/data/nrip/graveyard"

# Customize list format (order, fields, colors)
list.format = "{id} {icon} {kind} {deleted_at} {age} {basename} {original_path}"

# FZF preview command used for interactive modes
fzf.preview = "ls -l --color=always {trashed_path} || tree -C {trashed_path}"

# Confirmation policy
confirm.resurrect = true
confirm.cremate_all = "type-YES"
```

Knobs to expect:

* `graveyard_dir`
* `list.format` / `list.time_format`
* `fzf.preview` / `fzf.height`
* `color = auto|always|never` (honors `NO_COLOR`)

---

## FAQ from beyond

**What if the destination exists during resurrection?**  NRip refuses to overwrite; the living stay undisturbed.

**Cross‑device moves?**  On `EXDEV`, NRip copies to a temp in the graveyard, syncs, swaps into place, removes the source.

**Disable colors?**  Set `NO_COLOR=1` or pipe; NRip auto‑detects TTY.

**Uninstall**

* Cargo: `cargo uninstall nrip`
* AUR: `yay -Rns nrip`
* Optional: nuke `${XDG_DATA_HOME:-$HOME/.local/share}/nrip/`

---

## Contributing

Before opening a coffin—err, PR—please run `cargo fmt`, `cargo clippy -D warnings`, and `cargo test`.

---

## License

Dual‑licensed under **MIT** and **Apache‑2.0**. See `LICENSE*`.
