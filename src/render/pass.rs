use bevy::{
    ecs::prelude::*,
    prelude::Handle,
    render2::{
        color::Color,
        render_asset::{RenderAsset, RenderAssets},
        render_graph::{Node, NodeRunError, OutputSlotError, SlotInfo, SlotType},
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass},
        render_resource::CachedPipelineId,
        texture::Image,
    },
};
use wgpu::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

use crate::{AnticData, CollisionsData};
pub struct AnticPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub antic_data_handle: Handle<AnticData>,
    pub main_image_handle: Handle<Image>,
    pub collisions: bool,
}
pub struct CollisionsAggPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub antic_data_handle: Handle<AnticData>,
}

impl PhaseItem for AnticPhase {
    type SortKey = Entity;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl PhaseItem for CollisionsAggPhase {
    type SortKey = Entity;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

pub struct AssetOutputNode<T>
where
    T: RenderAsset,
{
    handle: Handle<T>,
}

impl<T> AssetOutputNode<T>
where
    T: RenderAsset,
{
    pub fn new(handle: Handle<T>) -> Self {
        Self { handle }
    }
    pub fn set_output(
        &self,
        world: &World,
        cb: &mut dyn FnMut(&T::PreparedAsset) -> Result<(), OutputSlotError>,
    ) -> Result<(), NodeRunError> {
        let assets = world.get_resource::<RenderAssets<T>>().unwrap();
        if let Some(asset) = assets.get(&self.handle) {
            cb(asset)?;
        };
        Ok(())
    }
}

impl Node for AssetOutputNode<Image> {
    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        _render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        self.set_output(world, &mut |asset| {
            graph.set_output("texture_view", asset.texture_view.clone())?;
            Ok(())
        })
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo {
            name: "texture_view".into(),
            slot_type: SlotType::TextureView,
        }]
    }
}

impl Node for AssetOutputNode<AnticData> {
    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        _render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        self.set_output(world, &mut |asset| {
            let collisions = asset.inner.collisions.as_ref().unwrap();
            graph.set_output(
                "collisions_texture_view",
                collisions.collisions_texture_view.clone(),
            )?;
            graph.set_output(
                "collisions_agg_texture_view",
                collisions.collisions_agg_texture_view.clone(),
            )?;
            Ok(())
        })
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo {
                name: "collisions_texture_view".into(),
                slot_type: SlotType::TextureView,
            },
            SlotInfo {
                name: "collisions_agg_texture_view".into(),
                slot_type: SlotType::TextureView,
            },
        ]
    }
}

#[derive(Default)]
pub struct AnticPassNode;

impl Node for AnticPassNode {
    // fn input(&self) -> Vec<SlotInfo> {
    //     vec![
    //         SlotInfo {
    //             name: "main_texture_view".into(),
    //             slot_type: SlotType::TextureView,
    //         },
    //         SlotInfo {
    //             name: "collisions_texture_view".into(),
    //             slot_type: SlotType::TextureView,
    //         },
    //     ]
    // }

    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let image_assets = world.get_resource::<RenderAssets<Image>>().unwrap();
        // let main_texture = graph.get_input_texture("main_texture_view")?;

        let _clear_color = Color::rgba(0.1, 0.1, 0.1, 1.0);
        let _collisions_clear_color = Color::rgba(0.0, 0.0, 0.0, 0.0);

        let render_phase = world.get_resource::<RenderPhase<AnticPhase>>().unwrap();
        for item in render_phase.items.iter() {
            let main_texture = if let Some(texture) = image_assets.get(&item.main_image_handle) {
                &texture.texture_view
            } else {
                continue
            };

            let main_texture_attachment = RenderPassColorAttachment {
                view: &main_texture,
                resolve_target: None,
                ops: Operations {
                    // load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                    load: LoadOp::Load,
                    store: true,
                },
            };
            let color_attachments = if item.collisions {
                let collisions_texture = graph.get_input_texture("collisions_texture_view")?;
                vec![
                    RenderPassColorAttachment {
                        view: main_texture,
                        resolve_target: None,
                        ops: Operations {
                            // load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                            load: LoadOp::Load,
                            store: true,
                        },
                    },
                    RenderPassColorAttachment {
                        view: collisions_texture,
                        resolve_target: None,
                        ops: Operations {
                            // load: LoadOp::Clear(collisions_clear_color.into()), // TODO: do not clear?
                            load: LoadOp::Load,
                            store: true,
                        },
                    },
                ]
            } else {
                vec![RenderPassColorAttachment {
                    view: main_texture,
                    resolve_target: None,
                    ops: Operations {
                        // load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                        load: LoadOp::Load,
                        store: true,
                    },
                }]
            };
            let pass_descriptor = RenderPassDescriptor {
                label: Some("antic_main_pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
            };

            {
                let draw_functions = world.get_resource::<DrawFunctions<AnticPhase>>().unwrap();

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);
                let mut draw_functions = draw_functions.write();

                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct CollisionsAggNode;

impl Node for CollisionsAggNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo {
            name: "collisions_agg_texture_view".into(),
            slot_type: SlotType::TextureView,
        }]
    }

    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let collisions_agg_texture = graph.get_input_texture("collisions_agg_texture_view")?;

        let _clear_color = Color::rgba(0.0, 0.0, 0.0, 0.0);

        let collisions_agg_render_phase = world
            .get_resource::<RenderPhase<CollisionsAggPhase>>()
            .unwrap();
        for item in collisions_agg_render_phase.items.iter() {
            let collisions_agg_pass_descriptor = RenderPassDescriptor {
                label: Some("collisioons_agg_pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: collisions_agg_texture,
                    resolve_target: None,
                    ops: Operations {
                        // load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            };

            {
                let draw_functions = world
                    .get_resource::<DrawFunctions<CollisionsAggPhase>>()
                    .unwrap();

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&collisions_agg_pass_descriptor);
                let mut draw_functions = draw_functions.write();

                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }
        Ok(())
    }
}

pub struct CollisionsAggReadNode {
    collisions_data: CollisionsData,
}

impl CollisionsAggReadNode {
    pub fn new(collisions_data: CollisionsData) -> Self {
        Self { collisions_data }
    }
}

impl Node for CollisionsAggReadNode {
    fn run(
        &self,
        _graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let assets = world.get_resource::<RenderAssets<AnticData>>().unwrap();
        let collisions_agg_render_phase = world
            .get_resource::<RenderPhase<CollisionsAggPhase>>()
            .unwrap();
        for item in collisions_agg_render_phase.items.iter() {
            let gpu_antic_data =
                if let Some(antic_data) = assets.get(&item.antic_data_handle) {
                    antic_data
                } else {
                    return Ok(());
                };

            let collisions = gpu_antic_data.inner.collisions.as_ref().unwrap();
            let copy_size = collisions.collisions_agg_texture_size;

            // consider moving this reading befor emulation step
            // for now we have additional 1 frame delay
            self.collisions_data
                .read_collisions(&render_context.render_device);
            render_context.command_encoder.copy_texture_to_buffer(
                collisions.collisions_agg_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &self.collisions_data.buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(
                                copy_size.width * crate::CollisionsData::BYTES_PER_PIXEL as u32,
                            )
                            .unwrap(),
                        ),
                        rows_per_image: None,
                    },
                },
                copy_size,
            );
        }
        Ok(())
    }
}
