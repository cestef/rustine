# rustine

binary diff tool. shipping 500mb when you only broke 3kb is a cry for help.

## usage

```bash
# generate patch
rustine generate old.bin new.bin -o patch.bin

# generate with checksums + bidirectional
rustine generate old.bin new.bin -o patch.bin --checksum -r

# apply forward
rustine apply old.bin patch.bin -o new.bin

# apply reverse (requires -r when generating)
rustine apply new.bin patch.bin -o old.bin --reverse

# inspect
rustine inspect patch.bin -v
```

## features

- bsdiff compression
- checksums (`--checksum`) + verification (`--verify`)
- bidirectional patches (`-r` / `--reverse`)
- streaming for large files (>100mb)
- reads raw BSDIFF4 patches

---

named after the french word for bicycle tire patches. wrapper around [`qbsdiff`](https://lib.rs/qbsdiff).
