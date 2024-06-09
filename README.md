# perfect

JIT playground for microbenchmarking, one-off experiments, and other half-baked
ideas. Unlike [eigenform/lamina](https://github.com/eigenform/lamina), this 
relies on the "raw events" exposed via the Linux `perf` API.

All of this relies heavily on [CensoredUsername/dynasm-rs](https://github.com/CensoredUsername/dynasm-rs)
for generating code during runtime, and you will probably want to read
[the `dynasm-rs` documentation](https://censoredusername.github.io/dynasm-rs/language/index.html).


```
perfect/         - Main library crate
perfect-zen2/    - Zen2 experiments
perfect-tremont/ - Tremont experiments
scripts/         - Miscellaneous scripts
```

## Environment

There are a bunch of scripts that you're expected to use to configure your 
environment before running any experiments:

- Most [if not all] experiments rely on the `RDPMC` instruction, which you'll 
  need to enable with [./scripts/rdpmc.sh](./scripts/rdpmc.sh)

- Most [if not all] experiments are intended to be used with SMT disabled, see
  [./scripts/smt.sh](./scripts/smt.sh)

- Most [if not all]  experiments rely on `vm.mmap_min_addr` being set to zero,
  see [./scripts/low-mmap.sh](./scripts/low-mmap.sh)

- [./scripts/freq.sh](./scripts/freq.sh) will disable `cpufreq` boost and 
  change the governor; you probably want to change this for your setup


You can also use the `perfect-env` binary to check/validate some of this: 

```
$ cargo build --release --bin perfect-env
...

$ sudo ./target/release/perfect-env
[*] 'perfect' environment summary:
  online cores                            : 32
  isolated cores                          : disabled
  nohz_full cores                         : disabled
  simultaneous multithreading (SMT)       : enabled [!!]
  cpufreq boost                           : enabled [!!]
  userspace rdpmc                         : disabled [!!]
  vm.mmap_min_addr                        : 65536
```

In some situations, it may also be advisable to use the following kernel 
command-line options while using this library (where `N` is the core you 
expect to be running experiments on):

```
isolcpus=nohz,domain,managed_irq,N nohz_full=N
```

This should [mostly] prevent interrupts, and [mostly] prevent Linux from 
scheduling tasks on the core.

## Harness Configuration

The default [`HarnessConfig`](./perfect/src/harness.rs) tries to `mmap()` the 
low 256MiB of virtual memory (from `0x0000_0000_0000_0000` to `0x0000_0000_1000_0000`). 
This is used to simplify some things by allowing us to emit loads and stores 
with simple immediate addressing. 
If the `vm.mmap_min_addr` sysctl knob isn't set to zero, this will cause you
to panic when emitting the harness.

The default configuration also pins the current process to core #15. 
You may want to change this to something suitable for your own setup, ie.

```rust
use perfect::*;

fn main() {
    let harness = HarnessConfig::default_zen2()
        .pinned_core(3)
        .emit();
    ...
}
```


## Running Experiments

Typical usage looks something like this: 

``` 
$ sudo ./scripts/smt.sh off
$ sudo ./scripts/rdpmc.sh on
$ sudo ./scripts/freq.sh on
$ sudo ./scripts/low-mmap.sh on

# Run an experiment
$ cargo run --release -p perfect-zen2 --bin <experiment>
...

```

