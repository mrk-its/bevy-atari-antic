use bevy::{
    app::AppExit,
    ecs::prelude::*,
    math::Vec3,
    prelude::{App, Assets, Handle, Transform},
    render2::{
        camera::OrthographicCameraBundle,
        view::Msaa,
    },
    window::WindowDescriptor,
    PipelinedDefaultPlugins,
};
use bevy_atari_antic::{AnticData, GTIARegs, ANTIC_IMAGE_HANDLE};
use bevy_atari_antic::{AtariAnticPlugin, ModeLineDescr};

use bevy::sprite2::{PipelinedSpriteBundle, Sprite};

fn main() {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        width: 1280.0,
        height: 720.0,
        scale_factor_override: Some(1.0),
        ..Default::default()
    });
    app.insert_resource(Msaa { samples: 1 });
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        if let Ok(Some(log_filter)) = local_storage.get_item("log") {
            app.insert_resource(bevy::log::LogSettings {
                filter: log_filter,
                level: bevy::utils::tracing::Level::INFO,
            });
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.insert_resource(bevy::log::LogSettings {
            level: bevy::utils::tracing::Level::INFO,
            filter: "".to_string(),
        });
    }

    app.add_plugins(PipelinedDefaultPlugins)
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(AtariAnticPlugin)
        .add_startup_system(setup)
        .add_system(update);

    #[cfg(not(target_arch = "wasm32"))]
    app.add_system(quit_after_few_frames);

    app.run();
}

#[allow(dead_code)]
fn quit_after_few_frames(mut cnt: Local<u32>, mut app_exit_events: EventWriter<AppExit>) {
    *cnt += 1;
    if *cnt > 5 {
        app_exit_events.send(AppExit);
    }
}

fn update(mut atari_data_assets: ResMut<Assets<AnticData>>, query: Query<&Handle<AnticData>>) {
    let span = bevy::utils::tracing::span!(bevy::utils::tracing::Level::INFO, "my_span");
    let _entered = span.enter();
    for handle in query.iter() {
        if let Some(atari_data) = atari_data_assets.get_mut(handle) {
            let mut inner = atari_data.inner.write();
            let c = &mut inner.memory[32 * 240 + 1024];
            *c = c.wrapping_add(1);
            let c = &mut inner.memory[32 * 240 + 1024 + 31];
            *c = c.wrapping_add(1);
        }
    }
}

fn setup(
    mut commands: Commands,
    mut atari_data_assets: ResMut<Assets<AnticData>>,
) {
    let mut atari_data = AnticData::default();

    let coffs = atari_data.reserve_antic_memory(1024, &mut |data| {
        data.copy_from_slice(include_bytes!("charset.dat"))
    });

    let voffs = atari_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    let voffs0 = voffs;
    atari_data.insert_mode_line(&ModeLineDescr {
        mode: 0,
        scan_line: 104,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });
    atari_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: 112,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });
    let voffs = atari_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 50, 101, 97, 100, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });

    atari_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: 120,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });
    let voffs = atari_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });

    atari_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: 128,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });

    atari_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: 136,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs0,
        charset_memory_offset: coffs,
    });
    atari_data.insert_mode_line(&ModeLineDescr {
        mode: 0,
        scan_line: 144,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });

    atari_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 50, 101, 97, 100, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });

    atari_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    for scan_line in 0..240 {
        atari_data.set_gtia_regs(
            scan_line,
            &GTIARegs {
                hposp: [64 - 4, 96 - 4, 128 - 4, 160 - 4],
                hposm: [192 - 4, 194 - 4, 196 - 4, 198 - 4],
                sizep: [16, 16, 16, 16],
                sizem: 0,
                grafp: [0x55, 0x55, 0x55, 0x55],
                grafm: 0x55,
                col: [0x2a, 0x4a, 0x6a, 0x8a, 40, 202, 148, 70, 0],
                prior: 4,
                vdelay: 0,
                gractl: 0,
                hitclr: 0,
                consol: 0,
            },
        )
    }

    let atari_data_handle = atari_data_assets.add(atari_data);

    commands.spawn().insert_bundle((atari_data_handle,));

    for x in -1..=1 {
        for y in -1..=1 {
            commands.spawn_bundle(PipelinedSpriteBundle {
                sprite: Sprite::default(),
                texture: crate::ANTIC_IMAGE_HANDLE.typed(),
                transform: Transform {
                    translation: Vec3::new(x as f32 * 400.0, y as f32 * 240.0, 0.0),
                    ..Default::default()
                },
                global_transform: Default::default(),
            });
        }
    }

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}
