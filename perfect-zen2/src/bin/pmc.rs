use std::collections::*;
use rand::prelude::*;
use rand::distributions::{Distribution, Standard};
use clap::Parser;
use clap::ValueEnum;

use itertools::*;
use perfect::stats::*;
use perfect::*;
use perfect::events::*;
use perfect::asm::*;

use perfect::experiments::pmcdisc::*;

/// Perform a predefined set of microbenchmarks. 
///
/// WARNING: This is not exactly user-friendly. Maybe some day...
///
#[derive(Parser)]
#[command(verbatim_doc_comment)]
pub struct Args { 

    /// A comma-separated list of test groups to run.
    #[arg(long, value_enum, value_delimiter = ',')]
    groups: Vec<TestGroupId>,

    /// A set of events to measure. 
    #[arg(long, value_enum)]
    event_set: Option<PmcHammerEventSet>,

    /// Run all test groups
    #[arg(short, long)]
    all_groups: bool,

    /// Target CPU core (#15 by default)
    #[arg(short, long, default_value = "15")]
    core: Option<usize>,
}

/// Predefined sets of events
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PmcHammerEventSet {
    /// Retired instructions/micro-ops
    RetiredOps,

    /// Speculatively-dispatched micro-ops
    DispatchedOps,

    /// Branch prediction events
    BranchPrediction,

    /// Microcode events
    Microcode,

    /// Stall cycles
    Stall,

    /// Unknown or undefined events
    Unknown,

}
impl PmcHammerEventSet {
    pub fn set(&self) -> EventSet<Zen2Event> {
        match self { 
            Self::Stall => EventSet::new_from_slice(&[
                Zen2Event::DsTokStall3(DsTokStall3Mask::Cop1Disp),
                Zen2Event::DsTokStall3(DsTokStall3Mask::Cop2Disp),
                Zen2Event::DsTokStall3(DsTokStall3Mask::Cop3Disp),
                Zen2Event::DsTokStall3(DsTokStall3Mask::Cop4Disp),
                Zen2Event::DsTokStall3(DsTokStall3Mask::Cop5Disp),

                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::ALUTokenStall
                ),
                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::ALSQ1RsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::ALSQ2RsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::ALSQ3_0_TokenStall
                ),
                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::AGSQTokenStall
                ),
                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::RetireTokenStall
                ),
                Zen2Event::DeDisDispatchTokenStalls0(
                    DeDisDispatchTokenStalls0Mask::ScAguDispatchStall
                ),

                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::IntPhyRegFileRsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::LoadQueueRsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::StoreQueueRsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::IntSchedulerMiscRsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::TakenBrnchBufferRsrc
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::FpSchRsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::FpRegFileRsrcStall
                ),
                Zen2Event::DeDisDispatchTokenStalls1(
                    DeDisDispatchTokenStalls1Mask::FpMiscRsrcStall
                ),
            ]),

            Self::BranchPrediction => EventSet::new_from_slice(&[
                Zen2Event::ExRetBrn(0x00),
                Zen2Event::ExRetBrnMisp(0x00),
                Zen2Event::ExRetBrnTaken(0x00),
                Zen2Event::ExRetBrnTakenMisp(0x00),
                Zen2Event::BpRedirect(BpRedirectMask::Unk(0x00)),
                Zen2Event::BpDeReDirect(0x00),
            ]),
            Self::Microcode => EventSet::new_from_slice(&[
                Zen2Event::ExRetUcodeInst(0x00),
                Zen2Event::ExRetUcodeOps(0x00),
                Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Microcode),
                Zen2Event::DeMsStall(DeMsStallMask::Serialize),
                Zen2Event::DeMsStall(DeMsStallMask::WaitForQuiet),
                Zen2Event::DeMsStall(DeMsStallMask::WaitForSegId),
                Zen2Event::DeMsStall(DeMsStallMask::WaitForStQ),
                Zen2Event::DeMsStall(DeMsStallMask::WaitForCount),
            ]),
            Self::RetiredOps => EventSet::new_from_slice(&[
                Zen2Event::ExRetCops(0x00),
                Zen2Event::ExRetInstr(0x00),
            ]),
            Self::DispatchedOps => EventSet::new_from_slice(&[
                Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Fp),
                Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Int),
                Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Microcode),
            ]),

            Self::Unknown => {
                let mut set = EventSet::new();
                //set.add_unknown(0x06);
                //set.add_unknown(0x0f);

                set.add_unknown(0x20);
                set.add_unknown(0x21);
                set.add_unknown(0x22);
                set.add_unknown(0x23);
                set.add_unknown(0x28);
                set.add_unknown(0x2a);
                set.add_unknown(0x2b);
                set.add_unknown(0x2c);
                set.add_unknown(0x2e);

                set.add_unknown(0x30);
                set.add_unknown(0x31);
                set.add_unknown(0x33);
                set.add_unknown(0x34);
                set.add_unknown(0x38);
                set.add_unknown(0x39);
                set.add_unknown(0x3a);
                set.add_unknown(0x3b);
                set.add_unknown(0x3c);
                set.add_unknown(0x3d);
                set.add_unknown(0x3e);
                set.add_unknown(0x3f);

                set.add_unknown(0x42);
                set.add_unknown(0x46);
                set.add_unknown(0x48);
                set.add_unknown(0x4d);
                set.add_unknown(0x4e);
                set.add_unknown_nomask(0x4f);

                set.add_unknown(0x53);
                set.add_unknown(0x54);
                set.add_unknown(0x55);
                set.add_unknown_nomask(0x56);
                set.add_unknown(0x5c);
                set.add_unknown(0x5d);
                set.add_unknown(0x5e);

                set.add_unknown(0x65);
                set.add_unknown(0x67);
                set.add_unknown(0x68);
                set.add_unknown(0x69);
                set.add_unknown(0x6a);
                set.add_unknown(0x6b);
                set.add_unknown(0x6e);

                set.add_unknown(0x73);
                set.add_unknown(0x74);
                set.add_unknown(0x75);
                set.add_unknown(0x7b);
                set.add_unknown(0x7d);
                set.add_unknown(0x7e);
                set.add_unknown(0x7f);

                set.add_unknown(0x86);
                set.add_unknown_nomask(0x88);
                set.add_unknown(0x89);

                set.add_unknown_nomask(0x90);
                set.add_unknown(0x92);
                set.add_unknown(0x93);
                set.add_unknown(0x95);
                set.add_unknown(0x96);
                set.add_unknown(0x97);
                set.add_unknown(0x98);
                set.add_unknown(0x99);
                set.add_unknown(0x9a);
                set.add_unknown_nomask(0x9b);
                set.add_unknown_nomask(0x9c);
                set.add_unknown_nomask(0x9d);
                set.add_unknown_nomask(0x9e);
                set.add_unknown_nomask(0x9f);

                set.add_unknown(0xa0);
                set.add_unknown(0xa1);
                set.add_unknown_nomask(0xa2);
                set.add_unknown_nomask(0xa3);
                set.add_unknown_nomask(0xa4);
                set.add_unknown(0xa5);
                set.add_unknown(0xa6);
                set.add_unknown(0xa7);
                set.add_unknown_nomask(0xac);
                set.add_unknown_nomask(0xad);

                set.add_unknown(0xb0);
                set.add_unknown(0xb9);
                set.add_unknown(0xba);
                set.add_unknown(0xbb);
                set.add_unknown_nomask(0xbc);
                set.add_unknown_nomask(0xbd);
                set.add_unknown(0xbf);

                set.add_unknown(0xcc);
                set.add_unknown(0xcd);
                set.add_unknown(0xce);
                set.add_unknown(0xcf);

                set.add_unknown_nomask(0xd0);
                set.add_unknown_nomask(0xd5);
                set.add_unknown(0xd6);
                set.add_unknown(0xd7);
                set.add_unknown(0xd8);
                set.add_unknown(0xd9);
                set.add_unknown(0xda);
                set.add_unknown(0xdb);
                set.add_unknown(0xdc);
                set.add_unknown(0xdd);
                set.add_unknown(0xde);
                set.add_unknown(0xdf);



                set.add_unknown(0x180);
                set.add_unknown(0x181);
                set.add_unknown(0x182);
                set.add_unknown(0x183);
                set.add_unknown(0x184);
                set.add_unknown(0x185);
                set.add_unknown(0x186);
                set.add_unknown(0x187);
                set.add_unknown(0x188);
                set.add_unknown(0x189);
                set.add_unknown(0x18a);
                set.add_unknown(0x18b);
                set.add_unknown(0x18c);
                set.add_unknown(0x18d);
                set.add_unknown(0x18e);
                set.add_unknown(0x18f);

                set.add_unknown(0x1a0);
                set.add_unknown(0x1a1);
                set.add_unknown(0x1a2);
                set.add_unknown(0x1a3);
                set.add_unknown(0x1a4);
                set.add_unknown(0x1a5);
                set.add_unknown(0x1a6);
                set.add_unknown(0x1a7);
                set.add_unknown(0x1a8);
                set.add_unknown(0x1a9);
                set.add_unknown(0x1aa);
                set.add_unknown(0x1ab);
                set.add_unknown(0x1ac);
                set.add_unknown(0x1ad);
                set.add_unknown(0x1ae);
                set.add_unknown(0x1af);


                set.add_unknown(0x1c0);
                set.add_unknown(0x1c4);
                set.add_unknown_nomask(0x1c5);
                set.add_unknown(0x1c6);
                set.add_unknown_nomask(0x1c9);
                set.add_unknown(0x1ca);
                set.add_unknown(0x1cf);

                set.add_unknown(0x1d0);
                set.add_unknown(0x1d1);
                set.add_unknown(0x1d2);
                set.add_unknown(0x1d3);
                set.add_unknown(0x1d4);
                set.add_unknown(0x1d5);
                set.add_unknown_nomask(0x1d6);
                set.add_unknown(0x1d7);
                set.add_unknown(0x1d8);
                set.add_unknown(0x1d9);
                set.add_unknown(0x1da);
                set.add_unknown_nomask(0x1dc);
                set.add_unknown(0x1dd);
                set.add_unknown(0x1de);
                set.add_unknown(0x1df);

                set
            },
            _ => unimplemented!("{:?}", self),
        }
    }
}

