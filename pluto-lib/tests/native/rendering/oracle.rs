// Frozen arithmetic from horizon-mods 1e870214. See oracle-sources.json.
// Only parameters/imports replace the enclosing Game, atlas and tank lookup.
use horizon_client::hud::{HudBatch, Rect};
use horizon_client::mesher::Vertex;

pub fn tiles(batch: &mut HudBatch, sprite: Rect, ox: f32, oy: f32,
    layer: f32, width: f32, height: f32, color: u32) {
    let (du,dv)=((sprite.u1-sprite.u0)/sprite.w as f32,(sprite.v1-sprite.v0)/sprite.h as f32);
    let mut yy=0.0;
    while yy<height { let h=(height-yy).min(16.0); let mut x=0.0;
        while x<width { let w=(width-x).min(16.0); let mut part=sprite;
            part.u1=part.u0+w*du; part.v1=part.v0+h*dv;
            batch.rect_uv(ox+8.0+x,oy+122.0-layer-yy-h,w,h,part,color); x+=w;
        } yy+=h;
    }
}

pub fn narrow(batch: &mut HudBatch, sprite: Rect, ox: f32, oy: f32,
    height: f32, color: u32, fuel: bool) {
    let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
    let mut y = 0.0;
    while y < height {
        let h = (height - y).min(16.0); let mut part = sprite;
        let width = if fuel { 16.0 } else { 12.0 };
        part.u1 = part.u0 + width * du; part.v1 = part.v0 + h * dv;
        if fuel { batch.rect_uv(ox + 152.0, oy + 122.0 - y - h, width, h, part, color); }
        else { batch.rect_uv(ox + 153.0, oy + 68.0 - y - h, width, h, part, color); }
        y += h;
    }
}

pub fn mask(batch: &mut HudBatch, sprite: Rect, x: f32, y: f32, color: u32) {
    for (left, top, width) in [(6., 3., 4.), (5., 4., 6.), (5., 5., 6.), (6., 6., 4.)] {
        let mut clipped = sprite;
        let du = (sprite.u1 - sprite.u0) / 16.0;
        let dv = (sprite.v1 - sprite.v0) / 16.0;
        clipped.u0 = sprite.u0 + left * du;
        clipped.u1 = clipped.u0 + width * du;
        clipped.v0 = sprite.v0 + top * dv;
        clipped.v1 = clipped.v0 + dv;
        batch.rect_uv(x + left, y + top, width, 1.0, clipped, color);
    }
}

pub fn mesh(out: &mut Vec<Vertex>, sprite: Rect, origin: glam::Vec3,
    bounds: [f32;6], sides: bool, color: u32, light: u32) {
    let [x0,bottom,z0,x1,top,z1] = bounds;
    let u0 = sprite.u0; let u1 = sprite.u0 + (sprite.u1 - sprite.u0) * (16.0 / sprite.w as f32);
    let v0 = sprite.v0; let v1 = sprite.v0 + (sprite.v1 - sprite.v0) * (16.0 / sprite.h as f32);
    let uv = [[u0,v0], [u1,v0], [u1,v1], [u0,v1]];
    let faces = [
        [[x0,top,z0], [x1,top,z0], [x1,top,z1], [x0,top,z1]],
        [[x0,top,z0], [x0,top,z1], [x0,bottom,z1], [x0,bottom,z0]],
        [[x1,top,z1], [x1,top,z0], [x1,bottom,z0], [x1,bottom,z1]],
        [[x1,top,z0], [x0,top,z0], [x0,bottom,z0], [x1,bottom,z0]],
        [[x0,top,z1], [x1,top,z1], [x1,bottom,z1], [x0,bottom,z1]],
    ];
    for face in faces.into_iter().take(if sides { 5 } else { 1 }) {
        for (p, uv) in face.into_iter().zip(uv) {
            out.push(Vertex { pos: (origin + glam::Vec3::from_array(p)).to_array(), uv, layer: 0,
                color, light, normal: [0, 0, 0, 0], glint_uv: [0.0; 2] });
        }
    }
}

pub fn light(world_light: u8, minimum: u8) -> u32 {
    let sky = u32::from(world_light >> 4) * 16;
    let block = u32::from((world_light & 15).max(minimum)) * 16;
    sky | (block << 8)
}
