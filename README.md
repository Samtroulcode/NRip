# Nrip

***nrip*** is a *safe* replacement for `rm` that moves your files to a **graveyard**
instead of deleting them permanently.

This project is inspired by [rip](https://github.com/nivekuil/rip), this is why the binary name is `nrip` (new rip).
*MVP v0.7.0 :* options `--prune`, `--list`, `--resurrect` and autocompletion feature.

> Default folder : `${XDG_DATA_HOME:-~/.local/share}/riptide/graveyard`  
> Index : `${XDG_DATA_HOME:-~/.local/share}/riptide/index.json`

## Installation

### From Arch User Repository (AUR)

```bash
yay -S riptide
# or 
paru -S riptide
```

### From cargo (Rust package manager)

```bash
cargo install nrip
```

### From source

```bash
git clone https://forgejo.dirty-flix-servarr.fr/Samda/riptide.git
cd riptide
cargo install --path .
# binaire dans ~/.cargo/bin/nrip
```

### Local build

```bash
cargo build --release
./target/release/riptide --help
```

## Use

The default location of the graveyard is `${XDG_DATA_HOME:-~/.local/share}/riptide/graveyard`.

```bash
Safe rm with a graveyard

Usage: nrip [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Files/dirs to remove (default action)

Options:
  -p, --prune [<TARGET>]      Prune graveyard; optional TARGET value allows `-p TARGET`
      --target <TARGET>       (optional) explicit target
  -r, --resurrect [<TARGET>]  Resurrect (restore) from graveyard; optional TARGET allows `-r TARGET`
  -l, --list                  List graveyard contents
      --dry-run
  -y, --yes                   (optional) skip confirmation prompts
  -h, --help                  Print help
  -V, --version               Print version
```

`rip` : move files to the graveyard. (default action)

```bash
nrip file1 file2
```

`--list (-l)` : list files in the graveyard.

```bash
nrip -l
```

`-p <target>` : graveyard pruning (permanent deletion). (--prune)

```bash
nrip -p # interactive pruning of files in the graveyard
nrip -p file # prune (permanently delete) specific file from graveyard
```

`-r <target>` : resurrect (restore) files from the graveyard. (--resurrect)

```bash
nrip -r file1 file2
nrip -r # interactive resurrection of files in the graveyard
```

`--help (-h)` : display help.

```bash
nrip -h
```

`--version (-v)` : display version.

```bash
nrip -v
```

## Bash and Zsh completion

To activate bash completion, add the following line to your `.bashrc` or `.zshrc` file:

```bash
_rip_complete() {
  local cur prev cmd
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
compdef _rip_complete nrip
```
