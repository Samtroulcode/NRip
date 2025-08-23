# riptide (v0.1.0)

**riptide** est un remplacement _safe_ de `rm` qui déplace vos fichiers dans un **graveyard**
au lieu de les supprimer définitivement.  
_MVP v0.1.0 :_ sous-commandes `rm` et `ls`, index JSON.

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

`rm` : déplace les fichiers dans le graveyard.

```bash
riptide rm fichier1 fichier2
```

`ls` : liste les fichiers dans le graveyard.

```bash
riptide ls
```
