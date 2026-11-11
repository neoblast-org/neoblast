# Setting up and running the spike (step 4)

This is the step-by-step for step 4 of `plans/todo/p2p-library-spike.md`:
one VPS, two computers and three networks, from an empty cloud account to
the records step 5 needs. `README.md` says what the code does; this file
says how to set it up and run it. It names no provider, address or
hostname, and no date of a run (plan, criterion 4); keep those in your own
notes.

## What you need

| Machine | Role | Network |
| --- | --- | --- |
| A VPS with a public IPv4 address | both relays, the mailbox, B in the two controls | the internet |
| A computer that stays on home 1 | A in `home1-vps`, `home1-cgnat` and `home1-home2` | home 1 |
| A laptop | A in `cgnat-vps`; B in `home1-cgnat` and `home1-home2` | the phone's hotspot, later home 2 |

Also a domain name you control, for `iroh-relay`'s certificate (1.4).
Section 1 is done from the home 1 computer; the laptop joins in section 2.

## 1. The VPS

### 1.1 Create it

Any VPS with a public IPv4 address works. A small VM with 1 vCPU, 6 GB of
memory and Ubuntu 24.04 is enough: the build and `iroh-relay`'s install
took about 20 minutes on one, and the relays use little. In the cloud's
console:

1. **Make the network first,** with a public subnet and an internet
   gateway. Some consoles cannot give a VM a public IPv4 address when the
   VM's own form makes the network.
2. **Open the cloud's firewall rules** to the whole internet
   (`0.0.0.0/0`):

   | Protocol | Ports | For |
   | --- | --- | --- |
   | TCP | 22 | ssh |
   | TCP | 80, 443 | `iroh-relay`: certificate challenge, captive-portal check, relaying |
   | TCP | 4001 | the libp2p relay over TCP |
   | UDP | 4001 | the libp2p relay over QUIC |
   | UDP | 7842 | `iroh-relay`'s QUIC address discovery |

3. **Make an ssh key just for this VPS,** on the home 1 computer. It has
   no passphrase, as `run.sh` uses ssh without prompting:

   ```sh
   ssh-keygen -t ed25519 -f ~/.ssh/p2p-spike-vps -N "" -C p2p-spike
   ```

4. **Create the VM:** Ubuntu 24.04, the network's public subnet, a public
   IPv4 address, and the contents of `~/.ssh/p2p-spike-vps.pub` as its ssh
   key. If the console says the size you chose is out of capacity, try
   another zone, or later.
5. **Log in once** with the VM's user (`ubuntu` on Ubuntu images), and
   answer `yes` to its host key:

   ```sh
   ssh -i ~/.ssh/p2p-spike-vps ubuntu@<VPS IPv4> uname -m
   ```

### 1.2 Its own firewall

Some images also block ports on the VM itself. List the rules with
`sudo iptables -L INPUT -n --line-numbers`. If they end in a `REJECT`,
insert the relays' ports at the `REJECT`'s number, so they come before it,
and save. With the `REJECT` at number 5:

```sh
sudo iptables -I INPUT 5 -p tcp -m state --state NEW -m multiport --dports 80,443,4001 -j ACCEPT
sudo iptables -I INPUT 6 -p udp -m multiport --dports 4001,7842 -j ACCEPT
sudo netfilter-persistent save
```

On an image that uses `ufw` instead, `sudo ufw allow` each port.

### 1.3 Tools and build

On the VPS:

```sh
sudo apt-get update && sudo apt-get install -y build-essential git tmux pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.97.0
. ~/.cargo/env
git clone https://github.com/neoblast-org/neoblast.git ~/neoblast
cd ~/neoblast/spike/p2p && cargo build --release
cargo install iroh-relay --version 1.4.1 --features server --locked
```

The install runs in `spike/p2p`, so it uses the same pinned toolchain as
the build.

### 1.4 The relay's name

`iroh-relay` gets a Let's Encrypt certificate, which needs a DNS name that
points at the VPS. Add an `A` record for a name in a domain you control,
pointing at the VPS's IPv4 address, and check it from **every** network you
will measure from:

```sh
dig +short <relay name> A      # must print the VPS's IPv4 address
```

If your DNS host offers to proxy the name, turn that off: the record must
be "DNS only". A proxy answers with its own addresses, presents its own
certificate and passes no UDP, so iroh's address discovery on UDP 7842
would never reach the relay, and iroh would reach its relay by another
route than libp2p does.

Free wildcard names such as `sslip.io` and `nip.io` are blocked by some
home and ISP web filters, which answer with a block page's address
instead. A peer on such a network cannot reach the iroh relay, so every
iroh attempt from it fails. A name in your own domain avoids this.

### 1.5 Start the relays

