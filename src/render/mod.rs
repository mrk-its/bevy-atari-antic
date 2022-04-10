#![allow(clippy::type_complexity)]
use bevy::{
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    prelude::Handle,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_phase::{
            DrawFunctions, RenderCommand, RenderCommandResult, RenderPhase, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::Image,
    },
    utils::HashMap,
};
use crevice::std140::{AsStd140, Std140};
pub mod pass;
use crate::resources::{AtariPalette, AnticConfig};
use pass::{AnticPhase, CollisionsAggPhase};
use std::sync::Arc;
use wgpu::BufferDescriptor;

pub use crate::antic_data::{AnticData, AnticDataInner, CollisionsData};
use crate::ANTIC_SHADER_HANDLE;

#[derive(Clone)]
pub struct GpuAnticCollisionsData {
    pub data: CollisionsData,
    _collisions_texture: Texture,
    collisions_texture_view: TextureView,
    collisions_agg_texture: Texture,
    collisions_agg_texture_view: TextureView,
    collisions_agg_index_buffer: Buffer,
    collisions_agg_vertex_buffer: Buffer,
    collisions_agg_bind_group: BindGroup,
}

#[derive(Clone)]
pub struct GpuAnticDataInner {
    palette_buffer: Buffer,
    config_buffer: Buffer,
    index_buffer: Buffer,
    vertex_buffer: Buffer,
    data_texture: Texture,
    main_image_handle: Handle<Image>,
    main_bind_group: BindGroup,
    _data_texture_view: TextureView,
    collisions: Option<GpuAnticCollisionsData>,
}

#[derive(Clone)]
pub struct GpuAnticData {
    inner: Arc<GpuAnticDataInner>,
    index_count: u32,
    config: AnticConfig,
}

pub const DATA_TEXTURE_SIZE: Extent3d = Extent3d {
    width: 256,
    height: 11 * 4 * 4 + (240 * 32 / 256),
    depth_or_array_layers: 1,
};

pub const COLLISIONS_TEXTURE_SIZE: Extent3d = Extent3d {
    width: 384,
    height: 240,
    depth_or_array_layers: 1,
};

impl RenderAsset for AnticData {
    type ExtractedAsset = AnticData;
    type PreparedAsset = GpuAnticData;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<AnticPipeline>,
        SRes<CollisionsAggPipeline>,
        SResMut<HashMap<Handle<Image>, GpuAnticData>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, render_queue, pipeline, collisions_agg_pipeline, cache): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {

        let inner = extracted_asset.inner.read();
        let entry = cache.entry(extracted_asset.main_image_handle.clone());
        let main_image_handle = extracted_asset.main_image_handle.clone();
        let collisions_data = extracted_asset
            .collisions_data
            .as_ref()
            .map(|data| (&**collisions_agg_pipeline, data.clone()));
        let gpu_data = entry.or_insert_with(|| {
            let gpu_data = GpuAnticData {
                inner: Self::create_gpu_data(
                    render_device,
                    pipeline,
                    main_image_handle,
                    collisions_data,
                ),
                index_count: 0,
                config: extracted_asset.config,
            };
            render_queue.write_buffer(
                &gpu_data.inner.palette_buffer,
                0,
                inner.palette.as_std140().as_bytes(),
            );
            render_queue.write_buffer(
                &gpu_data.inner.config_buffer,
                0,
                extracted_asset.config.as_std140().as_bytes(),
            );
            if let Some(collisions) = &gpu_data.inner.collisions {
                // and collisions vertex / index buffer
                let mesh = extracted_asset.create_collisions_agg_mesh();
                let vertex_data = mesh.get_vertex_buffer_data();
                let index_data = mesh.get_index_buffer_bytes();
                if let Some(index_data) = index_data {
                    render_queue.write_buffer(
                        &collisions.collisions_agg_index_buffer,
                        0,
                        index_data,
                    );
                }
                render_queue.write_buffer(
                    &collisions.collisions_agg_vertex_buffer,
                    0,
                    &vertex_data,
                );
            }

            gpu_data
        });

        // TODO - mesh change detection
        let mesh = extracted_asset.create_mesh();
        let vertex_data = mesh.get_vertex_buffer_data();
        let index_data = mesh.get_index_buffer_bytes();

        if let Some(index_data) = index_data {
            gpu_data.index_count = index_data.len() as u32 / 2;
            render_queue.write_buffer(&gpu_data.inner.index_buffer, 0, index_data);
        }
        render_queue.write_buffer(&gpu_data.inner.vertex_buffer, 0, &vertex_data);

        render_queue.write_texture(
            gpu_data.inner.data_texture.as_image_copy(),
            &inner.memory,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(256).unwrap()),
                rows_per_image: None,
            },
            DATA_TEXTURE_SIZE,
        );
        if extracted_asset.config != gpu_data.config {
            gpu_data.config = extracted_asset.config;
            render_queue.write_buffer(
                &gpu_data.inner.config_buffer,
                0,
                extracted_asset.config.as_std140().as_bytes(),
            );
        }
        Ok(gpu_data.clone())
    }
}

