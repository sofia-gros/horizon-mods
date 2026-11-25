//! Counterpart of Fabric's `fabric-command-api-v2`: mods add commands when the server builds its command tree.
//!
//! Fabric hands a Brigadier dispatcher to `CommandRegistrationCallback.EVENT`. PlutoLib hands a [`CommandRegistrar`] and describes a command
//! with one usage line, as Horizon does: words, then arguments `<name:kind>`, optional ones last in brackets `[name:kind]`
//! (`pay <player:player> <amount:integer>`). The handler gets the arguments as text, in order, and returns the command's result (Fabric: the
//! `int` of `Command.run`; 1 for success).

use std::sync::{Arc, Mutex};

use super::base::Event;
use super::player_api::ServerPlayer;

/// What a command handler receives (Fabric: `CommandContext<ServerCommandSource>`).
#[derive(Clone, Debug)]
pub struct CommandContext {
    /// The player who ran the command (Fabric: `source.getPlayer()`).
    pub source: ServerPlayer,
    /// The argument words, in the order the usage lists them; an omitted optional argument is absent.
    pub args: Vec<String>,
}

type Handler = Arc<dyn Fn(&CommandContext) -> i32 + Send + Sync>;

#[derive(Clone)]
struct RegisteredCommand {
    usage: String,
    level: u8,
    handler: Handler,
}

/// Collects the commands one registration pass adds.
pub struct CommandRegistrar(Mutex<Vec<RegisteredCommand>>);

impl CommandRegistrar {
    /// A command every player may run.
    pub fn register(&self, usage: &str, handler: impl Fn(&CommandContext) -> i32 + Send + Sync + 'static) {
        self.register_op(usage, 0, handler);
    }

    /// A command for operators of `level` and above (Fabric: `.requires(source -> source.hasPermissionLevel(level))`).
    pub fn register_op(&self, usage: &str, level: u8, handler: impl Fn(&CommandContext) -> i32 + Send + Sync + 'static) {
        if let Ok(mut all) = self.0.lock() {
            all.push(RegisteredCommand { usage: usage.to_owned(), level, handler: Arc::new(handler) });
        }
    }
}

static REGISTRATION: Event<CommandRegistrar, ()> = Event::void();
static COMMANDS: Mutex<Vec<RegisteredCommand>> = Mutex::new(Vec::new());

/// Fabric: `CommandRegistrationCallback`.
pub struct CommandRegistrationCallback;

impl CommandRegistrationCallback {
    /// Fabric: `CommandRegistrationCallback.EVENT`: the server is building its command tree.
    pub const EVENT: &'static Event<CommandRegistrar, ()> = &REGISTRATION;
}

/// Runs the registration event and remembers the commands; returns `(usage, level)` for each.
pub(crate) fn collect() -> Vec<(String, u8)> {
    let registrar = CommandRegistrar(Mutex::new(Vec::new()));
    REGISTRATION.invoke(&registrar);
    let found = registrar.0.into_inner().unwrap_or_default();
    let listed = found.iter().map(|c| (c.usage.clone(), c.level)).collect();
    if let Ok(mut all) = COMMANDS.lock() {
        *all = found;
    }
    listed
}

/// Matches `line` (no slash) against the usage: the literal words equal, then arguments. None if it is not this command.
fn match_usage(usage: &str, line: &str) -> Option<Vec<String>> {
    let wanted: Vec<&str> = usage.split_whitespace().collect();
    let given: Vec<&str> = line.split_whitespace().collect();
    let first_arg = wanted.iter().position(|w| w.starts_with('<') || w.starts_with('[')).unwrap_or(wanted.len());
    if given.len() < first_arg || wanted[..first_arg] != given[..first_arg] {
        return None;
    }
    let argument_words = &wanted[first_arg..];
    let required = argument_words.iter().filter(|w| w.starts_with('<')).count();
    let rest = &given[first_arg..];
    let last_is_text = argument_words.last().is_some_and(|w| w.contains(":text"));
    if rest.len() < required || (rest.len() > argument_words.len() && !last_is_text) {
        return None;
    }
    let mut args: Vec<String> = rest.iter().take(argument_words.len()).map(|w| (*w).to_owned()).collect();
    if last_is_text && rest.len() > argument_words.len() {
        let keep = argument_words.len() - 1;
        args.truncate(keep);
        args.push(rest[keep..].join(" "));
    }
    Some(args)
}

/// Runs the registered command that `line` names, for `player`; false if none does.
pub(crate) fn execute(uuid: u128, name: String, line: &str) -> bool {
    let commands = match COMMANDS.lock() {
        Ok(all) => all.clone(),
        Err(_) => return false,
    };
    for command in commands {
        if let Some(args) = match_usage(&command.usage, line) {
            (command.handler)(&CommandContext { source: ServerPlayer { uuid, name }, args });
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_commands_match_exactly_and_arguments_are_collected_in_order() {
        assert_eq!(match_usage("tconstruct parts", "tconstruct parts"), Some(vec![]));
        assert_eq!(match_usage("tconstruct parts", "tconstruct assemble"), None);
        assert_eq!(match_usage("tconstruct parts", "tconstruct parts extra"), None);
        assert_eq!(match_usage("pay <player:player> <amount:integer>", "pay Alex 5"), Some(vec!["Alex".into(), "5".into()]));
        assert_eq!(match_usage("pay <player:player> <amount:integer>", "pay Alex"), None);
        assert_eq!(match_usage("balance [player:player]", "balance"), Some(vec![]));
        assert_eq!(match_usage("balance [player:player]", "balance Alex"), Some(vec!["Alex".into()]));
        assert_eq!(match_usage("say <message:text>", "say hello there world"), Some(vec!["hello there world".into()]));
    }

    #[test]
    fn registered_commands_run_for_the_player_with_their_arguments_and_unknown_lines_are_not_taken() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        CommandRegistrationCallback::EVENT.register(move |registrar| {
            let sink = sink.clone();
            registrar.register("cmdtest greet <who:word>", move |ctx| {
                sink.lock().unwrap().push((ctx.source.uuid, ctx.args.clone()));
                1
            });
            registrar.register_op("cmdtest admin", 2, |_| 1);
        });
        let listed = collect();
        assert!(listed.contains(&("cmdtest greet <who:word>".to_owned(), 0)));
        assert!(listed.contains(&("cmdtest admin".to_owned(), 2)));
        assert!(execute(7, "p".into(), "cmdtest greet Sam"));
        assert!(!execute(7, "p".into(), "cmdtest nothing"));
        assert_eq!(*seen.lock().unwrap(), [(7, vec!["Sam".to_owned()])]);
    }
}
