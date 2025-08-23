# riptide (v0.6.0)

***riptide*** is a *safe* replacement for `rm` that moves your files to a **graveyard**
instead of deleting them permanently.
*MVP v0.6.0 :* options `--prune`, `--list`, `--resurrect` and autocompletion feature.

> Default folder : `${XDG_DATA_HOME:-~/.local/share}/riptide/graveyard`  
> Index : `${XDG_DATA_HOME:-~/.local/share}/riptide/index.json`

## Installation

### From Arch User Repository (AUR)

```bash
yay -S riptide
# or 
paru -S riptide
```

### From source

```bash
git clone https://forgejo.dirty-flix-servarr.fr/Samda/riptide.git
cd riptide
cargo install --path .
# binaire dans ~/.cargo/bin/riptide
```

### Local build

```bash
cargo build --release
./target/release/riptide --help
```

## Use

`rip` : move files to the graveyard. (default action)

```bash
rip file1 file2
```

`--list (-l)` : list files in the graveyard.

```bash
rip -l
```

`-p <target>` : graveyard pruning (permanent deletion). (--prune)

```bash
rip -p # interactive pruning of files in the graveyard
rip -p file # prune (permanently delete) specific file from graveyard
```

`-r <target>` : resurrect (restore) files from the graveyard. (--resurrect)

```bash
rip -r file1 file2
rip -r # interactive resurrection of files in the graveyard
```

`--help (-h)` : display help.

```bash
rip -h
```

`--version (-v)` : display version.

```bash
rip -v
```

## Bash and Zsh completion

To activate bash completion, add the following line to your `.bashrc` or `.zshrc` file:

```bash
_rip_complete() {
  local cur prev cmd
  cur=${words[-1]}
  prev=${words[-2]}

  if [[ $prev == "-p" || $prev == "--prune" ]]; then
    compadd -- ${(f)"$(rip --__complete prune "$cur")"}
    return 0
  elif [[ $prev == "-r" || $prev == "--resurrect" ]]; then
    compadd -- ${(f)"$(rip --__complete resurrect "$cur")"}
    return 0
  fi
  return 1
}
compdef _rip_complete rip
```
