use crate::{Catalog, diagnostic, specimen};
use horizon_mod_sdk::{Mod, Registration, command};

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
        if args.split_whitespace().next() == Some("specimen") {
            command::run(&specimen::command(&self.catalog, args)?);
            return Ok(Some(specimen::FEEDBACK.into()));
        }
        diagnostic(&self.catalog, args).map(Some)
    }
}

horizon_mod_sdk::export_mod!(Diagnostics);
