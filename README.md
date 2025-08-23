# riptide — Spécification de conception (v0.1)

> Objectif : un remplacement sûr et puissant de `rm`, avec un **graveyard** fiable, des **restaurations** robustes, et des **politiques de purge** configurables. Outil 100% CLI (plus tard TUI), rapide, scriptable, et reproductible.

---

## 1) Principes & portée

### Objectifs

* Remplacer `rm` dans le quotidien (drop-in avec garde‑fous) : `riptide rm …`.
* **Déplacer** vers un graveyard local au lieu de supprimer (moves atomiques intra‑FS, fallback copy+unlink inter‑FS).
* **Indexer** chaque entrée (métadonnées riches) pour lister, filtrer, restaurer, inspecter, et purger selon des **politiques**.
* **Ergonomie** : flags clairs, sortie JSON optionnelle, i18n minimal, autocomplétions.
* **Sécurité** : protections anti-catastrophes, dry‑run, journalisation.

### Non‑objectifs (v1)

* Pas de synchronisation réseau/multi‑host.
* Pas de gestion fine des ACLs propriétaires complexes (off par défaut, module optionnel plus tard).
* Pas de TUI en v1 (prévu v1.1+).

---

## 2) Nommage & identité

* Binaire : `riptide` (court, mémorisable, clin d’œil à `rip`).
* Namespace XDG : `riptide`.

---

## 3) Expérience CLI (UX)

### Sous-commandes

* `riptide rm <paths…>` : déplace vers graveyard + enregistre index.
* `riptide ls` : liste le contenu indexé (tableau par défaut, `--json`).
* `riptide inspect <ID|PATH>` : affiche toutes métadonnées connues, preview textuelle.
* `riptide restore <ID|PATH> [-t <dest>] [--force-overwrite]` : restaure en recréant l’arbo.
* `riptide prune` : applique stratégies (TTL, taille max, patterns, LRU).
* `riptide vacuum` : compaction/maintenance de l’index, purge tombstones.
* `riptide doctor` : diagnostic du graveyard (cohérence index↔fichiers, permissions, espace libre).
* `riptide config [get|set|path]` : lecture/écriture de configuration.

### Options communes

* `-n/--dry-run` : ne rien écrire, imprimer ce qui se passerait.
* `-v/--verbose` (cumulable), `-q/--quiet`.
* `--json` : sortie machine.
* `--color=auto|always|never`.
* `--no-confirm` : supprime les confirmations interactives.

### Options `rm`

* `-r/--recursive`, `-f/--force`, `-I` (confirmation « dangereuse »), `--one-file-system`.
* **Garde‑fous** : blocage par défaut sur `/`, `/home`, mount roots, symlink traversal contrôlé.
* `--no-guard` pour désactiver explicitement (impose `--no-confirm`).

### Options `ls`

* Filtres : `--since`, `--until`, `--user`, `--path <glob>`, `--type {file,dir}`,
  `--larger-than`, `--smaller-than`, `--expired`, `--policy <name>`.
* Formatage : `--long`, `--full-paths`, `--ids-only`, `--limit`, `--sort <key>`.

### Options `prune`

* `--ttl 30d`, `--max-size 20G`, `--max-items 10000`, `--policy <name>`,
  `--protect <glob>` (ne jamais purger), `--dry-run`.

---

## 4) Modèle de données & index

### Emplacement graveyard

* **Linux** : `${XDG_DATA_HOME:-~/.local/share}/riptide/graveyard`.
* Sous-dossiers par jour/mois (ex. `2025/08/23/…`) pour limiter la densité.
* Fichiers déplacés en conservant le nom d’origine + suffixe ID court en cas de collision.

### Index & schéma (v1 JSON → v1.1 SQLite)

* **v1 JSON** (simple, lisible, migration facile) :

  ```json
  {
    "version": 1,
    "entries": [
      {
        "id": "b3q1fk9",
        "original_path": "/home/sam/code/pokedex/target/foo.o",
        "stored_relpath": "2025/08/23/b3/b3q1fk9-foo.o",
        "deleted_at": "2025-08-23T15:41:23Z",
        "size": 12345,
        "inode": 99887766,
        "uid": 1000,
        "gid": 1000,
        "mode": "0o100644",
        "xattrs": null,
        "checksum": "blake3:…",
        "fs_uuid": "…",
        "mount_point": "/home",
        "notes": null
      }
    ]
  }
  ```
* **v1.1 SQLite** : tables `entries`, `tags`, `props(k,v)`, index sur `deleted_at`, `size`, `original_path`, `checksum`.
* **ID** : BLAKE3(triplet `path + nsecs + rand`) → 6–8 chars base62 pour UX.

### Invariants

* Un ID ↔ un fichier/dir déplacé. Dossiers gérés récursivement avec entrées multiples.
* L’index est la source de vérité ; le `stored_relpath` doit exister (doctor répare sinon).

---

## 5) Opérations & algorithmes

### `rm` (move-to-graveyard)

1. Résolution des chemins (ne pas suivre symlinks sauf opt-in).
2. Vérifs garde‑fous (racines critiques, mountpoints, ownership si `--preserve-owner`).
3. Tentative **rename(2)** intra-filesystem ; fallback **copy + unlink** si cross‑device.
4. Capture métadonnées (stat, xattrs si activé), calcul checksum **optionnel** (lazy, via tâche différée).
5. Écriture atomique dans l’index (file‑lock global + fsync).

