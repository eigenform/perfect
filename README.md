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

See the [`./perfect-zen2`](./perfect-zen2/src/bin/) crate for the entire list
of experiments. 

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


## Environment Configuration

> **NOTE**: Users can also use [`./config.sh`](./config.sh) and the provided 
> [scripts](./scripts/) to toggle certain features. In the near future, these
> scripts will be removed, and users will be expected to use the `perfect-env`
> binary. 

Users are expected to use the `perfect-env` binary in order to configure 
certain features on the target machine during runtime before experiments. 
Toggling these features requires root permissions on the target machine. 
See the `--help` flag for more details. 

```
# Build the `perfect-env` binary
$ cargo build --release --bin perfect-env
...

$ sudo ./target/release/perfect-env --help
...
```

In general, most experiments expect the following runtime configuration:

- Use of the `RDPMC` instruction is allowed in userspace
- The `vm.mmap_min_addr` `sysctl` knob is set to zero
- Simultaneous Multi-threading (SMT) is disabled
- The `cpufreq` governor is configured to disable frequency scaling

See documentation in the source for more details about which settings might 
be required/optional for a particular experiment.

Most [if not all] experiments also assume that a particular CPU core is 
isolated from interrupts and other tasks scheduled by the kernel. 
This requires the following kernel command-line options (where `N` is the core 
you expect to be running experiments on):

```
isolcpus=nohz,domain,managed_irq,N nohz_full=N
```

> **WARNING:**
>
> Under normal circumstances (*without* `isolcpus`), the Linux watchdog timer
> relies on counter #0 being configured automatically by the `perf` subsystem
> to count CPU cycles.
>
> Our use of the `perf-event` crate only ever configures the first available 
> counter. This means that, when `isolcpus` is *not* used, correct use of 
> `RDPMC` in measured code must read from counter #1 instead of counter #0. 
> Otherwise, attempted uses of `RDPMC` will read the CPU cycle counter instead
> of the desired PMC event. 
>
> You're expected to keep this in mind while writing/running experiments. 
> Currently, all experiments assume the use of `isolcpus`, and `RDPMC` is 
> always used with counter #0. 


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

3. The default configuration (Zen 2) pins the current process to core #15.
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
$ sudo ./target/release/perfect-env smt off
$ sudo ./target/release/perfect-env rdpmc on
$ sudo ./target/release/perfect-env boost off
$ sudo ./target/release/perfect-env mmap-min-addr 0

# Run an experiment
$ cargo run -r -p perfect-zen2 --bin <experiment>
...

```

