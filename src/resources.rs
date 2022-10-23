use bevy::render::color::Color;
use bytemuck::{Pod, Zeroable};
use crevice::std140::{Std140, AsStd140};

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct AtariPalette(pub [[f32; 4]; 256]);

impl Default for AtariPalette {
    fn default() -> Self {
        let data = include_bytes!("altirra.pal");
        let colors = data[..]
            .chunks(3)
            .map(|c| Color::rgba_u8(c[0], c[1], c[2], 255));
        let colors = colors.map(|c| c.as_linear_rgba_f32()).collect::<Vec<_>>();

        let mut arr = [[0f32; 4]; 256];
        arr.clone_from_slice(&colors[..256]);
        Self(arr)
    }
}

unsafe impl Std140 for AtariPalette {
    const ALIGNMENT: usize = 4 * 4 * 256;
}


#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod, AsStd140, PartialEq, Eq)]
pub struct AnticConfig {
    pub debug_scan_line: i32,
    pub cnt: i32,
    pub _padding_1: i32,
    pub _padding_2: i32,
}

impl Default for AnticConfig {
    fn default() -> Self {
        Self { debug_scan_line: 8, cnt: 0, _padding_1: 0, _padding_2: 0}
    }
}
