use bevy::math::vec2;
use bevy::reflect::TypeUuid;
use bevy::render2::mesh::{Indices, Mesh};
use wgpu::PrimitiveTopology;

use super::resources::{AtariPalette, GTIA1Regs, GTIA2Regs, GTIA3Regs};

#[derive(TypeUuid, Clone, Debug)]
#[uuid = "bea612c2-68ed-4432-8d9c-f03ebea97043"]
pub struct AtariData {
    pub memory: Vec<u8>,
    pub palette: AtariPalette,
    pub gtia1: GTIA1Regs,
    pub gtia2: GTIA2Regs,
    pub gtia3: GTIA3Regs,
    pub positions: Vec<[f32; 3]>,
    pub custom: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
}

impl Default for AtariData {
    fn default() -> Self {
        // max 30 lines of text mode and 240 lines x 48 bytes / line
        // let texture_data = Vec::with_capacity(30 * 1024 + 240 * 48);
        Self {
            memory: Vec::with_capacity(16384),
            palette: AtariPalette::default(),
            gtia1: GTIA1Regs::default(),
            gtia2: GTIA2Regs::default(),
            gtia3: GTIA3Regs::default(),
            positions: Default::default(),
            custom: Default::default(),
            uvs: Default::default(),
            indices: Default::default(),
        }
    }
}

impl AtariData {
    pub fn reserve_antic_memory(&mut self, len: usize) -> &mut [u8] {
        assert!(self.memory.capacity() == 16384);
        let dst_offset = self.memory.len();
        assert!(dst_offset + len <= self.memory.capacity());
        unsafe {
            self.memory.set_len(dst_offset + len);
        }
        return &mut self.memory[dst_offset..dst_offset + len];
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.custom.clear();
        self.uvs.clear();
        self.indices.clear();
    }

    pub fn create_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.set_attribute("Vertex_ZCustom", self.custom.clone());
        mesh.set_indices(Some(Indices::U16(self.indices.clone())));
        mesh
    }

    pub fn insert_mode_line(
        &mut self,
        scan_line: usize,
        width: usize,
        height: usize,
        mode: u8,
        hscrol: u8,
        line_voffset: usize,
        video_memory_offset: usize,
        charset_memory_offset: usize,
    ) {
        let index_offset = self.positions.len() as u16;

        let scan_line_y = scan_line as f32 - 8.0;

        let north_west = vec2(-192.0, 120.0 - scan_line_y);
        let north_east = vec2(192.0, 120.0 - scan_line_y);
        let south_west = vec2(-192.0, 120.0 - (scan_line_y + height as f32));
        let south_east = vec2(192.0, 120.0 - (scan_line_y + height as f32));

        self.positions.push([south_west.x, south_west.y, 0.0]);
        self.positions.push([north_west.x, north_west.y, 0.0]);
        self.positions.push([north_east.x, north_east.y, 0.0]);
        self.positions.push([south_east.x, south_east.y, 0.0]);

        self.uvs.push([0.0, 1.0]);
        self.uvs.push([0.0, 0.0]);
        self.uvs.push([1.0, 0.0]);
        self.uvs.push([1.0, 1.0]);

        let scan_line = scan_line_y as u32;
        let height = height as u32;
        let width = width as u32 / 2;

        let b0 = (mode as u32 | (scan_line << 8) | (height << 16)) as f32;
        let b1 = (hscrol as u32 | ((line_voffset as u32) << 8) | (width << 16)) as f32;
        let b2 = video_memory_offset as f32;
        let b3 = charset_memory_offset as f32;

        self.custom.push([b0, b1, b2, b3]);
        self.custom.push([b0, b1, b2, b3]);
        self.custom.push([b0, b1, b2, b3]);
        self.custom.push([b0, b1, b2, b3]);

        self.indices.extend(
            [
                index_offset + 0,
                index_offset + 2,
                index_offset + 1,
                index_offset + 0,
                index_offset + 3,
                index_offset + 2,
            ]
            .iter(),
        );
    }
}
