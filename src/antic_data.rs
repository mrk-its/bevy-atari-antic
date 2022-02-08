use std::sync::Arc;

use bevy::{
    math::vec2,
    prelude::Handle,
    reflect::TypeUuid,
    render::{
        mesh::{Indices, Mesh},
        render_resource::Buffer,
        renderer::RenderDevice,
        texture::Image,
    },
};
use futures_lite::future;
use parking_lot::RwLock;
use wgpu::{BufferDescriptor, BufferUsages, PrimitiveTopology};

use super::resources::{AnticConfig, AtariPalette};
use crate::ModeLineDescr;

#[derive(Default)]
pub struct AnticDataInner {
    pub scanlines: usize,
    pub memory: Vec<u8>,
    pub memory_used: usize,
    pub palette: AtariPalette,
    pub positions: Vec<[f32; 3]>,
    pub custom: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
}

pub struct CollisionsDataInner {
    pub data: [u64; 240],
    pub buffers: Vec<Buffer>,
    pub buffer_index: usize,
}
#[derive(Clone)]
pub struct CollisionsData {
    pub inner: Arc<RwLock<CollisionsDataInner>>,
}

impl CollisionsData {
    pub fn new(buffers: Vec<Buffer>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CollisionsDataInner {
                data: [0; 240],
                buffers,
                buffer_index: 0,
            })),
        }
    }
    pub fn read_collisions(&self, render_device: &RenderDevice) {
        let mut inner = self.inner.write();
        let len = inner.buffers.len();

        // collisions delay
        // TODO: make it configurable? It seems current setting causes some bugs in RiverRaid
        let offs = 0;

        let index = (inner.buffer_index + len - offs) % inner.buffers.len();
        inner.buffer_index = (inner.buffer_index + 1) % inner.buffers.len();
        let buffer = inner.buffers[index].clone();
        // bevy::log::info!("reading buffer {}", inner.buffer_index);
        let slice = buffer.slice(..);
        let map_future = slice.map_async(wgpu::MapMode::Read);
        render_device.poll(wgpu::Maintain::Wait);
        future::block_on(map_future).unwrap();
        {
            let buffer_view = slice.get_mapped_range();
            let data: &[u8] = &buffer_view;
            let data =
                unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u64, data.len() / 8) };
            let dest = &mut inner.data;
            for y in 0..crate::COLLISIONS_AGG_TEXTURE_SIZE.height as usize {
                if y == 0 {
                    for x in 0..240 {
                        dest[x] = data[y << 8 | x];
                    }
                } else {
                    for x in 0..240 {
                        dest[x] |= data[y << 8 | x];
                    }
                }
            }
        }
        buffer.unmap();
    }
}

#[derive(TypeUuid, Clone)]
#[uuid = "bea612c2-68ed-4432-8d9c-f03ebea97043"]
pub struct AnticData {
    pub main_image_handle: Handle<Image>,
    pub inner: Arc<RwLock<AnticDataInner>>,
    pub collisions_data: Option<CollisionsData>,
    pub config: AnticConfig,
}

const GTIA_REGS_MEMORY: usize = 240 * 32;

