use mouse_position::mouse_position::Mouse;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, WindowEvent,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

mod commands;
mod db;
mod sync;

pub struct ShortcutState {
    pub current: Mutex<String>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let shortcut_str = shortcut.to_string();
                        // Screenshot shortcut
                        if shortcut_str.contains('S') && !shortcut_str.contains("Shift+I") {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.emit("start-screenshot", ());
                            }
                            return;
                        }
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
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("focus-input", ());
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("app data dir should exist");
            std::fs::create_dir_all(&app_dir).ok();

            // Check for custom data directory
            let db_path = {
                let config_file = app_dir.join("db_path.txt");
                if let Ok(contents) = std::fs::read_to_string(&config_file) {
                    let custom = contents.trim().to_string();
                    if !custom.is_empty() {
                        let p = std::path::PathBuf::from(&custom);
                        std::fs::create_dir_all(&p).ok();
                        p.join("inspiration.db")
                    } else {
                        app_dir.join("inspiration.db")
                    }
                } else {
                    app_dir.join("inspiration.db")
                }
            };

            let db = db::Database::init(db_path)
                .expect("database should initialize");

            // Load saved shortcut or default
            let shortcut = db
                .get_setting("shortcut")
                .unwrap_or(None)
                .unwrap_or_else(|| "Ctrl+Shift+I".to_string());
            app.manage(ShortcutState {
                current: Mutex::new(shortcut.clone()),
            });

            // Register initial shortcut
            app.global_shortcut()
                .register(shortcut.as_str())
                .expect("global shortcut should register");

            // Screenshot shortcut (load saved or default)
            let ss_shortcut = db
                .get_setting("screenshot_shortcut")
                .unwrap_or(None)
                .unwrap_or_else(|| "Ctrl+Shift+S".to_string());
            app.global_shortcut()
                .register(ss_shortcut.as_str())
                .expect("screenshot shortcut should register");

            app.manage(db);

            // System tray icon
            let show_item = MenuItemBuilder::with_id("show", "Show").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Inspiration")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up, ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.minimize();
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
            commands::save_setting,
            commands::get_setting,
            commands::change_shortcut,
            commands::get_shortcut,
            commands::get_autostart,
            commands::set_autostart,
            commands::set_data_dir,
            commands::get_data_dir,
            commands::take_screenshot,
            commands::get_screenshot_shortcut,
            commands::change_screenshot_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
