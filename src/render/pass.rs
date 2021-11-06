use bevy::{
    asset::Asset,
    ecs::prelude::*,
    prelude::info,
    prelude::Handle,
    render2::{
        color::Color,
        render_asset::{RenderAsset, RenderAssets},
        render_graph::{Node, NodeRunError, OutputSlotError, SlotInfo, SlotType, SlotValue},
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass},
        render_resource::{CachedPipelineId, Texture, TextureView},
        renderer::RenderDevice,
        texture::Image,
    },
};
use wgpu::{
    BindGroupDescriptor, Extent3d, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor,
};

use crate::{AnticData, CollisionsData, ANTIC_DATA_HANDLE, ANTIC_IMAGE_HANDLE};
pub struct AnticPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub antic_data_handle: Handle<AnticData>,
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
            graph.set_output(
                "collisions_texture_view",
                asset.inner.collisions_texture_view.clone(),
            )?;
            graph.set_output(
                "collisions_agg_texture_view",
                asset.inner.collisions_agg_texture_view.clone(),
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
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo {
                name: "main_texture_view".into(),
                slot_type: SlotType::TextureView,
            },
            SlotInfo {
                name: "collisions_texture_view".into(),
                slot_type: SlotType::TextureView,
            },
        ]
    }

    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let main_texture = graph.get_input_texture("main_texture_view")?;
        let collisions_texture = graph.get_input_texture("collisions_texture_view")?;

        let clear_color = Color::rgba(0.1, 0.1, 0.1, 1.0);

        let render_phase = world.get_resource::<RenderPhase<AnticPhase>>().unwrap();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("antic_main_pass"),
            color_attachments: &[
                RenderPassColorAttachment {
                    view: main_texture,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                        store: true,
                    },
                },
                RenderPassColorAttachment {
                    view: collisions_texture,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        };

        {
            let draw_functions = world.get_resource::<DrawFunctions<AnticPhase>>().unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();

            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in render_phase.items.iter() {
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

pub struct CollisionsAggNode {
    collisions_data: CollisionsData,
}

impl CollisionsAggNode {
    pub fn new(collisions_data: CollisionsData) -> Self {
        Self { collisions_data }
    }
}

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
        let assets = world.get_resource::<RenderAssets<AnticData>>().unwrap();
        let collisions_agg_texture = graph.get_input_texture("collisions_agg_texture_view")?;

        let clear_color = Color::rgba(0.1, 0.1, 0.1, 1.0);

        let collisions_agg_render_phase = world
            .get_resource::<RenderPhase<CollisionsAggPhase>>()
            .unwrap();

        let collisions_agg_pass_descriptor = RenderPassDescriptor {
            label: Some("collisioons_agg_pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: collisions_agg_texture,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
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
            for item in collisions_agg_render_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }

        let gpu_antic_data =
            if let Some(antic_data) = assets.get(&ANTIC_DATA_HANDLE.typed::<AnticData>()) {
                antic_data
            } else {
                return Ok(());
            };

        // for _item in render_phase.items.iter() {
        //     render_context.command_encoder.copy_texture_to_buffer(
        //         collisions_texture.as_image_copy(),
        //         wgpu::ImageCopyBuffer {
        //             buffer: &self.collisions_data.buffer,
        //             layout: wgpu::ImageDataLayout {
        //                 offset: 0,
        //                 bytes_per_row: Some(std::num::NonZeroU32::new(384 * 8).unwrap()),
        //                 rows_per_image: None,
        //             },
        //         },
        //         Extent3d {
        //             width: 384,
        //             height: 240,
        //             depth_or_array_layers: 1,
        //         },
        //     );
        //     self.collisions_data.read_collisions(&render_context.render_device)
        // }

        for _item in collisions_agg_render_phase.items.iter() {
            render_context.command_encoder.copy_texture_to_buffer(
                gpu_antic_data.inner.collisions_agg_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &self.collisions_data.buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(super::COLLISIONS_AGG_TEXTURE_SIZE.width * 8)
                                .unwrap(),
                        ),
                        rows_per_image: None,
                    },
                },
                super::COLLISIONS_AGG_TEXTURE_SIZE,
            );
            self.collisions_data
                .read_collisions(&render_context.render_device)
        }
        Ok(())
    }
}
