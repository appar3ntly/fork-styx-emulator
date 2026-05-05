// SPDX-License-Identifier: BSD-2-Clause

use clap::ValueEnum;
use styx_emulator::{
    errors::UnknownError, hooks::StyxHook, prelude::Processor,
    processors::hexagon::hexagon::HexagonProcessorConfig,
};

pub(crate) mod pixel5;
pub(crate) mod s22;
pub(crate) mod tester;

#[derive(Clone, ValueEnum, Debug)]
pub enum HexagonTarget {
    S22,
    Pixel5,
    Tester,
}

pub trait HexagonDevice {
    // Hooks
    fn hooks(&self) -> Result<Vec<StyxHook>, UnknownError>;
    // Set the processor config
    fn proc_config(&self) -> Result<HexagonProcessorConfig, UnknownError>;
    // To do fixups. Is sort of unsafe, since it could overwrite anything of the above, which isn't cool.
    // Runs after the previous three functions.
    fn post_init(&self, proc: &mut Processor) -> Result<(), UnknownError>;
}

/*impl HexagonDevice for Box<dyn HexagonDevice> {
    fn hooks(&self) -> Result<Vec<StyxHook>, UnknownError> {
        self.hooks()
    }

    fn proc_config(&self) -> Result<HexagonProcessorConfig, UnknownError> {
        self.proc_config()
    }

    fn post_init(&self, proc: &mut Processor) -> Result<(), UnknownError> {
        self.post_init(proc)
    }
}*/
