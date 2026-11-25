// Host-independent candidate selection, adapted from pinned MIT Horizon
// merchant::Trades::offers_from_set. License: pluto-lib/LICENSE.horizon.
// Inputs include native and registered factories BEFORE any selection.
pub fn roll<T: Clone, O, R>(mut potential: Vec<T>, amount: i32, duplicates: bool,
    random: &mut R, draw: impl Fn(&mut R, i32) -> i32,
    mut create: impl FnMut(&T, &mut R) -> Result<Option<O>, String>) -> Result<Vec<O>, String> {
    let mut out = Vec::new();
    while (out.len() as i32) < amount && !potential.is_empty() {
        let bound = i32::try_from(potential.len()).map_err(|_| "trade pool exceeds Java int")?;
        let index = usize::try_from(draw(random, bound)).ok().filter(|i| *i < potential.len())
            .ok_or("trade RNG returned an out-of-range index")?;
        let listing = if duplicates { potential[index].clone() } else { potential.remove(index) };
        match create(&listing, random)? {
            Some(offer) => out.push(offer),
            None if duplicates => { potential.remove(index); }
            None => {},
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    #[test]
    fn mixed_pool_preserves_weights_declines_duplicate_policy_and_single_rng() {
        // V is native; repeated C entries are registered weight, not offers.
        let mut rng = VecDeque::from([(3,1),(7,6),(2,0),(1,0)]);
        let made = roll(vec!['V','C','C'],3,false,&mut rng,
            |r,b| { let (bound,n)=r.pop_front().unwrap(); assert_eq!(bound,b); n },
            |entry,r| if *entry=='C' {
                // The selected callback consumes the very same RNG stream.
                if r.front().is_some_and(|(bound,_)| *bound==7) {
                    assert_eq!(r.pop_front(),Some((7,6))); Ok(Some("custom"))
                } else { Ok(None) }
            } else { Ok(Some("native")) }).unwrap();
        assert_eq!(made,["custom","native"]); assert!(rng.is_empty());
    }
    #[test]
    fn declines_terminate_errors_never_publish_a_prefix_and_zero_amount_never_rolls() {
        let mut bounds = Vec::new();
        let made = roll(vec!['C','V'],2,true,&mut bounds,
            |r,b| { r.push(b); 0 },
            |entry,_| Ok(if *entry=='C' { None } else { Some("native") })).unwrap();
        assert_eq!(made,["native","native"]); assert_eq!(bounds,[2,1,1]);
        let mut draws=0;
        let result=roll(vec![1,2],2,false,&mut draws,|r,_|{*r+=1;0},|entry,_|
            if *entry==1 { Ok(Some("prefix")) } else { Err("build failure".into()) });
        assert_eq!(result.unwrap_err(),"build failure"); assert_eq!(draws,2);
        assert!(roll::<_,(),_>(vec![1],0,false,&mut (),|_,_|panic!(),|_,_|panic!()).unwrap().is_empty());
        assert!(roll::<_,(),_>(vec![1],1,false,&mut (),|_,b|b,|_,_|panic!()).is_err());
    }
}
