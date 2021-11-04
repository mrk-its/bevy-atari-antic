use bevy::{
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    prelude::Handle,
    render2::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_phase::{DrawFunctions, RenderCommand, RenderPhase, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};
pub mod pass;
use crate::resources::AtariPalette;
use pass::AnticPhase;
use std::sync::Arc;
use wgpu::BufferDescriptor;

pub use crate::atari_data::{AnticData, AnticDataInner};
use crate::ANTIC_SHADER_HANDLE;

use crevice::std140::{AsStd140, Std140};

#[derive(Clone)]
pub struct GpuAnticDataInner {
    palette_buffer: Buffer,
    index_buffer: Buffer,
    vertex_buffer: Buffer,
    // collisions_buffer: Buffer,
    texture: Texture,
    _texture_view: TextureView,
    bind_group: BindGroup,
}

#[derive(Clone)]
pub struct GpuAnticData {
    inner: Arc<GpuAnticDataInner>,
    index_count: u32,
}

pub const DATA_TEXTURE_SIZE: Extent3d = Extent3d {
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
            size: 1000000, // TODO
            mapped_at_creation: false,
        });

        let index_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("atari_index_buffer"),
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            size: 1000000, // TODO
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
            // collisions_buffer,
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

        AnticPipeline { atari_data_layout }
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
                targets: vec![
                    ColorTargetState {
                        format: TextureFormat::Rgba8UnormSrgb,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    },
                    ColorTargetState {
                        format: TextureFormat::Rg32Uint,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    },
                ],
            }),
            depth_stencil: None,
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

#[allow(clippy::too_many_arguments)]
pub fn queue_meshes(
    transparent_3d_draw_functions: Res<DrawFunctions<AnticPhase>>,
    antic_pipeline: Res<AnticPipeline>,
    mut render_phase: ResMut<RenderPhase<AnticPhase>>,
    mut pipelines: ResMut<SpecializedPipelines<AnticPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    atari_data: Query<(Entity, &Handle<AnticData>)>,
) {
    let draw_function = transparent_3d_draw_functions
        .read()
        .get_id::<SetAnticPipeline>()
        .unwrap();
    render_phase.items.clear();
    for (entity, antic_data_handle) in atari_data.iter() {
        let key = AnticPipelineKey;
        let pipeline = pipelines.specialize(&mut pipeline_cache, &antic_pipeline, key);
        render_phase.add(AnticPhase {
            pipeline,
            entity,
            draw_function,
            antic_data_handle: antic_data_handle.clone(),
        });
    }
}

pub struct SetAnticPipeline;
impl RenderCommand<AnticPhase> for SetAnticPipeline {
    type Param = (
        SRes<RenderPipelineCache>,
        SRes<RenderAssets<AnticData>>,
        SQuery<Read<Handle<AnticData>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: &AnticPhase,
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
