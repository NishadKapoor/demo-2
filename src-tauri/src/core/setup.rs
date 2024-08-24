use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    webview::DownloadEvent, App, LogicalPosition, Manager, PhysicalSize, WebviewBuilder,
    WebviewUrl, WindowBuilder, WindowEvent,
};
use tauri_plugin_shell::ShellExt;

#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;

use crate::core::{
    conf::AppConf,
    constant::{ASK_HEIGHT, INIT_SCRIPT, TITLEBAR_HEIGHT},
    template,
};

pub fn init(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();
    let conf = AppConf::load(handle)?;
    let ask_mode_height = if conf.ask_mode { ASK_HEIGHT } else { 0.0 };
    template::Template::new(AppConf::get_scripts_path(handle)?);

    tauri::async_runtime::spawn({
        let handle = handle.clone();
        let scale_factor: f64;

        async move {
            let mut core_window = WindowBuilder::new(&handle, "core")
                .title("ChatGPT")
                .resizable(true)
                .inner_size(800.0, 600.0)
                .min_inner_size(300.0, 200.0)
                .theme(Some(AppConf::get_theme(&handle)));

            #[cfg(target_os = "macos")]
            {
                core_window = core_window
                    .title_bar_style(TitleBarStyle::Overlay)
                    .hidden_title(true);
            }

            let core_window = core_window
                .build()
                .expect("[core:window] Failed to build window");

            let win_size = core_window.inner_size().expect("[core:window] Failed to get window size");
            let window = Arc::new(Mutex::new(core_window));
            scale_factor = window.lock().unwrap().scale_factor().unwrap();

            let download_path = Arc::new(Mutex::new(PathBuf::new()));
            let app_handle = handle.clone();

            let main_view = WebviewBuilder::new("main", WebviewUrl::App("https://chatgpt.com".into()))
                .auto_resize()
                .on_download(move |_, event| handle_download_event(&app_handle, &download_path, event))
                .initialization_script(&AppConf::load_script(&handle, "ask.js"))
                .initialization_script(INIT_SCRIPT);

            let titlebar_view = WebviewBuilder::new("titlebar", WebviewUrl::App("index.html".into()))
                .auto_resize();

            let ask_view = WebviewBuilder::new("ask", WebviewUrl::App("index.html".into()))
                .auto_resize();

            let win = window.lock().unwrap();
            let titlebar_height = (scale_factor * TITLEBAR_HEIGHT).round() as u32;
            let ask_height = (scale_factor * ask_mode_height).round() as u32;

            #[cfg(target_os = "macos")]
            setup_macos_views(&win, &main_view, &titlebar_view, &ask_view, win_size, titlebar_height, ask_height);

            #[cfg(not(target_os = "macos"))]
            setup_non_macos_views(&win, &main_view, &titlebar_view, &ask_view, win_size, titlebar_height, ask_height);

            let window_clone = Arc::clone(&window);
            win.on_window_event(move |event| {
                if let WindowEvent::Resized(size) = event {
                    update_view_positions(&window_clone, size, scale_factor, titlebar_height, ask_height);
                }
            });
        }
    });

    Ok(())
}

// Function to handle download events
fn handle_download_event(app_handle: &tauri::AppHandle, download_path: &Arc<Mutex<PathBuf>>, event: DownloadEvent) {
    match event {
        DownloadEvent::Requested { destination, .. } => {
            let download_dir = app_handle.path().download_dir().expect("[view:download] Failed to get download directory");
            let mut locked_path = download_path.lock().expect("[view:download] Failed to lock download path");
            *locked_path = download_dir.join(&destination);
            *destination = locked_path.clone();
        }
        DownloadEvent::Finished { success, .. } => {
            let final_path = download_path.lock().expect("[view:download] Failed to lock download path").clone();
            if success {
                app_handle.shell().open(final_path.to_string_lossy(), None).expect("[view:download] Failed to open file");
            }
        }
        _ => (),
    }
}

