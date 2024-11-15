
use std::collections::*;
use rand::prelude::*;
use rand::distributions::{Distribution, Standard};

use itertools::*;

use perfect::*;
use perfect::stats::*;
use perfect::events::*;

pub enum FuzzIR {
    Nop1,
    Fnop,
    XorIdiomQ(Gpr),
    XorIdiomD(Gpr),
    XorIdiomW(Gpr),
    XorIdiomH(Gpr),
    XorIdiomB(Gpr),

    AddQQ(Gpr, Gpr),
    AddDD(Gpr, Gpr),
    AddWW(Gpr, Gpr),
    AddHH(Gpr, Gpr),
    AddBB(Gpr, Gpr),
    AddQI(Gpr, i32),

    MovQI(Gpr, i32),
    MovDI(Gpr, i32),
    MovQQ(Gpr, Gpr),
    MovDD(Gpr, Gpr),
    MovWW(Gpr, Gpr),
    MovHH(Gpr, Gpr),
    MovBB(Gpr, Gpr),

    LoadQI(Gpr, i32),
    LoadQQ(Gpr, Gpr),
    StoreQQ(Gpr, Gpr),
    StoreIQ(i32, Gpr),
    IdivQ(Gpr),
    IdivD(Gpr),

    BswapQ(Gpr),
    BswapD(Gpr),
    AndnQQQ(Gpr, Gpr, Gpr),
    AndnDDD(Gpr, Gpr, Gpr),
    XchgQRax(Gpr),
    XchgRaxQ(Gpr),
    CmpXchgQQ(Gpr, Gpr),
    Crc32QQ(Gpr, Gpr),
    Crc32DD(Gpr, Gpr),
}
impl FuzzIR {
    pub fn emit(&self, f: &mut X64Assembler) {
        match self { 
            Self::Nop1            => dynasm!(f; nop),
            Self::Fnop            => dynasm!(f; fnop),
            Self::IdivQ(r)        => dynasm!(f; idiv Rq(*r as u8)),
            Self::IdivD(r)        => dynasm!(f; idiv Rd(*r as u8)),

            Self::AndnQQQ(d,x,y)  => dynasm!(f; andn Rq(*d as u8), Rq(*x as u8), Rq(*y as u8)),
            Self::AndnDDD(d,x,y)  => dynasm!(f; andn Rd(*d as u8), Rd(*x as u8), Rd(*y as u8)),

            Self::XorIdiomQ(r)    => dynasm!(f; xor Rq(*r as u8), Rq(*r as u8)),
            Self::XorIdiomD(r)    => dynasm!(f; xor Rd(*r as u8), Rd(*r as u8)),
            Self::XorIdiomW(r)    => dynasm!(f; xor Rw(*r as u8), Rw(*r as u8)),
            Self::XorIdiomH(r)    => dynasm!(f; xor Rh(*r as u8), Rh(*r as u8)),
            Self::XorIdiomB(r)    => dynasm!(f; xor Rb(*r as u8), Rb(*r as u8)),

            Self::MovQQ(d, s)     => dynasm!(f; mov Rq(*d as u8), Rq(*s as u8)),
            Self::MovDD(d, s)     => dynasm!(f; mov Rd(*d as u8), Rd(*s as u8)),
            Self::MovWW(d, s)     => dynasm!(f; mov Rw(*d as u8), Rw(*s as u8)),
            Self::MovHH(d, s)     => dynasm!(f; mov Rh(*d as u8), Rh(*s as u8)),
            Self::MovBB(d, s)     => dynasm!(f; mov Rb(*d as u8), Rb(*s as u8)),
            Self::MovQI(d, imm)   => dynasm!(f; mov Rq(*d as u8), *imm),
            Self::MovDI(d, imm)   => dynasm!(f; mov Rd(*d as u8), *imm),

            Self::AddQQ(d, s)     => dynasm!(f; add Rq(*d as u8), Rq(*s as u8)),
            Self::AddDD(d, s)     => dynasm!(f; add Rd(*d as u8), Rd(*s as u8)),
            Self::AddWW(d, s)     => dynasm!(f; add Rw(*d as u8), Rw(*s as u8)),
            Self::AddHH(d, s)     => dynasm!(f; add Rh(*d as u8), Rh(*s as u8)),
            Self::AddBB(d, s)     => dynasm!(f; add Rb(*d as u8), Rb(*s as u8)),
            Self::AddQI(d, imm)   => dynasm!(f; add Rq(*d as u8), *imm),

            Self::LoadQI(d, imm)  => dynasm!(f; mov Rq(*d as u8), [*imm]),
            Self::LoadQQ(d, s)    => dynasm!(f; mov Rq(*d as u8), [Rq(*s as u8)]),

            Self::StoreQQ(d, s)   => dynasm!(f; mov [Rq(*d as u8)], Rq(*s as u8)),
            Self::StoreIQ(imm, d) => dynasm!(f; mov [*imm], Rq(*d as u8)),

            Self::XchgQRax(d)     => dynasm!(f; xchg Rq(*d as u8), rax),
            Self::XchgRaxQ(s)     => dynasm!(f; xchg rax, Rq(*s as u8)),
            Self::BswapQ(d)     => dynasm!(f; bswap Rq(*d as u8)),
            Self::BswapD(d)     => dynasm!(f; bswap Rd(*d as u8)),

            Self::CmpXchgQQ(d, s) => dynasm!(f; cmpxchg Rq(*d as u8), Rq(*s as u8)),

            Self::Crc32QQ(d, s)   => dynasm!(f; crc32 Rq(*d as u8), Rq(*s as u8)),
            Self::Crc32DD(d, s)   => dynasm!(f; crc32 Rd(*d as u8), Rd(*s as u8)),
        }
    }
}

