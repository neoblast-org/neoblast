# Loaded before commit-object.awk and tag-object.awk. ssh_signature_ok(s)
# takes an SSH signature's base64, its lines joined, and is true only if it
# is laid out as git writes one with a plain ed25519 key, 173 bytes in all:
# SSHSIG version 1, the 32-byte key, namespace "git", an empty reserved
# field, sha512, and the 64-byte signature. A certificate's key carries its
# holder's name and principals, and the reserved field, which OpenSSH reads
# and then ignores, could carry any text (S-030). The key, the signature, and
# the few bits that share a base64 character with them stay free: only
# verifying the signature would tie them to the project (S-026).
function ssh_signature_ok(s) {
  return length(s) == 232 &&
    substr(s, 1, 44) == "U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg" &&
    substr(s, 89, 56) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA" &&
    s ~ /^[A-Za-z0-9+\/]+=$/
}
