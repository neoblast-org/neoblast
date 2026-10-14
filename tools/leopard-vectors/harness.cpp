// Prints Leopard-RS recovery shards for deterministic input, one hex line per shard.
// Usage: harness <original_count> <recovery_count> <shard_bytes>
// Input: one splitmix64 stream seeded with k*1000003 + m*1009 + shard_bytes; the low
// byte of each output fills the original shards in order. Built by generate.sh.
#include "leopard.h"
#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <vector>
static uint64_t s;
static uint64_t next() { uint64_t z = (s += 0x9E3779B97F4A7C15ULL); z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL; z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL; return z ^ (z >> 31); }
int main(int argc, char** argv) {
  if (argc != 4) { fprintf(stderr, "usage: harness <original_count> <recovery_count> <shard_bytes>\n"); return 1; }
  unsigned k = atoi(argv[1]), m = atoi(argv[2]); uint64_t bytes = atoll(argv[3]);
  s = (uint64_t)k * 1000003ULL + m * 1009ULL + bytes;
  if (leo_init()) { fprintf(stderr, "init failed\n"); return 2; }
  std::vector<std::vector<uint8_t>> orig(k, std::vector<uint8_t>(bytes));
  for (auto& sh : orig) for (auto& b : sh) b = (uint8_t)next();
  std::vector<const void*> optr(k); for (unsigned i = 0; i < k; i++) optr[i] = orig[i].data();
  unsigned wc = leo_encode_work_count(k, m);
  if (!wc) { fprintf(stderr, "unsupported k=%u m=%u\n", k, m); return 3; }
  std::vector<std::vector<uint8_t>> work(wc, std::vector<uint8_t>(bytes));
  std::vector<void*> wptr(wc); for (unsigned i = 0; i < wc; i++) wptr[i] = work[i].data();
  LeopardResult r = leo_encode(bytes, k, m, wc, optr.data(), wptr.data());
  if (r != Leopard_Success) { fprintf(stderr, "encode error %d\n", (int)r); return 4; }
  for (unsigned i = 0; i < m; i++) { for (uint64_t j = 0; j < bytes; j++) printf("%02x", work[i][j]); printf("\n"); }
  return 0;
}
