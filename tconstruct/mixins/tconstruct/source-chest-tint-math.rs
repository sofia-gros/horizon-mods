pub(super) fn apply(packed: u32, color: u32) -> u32 {
    let red=(packed&255)*((color>>16)&255)/255;
    let green=((packed>>8)&255)*((color>>8)&255)/255;
    let blue=((packed>>16)&255)*(color&255)/255;
    (packed&0xff00_0000)|(blue<<16)|(green<<8)|red
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_default_multiplies_shading_and_keeps_emission() {
        assert_eq!(apply(0xf0ff_ffff,0x407686),0xf086_7640);
        assert_eq!(apply(0xa080_8080,0xff0000),0xa000_0080);
    }
}
