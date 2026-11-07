# 0010: Open and heal check the whole codeword

- Status: accepted
- Date: 2026-11-02
- Settles: review finding S-002

## Context and problem statement

The file root commits to every shard's hash, but not to the shards being one
Reed-Solomon codeword. The publisher holds the read key, so they can seal two
files of one length under it and list the data shards of one with the parity
shards of the other. Each shard then matches the manifest, and decoding from
any k of them authenticates: before this decision, the data shards opened the
first file and the parity shards alone the second, from the same link. With
n − k ≥ k, as at the default 20-of-50, a reader whose data shards were
censored would read other content than everyone else, and nothing would warn
either of them.

A link should pin one content: any k shards that match the manifest open the
same bytes, or none do. The maintainer chose to enforce that in code and to
state it in the spec, over stating only that a link pins shard bytes.

## Method

The rule needs no shard beyond the k a reader already holds: rebuild the
segment's n shards from those k and compare each with its listed hash. If
they all match, the manifest lists one codeword, and by the hash's collision
resistance every k shards that match the manifest decode to it. If one does
not, no reader can trust what this segment decodes to.

Its cost was measured with the release build on the reference Mac (Apple M5,
macOS, arm64), on a random 256 MiB file sealed with the defaults (20-of-50,
1 MiB segments), against the code before the change, with `/usr/bin/time -l`.
The decision was taken before the cost was known, and the only limit set in
advance is decision 0009's: at most 32 MiB of working memory per operation.

## Findings

| Operation | Before | After |
| --- | --- | --- |
| open, every shard present | 0.59 s, 5.5 MB peak | 0.83 s, 19.4 MB peak |
| open from the 30 parity shards | 1.0 s, 21.5 MB | 1.3 s, 22.5 to 24.9 MB |
| heal, nothing missing | 0.60 s, 5.0 MB | 0.89 s, 18.5 to 25.9 MB |
| heal, 30 shards lost per segment | 4.3 to 4.9 s, 26 to 27 MB | 4.3 to 5.1 s, 27 to 29 MB |

1. An intact open or heal now runs the codec, which it skipped before, so it
   pays the codec's fixed cost: the same operation on a 1 KB file peaks at
   11.4 MB. Working memory, as decision 0009 measures it, is about 8 MB for
   open and 7 to 15 MB for heal, within the 32 MiB rule.
2. An intact open takes about 40% longer and an intact heal about 50%: one
   parity encode and n − k hashes per segment. From other shards it is one
   decode and encode, which decoding already needed, and up to n − k
   hashes.
3. Where shards are lost, decoding already dominated, and the check adds
   little.

## Decision outcome

**`open` and `heal` rebuild every segment's n shards from the k verified
ones they hold and check each against the manifest.** `open` refuses the
file at the first segment that fails, with `Error::RebuildMismatch`. `heal`
checks every segment, even one with nothing missing, still repairs what
matches, and reports the first such segment. `spec/header.md` makes the rule
a MUST for openers and healers, and `spec/threat-model.md` states what it
gives.

No byte changes: files that `seal` makes are one codeword by construction,
and the v1 vectors are unchanged.

### Consequences

- Good: a link pins one content against its own publisher, from any k
  shards, with no extra fetching; a storage node can find such a file with
  the verify link alone.
- Good: heal reports an inconsistent segment whichever k shards it holds,
  where before it reported one only if a rebuilt shard happened to differ
  (S-002's note).
- Bad: an intact open is about 40% slower and an intact heal about 50%, and
  both now pay the codec's fixed memory.
- Bad: a file whose manifest lists a shard of the wrong length cannot be
  opened at all; before, that shard counted as missing (S-019).
- Bad, found by the next security review (S-037, 2026-11-03): the check
  holds a segment's n shards at once, which a publisher can make about
  4 GiB (2-of-128 with 64 MiB segments). At the defaults it is 2.6 MB. An
  intact open or heal used to hold at most k shards; decoding from parity
  already held n. Checking the rebuilt shards one at a time would not
  remove it, as the codec works on as many again; decision 0011 bounds it
  with a memory budget in the CLI instead, after v0.0.3.

## Considered options

- **State the limit in the spec only**: a link pins shard bytes, not one
  plaintext against its publisher. Free, but it leaves the attack open to
  anyone who publishes, and a reader could not tell.
- **Check only in heal**, which runs on storage nodes. Cheaper for readers,
  but it protects a reader only if some node has healed the file, and a
  reader with only parity shards would still be misled.
- **Check only the shards a reader holds beyond k.** Needs more shards than
  k, which a censored reader may not have.
