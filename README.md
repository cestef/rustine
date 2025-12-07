# rustine

binary diff tool. named after the french word for a temporary fix that becomes permanent.

## what

creates patches between binary files. applies them too, if you're feeling optimistic.

## why

shipping 500mb when you only broke 3kb is a cry for help.

## usage

```bash
# make a patch
rustine generate working.bin broken.bin -o mistake.patch

# make a patch with checksums (trust issues edition)
rustine generate working.bin broken.bin -o mistake.patch --checksum

# make a bidirectional patch (go both ways)
rustine generate working.bin broken.bin -o bidir.patch --checksum -r

# apply a patch
rustine apply working.bin mistake.patch -o broken.bin

# apply a patch in reverse (undo your mistakes)
rustine apply broken.bin bidir.patch -o working.bin --reverse

# apply with verification (because you've been hurt before)
rustine apply working.bin mistake.patch -o broken.bin --verify

# see what you're about to break before you break it
rustine apply working.bin mistake.patch --dry-run -v

# inspect a patch (trust but verify)
rustine inspect mistake.patch -v
```

## features

- binary diffing with [`bsdiff(1)`](https://linux.die.net/man/1/bsdiff)
- compression, because misery loves company
- checksum verification (for when your download manager lies)
- patch inspection (look before you leap)
- bidirectional patches (apply forward or reverse with `--reverse/-R`)
- byte-level preview of what you're about to wreck
- extensible binary format (RUSTINE2) with backward compatibility
- streaming mode for large files (>100mb), because ram is expensive
- probably won't corrupt your files (no promises)

## etymology

"rustine" is french for those rubber patches you put on bicycle tires. you know, the ones that never hold but you keep riding anyway because buying a new tube means admitting defeat.

## license

do whatever you want

## notes

this is basically just a nice wrapper around the rust port of `bsdiff(1)`: [`qbsdiff`](https://lib.rs/qbsdiff)
