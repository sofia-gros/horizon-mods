// Source VeiningAOEIterator: same-block, six-way BFS by graph distance.
use std::collections::{HashSet, VecDeque};
type Pos = [i32; 3];
const DIRECTIONS: [Pos; 6] = [[0,-1,0],[0,1,0],[0,0,-1],[0,0,1],[-1,0,0],[1,0,0]];
fn offset(pos: Pos, dir: Pos) -> Option<Pos> {
    Some([pos[0].checked_add(dir[0])?, pos[1].checked_add(dir[1])?, pos[2].checked_add(dir[2])?])
}

pub(super) struct Vein {
    queue: VecDeque<(Pos,u32)>, visited: HashSet<Pos>, max_distance: u32,
}
impl Vein {
    pub(super) fn new(origin: Pos, max_distance: u32) -> Self {
        let mut out = Self { queue: VecDeque::new(), visited: HashSet::from([origin]), max_distance };
        if max_distance > 0 { out.enqueue(origin, 1); }
        out
    }
    fn enqueue(&mut self, pos: Pos, distance: u32) {
        for dir in DIRECTIONS { if let Some(next) = offset(pos, dir) {
            if self.visited.insert(next) { self.queue.push_back((next, distance)); }
        }}
    }
    pub(super) fn next(&mut self, same: impl Fn(Pos) -> bool) -> Option<Pos> {
        while let Some((pos, distance)) = self.queue.pop_front() {
            if same(pos) {
                if distance < self.max_distance { self.enqueue(pos, distance + 1); }
                return Some(pos);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn vein_uses_same_block_six_way_graph_distance_and_order() {
        let origin = [0,0,0];
        let blocks = HashSet::from([[0,1,0],[0,2,0],[0,3,0],[1,1,0],[1,0,1],[2,0,0]]);
        let mut v=Vein::new(origin,2); let mut out=Vec::new();
        while let Some(p)=v.next(|p| blocks.contains(&p)) { out.push(p); }
        assert_eq!(out,vec![[0,1,0],[0,2,0],[1,1,0]]);
        assert!(!out.contains(&[0,3,0])); assert!(!out.contains(&[1,0,1]));
    }
    #[test] fn zero_distance_never_finds_neighbors() {
        assert_eq!(Vein::new([0,0,0],0).next(|_| true),None);
    }
}