impl AnticData {
    fn create_gpu_data(
        render_device: &RenderDevice,
        pipeline: &AnticPipeline,
        main_image_handle: Handle<Image>,
        collisions_data: Option<(&CollisionsAggPipeline, CollisionsData)>,
    ) -> Arc<GpuAnticDataInner> {
        let texture_descriptor = wgpu::TextureDescriptor {
            size: DATA_TEXTURE_SIZE,
            dimension: TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            label: Some("data_texture"),
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        };

        let data_texture = render_device.create_texture(&texture_descriptor);
        let data_texture_view = data_texture.create_view(&TextureViewDescriptor::default());

        let palette_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: AtariPalette::std140_size_static() as u64,
            mapped_at_creation: false,
        });

        let config_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("config_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: AnticConfig::std140_size_static() as u64,
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

        let main_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&data_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: palette_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
            label: Some("atari_bind_group"),
            layout: &pipeline.data_layout,
        });

        let collisions = if let Some((collisions_agg_pipeline, data)) = collisions_data {
            let collisions_agg_texture_descriptor = wgpu::TextureDescriptor {
                size: crate::COLLISIONS_AGG_TEXTURE_SIZE,
                dimension: TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Uint,
                label: Some("collisions_agg_data_texture"),
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            };

            let collisions_agg_texture =
                render_device.create_texture(&collisions_agg_texture_descriptor);
            let collisions_agg_texture_view =
                collisions_agg_texture.create_view(&TextureViewDescriptor::default());

            let collisions_texture_descriptor = wgpu::TextureDescriptor {
                size: COLLISIONS_TEXTURE_SIZE,
                dimension: TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Uint,
                label: Some("collisions_texture"),
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            };

            let collisions_texture = render_device.create_texture(&collisions_texture_descriptor);
            let collisions_texture_view =
                collisions_texture.create_view(&TextureViewDescriptor::default());

            let collisions_agg_vertex_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("collisions_agg_vertex_buffer"),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                size: 4 * 36, // 36 is the size of custom vertex data
                mapped_at_creation: false,
            });

            let collisions_agg_index_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("collisions_agg_index_buffer"),
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                size: 12,
                mapped_at_creation: false,
            });

            let collisions_agg_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&collisions_texture_view),
                }],
                label: Some("collisions_agg_bind_group"),
                layout: &collisions_agg_pipeline.data_layout,
            });
            Some(GpuAnticCollisionsData {
                data,
                _collisions_texture: collisions_texture,
                collisions_texture_view,
                collisions_agg_texture,
                collisions_agg_texture_view,
                collisions_agg_index_buffer,
                collisions_agg_vertex_buffer,
                collisions_agg_bind_group,
            })
        } else {
            None
        };

        Arc::new(GpuAnticDataInner {
            main_image_handle,
            palette_buffer,
            config_buffer,
            index_buffer,
            vertex_buffer,
            data_texture,
            _data_texture_view: data_texture_view,
            main_bind_group,
            collisions,
        })
    }
}

pub struct AnticPipeline {
    data_layout: BindGroupLayout,
}

#[derive(Clone)]
pub struct CollisionsAggPipeline {
    data_layout: BindGroupLayout,
}

impl FromWorld for AnticPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let data_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            view_dimension: TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Uint,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
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
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                AnticConfig::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("atari_data_layout"),
            });

        AnticPipeline { data_layout }
    }
}
impl FromWorld for CollisionsAggPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let data_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    view_dimension: TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Uint,
                    multisampled: false,
                },
                count: None,
            }],
            label: Some("colissions_agg_data_layout"),
        });

        CollisionsAggPipeline { data_layout }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct AnticPipelineKey {
    collisions: bool,
}

impl SpecializedPipeline for AnticPipeline {
    type Key = AnticPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let targets = if key.collisions {
            vec![
                ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                },
                ColorTargetState {
                    format: TextureFormat::Rgba16Uint,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                },
            ]
        } else {
            vec![ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            }]
        };

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
                targets,
            }),
            layout: Some(vec![self.data_layout.clone()]),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CollisionsAggPipelineKey(u32);

