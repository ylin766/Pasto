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
    println!("[BLE_SCAN_DEBUG] Starting BLE device scan...");
    
    let manager = Manager::new()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to initialize BLE manager: {}", e);
            println!("[BLE_SCAN_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SCAN_DEBUG] BLE manager initialized");

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to get adapters: {}", e);
            println!("[BLE_SCAN_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SCAN_DEBUG] Found {} BLE adapters", adapters.len());

    let Some(adapter) = adapters.into_iter().next() else {
        let error_msg = "No Bluetooth adapters found".to_string();
        println!("[BLE_SCAN_ERROR] {}", error_msg);
        return Err(error_msg);
    };
    println!("[BLE_SCAN_DEBUG] Using first available adapter");

    // Stop any ongoing scan, then start scan
    println!("[BLE_SCAN_DEBUG] Stopping any previous scan...");
    adapter
        .stop_scan()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to stop previous scan: {}", e);
            println!("[BLE_SCAN_ERROR] {}", error_msg);
            error_msg
        })?;

    println!("[BLE_SCAN_DEBUG] Starting new scan...");
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to start scan: {}", e);
            println!("[BLE_SCAN_ERROR] {}", error_msg);
            error_msg
        })?;

    // Wait to gather results
    println!("[BLE_SCAN_DEBUG] Scanning for 3 seconds...");
    sleep(Duration::from_secs(3)).await;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to get peripherals: {}", e);
            println!("[BLE_SCAN_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SCAN_DEBUG] Found {} total peripherals", peripherals.len());

    let mut devices = Vec::new();
    let service_uuid = Uuid::parse_str(CLIPBOARD_SERVICE_UUID).unwrap();
    println!("[BLE_SCAN_DEBUG] Looking for devices with clipboard service UUID: {}", service_uuid);
    
    for (i, peripheral) in peripherals.iter().enumerate() {
        let address = peripheral.address().to_string();
        println!("[BLE_SCAN_DEBUG] Checking peripheral {}: {}", i + 1, address);
        
        let props_opt = peripheral
            .properties()
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to get peripheral properties for {}: {}", address, e);
                println!("[BLE_SCAN_ERROR] {}", error_msg);
                error_msg
            })?;
            
        if let Some(props) = props_opt {
            let name = props.local_name.clone();
            let rssi = props.rssi;
            
            println!("[BLE_SCAN_DEBUG] Device {}: name={:?}, rssi={:?}, services={:?}", 
                address, name, rssi, props.services);
            
            // 只返回包含剪贴板服务的设备
            if props.services.contains(&service_uuid) {
                println!("[BLE_SCAN_DEBUG] ✓ Device {} has clipboard service, adding to results", address);
                devices.push(BleDevice { name, address, rssi });
            } else {
                println!("[BLE_SCAN_DEBUG] ✗ Device {} does not have clipboard service", address);
            }
        } else {
            println!("[BLE_SCAN_DEBUG] Device {} has no properties available", address);
        }
    }
    
    println!("[BLE_SCAN_DEBUG] Scan completed. Found {} devices with clipboard service", devices.len());
    Ok(devices)
}

