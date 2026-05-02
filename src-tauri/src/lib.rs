use mouse_position::mouse_position::Mouse;
use tauri::{Emitter, Manager, PhysicalPosition, WindowEvent};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

mod commands;
mod db;
mod sync;

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            let pos = Mouse::get_mouse_position();
                            match pos {
                                Mouse::Position { x, y } => {
                                    let x = x as i32 + 15;
                                    let y = y as i32 + 15;
                                    if let Ok(size) = window.outer_size() {
                                        if let Ok(Some(monitor)) = window.current_monitor() {
                                            let mon = monitor.size();
                                            let mon_w = mon.width as i32;
                                            let mon_h = mon.height as i32;
                                            let scale = monitor.scale_factor();
                                            let win_w = (size.width as f64 / scale) as i32;
                                            let win_h = (size.height as f64 / scale) as i32;

                                            let adj_x = if x + win_w > mon_w { x - win_w - 15 } else { x };
                                            let adj_y = if y + win_h > mon_h { y - win_h - 15 } else { y };

                                            let _ = window.set_position(PhysicalPosition::new(
                                                adj_x.max(0),
                                                adj_y.max(0),
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    let _ = window.center();
                                }
                            }
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("focus-input", ());
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("app data dir should exist");
            std::fs::create_dir_all(&app_dir).ok();
            let db = db::Database::init(app_dir.join("inspiration.db"))
                .expect("database should initialize");
            app.manage(db);

            #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
            app.global_shortcut()
                .register("Ctrl+Shift+I")
                .expect("global shortcut should register");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_idea,
            commands::get_ideas,
            commands::update_idea,
            commands::delete_idea,
            commands::toggle_todo,
            commands::toggle_todo_done,
            commands::add_tag,
            commands::remove_tag,
            commands::get_tags,
            commands::rewrite_idea,
            commands::save_webdav_config,
            commands::get_webdav_config,
            commands::save_ai_config,
            commands::get_ai_config,
            commands::sync_now,
            commands::hide_window,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