impl SpecializedPipeline for CollisionsAggPipeline {
    type Key = CollisionsAggPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
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
                entry_point: "collision_agg_vertex".into(),
            },
            fragment: Some(FragmentState {
                shader: ANTIC_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![format!("T_{}", key.0)],
                entry_point: "collisions_agg_fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::Rgba32Uint,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![self.data_layout.clone()]),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_meshes(
    draw_functions: Res<DrawFunctions<AnticPhase>>,
    collisions_agg_draw_functions: Res<DrawFunctions<CollisionsAggPhase>>,
    antic_pipeline: Res<AnticPipeline>,
    collisions_agg_pipeline: Res<CollisionsAggPipeline>,
    mut render_phase: ResMut<RenderPhase<AnticPhase>>,
    mut collisions_agg_render_phase: ResMut<RenderPhase<CollisionsAggPhase>>,
    mut pipelines: ResMut<SpecializedPipelines<AnticPipeline>>,
    mut collision_agg_pipelines: ResMut<SpecializedPipelines<CollisionsAggPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    atari_datas: Res<RenderAssets<AnticData>>,
    antic_data_query: Query<(Entity, &Handle<AnticData>)>,
) {
    let draw_function = draw_functions.read().get_id::<SetAnticPipeline>().unwrap();
    let collisions_agg_draw_function = collisions_agg_draw_functions
        .read()
        .get_id::<SetCollisionsAggPipeline>()
        .unwrap();
    render_phase.items.clear();
    collisions_agg_render_phase.items.clear();

    for (entity, antic_data_handle) in antic_data_query.iter() {
        let atari_data = atari_datas.get(antic_data_handle).unwrap();
        let collisions = atari_data.inner.collisions.is_some();
        let pipeline = pipelines.specialize(
            &mut pipeline_cache,
            &antic_pipeline,
            AnticPipelineKey { collisions },
        );
        render_phase.add(AnticPhase {
            main_image_handle: atari_data.inner.main_image_handle.clone(),
            collisions,
            pipeline,
            entity,
            draw_function,
            antic_data_handle: antic_data_handle.clone(),
        });
        if atari_data.inner.collisions.is_some() {
            let collisions_agg_pipeline = collision_agg_pipelines.specialize(
                &mut pipeline_cache,
                &collisions_agg_pipeline,
                CollisionsAggPipelineKey(crate::COLLISIONS_AGG_TEXTURE_SIZE.height),
            );
            collisions_agg_render_phase.add(CollisionsAggPhase {
                pipeline: collisions_agg_pipeline,
                entity,
                draw_function: collisions_agg_draw_function,
                antic_data_handle: antic_data_handle.clone(),
            });
        }
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
    ) -> RenderCommandResult {
        let antic_data_handle = query.get(item.entity).unwrap();
        let gpu_atari_data = atari_data_assets
            .into_inner()
            .get(antic_data_handle)
            .unwrap();

        let index_count = gpu_atari_data.index_count;
        if index_count == 0 {
            return RenderCommandResult::Failure;
        }
        if let Some(pipeline) = pipeline_cache.into_inner().get(item.pipeline) {
            pass.set_render_pipeline(pipeline);
            pass.set_bind_group(0, &gpu_atari_data.inner.main_bind_group, &[]);
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
        RenderCommandResult::Success
    }
}

pub struct SetCollisionsAggPipeline;
impl RenderCommand<CollisionsAggPhase> for SetCollisionsAggPipeline {
    type Param = (
        SRes<RenderPipelineCache>,
        SRes<RenderAssets<AnticData>>,
        SQuery<Read<Handle<AnticData>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: &CollisionsAggPhase,
        (pipeline_cache, atari_data_assets, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let antic_data_handle = query.get(item.entity).unwrap();
        let gpu_atari_data = atari_data_assets
            .into_inner()
            .get(antic_data_handle)
            .unwrap();

        let collisions = gpu_atari_data.inner.collisions.as_ref().unwrap();

        let index_count = 6;
        if let Some(pipeline) = pipeline_cache.into_inner().get(item.pipeline) {
            pass.set_render_pipeline(pipeline);
            pass.set_bind_group(0, &collisions.collisions_agg_bind_group, &[]);
            // TODO - create separate, simple mesh for collision
            pass.set_vertex_buffer(0, collisions.collisions_agg_vertex_buffer.slice(..));
            pass.set_index_buffer(
                collisions
                    .collisions_agg_index_buffer
                    .slice(0..(index_count * 2) as u64),
                0,
                wgpu::IndexFormat::Uint16,
            );
            pass.draw_indexed(0..index_count, 0, 0..1);
        }
        RenderCommandResult::Success
    }
}