/// Create the set of default events.
fn create_default_events() -> EventSet<Zen2Event> {
    let mut res = EventSet::new();
    //res.add(Zen2Event::ExRetInstr(0x00));
    //res.add(Zen2Event::ExRetCops(0x00));
    //res.add(Zen2Event::ExRetUcodeOps(0x00));
    //res.add(Zen2Event::ExRetUcodeInst(0x00));

    res.add_unknown(0xa7);
    res
}


pub struct EventResults { 
    floor_min: usize,
    result_min: usize,
    normalized_min: i32,
}

pub struct EmitterResults { 
    name: String,
    disas: Vec<(String, String, bool)>,
    by_event: HashMap<Zen2Event, EventResults>,
}
impl EmitterResults { 
    pub fn record_for_event(&mut self, event: Zen2Event, res: EventResults) {
        self.by_event.insert(event, res);
    }
}

pub struct GroupResults { 
    by_emitter: Vec<EmitterResults>,
}


fn main() {
    let arg = Args::parse();

    let mut harness = HarnessConfig::default_zen2()
        .pinned_core(arg.core)
        .emit();

    // Select which events will be measured
    let event_set = if let Some(events) = arg.event_set {
        events.set()
    } 
    else {
        println!("[*] Using default events");
        create_default_events()
    };

    // The user wants to measure all instruction groups
    if arg.all_groups { 
        let all_groups = Vec::from_iter(
            TestGroupId::ALL_GROUPS.iter().map(|x| *x)
        );
        PmcHammer::run_groups(&mut harness, event_set, all_groups)
    } 
    // The user asked for a specific list of instruction groups
    else  {
        PmcHammer::run_groups(&mut harness, event_set, arg.groups)
    } 

}

