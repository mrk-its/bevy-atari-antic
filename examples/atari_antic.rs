use bevy::{
    app::AppExit,
    ecs::prelude::*,
    math::Vec3,
    prelude::{App, Assets, GlobalTransform, Handle, Transform},
    render2::{camera::OrthographicCameraBundle, view::Msaa},
    window::WindowDescriptor,
    PipelinedDefaultPlugins,
};
use bevy_atari_antic::atari_data::AnticData;
use bevy_atari_antic::{AtariAnticPlugin, ModeLineDescr};

fn main() {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        width: 768.0,
        height: 480.0,
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

fn quit_after_few_frames(mut cnt: Local<u32>, mut app_exit_events: EventWriter<AppExit>) {
    *cnt += 1;
    if *cnt > 5 {
        app_exit_events.send(AppExit);
    }
}

fn update(mut atari_data_assets: ResMut<Assets<AnticData>>, query: Query<&Handle<AnticData>>) {
    let span = bevy::utils::tracing::span!(bevy::utils::tracing::Level::INFO, "my_span");
    let entered = span.enter();
    for handle in query.iter() {
        if let Some(atari_data) = atari_data_assets.get_mut(handle) {
            let mut inner = atari_data.inner.write();
            let c = &mut inner.memory[1024];
            *c = c.wrapping_add(1);
            let c = &mut inner.memory[1024 + 31];
            *c = c.wrapping_add(1);
        }
    }
}

fn setup(mut commands: Commands, mut atari_data_assets: ResMut<Assets<AnticData>>) {
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

    {
        let mut atari_data = atari_data.inner.write();

        for scan_line in 0..240 {
            atari_data.gtia1.0[scan_line].colors[0] = 0;
            atari_data.gtia1.0[scan_line].colors[1] = 40;
            atari_data.gtia1.0[scan_line].colors[2] = 202;
            atari_data.gtia1.0[scan_line].colors[3] = 148;
            atari_data.gtia1.0[scan_line].colors[4] = 70;
            atari_data.gtia1.0[scan_line].colors[5] = 0;
            atari_data.gtia1.0[scan_line].colors[6] = 0;
            atari_data.gtia1.0[scan_line].colors[7] = 0;

            atari_data.gtia1.0[scan_line].colors_pm[0] = 0x2a;
            atari_data.gtia1.0[scan_line].colors_pm[1] = 0x4a;
            atari_data.gtia1.0[scan_line].colors_pm[2] = 0x6a;
            atari_data.gtia1.0[scan_line].colors_pm[3] = 0x8a;

            atari_data.gtia3.0[scan_line].hposp[0] = 64.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposp[1] = 96.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposp[2] = 128.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposp[3] = 160.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposm =
                [192.0 - 4.0, 194.0 - 4.0, 196.0 - 4.0, 198.0 - 4.0];
            atari_data.gtia3.0[scan_line].grafm = 0x55;
            atari_data.gtia3.0[scan_line].prior = 4;

            atari_data.gtia2.0[scan_line].player_size[0] = 16.0;
            atari_data.gtia2.0[scan_line].player_size[1] = 16.0;
            atari_data.gtia2.0[scan_line].player_size[2] = 16.0;
            atari_data.gtia2.0[scan_line].player_size[3] = 16.0;
            atari_data.gtia2.0[scan_line].missile_size = [4.0, 4.0, 4.0, 4.0];

            atari_data.gtia2.0[scan_line].grafp[0] = 0x55;
            atari_data.gtia2.0[scan_line].grafp[1] = 0x55;
            atari_data.gtia2.0[scan_line].grafp[2] = 0x55;
            atari_data.gtia2.0[scan_line].grafp[3] = 0x55;
        }
    }

    let atari_data_handle = atari_data_assets.add(atari_data);

    // cube
    commands.spawn().insert_bundle((
        Transform::from_xyz(-1.0, 0.0, 0.0),
        GlobalTransform::default(),
        atari_data_handle,
    ));

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.camera.name = Some("camera_3d".to_string());
    camera_bundle.transform.scale = Vec3::new(0.5, 0.5, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);

    // camera
    commands.spawn_bundle(camera_bundle);
}
