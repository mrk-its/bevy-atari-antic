use futures_lite::future;
use parking_lot::RwLock;
use std::sync::Arc;
use wgpu::BufferDescriptor;

use bevy::{
    prelude::{info, AddAsset, App, Assets, Handle, HandleUntyped, Plugin},
    reflect::TypeUuid,
    render2::{
        camera::{CameraProjection, OrthographicProjection},
        render_asset::RenderAssetPlugin,
        render_component::ExtractComponentPlugin,
        render_graph::RenderGraph,
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase},
        render_resource::*,
        renderer::RenderDevice,
        texture::Image,
        RenderApp, RenderStage,
    },
};

mod atari_data;
mod render;
mod resources;
use render::{
    pass::{AnticPassNode, AnticPhase, CollisionsAggPhase},
    COLLISIONS_AGG_TEXTURE_SIZE,
};

const ANTIC_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9390220767195311254);

// Public Interface

pub const ANTIC_IMAGE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Image::TYPE_UUID, 13064265395354330662);

pub const ANTIC_DATA_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(AnticData::TYPE_UUID, 11501023916499139379);

pub use atari_data::AnticData;

use crate::render::pass::{AssetOutputNode, CollisionsAggNode};

pub struct AtariAnticPlugin;

impl Plugin for AtariAnticPlugin {
    fn build(&self, app: &mut App) {
        let mut projection = OrthographicProjection::default();
        projection.update(384.0, 240.0);
        let projection_matrix = projection.get_projection_matrix();
        info!("projection matrix: {:?}", projection_matrix);

        let render_device = app.world.get_resource::<RenderDevice>().unwrap();
        let collisions_data = CollisionsData::new(&render_device);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let antic_shader = Shader::from_wgsl(include_str!("render/antic.wgsl"));
        shaders.set_untracked(ANTIC_SHADER_HANDLE, antic_shader);

        app.add_asset::<AnticData>()
            // .add_asset::<AnticMesh>()
            .insert_resource(collisions_data.clone())
            .add_plugin(ExtractComponentPlugin::<Handle<AnticData>>::default())
            .add_plugin(RenderAssetPlugin::<AnticData>::default());

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<DrawFunctions<AnticPhase>>()
            .init_resource::<RenderPhase<AnticPhase>>()
            .init_resource::<DrawFunctions<CollisionsAggPhase>>()
            .init_resource::<RenderPhase<CollisionsAggPhase>>()
            .init_resource::<render::AnticPipeline>()
            .init_resource::<render::CollisionsAggPipeline>()
            .init_resource::<Option<render::GpuAnticData>>()
            .init_resource::<SpecializedPipelines<render::AnticPipeline>>()
            .init_resource::<SpecializedPipelines<render::CollisionsAggPipeline>>()
            .add_render_command::<AnticPhase, render::SetAnticPipeline>()
            .add_render_command::<CollisionsAggPhase, render::SetCollisionsAggPipeline>()
            .add_system_to_stage(RenderStage::Queue, render::queue_meshes);

        let antic_node = AnticPassNode::default();

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("antic_node", antic_node);
        graph
            .add_node_edge(
                "antic_node",
                bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
            )
            .unwrap();

        graph.add_node(
            "main_texture_node",
            AssetOutputNode::new(ANTIC_IMAGE_HANDLE.typed::<Image>()),
        );
        graph
            .add_slot_edge(
                "main_texture_node",
                "texture_view",
                "antic_node",
                "main_texture_view",
            )
            .unwrap();
        graph.add_node(
            "antic_data_node",
            AssetOutputNode::new(ANTIC_DATA_HANDLE.typed::<AnticData>()),
        );
        graph
            .add_slot_edge(
                "antic_data_node",
                "collisions_texture_view",
                "antic_node",
                "collisions_texture_view",
            )
            .unwrap();

        if true {
            graph.add_node(
                "collisions_agg_node",
                CollisionsAggNode::new(collisions_data),
            );
            graph
                .add_node_edge(
                    "collisions_agg_node",
                    bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                )
                .unwrap();
            graph
                .add_slot_edge(
                    "antic_data_node",
                    "collisions_agg_texture_view",
                    "collisions_agg_node",
                    "collisions_agg_texture_view",
                )
                .unwrap();

            graph
                .add_node_edge("collisions_agg_node", "antic_node")
                .unwrap();
        }

        let mut image = Image::new(
            Extent3d {
                width: 384,
                height: 240,
                depth_or_array_layers: 1,
            },
            wgpu::TextureDimension::D2,
            vec![128; 384 * 240 * 4],
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        image.texture_descriptor.usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;

        let mut images = app.world.get_resource_mut::<Assets<Image>>().unwrap();
        images.set_untracked(ANTIC_IMAGE_HANDLE, image);
    }
}

#[derive(Debug)]
pub struct ModeLineDescr {
    pub mode: u8,
    pub scan_line: usize,
    pub width: usize,
    pub height: usize,
    pub n_bytes: usize,
    pub line_voffset: usize,
    pub data_offset: usize,
    pub chbase: u8,
    pub pmbase: u8,
    pub hscrol: u8,
    pub video_memory_offset: usize,
    pub charset_memory_offset: usize,
}

impl ModeLineDescr {
    pub fn next_mode_line(&self) -> usize {
        return self.scan_line + self.height;
    }
    pub fn charset_size(&self) -> usize {
        match self.mode {
            2..=5 => 1024,
            6..=7 => 512,
            _ => 0,
        }
    }
}

#[derive(Clone)]
pub struct CollisionsData {
    pub data: Arc<RwLock<[u64; 240]>>,
    pub(crate) buffer: Buffer,
}

impl CollisionsData {
    fn new(render_device: &RenderDevice) -> Self {
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("atari collisions buffer"),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            size: 384 * 240 * 8,
            mapped_at_creation: false,
        });
        Self {
            data: Arc::new(RwLock::new([0; 240])),
            buffer,
        }
    }
    fn read_collisions(&self, render_device: &RenderDevice) {
        let buffer = &self.buffer;
        let slice = buffer.slice(..);
        let map_future = slice.map_async(wgpu::MapMode::Read);
        render_device.poll(wgpu::Maintain::Wait);
        future::block_on(map_future).unwrap();
        {
            let buffer_view = slice.get_mapped_range();
            let data: &[u8] = &buffer_view;
            let data =
                unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u64, data.len() / 8) };
            let guard = &mut self.data.write();
            let dest = guard.as_mut();

            let (w, h) = (
                COLLISIONS_AGG_TEXTURE_SIZE.width as usize,
                COLLISIONS_AGG_TEXTURE_SIZE.height as usize,
            );

            let mut index = 0;
            for y in 0..h {
                dest[y] = 0;
                for _ in 0..w {
                    dest[y] |= data[index];
                    index += 1;
                }
            }
        }
        buffer.unmap();
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIARegs {
    pub hposp: [u8; 4],
    pub hposm: [u8; 4],
    pub sizep: [u8; 4],
    pub sizem: u8,
    pub grafp: [u8; 4],
    pub grafm: u8,
    pub col: [u8; 9],
    pub prior: u8,
    pub vdelay: u8,
    pub gractl: u8,
    pub hitclr: u8,
    pub consol: u8,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vec() {
        let v: Vec<u8> = Vec::with_capacity(16);
        assert!(v.capacity() == 16);
    }
}