Run each relay in its own `tmux` session, so it keeps running after you log
out. Copy the iroh settings out of the repo, then fill in `hostname` (the
name from 1.4) and `contact` (an address for Let's Encrypt):

```sh
cp ~/neoblast/spike/p2p/iroh-relay.toml ~/iroh-relay.toml
nano ~/iroh-relay.toml

tmux new -s iroh
cd ~ && sudo ~/.cargo/bin/iroh-relay --config-path ~/iroh-relay.toml
```

`sudo` is for ports 80 and 443. Press Ctrl-b, then d, to leave it running.
The certificate is fetched on first start; then
`curl -sI https://<relay name>/ | head -1` from any computer answers.

```sh
tmux new -s libp2p
cd ~/neoblast/spike/p2p && target/release/spike-libp2p relay --port 4001 \
    --external-ip <VPS IPv4> --key ~/p2p-spike-libp2p-relay.key
```

It prints a QUIC and a TCP address; the QUIC one
(`/ip4/…/udp/4001/quic-v1/p2p/12D3KooW…`) goes into `relays.env`. The key
file keeps that address the same between evenings. Ctrl-b, d again.
`tmux attach -t iroh` or `-t libp2p` brings either back.

Check both from one of your computers once it is built (section 2); each
prints nothing and exits 0 when its relay answers:

```sh
spike/p2p/target/release/spike-iroh probe --relay https://<relay name>/; echo $?
spike/p2p/target/release/spike-libp2p probe --relay <the QUIC address>; echo $?
```

To see iroh's address discovery as well, put `RUST_LOG=iroh=debug` in
front of the iroh probe. When the relay answered on UDP 7842, iroh 1.4.1
logs a `net_report generated` line with `udp_v4: true`.

### 1.6 `relays.env`

Every machine, the VPS included, needs `spike/p2p/relays.env`, made from
`relays.env.example`:

```sh
LIBP2P_RELAY=/ip4/<VPS IPv4>/udp/4001/quic-v1/p2p/12D3KooW...
IROH_RELAY=https://<relay name>/
IROH_PORTMAP=on
```

It is git-ignored. `IROH_PORTMAP=on` is fixed for every run (plan, notes of
2026-11-11).

## 2. Each computer

On the home 1 computer and the laptop:

1. **Rust:** `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.
   The spike's `rust-toolchain.toml` fetches its own toolchain.
2. **Code and build:** clone the repository (or `git pull --ff-only` an
   existing clone), then `cd spike/p2p && cargo build --release`.
3. **`relays.env`,** as in 1.6.
4. **An ssh key for the laptop.** The home 1 computer already has one
   (1.1); do not make another there, as `ssh-keygen` would overwrite the
   only key the VPS accepts. On the laptop:

   ```sh
   ssh-keygen -t ed25519 -f ~/.ssh/p2p-spike-vps -N "" -C p2p-spike-laptop
   ```

   Copy the laptop's `~/.ssh/p2p-spike-vps.pub` to the home 1 computer and
   add it to the VPS from there:

   ```sh
   ssh -i ~/.ssh/p2p-spike-vps ubuntu@<VPS IPv4> 'cat >> ~/.ssh/authorized_keys' < <the laptop's .pub>
   ```

5. **ssh without a prompt.** On both computers, add to `~/.ssh/config`:

   ```
   Host spike-vps
       HostName <VPS IPv4>
       User ubuntu
       IdentityFile ~/.ssh/p2p-spike-vps
       IdentitiesOnly yes
       StrictHostKeyChecking accept-new
   ```

   `accept-new` accepts the VPS's host key the first time, which
   `run.sh`'s ssh would otherwise refuse. Then check:
   `ssh -o BatchMode=yes spike-vps true && echo ok` prints `ok` with no
   question. `spike-vps` is what `--mailbox` takes below.

The mailbox is a directory in the home directory of the user running
`run.sh`. On the VPS, run `run.sh` as the same user the other computers log
in as (`ubuntu` above), or the two ends use different directories and every
attempt fails with `a-not-ready`.

`caffeinate -i` in the commands below keeps a Mac awake; leave it out on
any other system.

## 3. A smoke test

About ten minutes; it checks both computers' ssh, the mailbox, both relays
and the certificate before any counted run. Commands run from the
repository's root; on the VPS that is `~/neoblast`.

**Home 1 to the VPS.** On the home 1 computer (A):

```sh
caffeinate -i bash spike/p2p/run.sh a --mailbox spike-vps
```

Then on the VPS (B):

```sh
cd ~/neoblast && bash spike/p2p/run.sh b --mailbox local --pair home1-vps --run 0 --first libp2p --attempts 1
```

Good output is one line per library, `direct` or `relayed`:

```
attempt 1 libp2p (try 1): direct
attempt 1 iroh (try 1): direct
```

**The laptop on the hotspot to the VPS.** Stop A on home 1 (Ctrl-c), start
the same A command on the laptop, then on the VPS:

```sh
bash spike/p2p/run.sh b --mailbox local --pair cgnat-vps --run 0 --first iroh --attempts 1
```

A `harness-error` names its reason; see the table at the end. Delete the
test records before the counted runs: `rm ~/neoblast/spike/p2p/records/run0-*`
on the VPS.

## 4. The runs

Before every run:

- **One A at a time.** Stop the previous A with Ctrl-c before starting A
  on another computer.
- **No VPN** on any machine.
- **The laptop on the hotspot uses only the hotspot:** Wi-Fi to home off,
  no cable.
- Allow incoming connections if macOS asks about `spike-libp2p` or
  `spike-iroh`.

Start A, then B, from the repository's root (`cd ~/neoblast` on the VPS).
A's command is always:

```sh
caffeinate -i bash spike/p2p/run.sh a --mailbox spike-vps
```

| # | A on | B on | B's command | About |
| --- | --- | --- | --- | --- |
| 1 | home 1 | the VPS | `bash spike/p2p/run.sh b --mailbox local --pair home1-vps --run 1 --first libp2p` | 1 to 1½ hours |
| 2 | the laptop, on the hotspot | the VPS | `bash spike/p2p/run.sh b --mailbox local --pair cgnat-vps --run 1 --first iroh` | 1 to 1½ hours |
| 3 | home 1 | the laptop, on the hotspot | `caffeinate -i bash spike/p2p/run.sh b --mailbox spike-vps --pair home1-cgnat --run 1 --first libp2p` | 2½ to 3 hours |
| 4 | home 1 | the laptop, on home 2 | `caffeinate -i bash spike/p2p/run.sh b --mailbox spike-vps --pair home1-home2 --run 1 --first iroh` | 2½ to 3 hours |
| 5 | as above | as above | the pair's B command with `--ip dual` added, only if both ends have IPv6 | 30 minutes each |

An attempt takes about 90 to 100 seconds: up to 30 to connect, 60 more
after the first echo, a 10-second pause and the relay checks. A control is
40 attempts (20 per library), a decisive pair 100, an IPv6 run 20. The IPv6
runs write their own `…-dual.jsonl` files. Stopping B and running the same
command again carries on from the last attempt recorded.

**After runs 1 and 2,** check the controls on the VPS:
`python3 -I spike/p2p/summary.py` prints a `control … connected, passes`
or `does not pass` line per control and library. Each library must connect
in at least 18 of 20 (rule 5). If one does not pass, stop and find the
cause in the harness or the relay before going on; a blocked port is
moved, never counted against the library. Then run that control again with
the same command and `--run 2`.

One evening can hold the setup and runs 1 and 2; runs 3 and 4 can each
have their own.

## 5. The records

B writes `spike/p2p/records/run<n>-<pair>-<ip>.jsonl` on its own machine:
runs 1 and 2 on the VPS, runs 3 and 4 on the laptop. Copy them into one
clone's `spike/p2p/records/`, for example from the VPS:

```sh
scp spike-vps:neoblast/spike/p2p/records/*.jsonl spike/p2p/records/
```

Then `python3 -I spike/p2p/summary.py` prints the rates and the outcome;
step 5 of the plan commits the records and writes decision 0013. The logs
in `spike/p2p/logs/` can hold addresses and are never committed.

## 6. Between evenings and afterwards

- **Stop both relays between evenings:** `tmux attach -t iroh`, Ctrl-c,
  and the same for `libp2p`. Anyone who finds them can use them while they
  run. Start them again as in 1.5; the libp2p key keeps its address.
- **Afterwards:** delete the VM, then its network, and the DNS record.
  Delete `~/.ssh/p2p-spike-vps*` and the `spike-vps` block in
  `~/.ssh/config` on each computer.

## Problems seen, and their fixes

| What you see | Why | Fix |
| --- | --- | --- |
| No public IPv4 option when creating the VM | the VM's form made the network | make the network first (1.1) |
| "Out of capacity" for the VM's size | the zone is full | another zone, or later |
| Ports closed although the cloud's firewall allows them | the image's own `iptables` rejects them | 1.2 |
| `dig` on a home network answers an address that is not the VPS's | a web filter blocks the name service | a name in your own domain (1.4) |
| `Host key verification failed`, or every attempt `harness-error (mailbox)` | ssh to the VPS fails, often an unknown host key | `StrictHostKeyChecking accept-new` (2), then `ssh -o BatchMode=yes spike-vps true` |
| `harness-error (relay-before)` or `(relay-after)` | a relay is down or unreachable from B | `tmux attach` to check both relays; check the ports |
| `harness-error (a-not-ready)` | A is not running, runs as another user, a second A is running, or A's network cannot reach a relay | one A, started first, as the ssh user (2); probe both relays from A's computer (1.5); A's log is `spike/p2p/logs/a-*.log` there |
