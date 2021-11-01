use bevy::prelude::HandleUntyped;
use bevy::reflect::TypeUuid;

use bevy::{
    core_pipeline::Transparent3d,
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    prelude::{info, AddAsset, App, Assets, Handle, Plugin},
    render2::{
        camera::{CameraProjection, OrthographicProjection},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        RenderApp, RenderStage,
    },
};

pub const ANTIC_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4805239651767799999);

use std::sync::Arc;
pub mod atari_data;
pub mod resources;
use wgpu::BufferDescriptor;

use resources::AtariPalette;

pub use atari_data::{AnticData, AnticDataInner};

use crevice::std140::{AsStd140, Std140};

pub struct AtariAnticPlugin;

impl Plugin for AtariAnticPlugin {
    fn build(&self, app: &mut App) {
        let mut projection = OrthographicProjection::default();
        projection.update(384.0, 240.0);
        let projection_matrix = projection.get_projection_matrix();
        info!("projection matrix: {:?}", projection_matrix);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let antic_shader = Shader::from_wgsl(include_str!("antic.wgsl"));
        shaders.set_untracked(ANTIC_SHADER_HANDLE, antic_shader);

        app.add_asset::<AnticData>()
            // .add_asset::<AnticMesh>()
            .add_plugin(ExtractComponentPlugin::<Handle<AnticData>>::default())
            .add_plugin(RenderAssetPlugin::<AnticData>::default());
        app.sub_app(RenderApp)
            .add_render_command::<Transparent3d, SetAnticPipeline>()
            .init_resource::<AnticPipeline>()
            .init_resource::<Option<GpuAnticData>>()
            .init_resource::<SpecializedPipelines<AnticPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_meshes)
            .add_system_to_stage(RenderStage::Queue, queue_meshes);
    }
}

#[derive(Clone)]
pub struct GpuAnticDataInner {
    palette_buffer: Buffer,
    index_buffer: Buffer,
    vertex_buffer: Buffer,
    texture: Texture,
    _texture_view: TextureView,
    bind_group: BindGroup,
}

#[derive(Clone)]
pub struct GpuAnticData {
    inner: Arc<GpuAnticDataInner>,
    index_count: u32,
}

const DATA_TEXTURE_SIZE: Extent3d = Extent3d {
    width: 256,
    height: 11 * 4 * 4 + (240 * 32 / 256),
    depth_or_array_layers: 1,
};

impl RenderAsset for AnticData {
    type ExtractedAsset = AnticData;
    type PreparedAsset = GpuAnticData;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<AnticPipeline>,
        SResMut<Option<GpuAnticData>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, render_queue, custom_pipeline, cache): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let inner = extracted_asset.inner.read();

        if cache.is_none() {
            cache.replace(GpuAnticData {
                inner: Self::create_gpu_data(&render_device, &custom_pipeline),
                index_count: 0,
            });
        }

        let mut gpu_data = (**cache).as_mut().unwrap();
        let mesh = extracted_asset.create_mesh();

        let vertex_data = mesh.get_vertex_buffer_data();
        let index_data = mesh.get_index_buffer_bytes();
        gpu_data.index_count = inner.indices.len() as u32;

        if let Some(index_data) = index_data {
            render_queue.write_buffer(&gpu_data.inner.index_buffer, 0, index_data);
        }

        render_queue.write_buffer(&gpu_data.inner.vertex_buffer, 0, &vertex_data);

        render_queue.write_buffer(
            &gpu_data.inner.palette_buffer,
            0,
            inner.palette.as_std140().as_bytes(),
        );

        render_queue.write_texture(
            gpu_data.inner.texture.as_image_copy(),
            &inner.memory,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(256 as u32).unwrap()),
                rows_per_image: None,
            },
            DATA_TEXTURE_SIZE,
        );
        Ok(gpu_data.clone())
    }
}