impl Distribution<FuzzIR> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FuzzIR {
        // assume r15 is reserved
        let imm32: i32 = rng.gen();
        let imm32_addr: i32 = rng.gen_range(0x0000_0001..=0x0000_03f8);
        let gpr1: Gpr = rng.gen();
        let gpr2: Gpr = rng.gen();
        let gpr3: Gpr = rng.gen();

        let r = rng.gen_range(0..=20);
        match r {
            00 => FuzzIR::XorIdiomQ(gpr1),
            01 => FuzzIR::XorIdiomD(gpr1),
            02 => FuzzIR::XorIdiomW(gpr1),
            03 => FuzzIR::XorIdiomH(gpr1),
            04 => FuzzIR::XorIdiomB(gpr1),

            05 => FuzzIR::MovQQ(gpr1, gpr2),
            06 => FuzzIR::MovDD(gpr1, gpr2),
            07 => FuzzIR::MovWW(gpr1, gpr2),
            08 => FuzzIR::MovHH(gpr1, gpr2),
            09 => FuzzIR::MovBB(gpr1, gpr2),
            10 => FuzzIR::MovQI(gpr1, imm32),
            11 => FuzzIR::MovDI(gpr1, imm32),

            12 => FuzzIR::LoadQI(gpr1, imm32_addr),
            13 => FuzzIR::StoreIQ(imm32_addr, gpr1),

            14 => FuzzIR::BswapQ(gpr1),
            15 => FuzzIR::BswapD(gpr1),
            16 => FuzzIR::CmpXchgQQ(gpr1, gpr2),
            17 => FuzzIR::XchgRaxQ(gpr1),
            18 => FuzzIR::XchgQRax(gpr1),

            19 => FuzzIR::AndnQQQ(gpr1, gpr2, gpr3),
            20 => FuzzIR::AndnDDD(gpr1, gpr2, gpr3),

            21 => FuzzIR::Crc32QQ(gpr1, gpr2),
            22 => FuzzIR::Crc32DD(gpr1, gpr2),

            _ => unreachable!(),
        }
    }
}



pub struct FuzzArgs {
    pub name: &'static str,
    pub pre_emit: fn(&mut X64Assembler),
    pub spec_emit: fn(&mut X64Assembler),
    pub post_emit: fn(&mut X64Assembler),
    pub events: EventSet<Zen2Event>,
}



