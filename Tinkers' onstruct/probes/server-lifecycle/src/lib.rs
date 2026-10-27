//! Experimental API 0.3 probe. This has no world or inventory mutations.
#![cfg(target_arch = "wasm32")]
use horizon_mod_sdk::{
    server::{Args, BlockPos, Player, Registration, ServerMod, players},
    storage,
};

struct Probe {
    ticks: u64,
    last_tick: Option<u64>,
    duplicate_ticks: u64,
    joins: u64,
    leaves: u64,
    chats: u64,
    broken: u64,
    placed: u64,
    boots: u64,
    last: String,
}

impl ServerMod for Probe {
    fn init() -> Result<(Self, Registration), String> {
        let previous = storage::get("boots")
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let boots = previous
            .checked_add(1)
            .ok_or("probe boot counter exhausted")?;
        storage::set("boots", boots.to_string().as_bytes())?;
        Ok((
            Self {
                ticks: 0,
                last_tick: None,
                duplicate_ticks: 0,
                joins: 0,
                leaves: 0,
                chats: 0,
                broken: 0,
                placed: 0,
                boots,
                last: String::new(),
            },
            Registration::new()
                .command("tprobe")
                .on_tick()
                .on_joins()
                .on_chat()
                .on_blocks(),
        ))
    }
    fn tick(&mut self, tick: u64) {
        if self.last_tick.is_some_and(|previous| tick <= previous) {
            self.duplicate_ticks += 1;
        }
        self.last_tick = Some(tick);
        self.ticks += 1;
    }
    fn player_joined(&mut self, p: &Player) {
        self.joins += 1;
        self.last = format!("join:{}:{}", p.name, p.dimension);
        players::tell(&p.id, "M2 server probe joined");
    }
    fn player_left(&mut self, p: &Player) {
        self.leaves += 1;
        self.last = format!("leave:{}:{}", p.name, p.dimension);
    }
    fn chat(&mut self, p: &Player, text: &str) {
        self.chats += 1;
        self.last = format!("chat:{}:{}:{}", p.name, p.dimension, text);
    }
    fn block_broken(&mut self, p: &Player, pos: BlockPos, state: &str) {
        self.broken += 1;
        self.last = format!(
            "break:{}:{}:{},{},{}:{}",
            p.name, p.dimension, pos.x, pos.y, pos.z, state
        );
    }
    fn block_placed(&mut self, p: &Player, pos: BlockPos, state: &str) {
        self.placed += 1;
        self.last = format!(
            "place:{}:{}:{},{},{}:{}",
            p.name, p.dimension, pos.x, pos.y, pos.z, state
        );
    }
    fn command(&mut self, p: &Player, _: &str, args: &Args) -> Result<Option<String>, String> {
        let args = args.text();
        let (verb, request) = args.trim().split_once(' ').unwrap_or((args.trim(), ""));
        match verb {
            "" | "status" => Ok(Some(serde_json::json!({"probe": "M2", "request": request, "ticks": self.ticks,
                "last_tick": self.last_tick, "duplicate_ticks": self.duplicate_ticks,
                "joins": self.joins, "leaves": self.leaves, "chats": self.chats, "broken": self.broken,
                "placed": self.placed, "boots": self.boots, "last": self.last, "actor": p.name,
                "dimension": p.dimension, "online": players::online().iter().map(|p| &p.name).collect::<Vec<_>>()}).to_string())),
            "tell" => { players::tell(&p.id, "M2 targeted reply"); Ok(Some("tell sent".into())) }
            "broadcast" => { players::broadcast("M2 broadcast reply"); Ok(Some("broadcast sent".into())) }
            "error" => Err("M2 expected diagnostic error".into()),
            _ => Err("usage: /tprobe status|tell|broadcast|error".into()),
        }
    }
}

horizon_mod_sdk::export_server_mod!(Probe);