pub struct PmcHammer;
impl PmcHammer {

    /// Measure a single case (with a single event) from a [`TestGroup`].
    fn measure_event_for_case(harness: &mut PerfectHarness,
        desc: &EventDesc,
        floor_fn: MeasuredFn, asm_fn: MeasuredFn, 
    ) -> EventResults
    {
        let floor = harness.measure(floor_fn, 
            desc.id(), desc.mask(), 1024, InputMethod::Fixed(0,0)
        ).unwrap();
        let res = harness.measure(asm_fn, desc.id(), desc.mask(), 
            1024, InputMethod::Fixed(0, 0)
        ).unwrap();
        let floor_min = floor.get_min();
        let result_min = res.get_min();
        let normalized_min = (result_min as i32 - floor_min as i32);

        EventResults { floor_min, result_min, normalized_min }
    }

    /// Measure an entire [`TestGroup`].
    fn measure_group(harness: &mut PerfectHarness,
        group: &TestGroup,
        events: &EventSet<Zen2Event>,
    ) -> GroupResults
    {
        let mut res = GroupResults { 
            by_emitter: Vec::new()
        };

        // Emit a floor measurement for this group. 
        let floor_asm = if let Some(custom_floor) = group.floor {
            Self::emit(None, None, group.common_measured, custom_floor)
        } else {
            Self::emit(None, None, group.common_measured, |mut f| {})
        };
        let rdr = floor_asm.reader();
        let buf = rdr.lock();
        let ptr = buf.ptr(AssemblyOffset(0));
        let floor_fn: MeasuredFn = unsafe { 
            std::mem::transmute(ptr)
        };

        for emitter in group.emitters {
            // Emit the test
            let asm = Self::emit(
                group.prologue, group.epilogue, group.common_measured, 
                emitter.func,
            );
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            let inner = asm.labels()
                .resolve_static(&StaticLabel::global("inner"))
                .unwrap();
            let inner_start = asm.labels()
                .resolve_static(&StaticLabel::global("inner_start"))
                .unwrap();
            let inner_end = asm.labels()
                .resolve_static(&StaticLabel::global("inner_end"))
                .unwrap();

            let disas = disas_chunk(&asm_tgt_buf, inner_start, inner_end);
            let (istr, bstr) = disas_single(&asm_tgt_buf, inner);
            let first_inst = disas[0].0.clone();

            // FIXME: Account for presence of a prologue when using the 
            // disassembly output as the name of an emitter for a single 
            // instruction
            let emitter_name = if let Some(d) = emitter.desc {
                d.to_string()
            } else { 
                if emitter.single { 
                    istr
                } else { 
                    "unnamed".to_string()
                }
            };

            let mut emitter_results = EmitterResults { 
                name: emitter_name,
                disas,
                by_event: HashMap::new()
            };

            // For each event, take measurements and collect them
            for event in events.iter() {
                let desc = event.as_desc();
                let event_result = Self::measure_event_for_case(
                    harness, &desc, floor_fn, asm_fn
                );

                // NOTE: Skip empty results for now
                if event_result.normalized_min != 0 { 
                    emitter_results.record_for_event(*event, event_result);
                }
            }
            res.by_emitter.push(emitter_results);

        }
        res
    }

