// SPDX-License-Identifier: BSD-2-Clause
//! # Styx-Processors

use event_controller::HexagonEventController;
use styx_core::cpu::arch::hexagon::HexagonVariants;
use styx_core::cpu::{Arch, Backend};
use styx_core::loader::LoaderHints;
use styx_core::memory::physical::PhysicalMemoryVariant;
use styx_core::memory::Mmu;
use styx_core::prelude::Peripheral;
use styx_core::{
    core::{
        builder::{BuildProcessorImplArgs, ProcessorImpl},
        ProcessorBundle,
    },
    cpu::{ArchEndian, HexagonPcodeBackend},
    errors::{anyhow, UnknownError},
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

        let mmu = match self.variant {
            HexagonVariants::QDSP6V62 => Mmu::new(
                Box::new(HexagonTlb::new()),
                PhysicalMemoryVariant::FlatMemory,
                cpu.as_mut(),
            )?,
            _ => {
                return Err(UnknownError::msg(
                    "hexagon variant {self.variant:?} is not supported, only v62 is supported",
                ))
            }
        };

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
