# NRip

**NRip** is a *safe* replacement for `rm`: instead of permanently deleting files, it moves them to a **graveyard** from which you can **list**, **prune** (permanently delete), or **resurrect** (restore), with **fzf** feature.

Inspired by [rip](https://github.com/nivekuil/rip) — hence the binary name `nrip` (new rip).

**MVP v0.8.0**: `--prune`, `--list`, `--resurrect`, contextual shell completion, **interactive picker (fzf)**.

> **Default paths (XDG)**
>
> * **Graveyard**: `${XDG_DATA_HOME:-$HOME/.local/share}/nrip/graveyard`
> * **Index**: `${XDG_DATA_HOME:-$HOME/.local/share}/nrip/index.json`

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

**Runtime dependency**

Interactive `-p/--prune` and `-r/--resurrect` require [`fzf`](https://github.com/junegunn/fzf).

* Arch: `pacman -S fzf`
* Debian/Ubuntu: `sudo apt install fzf`
* macOS (Homebrew): `brew install fzf`

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
Safe rm with a graveyard

Usage: nrip [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Files/dirs to remove (default action)

Options:
  -p, --prune [<TARGET>]      Prune graveyard; optional TARGET value allows `-p TARGET`
      --target <TARGET>       (optional) explicit target (used with --prune)
  -r, --resurrect [<TARGET>]  Resurrect (restore) from graveyard; optional TARGET allows `-r TARGET`
  -l, --list                  List graveyard contents
      --dry-run               Simulation (no changes)
  -y, --yes                   Skip interactive confirmations
  -h, --help                  Print help
  -V, --version               Print version
```

### Basic actions

* **Bury (default action):**

  ```bash
  nrip file1 dir2
  ```

  Items are moved to the graveyard under a **unique name**:
  `YYYYMMDDTHHMMSS__RANDOM__basename`.

* **List:**

  ```bash
  nrip -l
  ```

  For each entry it shows:

  * a short **ID** (first 7 chars of `RANDOM`),
  * the `deleted_at` timestamp,
  * the `basename`,
  * the original path.

* **Prune (permanent deletion):**

  ```bash
  nrip -p               # FZF interactive menu
  nrip -p foo           # target by basename substring or ID prefix
  nrip -p --dry-run     # simulate
  nrip -p -y            # delete without confirmation (dangerous)
  ```

* **Resurrect (restore):**

  ```bash
  nrip -r               # FZF interactive menu
  nrip -r foo           # target by basename substring or ID prefix
  nrip -r --dry-run     # simulate
  nrip -r -y            # restore without confirmation
  ```

  Restoration is **non-destructive**: if the original destination already exists, restoration **fails** (no overwrite).

> **Matching rules (for prune/resurrect)**
> `TARGET` can be a **substring of the basename** or a **prefix of the short ID** (the 7 chars printed by `-l`).
> Without `TARGET`, an **interactive picker** is displayed (0=ALL).

### Interactive picker (fzf)

When `-p/--prune` or `-r/--resurrect` are used **without a TARGET**, NRip opens an `fzf` picker:

* **Multi-select** with **Tab** (press Tab repeatedly, Enter to confirm). :contentReference[oaicite:2]{index=2}
* Displayed fields: timestamp, original path, `->`, trashed path (index hidden via `--with-nth`). :contentReference[oaicite:3]{index=3}
* Output is parsed with **`--print0`** to handle arbitrary characters safely. :contentReference[oaicite:4]{index=4}

---

## Shell completion

NRip exposes a hidden completion endpoint used by the functions below:
`nrip --__complete <context> <prefix>` where `<context>` is `prune` or `resurrect`.

### Zsh

```zsh
# ~/.zshrc
_nrip_complete() {
  local cur prev
  cur=${words[-1]}
  prev=${words[-2]}

  if [[ $prev == "-p" || $prev == "--prune" ]]; then
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

  if [[ "$prev" == "-p" || "$prev" == "--prune" ]]; then
    mapfile -t COMPREPLY < <(nrip --__complete prune "$cur")
  elif [[ "$prev" == "-r" || "$prev" == "--resurrect" ]]; then
    mapfile -t COMPREPLY < <(nrip --__complete resurrect "$cur")
  fi
}
complete -F _nrip_complete nrip
```

---

## How it works (robustness & safety)

* **Atomic move when possible**: NRip first tries an atomic `rename(2)` to move the file/dir into the graveyard. If the move crosses filesystems (`EXDEV`), it falls back to **copy then remove**.

* **Durability**: after writing/renaming, the parent directory is `fdatasync`’d to ensure directory entries are persisted.

* **Index file**: `index.json` tracks original/trashed paths and timestamps. It is read/written under an advisory lock to avoid corruption across concurrent NRip processes.

* **Journal**: a plain-text `.journal` logs `PENDING/DONE` and `RESTORE_*` events for basic auditing and recovery hints.

* **Symlinks**: preserved (copied as links) during recursive operations when applicable.

> **Security note**: NRip is a user-space trash bin. It does **not** perform secure shredding/erasure.

---

## Environment & version

* **Paths** honor `XDG_DATA_HOME` (fallback to `$HOME/.local/share`).
* `nrip -V` prints the Cargo package version used at build time.

---

## Troubleshooting

* **Cross-device moves**: seeing a cross-device fallback is expected when source and graveyard live on different filesystems; NRip copies then removes.

---

## License

Dual-licensed under **MIT** and **Apache-2.0** (see `LICENSE*`).
