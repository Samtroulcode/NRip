# riptide (v0.3.0)

***riptide*** is a *safe* replacement for `rm` that moves your files to a **graveyard**
instead of deleting them permanently.
*MVP v0.3.0 :* options `--prune`, `--list`, `--resurrect`.

> Default folder : `${XDG_DATA_HOME:-~/.local/share}/riptide/graveyard`  
> Index : `${XDG_DATA_HOME:-~/.local/share}/riptide/index.json`

## Installation

### From source

```bash
git clone ssh://git@forgejo.dirty-flix-servarr.fr:2222/Samda/riptide.git
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
rip -p # graveyard pruning (permanent deletion) of all files
rip -p file # prune (permanently delete) specific file from graveyard
```

`-r <target>` : resurrect (restore) files from the graveyard. (--resurrect)

```bash
rip -r file1 file2
rip -r # list files available for resurrection
```

`--help (-h)` : display help.

```bash
rip -h
```

`--version (-v)` : display version.

```bash
rip -v
```
