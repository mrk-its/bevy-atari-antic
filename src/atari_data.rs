use std::sync::Arc;

use bevy::math::vec2;
use bevy::reflect::TypeUuid;
use bevy::render2::mesh::{Indices, Mesh};
use parking_lot::RwLock;
use wgpu::PrimitiveTopology;

use super::resources::AtariPalette;

use crate::AnticMesh;

pub const MEMORY_UNIFORM_SIZE: usize = 16384;

#[derive(Default)]
pub struct AnticDataInner {
    pub memory: Vec<u8>,
    pub memory_used: usize,
    pub palette: AtariPalette,
    pub positions: Vec<[f32; 3]>,
    pub custom: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "bea612c2-68ed-4432-8d9c-f03ebea97043"]
pub struct AnticData {
    pub inner: Arc<RwLock<AnticDataInner>>,
}

const GTIA_REGS_MEMORY: usize = 240 * 32;

impl Default for AnticData {
    fn default() -> Self {
        // max 30 lines of text mode and 240 lines x 48 bytes / line
        // let texture_data = Vec::with_capacity(30 * 1024 + 240 * 48);
        let mut memory = Vec::with_capacity(GTIA_REGS_MEMORY + 256 * 11 * 4 * 4);
        memory.resize(memory.capacity(), 0);
        Self {
            inner: Arc::new(RwLock::new(AnticDataInner {
                memory,
                memory_used: 0,
                palette: AtariPalette::default(),
                positions: Default::default(),
                custom: Default::default(),
                uvs: Default::default(),
                indices: Default::default(),
            })),
        }
    }
}

impl AnticData {
    pub fn set_gtia_regs(&mut self, scan_line: usize, regs: &crate::GTIARegs) {
        assert!(std::mem::size_of::<crate::GTIARegs>() == 32);
        let mut inner = self.inner.write();
        let ptr = inner.memory.as_mut_ptr() as *mut crate::GTIARegs;
        unsafe { *ptr.offset(scan_line as isize) = *regs }
    }

    pub fn reserve_antic_memory(&mut self, len: usize, cb: &mut dyn FnMut(&mut [u8])) -> usize {
        let mut inner = self.inner.write();
        let dst_offset = GTIA_REGS_MEMORY + inner.memory_used;
        assert!(dst_offset + len <= inner.memory.len());
        inner.memory_used += len;

        cb(&mut inner.memory[dst_offset..dst_offset + len]);
        // bevy::utils::tracing::info!("antic memory offs: {}, len: {}", dst_offset, len);
        dst_offset - GTIA_REGS_MEMORY
    }

    pub fn clear(&mut self) {
        let mut inner = self.inner.write();
        inner.memory_used = 0;
        inner.positions.clear();
        inner.custom.clear();
        inner.uvs.clear();
        inner.indices.clear();
    }

    pub fn create_mesh(&self) -> AnticMesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let inner = self.inner.read();
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, inner.positions.clone());
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, inner.uvs.clone());
        mesh.set_attribute("Vertex_ZCustom", inner.custom.clone());
        mesh.set_indices(Some(Indices::U16(inner.indices.clone())));
        AnticMesh(mesh)
    }

    pub fn insert_mode_line(&mut self, mode_line: &crate::ModeLineDescr) {
        let mut inner = self.inner.write();
        let index_offset = inner.positions.len() as u16;

        let scan_line_y = mode_line.scan_line as f32 - 8.0;

        let north_west = vec2(-192.0, 120.0 - scan_line_y);
        let north_east = vec2(192.0, 120.0 - scan_line_y);
        let south_west = vec2(-192.0, 120.0 - (scan_line_y + mode_line.height as f32));
        let south_east = vec2(192.0, 120.0 - (scan_line_y + mode_line.height as f32));

        inner.positions.push([south_west.x, south_west.y, 0.0]);
        inner.positions.push([north_west.x, north_west.y, 0.0]);
        inner.positions.push([north_east.x, north_east.y, 0.0]);
        inner.positions.push([south_east.x, south_east.y, 0.0]);

        inner.uvs.push([0.0, 1.0]);
        inner.uvs.push([0.0, 0.0]);
        inner.uvs.push([1.0, 0.0]);
        inner.uvs.push([1.0, 1.0]);

        let scan_line = scan_line_y as u32;
        let height = mode_line.height as u32;
        let width = mode_line.width as u32 / 2;

        let b0 = (mode_line.mode as u32 | (scan_line << 8) | (height << 16)) as f32;
        let b1 = (mode_line.hscrol as u32 | ((mode_line.line_voffset as u32) << 8) | (width << 16))
            as f32;
        let b2 = mode_line.video_memory_offset as f32;
        let b3 = mode_line.charset_memory_offset as f32;

        inner.custom.push([b0, b1, b2, b3]);
        inner.custom.push([b0, b1, b2, b3]);
        inner.custom.push([b0, b1, b2, b3]);
        inner.custom.push([b0, b1, b2, b3]);

        inner.indices.extend(
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
