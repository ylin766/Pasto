use btleplug::api::{Central, Manager as _, ScanFilter, Peripheral as _, WriteType, CharPropFlags};
use btleplug::platform::{Manager, Peripheral};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;
use tokio::time::{sleep, Duration};

// 剪贴板服务和特征的UUID
const CLIPBOARD_SERVICE_UUID: &str = "12345678-1234-1234-1234-1234567890AB";
const CLIPBOARD_CHAR_UUID: &str = "87654321-4321-4321-4321-BA0987654321";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BleDevice {
    pub name: Option<String>,
    pub address: String,
    pub rssi: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClipboardData {
    content: String,
    timestamp: u64,
}

// 全局连接状态
static CONNECTED_PERIPHERAL: Mutex<Option<Peripheral>> = Mutex::new(None);

// Windows平台的BLE广播功能（使用Windows原生API的占位符实现）
#[cfg(target_os = "windows")]
pub async fn start_ble_advertising_windows(_name: Option<String>, _service_uuid: Option<String>) -> Result<(), String> {
    // 注意：btleplug不支持Windows上的BLE广播功能
    // 这里需要使用Windows原生的Bluetooth API或其他库
    // 目前返回错误提示用户此功能在Windows上暂不可用
    Err("BLE advertising on Windows requires native Windows Bluetooth API implementation. This feature is not yet implemented.".to_string())
}

// Windows平台停止BLE广播
#[cfg(target_os = "windows")]
pub async fn stop_ble_advertising_windows() -> Result<(), String> {
    // 对应的停止实现
    Err("BLE advertising on Windows requires native Windows Bluetooth API implementation. This feature is not yet implemented.".to_string())
}

pub async fn scan_ble_devices_once() -> Result<Vec<BleDevice>, String> {
    let manager = Manager::new()
        .await
        .map_err(|e| format!("Failed to initialize BLE manager: {}", e))?;

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("Failed to get adapters: {}", e))?;

    let Some(adapter) = adapters.into_iter().next() else {
        return Err("No Bluetooth adapters found".to_string());
    };

    // Stop any ongoing scan, then start scan
    adapter
        .stop_scan()
        .await
        .map_err(|e| format!("Failed to stop previous scan: {}", e))?;

    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| format!("Failed to start scan: {}", e))?;

    // Wait to gather results
    sleep(Duration::from_secs(3)).await;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Failed to get peripherals: {}", e))?;

    let mut devices = Vec::new();
    for peripheral in peripherals {
        let props_opt = peripheral
            .properties()
            .await
            .map_err(|e| format!("Failed to get peripheral properties: {}", e))?;
        if let Some(props) = props_opt {
            let address = peripheral.address().to_string();
            let name = props.local_name;
            let rssi = props.rssi;
            
            // 只返回包含剪贴板服务的设备
            let service_uuid = Uuid::parse_str(CLIPBOARD_SERVICE_UUID).unwrap();
            if props.services.contains(&service_uuid) {
                devices.push(BleDevice { name, address, rssi });
            }
        }
    }

    Ok(devices)
}

pub async fn connect_to_clipboard_device(address: &str) -> Result<(), String> {
    let manager = Manager::new()
        .await
        .map_err(|e| format!("Failed to initialize BLE manager: {}", e))?;

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("Failed to get adapters: {}", e))?;

    let Some(adapter) = adapters.into_iter().next() else {
        return Err("No Bluetooth adapters found".to_string());
    };

    // 开始扫描寻找目标设备
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| format!("Failed to start scan: {}", e))?;

    sleep(Duration::from_secs(2)).await;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Failed to get peripherals: {}", e))?;

    // 查找目标设备
    let target_peripheral = peripherals
        .into_iter()
        .find(|p| p.address().to_string() == address)
        .ok_or_else(|| format!("Device with address {} not found", address))?;

    // 连接到设备
    target_peripheral
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to device: {}", e))?;

    // 发现服务
    target_peripheral
        .discover_services()
        .await
        .map_err(|e| format!("Failed to discover services: {}", e))?;

    // 查找剪贴板服务
    let service_uuid = Uuid::parse_str(CLIPBOARD_SERVICE_UUID).unwrap();
    let services = target_peripheral.services();
    let clipboard_service = services
        .iter()
        .find(|s| s.uuid == service_uuid)
        .ok_or_else(|| "Clipboard service not found".to_string())?;

    // 查找剪贴板特征
    let char_uuid = Uuid::parse_str(CLIPBOARD_CHAR_UUID).unwrap();
    let clipboard_char = clipboard_service
        .characteristics
        .iter()
        .find(|c| c.uuid == char_uuid)
        .ok_or_else(|| "Clipboard characteristic not found".to_string())?;

    // 订阅通知
    if clipboard_char.properties.contains(CharPropFlags::NOTIFY) {
        target_peripheral
            .subscribe(clipboard_char)
            .await
            .map_err(|e| format!("Failed to subscribe to notifications: {}", e))?;
    }

    // 保存连接的设备
    {
        let mut connected = CONNECTED_PERIPHERAL.lock().unwrap();
        *connected = Some(target_peripheral);
    }

    Ok(())
}

pub async fn send_clipboard_update(content: &str) -> Result<(), String> {
    let peripheral = {
        let connected = CONNECTED_PERIPHERAL.lock().unwrap();
        connected.clone()
    };

    let Some(peripheral) = peripheral else {
        return Err("No device connected".to_string());
    };

    // 创建剪贴板数据
    let clipboard_data = ClipboardData {
        content: content.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let data = serde_json::to_string(&clipboard_data)
        .map_err(|e| format!("Failed to serialize clipboard data: {}", e))?;

    // 查找剪贴板特征
    let service_uuid = Uuid::parse_str(CLIPBOARD_SERVICE_UUID).unwrap();
    let char_uuid = Uuid::parse_str(CLIPBOARD_CHAR_UUID).unwrap();
    
    let services = peripheral.services();
    let clipboard_service = services
        .iter()
        .find(|s| s.uuid == service_uuid)
        .ok_or_else(|| "Clipboard service not found".to_string())?;

    let clipboard_char = clipboard_service
        .characteristics
        .iter()
        .find(|c| c.uuid == char_uuid)
        .ok_or_else(|| "Clipboard characteristic not found".to_string())?;

    // 写入数据
    peripheral
        .write(clipboard_char, data.as_bytes(), WriteType::WithoutResponse)
        .await
        .map_err(|e| format!("Failed to write clipboard data: {}", e))?;

    Ok(())
}