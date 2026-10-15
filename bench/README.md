# Benchmarks

## Segment size (ROADMAP open item 2)

```sh
bench/segment-size.sh                 # 1 GiB random file; 1, 4, 8 and 16 MiB segments
bench/segment-size.sh 256 1 4         # quicker: 256 MiB file, two sizes
```

For each segment size the script seals a random file at 20-of-50, then times
four operations and records their peak memory (maximum resident set size, from
`/usr/bin/time`):

| Operation | What it does |
| --- | --- |
| `seal` | encrypt, erasure-code and write every shard |
| `open` | read the file back with every shard present (data shards only, no decoding) |
| `heal` | rebuild after losing n − k = 30 shards in every segment |
| `open_from_parity` | read the file back after losing all 20 data shards (worst case) |

Each operation also runs on a 1 KB file. That fixed cost is about 3 MiB for
the program, plus about 8 MiB for the erasure library's multiplication table
whenever it encodes or decodes. **Working** memory is the peak minus that
fixed cost: the part that grows with the segment size. The script checks that
both opens return the original bytes.

The script writes one JSON record to `bench/results/`: times, peak and working
memory per operation, commit, file size, and OS, architecture and CPU model. It
holds no host name, user name or path. It needs about 4.6× the file size in
free disk space while it runs (the input, the shards, and one recovered copy);
everything is deleted afterwards, including after Ctrl-C.

## Heal memory check

```sh
bench/heal-check.sh                   # 256 and 1024 MiB files; 1 and 4 MiB segments
```

Seals a random file, deletes n − k = 30 shards in every segment, heals, and
prints peak memory (and, on macOS, peak memory footprint) for each file and
segment size. Heal's working memory must not grow with the file size. Prints
only; it writes no record.

## Reference machine

The reference machine for decisions is the maintainer's Apple Silicon Mac.
Commit its record together with the decision it supports.
