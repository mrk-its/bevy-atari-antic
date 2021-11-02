use bevy::{
    core_pipeline::ClearColor,
    ecs::prelude::*,
    prelude::info,
    render2::{
        color::Color,
        render_asset::RenderAssets,
        render_graph::Node,
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass},
        render_resource::{CachedPipelineId, TextureView},
        renderer::RenderDevice,
        texture::{Image, TextureCache},
    },
};
use wgpu::{
    Extent3d, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor,
    TextureDescriptor,
};

use crate::{ANTIC_COLLISIONS_HANDLE, ANTIC_IMAGE_HANDLE};
pub struct AnticPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
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

pub struct AnticPassNode {
    query: QueryState<(&'static RenderPhase<AnticPhase>,)>,
    texture_view: Option<TextureView>,
    collisions_texture_view: Option<TextureView>,
}

impl AnticPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            texture_view: None,
            collisions_texture_view: None,
        }
    }
}

impl Node for AnticPassNode {
    fn input(&self) -> Vec<bevy::render2::render_graph::SlotInfo> {
        Vec::new()
    }

    fn output(&self) -> Vec<bevy::render2::render_graph::SlotInfo> {
        Vec::new()
    }

    fn update(&mut self, world: &mut World) {
        let render_images = world.get_resource::<RenderAssets<Image>>().unwrap();
        if let (None, Some(image)) = (
            &self.texture_view,
            render_images.get(&ANTIC_IMAGE_HANDLE.typed()),
        ) {
            self.texture_view = Some(image.texture_view.clone());
        }
        if let (None, Some(image)) = (
            &self.collisions_texture_view,
            render_images.get(&ANTIC_COLLISIONS_HANDLE.typed()),
        ) {
            self.collisions_texture_view = Some(image.texture_view.clone());
        }
    }

    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        let (texture_view, collisions_texture_view) = match (&self.texture_view, &self.collisions_texture_view) {
            (Some(texture_view), Some(collisions_texture_view)) => {
                (texture_view, collisions_texture_view)
            }
            _ => return Ok(()),
        };
        let clear_color = Color::rgba(0.0, 0.0, 0.0, 0.0);
        let render_phase = world.get_resource::<RenderPhase<AnticPhase>>().unwrap();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("main_pass_3d"),
            color_attachments: &[
                RenderPassColorAttachment {
                    view: texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()),
                        store: true,
                    },
                },
                RenderPassColorAttachment {
                    view: collisions_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()),
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        };

        let draw_functions = world.get_resource::<DrawFunctions<AnticPhase>>().unwrap();

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);
        let mut draw_functions = draw_functions.write();

        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        // let antic_phase = self.query.get(world, entity)
        for item in render_phase.items.iter() {
            let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
            draw_function.draw(
                world,
                &mut tracked_pass,
                bevy::ecs::entity::Entity::new(0),
                item,
            );
        }
        Ok(())
    }
}
