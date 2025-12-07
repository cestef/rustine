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

# apply a patch
rustine apply working.bin mistake.patch -o broken.bin

# apply with verification (because you've been hurt before)
rustine apply working.bin mistake.patch -o broken.bin --verify

# see what you're about to break before you break it
rustine apply working.bin mistake.patch --dry-run -v

# inspect a patch (trust but verify)
rustine inspect mistake.patch -v

# create a reverse patch (for when you inevitably regret this)
rustine reverse working.bin broken.bin -o undo.patch

# the circle of life
```

## features

- binary diffing with [`bsdiff(1)`](https://linux.die.net/man/1/bsdiff)
- compression, because misery loves company
- checksum verification (for when your download manager lies)
- patch inspection (look before you leap)
- reverse patches (ctrl+z for adults)
- byte-level preview of what you're about to wreck
- streaming mode for large files (>100mb), because ram is expensive
- probably won't corrupt your files (no promises)

## etymology

"rustine" is french for those rubber patches you put on bicycle tires. you know, the ones that never hold but you keep riding anyway because buying a new tube means admitting defeat.

## license

do whatever you want

## notes

this is basically just a nice wrapper around the rust port of `bsdiff(1)`: [`qbsdiff`](https://lib.rs/qbsdiff)
