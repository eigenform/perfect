
use clap::Parser;
use clap::ValueEnum;

use perfect::PerfectEnv;
use perfect::util::msr::Msr;
use std::io::{ Error, ErrorKind };

pub fn parse_enable(s: &str) -> Result<bool, String> {
    match s {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("<ENABLE> must be 'on' or 'off'")),
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CpuFeature {
    /// Predictive Store Forwarding (Zen 3 and later)
    Psf,
    /// Speculative Store Bypass (Zen 3 and later)
    Ssb,
    /// Single-thread Indirect Branch Predictor (Zen 3 and later)
    Stibp,
    /// Indirect Branch Restricted Speculation (Zen 3 and later)
    Ibrs,

    /// Op Cache (known on Zen 2)
    OpCache,
    /// Floating-Point/Vector Move Elimination (known on Zen 2)
    FpMovElim,
    /// Branch Prediction for Non-Branch Instructions (known on Zen 2)
    NonBrPred,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FeatureState { On, Off, }
impl FeatureState {
    pub fn as_bool(&self) -> bool {
        match self { 
            Self::On => true,
            Self::Off => false,
        }
    }
}


#[derive(Clone, Copy, Parser)]
pub enum Command { 

    /// Toggle de-feature bits on a particular CPU core (requires root).
    ///
    /// NOTE: This command does not validate whether or not the requested
    /// feature bit or associated MSR actually exist on the target CPU core. 
    #[clap(verbatim_doc_comment)]
    CpuFeature { 
        /// Target Feature
        feature: CpuFeature,

        /// Target CPU core number
        cpu: usize,

        /// Target state
        state: FeatureState,
    },

    /// Toggle SMT (requires root).
    Smt { state: FeatureState, },

    /// Toggle userspace RDPMC (requires root).
    Rdpmc { state: FeatureState, },

    /// Toggle 'cpufreq' frequency boosting (requires root).
    Boost { state: FeatureState },

    /// Set the value of the 'vm.mmap_min_addr' sysctl knob (requires root).
    MmapMinAddr { addr: usize },

    /// Show the current state of the environment. 
    Show,

    /// Apply the "default" configuration (requires root). 
    ///   - Enable RDPMC
    ///   - Disable SMT
    ///   - Disable Boost
    ///   - Set vm.mmap_min_addr to zero
    #[clap(verbatim_doc_comment)]
    Defaults,
}

#[derive(Parser)]
#[command(verbatim_doc_comment)]
pub struct Args { 
    #[command(subcommand)]
    pub cmd: Command,
}


fn print_env() {
    let num_cores = nix::unistd::sysconf(
        nix::unistd::SysconfVar::_NPROCESSORS_ONLN
    ).unwrap().unwrap();

    let isol = PerfectEnv::sysfs_isolated();
    let nohz = PerfectEnv::sysfs_nohz();
    let smt  = match PerfectEnv::sysfs_smt_enabled() {
        true => "enabled [!!]",
        false => "disabled",
    };
    let boost = match PerfectEnv::sysfs_cpufreq_boost_enabled() {
        Ok(true) => "enabled [!!]",
        Ok(false) => "disabled",
        Err(e) => "<couldn't read boost status?>",
    };

    //let gov = match PerfectEnv::sysfs_cpufreq_governor(15) {
    //    Ok(s) => s,
    //    Err(e) => unimplemented!("{:?}", e),
    //};

    let rdpmc_enabled = PerfectEnv::sysfs_rdpmc_enabled();

    let rdpmc_str = match rdpmc_enabled { 
        Ok(true) => "enabled",
        Ok(false) => "disabled [!!]",
        Err(std::io::ErrorKind::PermissionDenied) => "<read error; are you root?>",
        Err(e) => unimplemented!("{:?}", e),
    };
    let mmap_min_addr = PerfectEnv::procfs_mmap_min_addr();

    println!("[*] 'perfect' environment summary:");
    println!("  {:<40}: {}", "online cores", num_cores);
    println!("  {:<40}: {}", "isolated cores", isol);
    println!("  {:<40}: {}", "nohz_full cores", nohz);
    println!("  {:<40}: {}", "simultaneous multithreading (SMT)", smt);
    println!("  {:<40}: {}", "cpufreq boost", boost);
    //println!("  {:<40}: {}", "cpufreq scaling", gov);
    println!("  {:<40}: {}", "userspace rdpmc", rdpmc_str);
    println!("  {:<40}: {}", "vm.mmap_min_addr", mmap_min_addr);
}


fn main() -> Result<(), String> {
    let args = Args::parse();

    match args.cmd {
        Command::CpuFeature { feature, cpu, state } => {
            match feature {
                CpuFeature::Psf => {
                    PerfectEnv::toggle_psf(cpu, state.as_bool())?;
                },
                CpuFeature::Ssb => {
                    PerfectEnv::toggle_ssb(cpu, state.as_bool())?;
                },
                CpuFeature::Stibp => {
                    PerfectEnv::toggle_stibp(cpu, state.as_bool())?;
                },
                CpuFeature::Ibrs => {
                    PerfectEnv::toggle_ibrs(cpu, state.as_bool())?;
                },
                CpuFeature::OpCache => {
                    PerfectEnv::toggle_opcache(cpu, state.as_bool())?;
                },
                CpuFeature::FpMovElim => {
                    PerfectEnv::toggle_fp_mov_elim(cpu, state.as_bool())?;
                },
                CpuFeature::NonBrPred => {
                    PerfectEnv::toggle_nobr_pred(cpu, state.as_bool())?;
                },
            }
            println!("[!] Core {}: CPU feature {:?} set to {:?}", 
                cpu, feature, state
            );
        },

        Command::Smt { state } => {
            PerfectEnv::sysfs_smt_set(state.as_bool())
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;
            println!("[!] SMT set to {:?}", state);
        },
        Command::Rdpmc { state } => {
            PerfectEnv::sysfs_rdpmc_set(state.as_bool())
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;
            println!("[!] Userspace RDPMC set to {:?}", state);
        },
        Command::Boost { state } => {
            PerfectEnv::sysfs_cpufreq_boost_set(state.as_bool())
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;
            println!("[!] cpufreq boost set to {:?}", state);
        },
        Command::MmapMinAddr { addr } => {
            PerfectEnv::procfs_mmap_min_addr_set(addr)
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;
            println!("[!] vm.mmap_min_addr set to {}", addr);
        },

        Command::Defaults => {
            PerfectEnv::sysfs_smt_set(false)
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;
            PerfectEnv::sysfs_rdpmc_set(true)
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;
            PerfectEnv::procfs_mmap_min_addr_set(0)
                .map_err(|e: std::io::ErrorKind| format!("{:?}", e))?;

            if let Ok(_) = PerfectEnv::sysfs_cpufreq_boost_set(false) {
            } else {
                println!("[!] Couldn't change boost settings?");
            }

            println!("[*] Successfully applied default configuration");
            print_env();
        },

        Command::Show => { 
            print_env();
        },

    }

    Ok(())
}

