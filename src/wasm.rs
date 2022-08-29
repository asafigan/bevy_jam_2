use bevy::prelude::*;

pub struct WasmPlugin;

impl Plugin for WasmPlugin {
    fn build(&self, app: &mut App) {
        app
            // .insert_resource(WindowDescriptor {
            //     width: 200.0,
            //     height: 200.0,
            //     ..Default::default()
            // })
            .add_system(change_window_size);
    }
}

fn change_window_size(mut windows: ResMut<Windows>) {
    if let Some(window) = web_sys::window() {
        let width = window
            .inner_width()
            .ok()
            .and_then(|x| x.as_f64())
            .map(|x| (x - 1.0).floor() as f32);
        let height = window
            .inner_height()
            .ok()
            .and_then(|x| x.as_f64())
            .map(|x| (x - 1.0).floor() as f32);

        if let (Some(width), Some(height)) = (width, height) {
            let window = windows.get_primary_mut().unwrap();
            window.set_resolution(width, height);
        }
    }
}