impl AnticData {
    fn create_gpu_data(
        render_device: &RenderDevice,
        custom_pipeline: &AnticPipeline,
    ) -> Arc<GpuAnticDataInner> {
        bevy::prelude::info!("creating atari buffers");
        let texture_descriptor = wgpu::TextureDescriptor {
            size: DATA_TEXTURE_SIZE,
            dimension: TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            label: Some("data_texture"),
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        };

        let texture = render_device.create_texture(&texture_descriptor);
        let _texture_view = texture.create_view(&TextureViewDescriptor::default());

        let palette_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: AtariPalette::std140_size_static() as u64,
            mapped_at_creation: false,
        });

        let vertex_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("atari_vertex_buffer"),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            size: 1000000,
            mapped_at_creation: false,
        });

        let index_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("atari_index_buffer"),
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            size: 1000000,
            mapped_at_creation: false,
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: palette_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&_texture_view),
                },
            ],
            label: None,
            layout: &custom_pipeline.atari_data_layout,
        });

        Arc::new(GpuAnticDataInner {
            palette_buffer,
            index_buffer,
            vertex_buffer,
            texture,
            _texture_view,
            bind_group,
        })
    }
}

pub struct AnticPipeline {
    atari_data_layout: BindGroupLayout,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for AnticPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let atari_data_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                AtariPalette::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            view_dimension: TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Uint,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
                label: Some("atari_data_layout"),
            });

        AnticPipeline {
            atari_data_layout,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct AnticPipelineKey;

impl SpecializedPipeline for AnticPipeline {
    type Key = AnticPipelineKey;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: None,
            vertex: VertexState {
                shader_defs: vec![],
                shader: ANTIC_SHADER_HANDLE.typed::<Shader>(),
                buffers: vec![VertexBufferLayout {
                    array_stride: 36,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // Uv
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                        // RCustom
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 20,
                            shader_location: 2,
                        },
                    ],
                }],
                entry_point: "vertex".into(),
            },
            fragment: Some(FragmentState {
                shader: ANTIC_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            layout: Some(vec![
                // self.view_layout.clone(),
                self.atari_data_layout.clone(),
                // self.mesh_layout.clone(),
            ]),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
        }
    }
}

pub fn extract_meshes(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Query<Entity>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for entity in query.iter() {
        values.push((entity, (1,)));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

#[allow(clippy::too_many_arguments)]
pub fn queue_meshes(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    antic_pipeline: Res<AnticPipeline>,
    mut pipelines: ResMut<SpecializedPipelines<AnticPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    atari_data: Query<Entity, With<Handle<AnticData>>>,
    mut views: Query<(Entity, &mut RenderPhase<Transparent3d>)>,
) {
    for (_entity, mut transparent_phase) in views.iter_mut() {
        let draw_function = transparent_3d_draw_functions
            .read()
            .get_id::<SetAnticPipeline>()
            .unwrap();

        for entity in atari_data.iter() {
            let key = AnticPipelineKey;
            let pipeline = pipelines.specialize(&mut pipeline_cache, &antic_pipeline, key);

            transparent_phase.add(Transparent3d {
                pipeline,
                entity,
                draw_function,
                distance: 0.0,
            });
        }
    }
}

struct SetAnticPipeline;
impl RenderCommand<Transparent3d> for SetAnticPipeline {
    type Param = (
        SRes<RenderPipelineCache>,
        SRes<RenderAssets<AnticData>>,
        SQuery<Read<Handle<AnticData>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: &Transparent3d,
        (pipeline_cache, atari_data_assets, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let antic_data_handle = query.get(item.entity).unwrap();
        let gpu_atari_data = atari_data_assets
            .into_inner()
            .get(antic_data_handle)
            .unwrap();

        let index_count = gpu_atari_data.index_count;
        if let Some(pipeline) = pipeline_cache.into_inner().get(item.pipeline) {
            pass.set_render_pipeline(pipeline);
            pass.set_bind_group(0, &gpu_atari_data.inner.bind_group, &[]);
            pass.set_vertex_buffer(0, gpu_atari_data.inner.vertex_buffer.slice(..));
            pass.set_index_buffer(
                gpu_atari_data
                    .inner
                    .index_buffer
                    .slice(0..(index_count * 2) as u64),
                0,
                wgpu::IndexFormat::Uint16,
            );
            pass.draw_indexed(0..index_count, 0, 0..1);
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
