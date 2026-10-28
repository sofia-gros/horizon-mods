use horizon_mod_sdk::server::{
    GuiWorkstationOutput, GuiWorkstationPlan, Registration, ServerMod, WorkstationInput,
    workstations,
};
struct Fixture {
    tick: u64,
    boost_until: u64,
}
impl ServerMod for Fixture {
    fn init() -> Result<(Self, Registration), String> {
        workstations::register_gui(
            "gui_fixture:storage",
            "Storage Fixture",
            &["gui_fixture".into(), "state".into()],
            &[],
            r#"{"width":176,"height":166,"slots":[{"id":"a","x":8,"y":46,"role":"storage"},{"id":"b","x":26,"y":46,"role":"storage"},{"id":"c","x":44,"y":46,"role":"storage"},{"id":"d","x":62,"y":46,"role":"storage"},{"id":"e","x":80,"y":46,"role":"storage"},{"id":"f","x":98,"y":46,"role":"storage"},{"id":"g","x":116,"y":46,"role":"storage"}],"elements":[{"type":"label","x":8,"y":18,"text":"Machine"},{"type":"progress","id":"work","x":80,"y":18,"width":60,"height":8,"value":50}]}"#,
        )?;
        workstations::register_gui(
            "gui_fixture:machine",
            "Machine Fixture",
            &["gui_fixture".into(), "state".into()],
            &[],
            r#"{"width":176,"height":166,"slots":[{"id":"input","x":8,"y":46,"role":"input","accepts":["minecraft:flint"]},{"id":"buffer","x":44,"y":46,"role":"storage"},{"id":"out_a","x":98,"y":46,"role":"output"},{"id":"out_b","x":116,"y":46,"role":"output"}],"elements":[{"type":"label","x":8,"y":18,"text":"Machine"},{"type":"button","id":"boost","x":8,"y":30,"width":40,"height":12,"text":"Boost"},{"type":"progress","id":"work","x":80,"y":18,"width":60,"height":8,"value":50}]}"#,
        )?;
        Ok((
            Self {
                tick: 0,
                boost_until: 0,
            },
            Registration::new().on_tick(),
        ))
    }
    fn tick(&mut self, tick: u64) {
        self.tick = tick;
        if tick >= self.boost_until && tick.is_multiple_of(40) {
            let value = if (tick / 40).is_multiple_of(2) {
                20
            } else {
                80
            };
            for block in ["gui_fixture:machine", "gui_fixture:storage"] {
                let _ = workstations::set_progress(
                    block,
                    "minecraft:overworld",
                    0,
                    71,
                    2,
                    "work",
                    value,
                );
            }
        }
    }
    fn gui_button(
        &mut self,
        block: &str,
        _: &str,
        dimension: &str,
        pos: [i32; 3],
        id: &str,
    ) -> Result<(), String> {
        if block == "gui_fixture:machine" && id == "boost" {
            self.boost_until = self.tick + 200;
            workstations::set_progress(block, dimension, pos[0], pos[1], pos[2], "work", 99)?;
        }
        Ok(())
    }
    fn gui_workstation_preview(
        &mut self,
        block: &str,
        inputs: &[Option<WorkstationInput>],
        _: Option<&str>,
    ) -> Result<Option<GuiWorkstationPlan>, String> {
        if block != "gui_fixture:machine"
            || inputs.len() != 4
            || inputs[0]
                .as_ref()
                .is_none_or(|s| s.item != "minecraft:flint")
        {
            return Ok(None);
        }
        let output = |item: &str, count| GuiWorkstationOutput {
            item: item.into(),
            count,
            source_slot: None,
            text: None,
            tool: None,
            durability: None,
            melee: None,
        };
        Ok(Some(GuiWorkstationPlan {
            consume: vec![1, 0, 0, 0],
            outputs: vec![output("minecraft:diamond", 2), output("minecraft:stick", 3)],
        }))
    }
}
horizon_mod_sdk::export_server_mod!(Fixture);
