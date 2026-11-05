// Pinned OverburnModule arithmetic. Fuel durations use world game time.
pub(super) const ID: &str = "tconstruct:overburn";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fuel {
    pub amount: u32,
    pub duration: u32,
    pub rate: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FuelInfo {
    pub expiration: i64,
    pub rate: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FuelStep {
    pub consumed: u32,
    pub active: Option<FuelInfo>,
    pub saved: Option<FuelInfo>,
}

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()).is_some_and(|v| v == serde_json::json!({
        "level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:overburn"},
            {"type":"tconstruct:stat_boost","flat":1000.0,"operation":"add","stat":"tconstruct:tank_capacity"},
            {"type":"tconstruct:trait","fixed_level":true,"level":1,"name":"tconstruct:tank_handler"}
        ]
    }))
}
pub(super) fn advance(phase: &mut u16) -> u16 {
    *phase = (*phase % 160 + 1) % 160;
    *phase
}
pub(super) fn interval(level: i32) -> Option<u32> {
    (level > 0).then(|| 5 << (6 - level.min(6)))
}
pub(super) fn fuel_step(
    now: i64,
    interval: u32,
    old: Option<FuelInfo>,
    fuel: Option<(u32, Fuel)>,
) -> Option<FuelStep> {
    if ![5, 10, 20, 40, 80, 160].contains(&interval) {
        return None;
    }
    if let Some(info) = old {
        if info.rate == 0 || info.rate > i32::MAX as u32 {
            return None;
        }
        // Expiration is inclusive in the pinned source.
        if info.expiration >= now {
            return Some(FuelStep {
                consumed: 0,
                active: old,
                saved: old,
            });
        }
    }
    let Some((available, fuel)) = fuel else {
        return Some(FuelStep {
            consumed: 0,
            active: None,
            saved: None,
        });
    };
    if available == 0
        || fuel.amount == 0
        || fuel.duration == 0
        || fuel.rate == 0
        || fuel.rate > i32::MAX as u32
    {
        return None;
    }
    let consumed = available.min(fuel.amount);
    let mut duration = i32::try_from(fuel.duration)
        .ok()?
        .checked_mul(interval as i32)?
        .checked_div(5)?;
    if available < fuel.amount {
        duration = duration
            .checked_mul(i32::try_from(available).ok()?)?
            .checked_div(i32::try_from(fuel.amount).ok()?)?;
    }
    let info = FuelInfo {
        expiration: now.checked_add(i64::from(duration))?,
        rate: fuel.rate,
    };
    let saved = (info.expiration >= now.checked_add(i64::from(interval))?).then_some(info);
    Some(FuelStep {
        consumed,
        active: Some(info),
        saved,
    })
}
pub(super) fn restore(rate: u32, overworked: i32, roll: Option<u32>) -> Option<u32> {
    if rate == 0 || rate > i32::MAX as u32 || overworked < 0 {
        return None;
    }
    let remainder = rate % 10;
    let bonus = if remainder == 0 {
        0
    } else {
        let roll = roll?;
        if roll >= 10 {
            return None;
        }
        u32::from(roll < remainder)
    };
    (rate / 10 + bonus).checked_mul(u32::try_from(overworked.checked_add(1)?).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lava() -> Fuel {
        Fuel {
            amount: 50,
            duration: 100,
            rate: 10,
        }
    }
    #[test]
    fn cadence_caps_at_six_and_is_per_holder() {
        assert_eq!(
            (1..=7).map(interval).collect::<Vec<_>>(),
            vec![
                Some(160),
                Some(80),
                Some(40),
                Some(20),
                Some(10),
                Some(5),
                Some(5)
            ]
        );
        assert_eq!(interval(0), None);
        assert_eq!(interval(-1), None);
        let (mut a, mut b) = (0, 0);
        for n in 1..160 {
            assert_eq!(advance(&mut a), n);
        }
        assert_eq!(advance(&mut a), 0);
        assert_eq!(advance(&mut b), 1);
    }
    #[test]
    fn source_scales_duration_and_keeps_expiration_inclusive() {
        let s = fuel_step(1000, 160, None, Some((1000, lava()))).unwrap();
        assert_eq!(s.consumed, 50);
        assert_eq!(
            s.saved,
            Some(FuelInfo {
                expiration: 4200,
                rate: 10
            })
        );
        let same = fuel_step(4200, 160, s.saved, None).unwrap();
        assert_eq!(same.consumed, 0);
        assert_eq!(same.active, s.saved);
        assert_eq!(fuel_step(4201, 160, s.saved, None).unwrap().active, None);
        assert_eq!(
            fuel_step(0, 20, None, Some((50, lava())))
                .unwrap()
                .saved
                .unwrap()
                .expiration,
            400
        );
    }
    #[test]
    fn partial_fuel_consumes_only_available_and_short_fuel_still_restores_once() {
        let s = fuel_step(0, 160, None, Some((25, lava()))).unwrap();
        assert_eq!(s.consumed, 25);
        assert_eq!(s.saved.unwrap().expiration, 1600);
        let s = fuel_step(0, 160, None, Some((1, lava()))).unwrap();
        assert_eq!(s.consumed, 1);
        assert_eq!(s.active.unwrap().expiration, 64);
        assert_eq!(s.saved, None);
    }
    #[test]
    fn remainder_roll_and_overworked_match_source() {
        assert_eq!(restore(10, 0, None), Some(1));
        assert_eq!(restore(30, 2, None), Some(9));
        assert_eq!(
            (0..10)
                .map(|r| restore(13, 0, Some(r)).unwrap())
                .sum::<u32>(),
            13
        );
        assert_eq!(restore(3, 2, Some(2)), Some(3));
        assert_eq!(restore(3, 2, Some(3)), Some(0));
        assert_eq!(restore(13, 0, None), None);
        assert_eq!(restore(13, 0, Some(10)), None);
    }
    #[test]
    fn arithmetic_failure_is_non_consuming() {
        let mut fuel = lava();
        fuel.duration = u32::MAX;
        assert_eq!(fuel_step(0, 160, None, Some((50, fuel))), None);
        assert_eq!(fuel_step(i64::MAX, 160, None, Some((50, lava()))), None);
        assert_eq!(fuel_step(0, 7, None, Some((50, lava()))), None);
        assert_eq!(restore(10, i32::MAX, None), None);
    }
    #[test]
    fn definition_changes_fail_closed() {
        let bytes=br#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:overburn"},{"type":"tconstruct:stat_boost","flat":1000.0,"operation":"add","stat":"tconstruct:tank_capacity"},{"type":"tconstruct:trait","fixed_level":true,"level":1,"name":"tconstruct:tank_handler"}]}"#;
        assert!(known(Some(bytes)));
        assert!(!known(None));
        assert!(!known(Some(b"{}")));
        let mut v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        v["modules"][1]["flat"] = serde_json::json!(2000.0);
        assert!(!known(Some(&serde_json::to_vec(&v).unwrap())));
    }
}
