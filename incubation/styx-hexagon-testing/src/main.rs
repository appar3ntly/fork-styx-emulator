// SPDX-License-Identifier: BSD-2-Clause

use clap::Parser;
use devices::pixel5::Pixel5;
use devices::s22::S22;
use devices::tester::Tester;
use devices::{HexagonDevice, HexagonTarget};
use runner::{HexagonBinaryType, HexagonDebuggerInfo};
use styx_emulator::cpu::arch::hexagon::HexagonRegister;
use styx_emulator::prelude::log::{info, warn};
use styx_emulator::prelude::logging::init_logging;
use styx_emulator::prelude::styx_async::sync::broadcast;
use styx_emulator::prelude::*;

mod devices;
mod runner;

mod tester;

const CHANNEL_SIZE: usize = 1024;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct HexagonTesterArgs {
    #[arg(required = true)]
    file_name: String,

    // default_value_t doesn't work here
    #[arg(short, long, default_value = "s22")]
    target: HexagonTarget,

    #[arg(short, long, default_value_t = 9999)]
    gdb_remote_port: u16,

    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let args = HexagonTesterArgs::parse();

    let device: Box<dyn HexagonDevice> = match args.target {
        HexagonTarget::S22 => Box::new(S22::default()),
        HexagonTarget::Pixel5 => Box::new(Pixel5::default()),
        HexagonTarget::Tester => Box::new(Tester::default()),
    };

    // Semihosting channel
    let (tx, mut rx) = broadcast::channel(CHANNEL_SIZE);

    let mut proc = runner::setup_load_hexagon(
        HexagonBinaryType::FilePath(args.file_name),
        if args.debug {
            Some(HexagonDebuggerInfo {
                gdb_remote_port: args.gdb_remote_port,
            })
        } else {
            None
        },
        device.as_ref(),
        Arc::new(tx),
    )?;

    add_debugging_hooks(&mut proc)?;

    // Start thread that gets TX output and prints it
    thread::spawn(move || {
        while let Ok(chr) = rx.blocking_recv() {
            print!("{}", chr as char);
        }
    });

    info!("Starting emulator");
    proc.run(Forever)?;

    Ok(())
}

fn add_debugging_hooks(proc: &mut Processor) -> Result<(), Box<dyn std::error::Error>> {
    //
    //
    /*
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys reading from 8, rs 8 rt 0
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys read 100
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys reading from f80044, rs 44 rt 1f00
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys read 0
    WARN styx_cpu_pcode_backend::call_other: Handle index 44 (name: Some("syncht")) does not exist. Called with: [] -> Some(Unique(0x5BBF00, 4)) @ 0x8cc00494
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys reading from c, rs c rt 0
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys read 0
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys reading from 10, rs 10 rt 0
    INFO styx_cpu_pcode_backend::arch_spec::hexagon::system::mem: memw_phys read 0
       */

    // we then read from, (((X << 5) - 0x100) << 11) + struct offset
    // subsystem base
    /*proc.core.cpu.add_hook(StyxHook::MemoryReadVirtual(
        (0xFE11db04..0xFE11db14).into(),
        Box::new(
            |mut proc: CoreHandle, address: u64, size: u32, data: &mut [u8]| {
                let lr = proc.cpu.read_register::<u32>(HexagonRegister::Lr).unwrap();
                unimplemented!(
                    "memory read hook at {:x?}, address {address:x} lr {lr:x}",
                    proc.pc()
                )
            },
        ),
    ))?;*/

    /*proc.core.cpu.add_hook(StyxHook::RegisterWrite(
        HexagonRegister::BadVa0.into(),
        Box::new(|proc: CoreHandle, reg, data: &RegisterValue| {
            warn!("badva0 written pc {:x?}", proc.cpu.pc());
            Ok(())
        }),
    ))?;*/

    proc.core.cpu.add_hook(StyxHook::MemoryReadVirtual(
        (0x9db6c000..0x9db6c004).into(),
        Box::new(
            |mut proc: CoreHandle, address: u64, size: u32, data: &mut [u8]| {
                warn!(
                    "l2ecomem written {:x?}, address is {address:x} value is {data:x?} sz {size}",
                    proc.pc()
                );
                Ok(())
            },
        ),
    ))?;

    proc.core.cpu.add_hook(StyxHook::MemoryRead(
        (0xd8200000..0xd8210000).into(),
        Box::new(
            |mut proc: CoreHandle, address: u64, size: u32, data: &mut [u8]| {
                warn!(
                    "l2ecomem read {:x?}, address is {address:x} value is {data:?} sz {size}",
                    proc.pc()
                );
                Ok(())
            },
        ),
    ))?;

    proc.core.cpu.add_hook(StyxHook::MemoryWriteVirtual(
        (0xfe1bfd40..0xfe1bfd48).into(),
        Box::new(
            |mut proc: CoreHandle, address: u64, size: u32, data: &[u8]| {
                let lr = proc.cpu.read_register::<u32>(HexagonRegister::Lr).unwrap();
                warn!(
                    "memory write hook at {:x?}, address is {address:x} lr {lr:x} value is {data:?} sz {size}",
                    proc.pc()
                );
                Ok(())
            },
        ),
    ))?;
    proc.core.cpu.add_hook(StyxHook::MemoryReadVirtual(
        (0xfe1bfd40..0xfe1bfd48).into(),
        Box::new(
            |mut proc: CoreHandle, address: u64, size: u32, data: &mut [u8]| {
                let lr = proc.cpu.read_register::<u32>(HexagonRegister::Lr).unwrap();
                warn!(
                    "memory read hook at {:x?}, address is {address:x} lr {lr:x} value is {data:?} sz {size}",
                    proc.pc()
                );
                Ok(())
            },
        ),
    ))?;

    proc.core.cpu.add_hook(StyxHook::MemoryWriteVirtual(
        (0xfe1bfc40..0xfe1bfc48).into(),
        Box::new(
            |mut proc: CoreHandle, address: u64, size: u32, data: &[u8]| {
                let lr = proc.cpu.read_register::<u32>(HexagonRegister::Lr).unwrap();
                warn!(
                    "memory write hook at {:x?}, address is {address:x} lr {lr:x} value is {data:?} sz {size}",
                    proc.pc()
                );
                Ok(())
            },
        ),
    ))?;

    // proc.core.mmu.write_u32_le_phys_data(0xf80044, 0x100)?;
    Ok(())
}