// Setup views for macOS
#[cfg(target_os = "macos")]
fn setup_macos_views(win: &tauri::Window, main_view: &WebviewBuilder, titlebar_view: &WebviewBuilder, ask_view: &WebviewBuilder, win_size: PhysicalSize<u32>, titlebar_height: u32, ask_height: u32) {
    win.add_child(
        titlebar_view,
        LogicalPosition::new(0, 0),
        PhysicalSize::new(win_size.width, titlebar_height),
    ).unwrap();
    win.add_child(
        ask_view,
        LogicalPosition::new(0.0, (win_size.height as f64 / win.scale_factor().unwrap()) - ASK_HEIGHT),
        PhysicalSize::new(win_size.width, ask_height),
    ).unwrap();
    win.add_child(
        main_view,
        LogicalPosition::new(0.0, TITLEBAR_HEIGHT),
        PhysicalSize::new(win_size.width, win_size.height - titlebar_height - ask_height),
    ).unwrap();
}

// Setup views for non-macOS
#[cfg(not(target_os = "macos"))]
fn setup_non_macos_views(win: &tauri::Window, main_view: &WebviewBuilder, titlebar_view: &WebviewBuilder, ask_view: &WebviewBuilder, win_size: PhysicalSize<u32>, titlebar_height: u32, ask_height: u32) {
    win.add_child(
        ask_view,
        LogicalPosition::new(0.0, (win_size.height as f64 / win.scale_factor().unwrap()) - ASK_HEIGHT),
        PhysicalSize::new(win_size.width, ask_height),
    ).unwrap();
    win.add_child(
        titlebar_view,
        LogicalPosition::new(0.0, (win_size.height as f64 / win.scale_factor().unwrap()) - ASK_HEIGHT - TITLEBAR_HEIGHT),
        PhysicalSize::new(win_size.width, titlebar_height),
    ).unwrap();
    win.add_child(
        main_view,
        LogicalPosition::new(0.0, 0.0),
        PhysicalSize::new(win_size.width, win_size.height - (ask_height + titlebar_height)),
    ).unwrap();
}

// Update view positions based on window resize
fn update_view_positions(window_clone: &Arc<Mutex<tauri::Window>>, size: PhysicalSize<u32>, scale_factor: f64, titlebar_height: u32, ask_height: u32) {
    let win = window_clone.lock().unwrap();
    let main_view = win.get_webview("main").expect("[view:main] Failed to get webview window");
    let titlebar_view = win.get_webview("titlebar").expect("[view:titlebar] Failed to get webview window");
    let ask_view = win.get_webview("ask").expect("[view:ask] Failed to get webview window");

    #[cfg(target_os = "macos")]
    {
        set_view_properties(
            &main_view,
            LogicalPosition::new(0.0, TITLEBAR_HEIGHT),
            PhysicalSize::new(size.width, size.height - (titlebar_height + ask_height)),
        );
        set_view_properties(
            &titlebar_view,
            LogicalPosition::new(0.0, 0.0),
            PhysicalSize::new(size.width, titlebar_height),
        );
        set_view_properties(
            &ask_view,
            LogicalPosition::new(0.0, (size.height as f64 / scale_factor) - ASK_HEIGHT),
            PhysicalSize::new(size.width, ask_height),
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        set_view_properties(
            &main_view,
            LogicalPosition::new(0.0, 0.0),
            PhysicalSize::new(size.width, size.height - (ask_height + titlebar_height)),
        );
        set_view_properties(
            &titlebar_view,
            LogicalPosition::new(0.0, (size.height as f64 / scale_factor) - TITLEBAR_HEIGHT),
            PhysicalSize::new(size.width, titlebar_height),
        );
        set_view_properties(
            &ask_view,
            LogicalPosition::new(0.0, (size.height as f64 / scale_factor) - ASK_HEIGHT - TITLEBAR_HEIGHT),
            PhysicalSize::new(size.width, ask_height),
        );
    }
}

// Helper function to set view properties
fn set_view_properties(view: &tauri::Webview, position: LogicalPosition<f64>, size: PhysicalSize<u32>) {
    if let Err(e) = view.set_position(position) {
        eprintln!("[view:position] Failed to set view position: {}", e);
    }
    if let Err(e) = view.set_size(size) {
        eprintln!("[view:size] Failed to set view size: {}", e);
    }
}
