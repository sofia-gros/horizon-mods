//! Shapes from the pinned TConstruct FaucetBlock and casting blocks.
#[derive(Clone)]
pub(super) struct Shape { pub grid: Vec<i8>, pub boxes: Vec<[f64; 6]>, pub edges: Vec<[f64; 6]> }

fn inside(p: [usize; 3], b: [usize; 6]) -> bool {
    (0..3).all(|i| p[i] >= b[i] && p[i] < b[i+3])
}
fn filled(kind: &str, facing: &str, p: [usize; 3]) -> bool {
    let (outer, holes): ([usize; 6], Vec<[usize; 6]>) = match kind {
        "tconstruct:seared_faucet" => {
            let (a,b) = match facing {
                "down" => ([4,10,4,12,16,12], [6,10,6,10,16,10]),
                "north" => ([4,4,10,12,10,16], [6,6,10,10,10,16]),
                "south" => ([4,4,0,12,10,6], [6,6,0,10,10,6]),
                "west" => ([10,4,4,16,10,12], [10,6,6,16,10,10]),
                "east" => ([0,4,4,6,10,12], [0,6,6,6,10,10]),
                _ => return false,
            }; (a, vec![b])
        },
        "tconstruct:seared_basin" => ([0,0,0,16,16,16], vec![[0,0,5,16,2,11],[5,0,0,11,2,16],[2,4,2,14,16,14]]),
        "tconstruct:seared_table" => ([0,0,0,16,16,16], vec![[4,0,0,12,10,16],[0,0,4,16,10,12],[1,15,1,15,16,15]]),
        _ => return false,
    };
    inside(p, outer) && !holes.into_iter().any(|b| inside(p,b))
}

pub(super) fn shape(kind: &str, facing: &str) -> Option<Shape> {
    if !matches!(kind, "tconstruct:seared_faucet" | "tconstruct:seared_basin" | "tconstruct:seared_table") { return None; }
    if kind.ends_with("faucet") && !matches!(facing, "down"|"north"|"south"|"west"|"east") { return None; }
    let mut grid = vec![0i8; 4096];
    let mut boxes = Vec::new();
    let scale = |v: usize| v as f64 / 16.0;
    for x in 0..16 { for y in 0..16 { for z in 0..16 {
        if filled(kind, facing, [x,y,z]) {
            grid[(x*16+y)*16+z] = 1;

        }
    }}}
    // Collapse adjacent Z cells into boxes; selection tests need the union,
    // not thousands of individual voxel boxes.
    for x in 0..16 { for y in 0..16 {
        let mut start = None;
        for z in 0..=16 {
            let full = z < 16 && grid[(x*16+y)*16+z] != 0;
            if full && start.is_none() { start = Some(z); }
            if !full { if let Some(first) = start.take() {
                boxes.push([scale(x),scale(y),scale(first),scale(x+1),scale(y+1),scale(z)]);
            }}
        }
    }}
    let at = |p: [i32;3]| -> bool {
        if p.iter().any(|v| *v < 0 || *v >= 16) { return false; }
        grid[(p[0] as usize*16+p[1] as usize)*16+p[2] as usize] != 0
    };
    let mut edges = Vec::new();
    for axis in 0..3 { let a = (axis+1)%3; let b = (axis+2)%3;
        for u in 0..=16 { for v in 0..=16 {
            let mut start = None;
            for t in 0..=16 {
                let boundary = if t == 16 { false } else {
                    let mut p = [0i32;3]; p[axis]=t; p[a]=u; p[b]=v;
                    let mut q = p; q[a]-=1; let mut r=p; r[b]-=1; let mut s=q; s[b]-=1;
                    let (w,x,y,z)=(at(p),at(q),at(r),at(s));
                    let count = w as u8+x as u8+y as u8+z as u8;
                    count==1 || count==3 || (count==2 && w==z && x==y)
                };
                if boundary && start.is_none() { start=Some(t); }
                if !boundary { if let Some(begin)=start.take() {
                    let mut line=[0f64;6]; line[a]=scale(u as usize); line[b]=scale(v as usize);
                    line[a+3]=line[a]; line[b+3]=line[b]; line[axis]=scale(begin as usize); line[axis+3]=scale(t as usize);
                    edges.push(line);
                }}
            }
        }}
    }
    Some(Shape {grid,boxes,edges})
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn source_faucet_direction_and_opening() {
  for direction in ["north","south","east","west","down"] {
   let s=shape("tconstruct:seared_faucet",direction).unwrap();
   assert_eq!(s.grid.iter().filter(|x| **x!=0).count(), if direction == "down" { 288 } else { 192 });
   assert!(!s.edges.is_empty());
  }
  assert!(!filled("tconstruct:seared_faucet","north",[8,8,12]));
  assert!(filled("tconstruct:seared_faucet","north",[5,8,12]));
  assert!(!filled("tconstruct:seared_faucet","north",[8,8,2]));
 }
 #[test] fn basin_hollow_table_legs() {
  assert!(!filled("tconstruct:seared_basin","north",[8,8,8]));
  assert!(filled("tconstruct:seared_basin","north",[1,8,8]));
  assert!(!filled("tconstruct:seared_table","north",[8,4,8]));
  assert!(filled("tconstruct:seared_table","north",[1,4,1]));
 }
}
