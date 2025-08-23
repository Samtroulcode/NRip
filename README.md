# riptide (v0.2.0)

**riptide** est un remplacement _safe_ de `rm` qui déplace vos fichiers dans un **graveyard**
au lieu de les supprimer définitivement.  
_MVP v0.2.0 :_ options `--prune` et `--list`.

> Emplacement par défaut : `${XDG_DATA_HOME:-~/.local/share}/riptide/graveyard`  
> Index : `${XDG_DATA_HOME:-~/.local/share}/riptide/index.json`

## Installation

### Depuis la source

```bash
git clone ssh://git@forgejo.dirty-flix-servarr.fr:2222/Samda/riptide.git
cd riptide
cargo install --path .
# binaire dans ~/.cargo/bin/riptide
```

### Build local

```bash
cargo build --release
./target/release/riptide --help
```

## Usage

`rip` : déplace les fichiers dans le graveyard.

```bash
rip fichier1 fichier2
```

`-l` : liste les fichiers dans le graveyard. (--list)

```bash
rip -l
```

`-p <target>` : prune le graveyard (supprime définitivement les fichiers) ou target. (--prune)

```bash
rip -p # supprime tout le graveyard
rip -p monfichier # supprime monfichier du graveyard
```