impl AnticData {
    pub fn new(
        render_device: &RenderDevice,
        main_image_handle: Handle<Image>,
        collisions: bool,
    ) -> Self {
        let mut memory = Vec::with_capacity(GTIA_REGS_MEMORY + 256 * 11 * 4 * 4);
        memory.resize(memory.capacity(), 0);
        let buffer_desc = BufferDescriptor {
            label: Some("atari collisions buffer"),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            size: ((crate::COLLISIONS_AGG_TEXTURE_SIZE.width
                * crate::COLLISIONS_AGG_TEXTURE_SIZE.height) as usize
                * crate::COLLISIONS_AGG__BYTES_PER_PIXEL) as u64,
            mapped_at_creation: false,
        };
        let buffers = vec![
            render_device.create_buffer(&buffer_desc),
            render_device.create_buffer(&buffer_desc),
            render_device.create_buffer(&buffer_desc),
            render_device.create_buffer(&buffer_desc),
        ];
        let collisions_data = if collisions {
            Some(CollisionsData::new(buffers))
        } else {
            None
        };
        Self {
            main_image_handle,
            collisions_data,
            config: AnticConfig::default(),
            inner: Arc::new(RwLock::new(AnticDataInner {
                scanlines: 0,
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
    pub fn set_gtia_regs(&mut self, scan_line: usize, regs: &crate::GTIARegs) {
        assert!(scan_line < 248);
        assert!(std::mem::size_of::<crate::GTIARegs>() == 32);
        let mut inner = self.inner.write();
        let ptr = inner.memory.as_mut_ptr() as *mut crate::GTIARegs;
        unsafe { *ptr.add(scan_line) = *regs }
    }

    pub fn reserve_antic_memory(&mut self, len: usize, cb: &mut dyn FnMut(&mut [u8])) -> usize {
        let mut inner = self.inner.write();
        let dst_offset = GTIA_REGS_MEMORY + inner.memory_used;
        assert!(dst_offset + len <= inner.memory.len());
        inner.memory_used += len;

        cb(&mut inner.memory[dst_offset..dst_offset + len]);
        dst_offset - GTIA_REGS_MEMORY
    }

    pub fn clear(&mut self) {
        let mut inner = self.inner.write();
        inner.memory_used = 0;
        inner.scanlines = 0;
        inner.positions.clear();
        inner.custom.clear();
        inner.uvs.clear();
        inner.indices.clear();
    }

    pub fn create_collisions_agg_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let w = 120.0;
        let h = 384.0;

        let north_west = vec2(0.0, h);
        let north_east = vec2(w, h);
        let south_west = vec2(0.0, 0.0);
        let south_east = vec2(w, 0.0);

        let positions = vec![
            [south_west.x, south_west.y, 0.0],
            [north_west.x, north_west.y, 0.0],
            [north_east.x, north_east.y, 0.0],
            [south_east.x, south_east.y, 0.0],
        ];

        let uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        let custom = vec![
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ];
        let indices = vec![0, 2, 1, 0, 3, 2];

        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_attribute("Vertex_ZCustom", custom);
        mesh.set_indices(Some(Indices::U16(indices)));
        mesh
    }

    pub fn create_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let inner = self.inner.read();
        let mut positions = inner.positions.clone();
        let mut uvs = inner.uvs.clone();
        let mut custom = inner.custom.clone();
        let mut indices = inner.indices.clone();
        let scan_line = inner.scanlines as usize + 8;
        if scan_line < 248 {
            // hack for paused mode
            // to display scan_line we need to add additional empty rect
            // on the end of mesh with height = 1
            // TODO: move this to some postprocessing pass
            let index_offset = positions.len() as u16;

            let mode_line = ModeLineDescr {
                mode: 0,
                scan_line,
                width: 384,
                height: 1,
                ..Default::default()
            };

            push_positions(&mut positions, &mode_line);
            push_uvs(&mut uvs);
            push_custom(&mut custom, &mode_line);
            push_indices(&mut indices, index_offset);
        }

        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_attribute("Vertex_ZCustom", custom);
        mesh.set_indices(Some(Indices::U16(indices)));
        mesh
    }

    pub fn insert_mode_line(&self, mode_line: &crate::ModeLineDescr) {
        let mut inner = self.inner.write();

        let index_offset = inner.positions.len() as u16;

        inner.scanlines = mode_line.scan_line + mode_line.height - 8;

        push_positions(&mut inner.positions, mode_line);
        push_uvs(&mut inner.uvs);
        push_custom(&mut inner.custom, mode_line);
        push_indices(&mut inner.indices, index_offset);
    }
}

fn push_positions(positions: &mut Vec<[f32; 3]>, mode_line: &crate::ModeLineDescr) {
    let scan_line_y = mode_line.scan_line as f32 - 8.0;

    // TODO - flip y using projection matrix, for some reason this didn't worked

    let north_west = vec2(0.0, 240.0 - scan_line_y);
    let north_east = vec2(384.0, 240.0 - scan_line_y);
    let south_west = vec2(0.0, 240.0 - (scan_line_y + mode_line.height as f32));
    let south_east = vec2(384.0, 240.0 - (scan_line_y + mode_line.height as f32));

    positions.push([south_west.x, south_west.y, 0.0]);
    positions.push([north_west.x, north_west.y, 0.0]);
    positions.push([north_east.x, north_east.y, 0.0]);
    positions.push([south_east.x, south_east.y, 0.0]);
}

fn push_custom(custom: &mut Vec<[f32; 4]>, mode_line: &crate::ModeLineDescr) {
    let scan_line_y = mode_line.scan_line as f32 - 8.0;
    let scan_line = scan_line_y as u32;
    let height = mode_line.height as u32;
    let width = mode_line.width as u32 / 2;

    let b0 = (mode_line.mode as u32 | (scan_line << 8) | (height << 16)) as f32;
    let b1 =
        (mode_line.hscrol as u32 | ((mode_line.line_voffset as u32) << 8) | (width << 16)) as f32;
    let b2 = mode_line.video_memory_offset as f32;
    let b3 = mode_line.charset_memory_offset as f32;

    custom.push([b0, b1, b2, b3]);
    custom.push([b0, b1, b2, b3]);
    custom.push([b0, b1, b2, b3]);
    custom.push([b0, b1, b2, b3]);
}

fn push_indices(indices: &mut Vec<u16>, index_offset: u16) {
    indices.extend(
        [
            index_offset,
            index_offset + 2,
            index_offset + 1,
            index_offset,
            index_offset + 3,
            index_offset + 2,
        ]
        .iter(),
    );
}

fn push_uvs(uvs: &mut Vec<[f32; 2]>) {
    uvs.push([0.0, 1.0]);
    uvs.push([0.0, 0.0]);
    uvs.push([1.0, 0.0]);
    uvs.push([1.0, 1.0]);
}
