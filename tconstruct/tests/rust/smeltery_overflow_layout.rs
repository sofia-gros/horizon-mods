#[path = "../../mixins/tconstruct/source-smeltery-layout.rs"]
mod layout;

// Independent, literal pixel-removal algorithm from fixed GuiSmelteryTank.
// The wrapper supplies max(contained,capacity), then the source static method
// uses Java float division/multiplication, ceil, minimum 3, first-largest ties.
fn source_heights(amounts: &[u32], capacity: u32) -> Vec<u32> {
    if amounts.is_empty() { return Vec::new(); }
    let total: u64 = amounts.iter().map(|n| u64::from(*n)).sum();
    let denominator = total.max(u64::from(capacity));
    if denominator == 0 { return vec![0; amounts.len()]; }
    let mut heights: Vec<u32> = amounts.iter().map(|n|
        ((*n as f32 / denominator as f32 * 106.0).ceil() as u32).max(3)).collect();
    let available = if total < denominator { 103 } else { 106 };
    while heights.iter().sum::<u32>() > available {
        let mut largest = 0;
        for i in 1..heights.len() { if heights[i] > heights[largest] { largest = i; } }
        if heights[largest] == 0 { break; }
        heights[largest] -= 1;
    }
    heights
}

#[test]
fn overflow_layers_and_every_hover_pixel_match_source() {
    for (amounts, capacity) in [
        (vec![1800,200],1000), (vec![200,1800],1000),
        (vec![1,999,1000],1000), (vec![810,90,900],1),
        (vec![1000,1000],1000), (vec![90],180),
        (vec![90,90],180), (vec![1;40],20), (vec![1;108],50),
    ] {
        let expected = source_heights(&amounts,capacity);
        assert_eq!(layout::fluid_heights(&amounts,capacity),expected);
        for y in 16..122 {
            // Source getFluidFromMouse: bottom - mouse - 1, strict < height.
            let mut offset = 122 - y - 1;
            let mut selected = None;
            for (i,height) in expected.iter().enumerate() {
                if offset < *height as i32 { selected=Some(i); break; }
                offset -= *height as i32;
            }
            assert_eq!(layout::fluid_at(&amounts,capacity,y as f32),selected,"amounts={amounts:?}, y={y}");
        }
    }
    // This pixel previously selected the smaller upper fluid (84/22 split).
    assert_eq!(layout::fluid_at(&[1800,200],1000,30.0),Some(0));
    assert_eq!(layout::fluid_at(&[1800,200],1000,26.0),Some(1));
}

#[test]
fn optimized_layer_trimming_matches_literal_source_across_tanks() {
    let mut seed = 0x1234abcd_u32;
    for count in 1..=48 {
        for trial in 0..80 {
            let amounts: Vec<_> = (0..count).map(|_| {
                seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);
                1+seed%1_000_000
            }).collect();
            let total: u32 = amounts.iter().sum();
            let capacity = match trial%4 { 0=>total/2,1=>total,2=>total.saturating_mul(2),_=>0 };
            assert_eq!(layout::fluid_heights(&amounts,capacity),source_heights(&amounts,capacity));
        }
    }
}

#[test]
fn empty_and_oversized_native_values_are_bounded() {
    assert!(layout::fluid_heights(&[],0).is_empty());
    assert_eq!(layout::fluid_heights(&[0,0],0),vec![0,0]);
    assert_eq!(layout::fluid_heights(&[u32::MAX,u32::MAX],1),vec![53,53]);
    for y in [f32::NAN, f32::INFINITY, -1.0, 15.0, 122.0] {
        assert_eq!(layout::fluid_at(&[1800,200],1000,y),None);
    }
}
