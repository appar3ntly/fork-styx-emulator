// SPDX-License-Identifier: BSD-2-Clause

use styx_emulator::{
    errors::UnknownError,
    hooks::StyxHook,
    prelude::Processor,
    processors::hexagon::hexagon::{HexagonConfigTable, HexagonProcessorConfig},
};

use super::HexagonDevice;

#[derive(Default)]
pub struct Tester {}

impl HexagonDevice for Tester {
    fn hooks(&self) -> Result<Vec<StyxHook>, UnknownError> {
        Ok(vec![])
    }

    fn proc_config(&self) -> Result<HexagonProcessorConfig, UnknownError> {
        Ok(HexagonProcessorConfig {
            config_table: [
                (HexagonConfigTable::L2TCM, 0x540),
                (HexagonConfigTable::L2EcomemSize, 0x800),
                (HexagonConfigTable::L2Config, 0x57a),
                (HexagonConfigTable::Etm, 0x57a),
                (HexagonConfigTable::L2InstructionTCM, 0x560),
                (HexagonConfigTable::Clade1, 0x57d),
                (HexagonConfigTable::Clade2, 0x55b),
                (HexagonConfigTable::L2TagSize, 0x800),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        })
    }

    fn post_init(&self, proc: &mut Processor) -> Result<(), UnknownError> {
        // the test case tries to set up the ISDB, which
        // we are not implementing.
        //
        // according to qemu "hw/hexagon/hexagon_dsp.c"
        // the isdb_secure flag is at 0x30 anjd isdb_trusted is
        // 0x34. we will set these to true to avoid the isdb being used
        // or something.
        //
        // isdb = in-silicon debugger
        // isdb_secure
        proc.core.mmu.write_u32_le_phys_data(0x30, 1)?;
        // isdb_trusted
        proc.core.mmu.write_u32_le_phys_data(0x34, 1)?;

        Ok(())
    }
}
