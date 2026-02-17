// SPDX-License-Identifier: BSD-2-Clause
//! # Styx-Processors

use angel::handle_angel;
use event_controller::HexagonEventController;
use styx_core::arch::hexagon::register_fields::Ssr;
use styx_core::arch::hexagon::HexagonRegister;
use styx_core::cpu::arch::hexagon::HexagonVariants;
use styx_core::cpu::{Arch, Backend};
use styx_core::loader::LoaderHints;
use styx_core::memory::physical::PhysicalMemoryVariant;
use styx_core::memory::{MemoryPermissions, Mmu};
use styx_core::prelude::log::info;
use styx_core::prelude::{Context, Peripheral};
use styx_core::{
    core::{
        builder::{BuildProcessorImplArgs, ProcessorImpl},
        ProcessorBundle,
    },
    cpu::{ArchEndian, CpuBackendExt, HexagonPcodeBackend},
    errors::{anyhow, UnknownError},
    hooks::{CoreHandle, Hookable, StyxHook},
};
use tlb::HexagonTlb;

mod angel;
mod event_controller;
mod tlb;

#[derive(serde::Deserialize)]
pub struct HexagonBuilder {
    pub variant: HexagonVariants,
}

impl Default for HexagonBuilder {
    fn default() -> Self {
        Self {
            variant: HexagonVariants::QDSP6V62,
        }
    }
}

impl ProcessorImpl for HexagonBuilder {
    fn build(&self, args: &BuildProcessorImplArgs) -> Result<ProcessorBundle, UnknownError> {
        let mut cpu = if let Backend::Pcode = args.backend {
            Box::new(HexagonPcodeBackend::new_engine_config(
                self.variant.clone(),
                ArchEndian::LittleEndian,
                &args.into(),
            ))
        } else {
            return Err(anyhow::anyhow!(
                "hexagon processor only supports pcode backend"
            ));
        };

        // This should always be triggered at the end of a packet (see `HexagonPcodeBackend` implementation,
        // specifically details about the `DelayedInterrupt`, for more information), after the pc has
        // been incremented, so at this point, the Elr register will be set to the pc to return to.
        //
        // FIXME: multicore
        let interrupt_handler = |handle: CoreHandle, interrupt_number: i32| {
            // get cause, if the cause is 0 then we need to do the angel stuff
            let ssr = Ssr::new_with_raw_value(
                handle
                    .cpu
                    .read_register::<u32>(HexagonRegister::Ssr)
                    .with_context(|| "couldn't read ssr in interrupt")?,
            );

            // SSR.CAUSE equals 0 implies that we must handle ANGEL calls
            // See QUIC QEMU's (branch hex-next) file target/hexagon/hexswi.c,
            // specifically the case for HEX_EVENT_TRAP0 in
            // hex_cpu_do_interrupt.
            if ssr.cause() == 0 {
                let swi_no = handle
                    .cpu
                    .read_register::<u32>(HexagonRegister::R0)
                    .with_context(|| "couldn't read r0 in interrupt")?;
                let arg = handle
                    .cpu
                    .read_register::<u32>(HexagonRegister::R1)
                    .with_context(|| "couldn't read r1 in interrupt")?;

                handle_angel(swi_no, arg);
            }

            // get evb which is the interrupt vector base
            let evb = handle
                .cpu
                .read_register::<u32>(HexagonRegister::Evb)
                .with_context(|| "couldn't read interrupt vector base")?;
            let jump_point = evb + (interrupt_number * 4) as u32;

            info!("interrupt jumping to {jump_point:x}");

            // set elr to pc
            let pc = handle
                .cpu
                .pc()
                .with_context(|| "couldn't get pc to write to elr")?;

            info!("interrupt setting elr to {pc:x}");

            handle
                .cpu
                .write_register(HexagonRegister::Elr, pc)
                .with_context(|| "couldn't write old pc to elr")?;
            handle
                .cpu
                .write_register(HexagonRegister::Pc, jump_point)
                .with_context(|| "couldn't write interrupt jump point to pc")?;

            Ok(())
        };

        cpu.add_hook(StyxHook::interrupt(interrupt_handler))?;

        let mut mmu = match self.variant {
            HexagonVariants::QDSP6V62 => Mmu::new(
                Box::new(HexagonTlb::new()),
                PhysicalMemoryVariant::RegionStore,
                cpu.as_mut(),
            )?,
            _ => {
                return Err(UnknownError::msg(
                    "hexagon variant {self.variant:?} is not supported, only v62 is supported",
                ))
            }
        };

        mmu.memory_map(0, 2u64.pow(32), MemoryPermissions::all())?;

        let hec = Box::new(HexagonEventController::default());

        let peripherals: Vec<Box<dyn Peripheral>> = Vec::new();

        let mut hints = LoaderHints::new();
        hints.insert("arch".to_string().into_boxed_str(), Box::new(Arch::Hexagon));

        Ok(ProcessorBundle {
            cpu,
            mmu,
            event_controller: hec,
            peripherals,
            loader_hints: hints,
        })
    }
}
