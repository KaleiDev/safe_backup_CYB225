# safe_backup_rust 

A safe, **pure-Rust** command-line tool for backing up, restoring, deleting and listing files.
No Makefile — use `cargo` for build/test/run.

## Features
- Copy files into a versioned backup directory with timestamp suffix
- Safe restore back to original path (guards against path traversal)
- Delete a specific backup by ID
- List backups for a file with checksums
- Uses temp files + atomic rename to avoid partial writes
- Cross-platform (Linux/macOS/Windows)

## Usage
```bash
# build
cargo build --release

# help
cargo run -- --help

# create a backup (stores into ./backups by default)
cargo run -- backup tests/fixtures/data_test/data.txt

# list backups for a file
cargo run -- list tests/fixtures/data_test/data.txt

# restore the latest backup
cargo run -- restore tests/fixtures/data_test/data.txt

# delete a backup by printed ID (from list)
cargo run -- delete <BACKUP_ID>
```

## Folder layout
```
safe_backup_rust/
  Cargo.toml
  README.md
  .gitignore
  backups/                 # created at runtime
  src/
    main.rs
    ops.rs
    fsx.rs
  tests/
    cli_smoke.rs
    fixtures/
      data_test/
        data.txt
        sample.txt
```

## Notes
- By default, backups are stored under `./backups` within the current working directory.
- You can override the backup directory with `--backup-dir <DIR>` in any command.
