# perfect

An x86 JIT playground for writing microbenchmarks and other experiments for 
understanding microarchitectural implementation details. 

The crates in this repository rely heavily on 
[CensoredUsername/dynasm-rs](https://github.com/CensoredUsername/dynasm-rs) 
for generating code during runtime, and you will probably want to read 
[the `dynasm-rs` documentation](https://censoredusername.github.io/dynasm-rs/language/index.html) if
you intend on writing your own experiments.
Unlike [eigenform/lamina](https://github.com/eigenform/lamina), this 
relies on the "raw events" exposed via the Linux `perf` API in userspace. 

```
config.sh        - Wrapper for invoking setup scripts
perfect/         - Main library crate
perfect-zen2/    - Zen2 experiments
perfect-tremont/ - Tremont experiments
scripts/         - Miscellaneous scripts
```

## Experiments

All of the experiments here are small programs used to demonstrate, observe, 
and document different microarchitectural implementation details. 
This includes things like: 

- Measuring the sizes of hardware buffers
- Demonstrating the behavior of certain hardware optimizations 
- Demonstrating problems in microarchitectural security

In general, all of the experiments are implemented by: 

1. Emitting chunks of code with a run-time assembler
2. Running emitted code after configuring performance-monitoring counters
3. Printing some results to `stdout`

These experiments are meant to serve as a kind of *executable documentation*
for certain things. This will not be very useful to you unless you're planning
on reading and understanding the code! 

Note that most of the interesting experiments here are probably only relevant
for the Zen 2 microarchitecture (and potentially previous/later Zen iterations,
depending on the particular experiment). These are *not* intended to be 
portable to different platforms since they necessarily take advantage of 
implementation details specific to the microarchitecture. 

See the [`./perfect-zen2`](./perfect-zen2/src/bin/) crate for the entire list of 
experiments. 

### Optimizations

- [Memory Renaming Eligibility](./perfect-zen2/src/bin/memfile.rs)
- [Store-to-Load Forwarding Eligibility](./perfect-zen2/src/bin/stlf.rs)
- [Move Elimination and Zero Idioms](./perfect-zen2/src/bin/rename.rs)

### Resources

- [Integer PRF Capacity](./perfect-zen2/src/bin/int.rs)
- [FP/Vector PRF Capacity](./perfect-zen2/src/bin/fp.rs)
- [Store Queue Capacity](./perfect-zen2/src/bin/stq.rs)
- [Load Queue Capacity](./perfect-zen2/src/bin/ldq.rs)
- [Reorder Buffer Capacity](./perfect-zen2/src/bin/rob.rs)
- [Taken Branch Buffer Capacity](./perfect-zen2/src/bin/tbb.rs)
- [Dispatch Behavior](./perfect-zen2/src/bin/dispatch.rs)

### Predictors

- [Branch Direction Prediction](./perfect-zen2/src/bin/bp.rs)
- [Branch Target Prediction](./perfect-zen2/src/bin/btb.rs)
- [Direction Predictor Stimulus/Response](./perfect-zen2/src/bin/bp-pattern.rs)
- [L1D Way Prediction](./perfect-zen2/src/bin/dcache.rs)

### Security

- [Observing CVE-2023-20593 (Zenbleed)](./perfect-zen2/src/bin/zenbleed.rs)
- [Observing Speculative Loads with Timing](./perfect-zen2/src/bin/flush-reload.rs)
- [Observing CVE-2021-26318/AMD-SB-1017 (PREFETCH Behavior Across Privilege Domains)](./perfect-zen2/src/bin/prefetch.rs)
- [Observing CVE-2022-4543 (EntryBleed)](./perfect-zen2/src/bin/entrybleed.rs)

### Miscellania

- [Validating/Discovering PMC Events](./perfect-zen2/src/bin/pmc.rs)
- [Speculatively Fuzzing x86 Instructions](./perfect-zen2/src/bin/specdec.rs)


## Run-time Configuration

There are a bunch of scripts (see [`/.scripts/`](./scripts/)) here that you're 
expected to use to configure Linux before running any experiments. 

- Most experiments rely on the `RDPMC` instruction
- Most experiments rely on `vm.mmap_min_addr` being set to zero
- Most experiments are intended to be used with SMT disabled
- The `cpufreq` governor should be configured to disable frequency scaling

If you don't want to run them individually, you can just use 
[`./config.sh`](./config.sh) (as root) to enable/disable all of them at once.
Since your system is probably different from mine, you may want to read this
before using it. 

Otherwise, see documentation in the source for more details about which 
settings might be required/optional for a particular experiment.

## Boot-time Configuration

Many experiments here are intended to be used while booting Linux with the 
following kernel command-line options (where `N` is the core you expect to be 
running experiments on):

```
isolcpus=nohz,domain,managed_irq,N nohz_full=N
```

This should [mostly] prevent interrupts, and [mostly] prevent Linux from 
scheduling tasks on the core.

You can also use the `perfect-env` binary to check/validate that the settings
on your machine are correct:

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

> **WARNING:**
>
> Under normal circumstances (*without* `isolcpus`), the Linux watchdog timer
> relies on counter #0 being configured automatically by the `perf` subsystem. 
>
> Our use of the `perf-event` crate only ever configures the first available 
> counter. This means that uses of `RDPMC` in measured code must read from 
> counter #1. However, while using an isolated core, the watchdog timer is not 
> configured, and measured code *must* use `RDPMC` to read from counter #0 
> instead.
>
> You're expected to keep this in mind while writing experiments. 
> Currently, all experiments assume the use of `isolcpus`.


## Harness Configuration

The "harness" is a trampoline [emitted during runtime] that jumps into other 
code emitted during runtime. In most experiments, this is used to collect 
measurements with the `RDPMC` instruction and manage all of the state 
associated with running experiments. 

A few important details: 

1. The default configuration tries allocate the low 256MiB of virtual 
   memory (from `0x0000_0000_0000_0000` to `0x0000_0000_1000_0000`). This is 
   used to simplify some things by allowing us to emit loads and stores with 
   simple immediate addressing. If the `vm.mmap_min_addr` sysctl knob isn't 
   set to zero, this will cause you to panic when emitting the harness.

2. The default configuration tries to allocate 64MiB at virtual address 
   `0x0000_1337_0000_0000` for emitting the harness itself.

3. The default configuration pins the current process to core #15.
   This reflects my own setup (on 16-core the Ryzen 3950X), and you may want 
   to change this to something suitable for your own setup, ie.
   ```rust
   use perfect::*;
   fn main() {
       let harness = HarnessConfig::default_zen2()
           .pinned_core(3)
           .emit();
       ...
   }
   ```

See [`./perfect/src/harness.rs`](./perfect/src/harness.rs) for more details. 

## Running Experiments

Typical usage looks something like this: 

``` 
# Disable SMT, enable RDPMC, disable frequency scaling, enable low mmap() 
$ sudo ./config.sh on

# Run an experiment
$ cargo run -r -p perfect-zen2 --bin <experiment>
...

```

