use crate::{Catalog, diagnostic};
use horizon_mod_sdk::{Mod, Registration};

struct Diagnostics {
    catalog: Catalog,
}

impl Mod for Diagnostics {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Self {
                catalog: Catalog::embedded()?,
            },
            Registration::new().command("tconstruct"),
        ))
    }
    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        diagnostic(&self.catalog, args).map(Some)
    }
}

horizon_mod_sdk::export_mod!(Diagnostics);
