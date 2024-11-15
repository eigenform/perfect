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

    /// A comma-separated list of test groups to run and measure. 
    #[arg(long, value_enum, value_delimiter = ',')]
    groups: Vec<TestGroupId>,

    /// A set of events to measure. 
    #[arg(long, value_enum)]
    event_set: Option<PmcHammerEventSet>,

    /// Run with all groups
    #[arg(short, long)]
    all_groups: bool,


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
            _ => unimplemented!("{:?}", self),
        }
    }
}

fn main() {
    let arg = Args::parse();

    let mut harness = HarnessConfig::default_zen2()
        .emit();

    // The set of default PMC events
    let mut default_events = EventSet::new();
    default_events.add_unknown(0x40);
    default_events.add_unknown(0x41);
    default_events.add_unknown(0x42);
    default_events.add_unknown(0x43);
    default_events.add_unknown(0x44);
    default_events.add_unknown(0x45);
    default_events.add_unknown(0x46);
    default_events.add_unknown(0x47);
    default_events.add_unknown(0x48);
    default_events.add_unknown(0x49);
    default_events.add_unknown(0x4a);
    default_events.add_unknown(0x4b);
    default_events.add_unknown(0x4c);
    default_events.add_unknown(0x4d);
    default_events.add_unknown(0x4e);
    default_events.add_unknown(0x4f);

    default_events.add(Zen2Event::LsNotHaltedCyc(0x00));



    let event_set = if let Some(events) = arg.event_set {
        events.set()
    } 
    else {
        println!("[*] Using default events");
        default_events
    };

    if arg.all_groups { 
        let all_groups = Vec::from_iter(
            TestGroupId::ALL_GROUPS.iter().map(|x| *x)
        );
        PmcHammer::run(&mut harness, event_set, all_groups)
    } else { 
        PmcHammer::run(&mut harness, event_set, arg.groups)
    }
}

pub struct PmcHammerArg {
    groups: Vec<TestGroupId>,
}

pub struct PmcHammer;
impl PmcHammer {
    fn emit(
        prologue: Option<fn(&mut X64Assembler)>, 
        epilogue: Option<fn(&mut X64Assembler)>, 
        common: Option<fn(&mut X64Assembler)>, 
        emit_inner: fn(&mut X64Assembler)) 
        -> X64Assembler 
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
            ; lfence
            ; mfence
        );

        f.emit_nop_sled(1024);
        dynasm!(f
            ; .align 64
        );

        // WARNING: This clobbers RAX, RCX, R15
        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        if let Some(emit_common) = common { 
            emit_common(&mut f);
        }

        dynasm!(f ; ->inner:);
        emit_inner(&mut f);

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

    fn run(harness: &mut PerfectHarness, 
        events: EventSet<Zen2Event>,
        groups: Vec<TestGroupId>,
    ) 
    {
        println!("{:016x?}", harness.harness_stack.as_ptr());

        // Collect results keyed by PMC event
        let mut map: HashMap<Zen2Event, Vec<(String, i32, usize, usize)>>
            = HashMap::new();

        for group_id in groups { 
            let group = group_id.group();

            // Emit the floor measurement for this group
            let floor_asm = if let Some(custom_floor) = group.floor {
                Self::emit(None, None, group.common, custom_floor)
            } else {
                Self::emit(None, None, group.common, |mut f| {} )
            };
            let rdr = floor_asm.reader();
            let buf = rdr.lock();
            let ptr = buf.ptr(AssemblyOffset(0));
            let floor_fn: MeasuredFn = unsafe { 
                std::mem::transmute(ptr)
            };


            println!("=======================================");
            println!("[*] Test group: '{}'", group.name);
            for emitter in group.emitters {
                let mut results: HashMap<Zen2Event, MeasureResults> 
                    = HashMap::new();

                // Emit this test
                let asm = Self::emit(
                    group.prologue, group.epilogue, group.common, 
                    *emitter
                );
                let asm_reader = asm.reader();
                let asm_tgt_buf = asm_reader.lock();
                let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
                let asm_fn: MeasuredFn = unsafe { 
                    std::mem::transmute(asm_tgt_ptr)
                };
                let start_label_off = asm.labels()
                    .resolve_static(&StaticLabel::global("inner"))
                    .unwrap();
                let (bytestr, inst_str) = disas_str_single(
                    &asm_tgt_buf, start_label_off
                );
                let test_name = format!("{} ({})", inst_str, bytestr);
                println!("{} ({})", inst_str, bytestr);

                for event in events.iter() {
                    let desc = event.as_desc();
                    let floor = harness.measure(floor_fn, 
                        desc.id(), desc.mask(), 1024, InputMethod::Fixed(0,0)
                    ).unwrap();
                    let res = harness.measure(asm_fn, desc.id(), desc.mask(), 
                        1024, InputMethod::Fixed(0, 0)
                    ).unwrap();

                    let floor_min = floor.get_min();
                    let res_min = res.get_min();
                    let norm_min = (res_min as i32 - floor_min as i32);

                    // NOTE: Skip empty results for now?
                    if norm_min == 0 { 
                        continue;
                    }

                    println!("  {:03x}:{:02x} {:42} min={:4} (floor={:4} obs={:4})",
                        desc.id(), desc.mask(), desc.name(), 
                        norm_min,
                        floor_min, res_min,
                    );
                    if let Some(r) = map.get_mut(event) {
                        r.push((test_name.clone(), norm_min, floor_min, res_min));
                    } else {
                        let mut r = Vec::new();
                        r.push((test_name.clone(), norm_min, floor_min, res_min));
                        map.insert(*event, r);
                    }
                }
            }
        }

        for (event, results) in map.iter()
            .sorted_by(|x,y| {
                x.0.as_desc().id().cmp(&y.0.as_desc().id())
                    .then(x.0.as_desc().mask().cmp(&y.0.as_desc().mask()))
            }) 
        {
            let desc = event.as_desc();
            println!("{:03x}:{:02x} {:?}", desc.id(),desc.mask(), desc.name());
            for (name, norm_min, floor_min, result_min) in results.iter() {
                //println!("  {}", name);
                println!("    min={:4} (floor={:4} obs={:4}) {}", 
                    norm_min, floor_min, result_min, name
                );
            }
        }


    }
}


