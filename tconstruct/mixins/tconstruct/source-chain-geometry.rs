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

// Unexpanded source TreeAOEIterator: one-column trunk and bounded branches.
// World checks happen lazily between breaks, as in the original iterator.
const UP: Pos = [0,1,0];
const HORIZONTAL: [Pos;4] = [[0,0,-1],[1,0,0],[0,0,1],[-1,0,0]];
#[derive(Clone, Copy)]
struct TreePos { pos: Pos, dir: Pos, checked: bool }
pub(super) struct Tree {
    origin: Pos, queue: VecDeque<TreePos>, visited: HashSet<Pos>,
}
impl Tree {
    pub(super) fn new(origin: Pos) -> Self {
        let mut queue=VecDeque::new();
        if let Some(pos)=offset(origin, UP) { queue.push_back(TreePos {pos,dir:UP,checked:false}); }
        Self { origin, queue, visited:HashSet::new() }
    }
    fn outside(&self,p:Pos)->bool { p[0]!=self.origin[0] || p[2]!=self.origin[2] }
    fn is_branch(&mut self,p:Pos,same:&impl Fn(Pos)->bool)->bool {
        if !self.outside(p) { return false; }
        let d=i64::from(p[0]).abs_diff(i64::from(self.origin[0])) + i64::from(p[2]).abs_diff(i64::from(self.origin[2]));
        if d>10 || !self.visited.insert(p) { return false; }
        same(p)
    }
    fn try_branch(&mut self,p:Pos,dir:Pos,same:&impl Fn(Pos)->bool,occludes:&impl Fn(Pos)->bool) {
        if self.is_branch(p,same) && offset(p,[0,-1,0]).is_some_and(|below| !occludes(below)) {
            self.queue.push_back(TreePos{pos:p,dir,checked:true});
        }
    }
    fn follow(&mut self, options:&[Option<Pos>],dir:Pos,same:&impl Fn(Pos)->bool) {
        for p in options.iter().flatten().copied() {
            if self.is_branch(p,same) { self.queue.push_back(TreePos{pos:p,dir,checked:true}); break; }
        }
    }
    pub(super) fn next(&mut self,same:impl Fn(Pos)->bool,occludes:impl Fn(Pos)->bool)->Option<Pos> {
        while let Some(node)=self.queue.pop_front() {
            if node.dir==UP {
                let is_tree=node.checked || same(node.pos);
                for dir in HORIZONTAL { if let Some(p)=offset(node.pos,dir) {
                    self.try_branch(p,if is_tree {dir} else {UP},&same,&occludes);
                }}
                if is_tree {
                    for (dx,dz,dir) in [(-1,-1,[-1,0,0]),(-1,1,[-1,0,0]),(1,-1,[1,0,0]),(1,1,[1,0,0])] {
                        if let Some(p)=offset(node.pos,[dx,0,dz]) { self.try_branch(p,dir,&same,&occludes); }
                    }
                    if let Some(next)=offset(node.pos,UP) {
                        self.queue.push_back(TreePos{pos:next,dir:UP,checked:false});
                        if self.outside(next) { self.visited.insert(next); }
                    }
                    return Some(node.pos);
                }
            } else {
                let ahead=offset(node.pos,node.dir);
                self.follow(&[offset(node.pos,UP),ahead,ahead.and_then(|p|offset(p,UP))],node.dir,&same);
                let clockwise=[-node.dir[2],0,node.dir[0]];
                for side in [clockwise,[-clockwise[0],0,-clockwise[2]]] {
                    let p=offset(node.pos,side);let ahead=p.and_then(|p|offset(p,node.dir));
                    self.follow(&[p,p.and_then(|p|offset(p,UP)),ahead,ahead.and_then(|p|offset(p,UP))],side,&same);
                }
                return Some(node.pos);
            }
        }
        None
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;
    #[test] fn trunk_and_unobstructed_branch_preserve_original_type() {
        let logs=HashSet::from([[0,1,0],[0,2,0],[1,2,0],[2,2,0],[0,3,0]]);
        let solid=HashSet::from([[0,0,0]]);
        let mut tree=Tree::new([0,0,0]);let mut out=Vec::new();
        while let Some(p)=tree.next(|p|logs.contains(&p),|p|solid.contains(&p)) {out.push(p);}
        assert_eq!(out,vec![[0,1,0],[0,2,0],[1,2,0],[0,3,0],[2,2,0]]);
    }
    #[test] fn branch_supported_by_solid_block_is_not_selected() {
        let logs=HashSet::from([[0,1,0],[1,1,0]]);let solid=HashSet::from([[1,0,0]]);
        let mut tree=Tree::new([0,0,0]);let mut out=Vec::new();
        while let Some(p)=tree.next(|p|logs.contains(&p),|p|solid.contains(&p)) {out.push(p);}
        assert_eq!(out,vec![[0,1,0]]);
    }
}