    fn run_groups(harness: &mut PerfectHarness, 
        events: EventSet<Zen2Event>,
        groups: Vec<TestGroupId>,
    ) 
    {
        // Measure all groups the user asked for
        for group_id in groups { 
            let group = group_id.group();

            println!("=======================================================");
            println!("[*] Running test group: '{}'", group.name);
            let group_results = Self::measure_group(harness, group, &events);
            for emitter_result in group_results.by_emitter {
                println!("Emitter '{}'", emitter_result.name);
                for line in emitter_result.disas {
                    println!("  {:<32} {}", line.1, line.0);
                }

                for (event, result) in emitter_result.by_event.iter()
                    .sorted_by(|x,y| { 
                        x.0.as_desc().id().cmp(&y.0.as_desc().id())
                        .then(x.0.as_desc().mask().cmp(&y.0.as_desc().mask())) 
                    }) 
                {
                    if result.normalized_min == 0 { continue; }

                    let desc = event.as_desc();
                    println!("    min={:4} (flr={:4} obs={:4}) {:03x}:{:02x}:{}", 
                        result.normalized_min,
                        result.floor_min,
                        result.result_min,
                        desc.id(), desc.mask(),
                        desc.name(),
                    );
                }
                println!();
            }
            println!();

        }
    }
}

impl PmcHammer { 
    fn emit(
        prologue: Option<fn(&mut X64Assembler)>, 
        epilogue: Option<fn(&mut X64Assembler)>, 
        common_measured: Option<fn(&mut X64Assembler)>, 
        emitter: fn(&mut X64Assembler),
    ) -> X64Assembler 
    {
        let mut f = X64Assembler::new().unwrap();
        dynasm!(f
            // Carve out some room on the stack 
            ; mov rbp, rsp
            ; sub rsp, 0x100
            ; .align 64
        );

        if let Some(emit_prologue) = prologue {
            emit_prologue(&mut f);
        }

        // Align the inner emitter to a cacheline boundary
        dynasm!(f 
            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; mfence
            ; lfence
        );

        f.emit_nop_sled(1024);
        dynasm!(f
            ; .align 64
        );

        // WARNING: This clobbers RAX, RCX, R15
        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f ; ->inner_start:);

        if let Some(emit_common) = common_measured { 
            emit_common(&mut f);
        }

        dynasm!(f ; ->inner:);
        emitter(&mut f);

        dynasm!(f ; ->inner_end:);
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        if let Some(emit_epilogue) = epilogue {
            emit_epilogue(&mut f);
        }

        dynasm!(f
            ; mov rsp, rbp
        );

        f.emit_ret();
        f.commit().unwrap();
        f
    }
}