pub async fn connect_to_clipboard_device(address: &str) -> Result<(), String> {
    println!("[BLE_DEBUG] Starting connection to device: {}", address);
    
    let manager = Manager::new()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to initialize BLE manager: {}", e);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] BLE manager initialized successfully");

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to get adapters: {}", e);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] Found {} BLE adapters", adapters.len());

    let Some(adapter) = adapters.into_iter().next() else {
        let error_msg = "No Bluetooth adapters found".to_string();
        println!("[BLE_ERROR] {}", error_msg);
        return Err(error_msg);
    };
    println!("[BLE_DEBUG] Using BLE adapter");

    // 开始扫描寻找目标设备
    println!("[BLE_DEBUG] Starting scan for target device...");
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to start scan: {}", e);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;

    sleep(Duration::from_secs(2)).await;
    println!("[BLE_DEBUG] Scan completed, retrieving peripherals...");

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to get peripherals: {}", e);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] Found {} peripherals", peripherals.len());

    // 查找目标设备
    println!("[BLE_DEBUG] Searching for target device with address: {}", address);
    let target_peripheral = peripherals
        .into_iter()
        .find(|p| {
            let device_address = p.address().to_string();
            println!("[BLE_DEBUG] Checking device: {}", device_address);
            device_address == address
        })
        .ok_or_else(|| {
            let error_msg = format!("Device with address {} not found", address);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] Target device found, attempting connection...");

    // 连接到设备
    target_peripheral
        .connect()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to connect to device: {}", e);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] Successfully connected to device");

    // 发现服务
    println!("[BLE_DEBUG] Discovering services...");
    target_peripheral
        .discover_services()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to discover services: {}", e);
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    
    let services = target_peripheral.services();
    println!("[BLE_DEBUG] Discovered {} services", services.len());
    
    // 打印所有发现的服务UUID用于调试
    for (i, service) in services.iter().enumerate() {
        println!("[BLE_DEBUG] Service {}: UUID = {}", i + 1, service.uuid);
        for (j, characteristic) in service.characteristics.iter().enumerate() {
            println!("[BLE_DEBUG]   Characteristic {}: UUID = {}, Properties = {:?}", 
                j + 1, characteristic.uuid, characteristic.properties);
        }
    }

    // 查找剪贴板服务
    let service_uuid = Uuid::parse_str(CLIPBOARD_SERVICE_UUID).unwrap();
    println!("[BLE_DEBUG] Looking for clipboard service with UUID: {}", service_uuid);
    
    let clipboard_service = services
        .iter()
        .find(|s| s.uuid == service_uuid)
        .ok_or_else(|| {
            let error_msg = format!(
                "Clipboard service not found. Expected UUID: {}, Available services: [{}]", 
                service_uuid,
                services.iter().map(|s| s.uuid.to_string()).collect::<Vec<_>>().join(", ")
            );
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] Clipboard service found successfully");

    // 查找剪贴板特征
    let char_uuid = Uuid::parse_str(CLIPBOARD_CHAR_UUID).unwrap();
    println!("[BLE_DEBUG] Looking for clipboard characteristic with UUID: {}", char_uuid);
    println!("[BLE_DEBUG] Service has {} characteristics", clipboard_service.characteristics.len());
    
    // 打印服务中的所有特征用于调试
    for (i, characteristic) in clipboard_service.characteristics.iter().enumerate() {
        println!("[BLE_DEBUG] Characteristic {}: UUID = {}, Properties = {:?}", 
            i + 1, characteristic.uuid, characteristic.properties);
    }
    
    let clipboard_char = clipboard_service
        .characteristics
        .iter()
        .find(|c| c.uuid == char_uuid)
        .ok_or_else(|| {
            let error_msg = format!(
                "Clipboard characteristic not found. Expected UUID: {}, Available characteristics: [{}]",
                char_uuid,
                clipboard_service.characteristics.iter()
                    .map(|c| c.uuid.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("[BLE_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_DEBUG] Clipboard characteristic found successfully");
    println!("[BLE_DEBUG] Characteristic properties: {:?}", clipboard_char.properties);

    // 订阅通知
    if clipboard_char.properties.contains(CharPropFlags::NOTIFY) {
        println!("[BLE_DEBUG] Characteristic supports notifications, subscribing...");
        target_peripheral
            .subscribe(clipboard_char)
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to subscribe to notifications: {}", e);
                println!("[BLE_ERROR] {}", error_msg);
                error_msg
            })?;
        println!("[BLE_DEBUG] Successfully subscribed to notifications");
    } else {
        println!("[BLE_DEBUG] Characteristic does not support notifications");
    }

    // 保存连接的设备
    {
        let mut connected = CONNECTED_PERIPHERAL.lock().unwrap();
        *connected = Some(target_peripheral);
    }
    println!("[BLE_DEBUG] Device connection completed successfully");

    Ok(())
}

pub async fn send_clipboard_update(content: &str) -> Result<(), String> {
    println!("[BLE_SEND_DEBUG] Starting clipboard update send, content length: {} chars", content.len());
    
    let peripheral = {
        let connected = CONNECTED_PERIPHERAL.lock().unwrap();
        connected.clone()
    };

    let Some(peripheral) = peripheral else {
        let error_msg = "No device connected".to_string();
        println!("[BLE_SEND_ERROR] {}", error_msg);
        return Err(error_msg);
    };
    println!("[BLE_SEND_DEBUG] Connected device found");

    // 创建剪贴板数据
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let clipboard_data = ClipboardData {
        content: content.to_string(),
        timestamp,
    };
    println!("[BLE_SEND_DEBUG] Created clipboard data with timestamp: {}", timestamp);

    let data = serde_json::to_string(&clipboard_data)
        .map_err(|e| {
            let error_msg = format!("Failed to serialize clipboard data: {}", e);
            println!("[BLE_SEND_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SEND_DEBUG] Serialized data length: {} bytes", data.len());

    // 查找剪贴板特征
    let service_uuid = Uuid::parse_str(CLIPBOARD_SERVICE_UUID).unwrap();
    let char_uuid = Uuid::parse_str(CLIPBOARD_CHAR_UUID).unwrap();
    println!("[BLE_SEND_DEBUG] Looking for service UUID: {} and characteristic UUID: {}", service_uuid, char_uuid);
    
    let services = peripheral.services();
    println!("[BLE_SEND_DEBUG] Device has {} services available", services.len());
    
    let clipboard_service = services
        .iter()
        .find(|s| s.uuid == service_uuid)
        .ok_or_else(|| {
            let error_msg = format!(
                "Clipboard service not found during send. Expected: {}, Available: [{}]",
                service_uuid,
                services.iter().map(|s| s.uuid.to_string()).collect::<Vec<_>>().join(", ")
            );
            println!("[BLE_SEND_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SEND_DEBUG] Clipboard service found");

    let clipboard_char = clipboard_service
        .characteristics
        .iter()
        .find(|c| c.uuid == char_uuid)
        .ok_or_else(|| {
            let error_msg = format!(
                "Clipboard characteristic not found during send. Expected: {}, Available: [{}]",
                char_uuid,
                clipboard_service.characteristics.iter()
                    .map(|c| c.uuid.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("[BLE_SEND_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SEND_DEBUG] Clipboard characteristic found, properties: {:?}", clipboard_char.properties);

    // 写入数据
    println!("[BLE_SEND_DEBUG] Writing {} bytes to characteristic...", data.as_bytes().len());
    peripheral
        .write(clipboard_char, data.as_bytes(), WriteType::WithoutResponse)
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to write clipboard data: {}", e);
            println!("[BLE_SEND_ERROR] {}", error_msg);
            error_msg
        })?;
    println!("[BLE_SEND_DEBUG] Clipboard data sent successfully");

    Ok(())
}