### `restore`

1. Résolution: ID ou recherche par `original_path` dernier.
2. Si destination existe → stratégies: `--force-overwrite` | suffixe `(restored-N)` | abort interactif.
3. Recréation des dossiers, perms, timestamps, xattrs, puis move back (rename, sinon copy+unlink).
4. Marquage dans l’index (champ `restored_at` et tombstone) — option `--keep` pour garder une copie.

### `prune`

* Calcul ensemble supprimable selon politiques (TTL, quotas).
* Suppression réelle + suppression des entrées index.
* Mode `--dry-run` montre taille récupérée.

### Concurrence

* **Advisory lock** sur le dossier graveyard + mutex sur l’index (flock).
* Transactions (journalisation) pour éviter index orphelin.

---

## 6) Fichiers & configuration

### Fichiers

* Graveyard : `${XDG_DATA_HOME}/riptide/graveyard/…`
* Index : `${XDG_DATA_HOME}/riptide/index.json` (→ `index.sqlite` plus tard)
* Config : `${XDG_CONFIG_HOME:-~/.config}/riptide/config.toml`
* Logs (optionnels) : `${XDG_STATE_HOME:-~/.local/state}/riptide/logs/…`

### Config TOML (exemple)

```toml
[graveyard]
path = "" # vide = chemin par défaut XDG
checksum = "lazy" # off|lazy|eager
follow_symlinks = false

[guard]
protect_roots = true
protect_mounts = true
confirm_dangerous = true

[prune]
ttl = "30d"
max_size = "20G"
policy = "ttl-then-lru"

[output]
color = "auto"
locale = "fr"
```

### Priorité de config

1. Flags CLI > 2. Variables d’environnement (`RIPTIDE_*`) > 3. `config.toml` > 4. défauts.

---

## 7) Ergonomie & DX

* **Sortie tabulaire** par défaut, colonnes: `ID  When  Size  Type  Path`.
* `--json` pour scripting (stable schema), `--quiet` pour usage pipeline.
* Complétions shell (bash/zsh/fish) générées par `clap`.
* Messages d’erreurs courts + aide contextualisée `--help`.

---

## 8) Sécurité

* Refus explicite sur patterns catastrophes (`/`, `/*`, `~`, `~/`, `/home`, `/root`, `/etc`, mount roots) sauf `--no-guard` + `--no-confirm`.
* Dry‑run global, confirmation interactive si >N fichiers ou si racine sensible.
* Gestion des permissions : refuse si pas d’ownership (option `--sudo-handoff` v2?).
* Mode “audit” : log JSON des opérations (optionnel) pour SIEM.

---

## 9) Performances

* `rename(2)` prioritaire ; détection cross-device via `st_dev`.
* Checksums en tâche différée (file queue) si `checksum=lazy`.
* Parallélisme contrôlé (rayon) pour gros dossiers ; limite I/O (semaphore).
* Benchmarks critiqués : 1k/10k petits fichiers, 10–100 Go, HDD vs SSD, btrfs vs ext4.

---

## 10) Compatibilité & interop

* Option `--xdg-trash` pour envoyer vers la corbeille FreeDesktop (intégration DE).
* Option `--rip-compat` : accepter quelques flags/UX de `rip`.
* Sortie `--json` stable → scripts & TUI.

---

## 11) Journalisation & observabilité

* Niveaux: error/warn/info/debug/trace (env `RUST_LOG`).
* `riptide doctor` : vérif index↔fichiers, permissions, espace, versions.

---

## 12) Internationalisation

* Messages FR/EN via table interne (pas de framework lourd v1).

---

## 13) Erreurs & codes de sortie

* `0` succès, `1` erreur générique, `2` arguments, `3` garde‑fou, `4` I/O, `5` index, `6` permission.
* En mode batch/`--json`, toujours émettre un objet d’erreur structuré.

---

## 14) Tests & QA

* Tests unitaires (path, id, index atomique, guards), intégration (rename/copy, restore),
  tests de concurrence (flock), E2E via `assert_cmd`.
* Matrix CI : Linux x86\_64 (ext4, btrfs via container), musl.

---

## 15) Roadmap

* **v0.1** : `rm`, `ls`, index JSON, guards, restore simple, prune TTL, doctor basique.
* **v0.2** : conflits de restore, `max-size`, sorties JSON stables, complétions.
* **v1.0** : SQLite, policies composables, vacuum, i18n, manpages.
* **v1.1** : TUI (ratatui), hooks, XDG Trash optionnel, tags/notes sur entrées.

---

## 16) Exemples d’usage

```bash
# Remplacer rm (safe):
riptide rm *.log

# Lister le graveyard depuis 7 jours, gros fichiers, JSON
riptide ls --since 7d --larger-than 100M --json | jq '.entries | length'

# Restaurer par ID
riptide restore b3q1fk9

# Purger selon TTL mais montrer ce qui se passerait
riptide prune --dry-run

# Diagnostics\Nriptide doctor --json | jq
```

---

## 17) Questions ouvertes

* Chemin par défaut du graveyard sur systèmes multi‑disques : par mountpoint d’origine ? (v1: unique)
* Checksums : par défaut `lazy` (calcule lors du temps faible) ou `off` ?
* ACLs/xattrs : module feature‑flag `acl` pour systèmes qui en ont besoin.

