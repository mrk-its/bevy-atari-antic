pub use wgpu;
use bevy::{prelude::{info, AddAsset, App, Assets, Handle, HandleUntyped, Plugin}, reflect::TypeUuid, render2::{
        camera::{CameraProjection, OrthographicProjection, WindowOrigin},
        render_asset::RenderAssetPlugin,
        render_component::ExtractComponentPlugin,
        render_graph::RenderGraph,
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase},
        render_resource::*,
        texture::Image,
        RenderApp, RenderStage,
    }, utils::HashMap};

mod antic_data;
mod palette;
mod render;
use render::pass::{AnticPassNode, AnticPhase, CollisionsAggPhase};

const ANTIC_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9390220767195311254);

// Public Interface

pub use antic_data::{AnticData, CollisionsData};

use crate::render::pass::{CollisionsAggNode, CollisionsAggReadNode};

pub struct AtariAnticPlugin {
    pub collisions: bool,
}

const COLLISIONS_AGG_TEXTURE_SIZE: Extent3d = Extent3d {
    width: 128,
    height: 8,
    depth_or_array_layers: 1,
};
const COLLISIONS_AGG__BYTES_PER_PIXEL: usize = 16;


pub fn create_main_image(images: &mut Assets<Image>) -> Handle<Image> {
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
    images.add(image)
}

impl Plugin for AtariAnticPlugin {
    fn build(&self, app: &mut App) {
        let projection = OrthographicProjection {
            left: 0.0,
            top: 0.0,
            right: 384.0,
            bottom: 240.0,
            window_origin: WindowOrigin::BottomLeft,
            ..OrthographicProjection::default()
        };
        // let mut projection = OrthographicProjection::default();
        // projection.window_origin = WindowOrigin::BottomLeft;
        // projection.update(384.0, 240.0);
        let projection_matrix = projection.get_projection_matrix();
        info!("projection matrix: {:?}", projection_matrix);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let antic_shader = Shader::from_wgsl(include_str!("render/antic.wgsl"));
        shaders.set_untracked(ANTIC_SHADER_HANDLE, antic_shader);

        app.add_asset::<AnticData>()
            // .add_asset::<AnticMesh>()
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
            .init_resource::<HashMap<Handle<Image>, render::GpuAnticData>>()
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

        if self.collisions {
            graph.add_node("collisions_agg_node", CollisionsAggNode::default());

            graph
                .add_node_edge(
                    "collisions_agg_node",
                    bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                )
                .unwrap();
            graph
                .add_node_edge("antic_node", "collisions_agg_node")
                .unwrap();

            graph.add_node(
                "collisions_agg_read_node",
                CollisionsAggReadNode::default(),
            );
            graph
                .add_node_edge("collisions_agg_node", "collisions_agg_read_node")
                .unwrap();
            graph
                .add_node_edge(
                    "collisions_agg_read_node",
                    bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                )
                .unwrap();
        }
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
        self.scan_line + self.height
    }
    pub fn charset_size(&self) -> usize {
        match self.mode {
            2..=5 => 1024,
            6..=7 => 512,
            _ => 0,
        }
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
