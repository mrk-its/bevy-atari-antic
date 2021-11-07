use bytemuck::{Pod, Zeroable};
use crevice::std140::{Std140, Std140Padded};

use bevy::render2::color::Color;

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
    type Padded = Std140Padded<Self, 0>;
}
