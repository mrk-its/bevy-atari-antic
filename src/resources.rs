use bytemuck::{Pod, Zeroable};
use crevice::std140::{Std140, Std140Padded};

use bevy::{
    reflect::TypeUuid,
    render2::texture::Image,
    render2::color::Color,
    prelude::Handle,
};


#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct AtariPalette(pub [[f32; 4]; 256]);

impl Default for AtariPalette {
    fn default() -> Self {
        let data = include_bytes!("altirra.pal");
        let colors = data[..].chunks(3).map(|c| Color::rgba_u8(c[0], c[1], c[2], 255));
        let colors = colors.map(|c| c.as_linear_rgba_f32()).collect::<Vec<_>>();

        let mut arr = [[0f32; 4]; 256];
        for i in 0..256 {
            arr[i] = colors[i];
        }
        Self(arr)
    }
}

unsafe impl Std140 for AtariPalette {
    const ALIGNMENT: usize = 4 * 4 * 256;
    type Padded = Std140Padded<Self, 0>;
}


#[repr(C)]
#[derive(Default, Clone, Pod, Zeroable, Copy, Debug)]
pub struct GTIA1 {
    pub colors: [u32; 8],    // 32
    pub colors_pm: [u32; 4], // 16
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIA2 {
    pub player_size: [u32; 4], // 16
    pub missile_size: [u32; 4], // 16
    pub grafp: [u32; 4],
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIA3 {
    pub hposp: [f32; 4],     // 16
    pub hposm: [u32; 4],     // 16
    pub prior: u32,
    pub sizem: u32,
    pub grafm: u32,
    pub _fill: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct GTIA1Regs(pub [GTIA1; 240]);
impl Default for GTIA1Regs {
    fn default() -> Self {
        Self([GTIA1::default(); 240])
    }
}

unsafe impl Zeroable for GTIA1Regs {}
unsafe impl Pod for GTIA1Regs {}
unsafe impl Std140 for GTIA1Regs {
    const ALIGNMENT: usize = 240 * 3 * 16;
    type Padded = Std140Padded<Self, 0>;
}


#[derive(Clone, Copy, Debug)]
pub struct GTIA2Regs(pub [GTIA2; 240]);

impl Default for GTIA2Regs {
    fn default() -> Self {
        Self([GTIA2::default(); 240])
    }
}

unsafe impl Zeroable for GTIA2Regs {}
unsafe impl Pod for GTIA2Regs {}
unsafe impl Std140 for GTIA2Regs {
    const ALIGNMENT: usize = 240 * 3 * 16;
    type Padded = Std140Padded<Self, 0>;
}


#[derive(Clone, Copy, Debug)]
pub struct GTIA3Regs(pub [GTIA3; 240]);

impl Default for GTIA3Regs {
    fn default() -> Self {
        Self([GTIA3::default(); 240])
    }
}

unsafe impl Zeroable for GTIA3Regs {}
unsafe impl Pod for GTIA3Regs {}
unsafe impl Std140 for GTIA3Regs {
    const ALIGNMENT: usize = 240 * 3 * 16;
    type Padded = Std140Padded<Self, 0>;
}
