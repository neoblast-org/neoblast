# Loaded before commit-object.awk and tag-object.awk. ssh_signature_ok(s)
# takes an SSH signature's base64, its lines joined, and is true only if it
# is laid out as git writes one with a plain ed25519 key, 173 bytes in all:
# SSHSIG version 1, the 32-byte key, namespace "git", an empty reserved
# field, sha512, and the 64-byte signature. A certificate's key carries its
# holder's name and principals, and the reserved field, which OpenSSH reads
# and then ignores, could carry any text (S-030). Where a base64 character
# holds bits of the key or the signature and of a fixed byte, the fixed bits
# are checked too: characters 87 and 88 hold the namespace length's high
# byte, 145 and 146 the signature length's low byte (0x40), and 231 the
# padding bits after the last byte (S-034). Only the key and the signature
# stay free: only verifying the signature would tie them to the project
# (S-026).
function ssh_signature_ok(s) {
  return length(s) == 232 &&
    substr(s, 1, 44) == "U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg" &&
    index("AEIMQUYcgkosw048", substr(s, 87, 1)) && substr(s, 88, 1) == "A" &&
    substr(s, 89, 56) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA" &&
    substr(s, 145, 1) == "Q" && index("ABCDEFGHIJKLMNOP", substr(s, 146, 1)) &&
    index("AEIMQUYcgkosw048", substr(s, 231, 1)) &&
    s ~ /^[A-Za-z0-9+\/]+=$/
}