pub struct Fuzz;
impl Fuzz {
    const SRC1_REG: Gpr = Gpr::R8;
    const SRC2_REG: Gpr = Gpr::R9;
    const TGT_REG:  Gpr = Gpr::R10;
    const SRC1_RQ:  u8 = Self::SRC1_REG as u8;
    const SRC2_RQ:  u8 = Self::SRC2_REG as u8;
    const TGT_RQ:   u8 = Self::TGT_REG as u8;

    fn emit(args: &FuzzArgs) -> X64Assembler 
    {
        let mut f = X64Assembler::new().unwrap();

        // Flush BTB
        for _ in 0..8192 { dynasm!(f ; jmp >next ; next:); }

        let x = thread_rng().gen::<FuzzIR>();

        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        let lab = f.new_dynamic_label();
        dynasm!(f
            ; mov Rq(Self::SRC1_RQ), 0x1111_1111
            ; mov Rq(Self::SRC2_RQ), 0x2222_2222
            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP6
            ; lea r14, [=>lab]
            ; lfence
        );

        dynasm!(f ; ->start:);

        // NOTE: This speculatively dispatches up to 37/38 NOPs
        (args.pre_emit)(&mut f);
        dynasm!(f ; jmp r14);
        (args.spec_emit)(&mut f);
        f.place_dynamic_label(lab);
        (args.post_emit)(&mut f);

        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let args: &[FuzzArgs] = &[
            //FuzzArgs { 
            //    name: "Dispatch test",
            //    pre_emit: |f| { },
            //    spec_emit: |f| { f.emit_nop_sled(37); f.emit_fnop_sled(1); },
            //    post_emit: |f| { f.emit_nop_sled(512); }
            //},
            FuzzArgs { 
                name: "Wuhhhh",
                events: EventSet::new_from_slice(&[
                    Zen2Event::MemFileHit(0x00),
                ]),
                pre_emit: |mut f| { 
                    dynasm!(f
                        ; mov [0x0000_0288], Rq(Self::SRC1_RQ)
                    );
                },
                spec_emit: |mut f| { 
                    dynasm!(f
                        ; mov [0x0000_0288], Rq(Self::SRC2_RQ)
                    );
                },
                post_emit: |mut f| { 
                    dynasm!(f
                        ; mov Rq(Self::TGT_RQ), [0x0000_0288]
                    );
                }
            },
        ];

        //let event = Zen2Event::ExRetBrnIndMisp(0x00);
        let event = Zen2Event::MemFileHit(0x00);

        //'top: for arg in args {
        'top: for testno in 0..1 {
            let asm = Self::emit(&args[0]);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };
            let start_label_off = asm.labels()
                .resolve_static(&StaticLabel::global("start"))
                .unwrap();

            disas(&asm_tgt_buf, start_label_off, None);

            let desc = event.as_desc();
            let results = harness.measure(asm_fn, 
                desc.id(), desc.mask(), 16, InputMethod::Fixed(0, 0)
            ).unwrap();

            let min = results.get_min();
            let max = results.get_max();
            let dist = results.get_distribution();
            println!("    {:03x}:{:02x} {:032} min={} max={} dist={:?}",
                desc.id(), desc.mask(), desc.name(), min, max, dist);

            if let Some(gpr_dumps) = results.gpr_dumps {
                for dump in &gpr_dumps {
                    let src = dump.read_gpr(Self::SRC1_REG);
                    let tgt = dump.read_gpr(Self::TGT_REG);

                    //println!("src={:?}={:016x?} ===> tgt={:?}={:016x?}", 
                    //    Self::SRC_REG, src, Self::TGT_REG, tgt);
                    if src != tgt {
                        println!("======================================");
                        println!("????????????????????????????????");
                        println!();
                        disas(&asm_tgt_buf, start_label_off, None);
                        println!();
                        println!("{:016x?}", dump);
                        println!("======================================");
                        break 'top;
                    }
                }
            }
            println!("[*] Finished test #{}", testno);

        }

        println!();
    }
}



fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .dump_gpr(true)
        .emit();
    //harness.set_pmc_use(false);
    Fuzz::run(&mut harness);
}


