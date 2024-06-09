

use perfect::PerfectEnv;
use std::io::{ Error, ErrorKind };

fn main() {

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
        true => "enabled [!!]",
        false => "disabled",
    };
    let gov = match PerfectEnv::sysfs_cpufreq_governor(15) {
        Ok(s) => s,
        Err(e) => unimplemented!("{:?}", e),
    };

    let rdpmc = match PerfectEnv::sysfs_rdpmc_enabled() {
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
    println!("  {:<40}: {}", "cpufreq scaling", gov);
    println!("  {:<40}: {}", "userspace rdpmc", rdpmc);
    println!("  {:<40}: {}", "vm.mmap_min_addr", mmap_min_addr);


}
