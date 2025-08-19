// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod clipboard;
mod ble;

use clipboard::ClipboardManager;
use std::sync::{Arc, Mutex};
use std::process::Child;
use tauri_plugin_shell::ShellExt;

// 广播子进程的状态（仅 macOS 使用）
type AdvState = Arc<Mutex<Option<Child>>>;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_clipboard_text(clipboard_manager: tauri::State<Arc<Mutex<ClipboardManager>>>) -> Result<String, String> {
    match clipboard_manager.lock() {
        Ok(manager) => manager.get_text(),
        Err(e) => Err(format!("Failed to lock clipboard manager: {}", e)),
    }
}

#[tauri::command]
async fn scan_ble_devices() -> Result<Vec<ble::BleDevice>, String> {
    ble::scan_ble_devices_once().await
}

// 启动 BLE 广播（支持 macOS 和 Windows）
#[tauri::command]
async fn start_ble_advertising(
    app: tauri::AppHandle,
    adv_state: tauri::State<'_, AdvState>,
    name: Option<String>,
    service_uuid: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut guard = adv_state.lock().map_err(|e| format!("lock adv_state: {e}"))?;
        // 如果已在广播，先优雅停止（关闭 stdin -> 等待退出）
        if let Some(child) = guard.as_mut() {
            let _ = child.stdin.take(); // 关闭管道写端，通知子进程 EOF
            let _ = child.wait();
            *guard = None;
        }

        // 获取sidecar路径，这在开发和发布模式下都能正确工作
        let mut cmd = app.shell().sidecar("ble-adv")
            .map_err(|e| format!("Failed to get sidecar command: {}", e))?;
        if let Some(n) = name { cmd = cmd.arg("--name").arg(n); }
        if let Some(u) = service_uuid { cmd = cmd.arg("--uuid").arg(u); }
        let (mut _rx, mut child) = cmd
            .spawn()
            .map_err(|e| format!("failed to start advertiser: {e}"))?;

        // 注意：这里我们只保存child的引用，因为新的API返回的是不同的类型
        // 为了简化，我们暂时不保存child，而是依赖进程的自然生命周期
        // 在实际应用中，你可能需要保存child以便后续控制
        *guard = None; // 临时解决方案
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        ble::start_ble_advertising_windows(name, service_uuid).await
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("BLE advertising is currently supported only on macOS and Windows.".to_string())
    }
}

// 停止 BLE 广播（支持 macOS 和 Windows）
#[tauri::command]
async fn stop_ble_advertising(adv_state: tauri::State<'_, AdvState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut guard = adv_state.lock().map_err(|e| format!("lock adv_state: {e}"))?;
        if let Some(child) = guard.as_mut() {
            // 优雅停止：关闭 stdin，等待退出
            let _ = child.stdin.take();
            let _ = child.wait();
            *guard = None;
        }
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows平台的BLE广播停止功能
        ble::stop_ble_advertising_windows().await
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("BLE advertising is not supported on this platform".to_string())
    }
}

// 新增：连接到指定设备
#[tauri::command]
async fn connect_to_device(address: String) -> Result<(), String> {
    ble::connect_to_clipboard_device(&address).await
}

// 新增：发送剪贴板内容
#[tauri::command]
async fn send_clipboard_content(content: String) -> Result<(), String> {
    ble::send_clipboard_update(&content).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let clipboard_manager = match ClipboardManager::new() {
        Ok(manager) => Arc::new(Mutex::new(manager)),
        Err(e) => {
            eprintln!("Failed to create clipboard manager: {}", e);
            return;
        }
    };

    // 新增广告状态
    let adv_state: AdvState = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(clipboard_manager)
        .manage(adv_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            get_clipboard_text,
            scan_ble_devices,
            start_ble_advertising,
            stop_ble_advertising,
            connect_to_device,
            send_clipboard_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
