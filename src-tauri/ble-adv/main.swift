import Foundation
import CoreBluetooth

func parseArg(_ name: String, default def: String) -> String {
    if let idx = CommandLine.arguments.firstIndex(of: name), idx + 1 < CommandLine.arguments.count {
        return CommandLine.arguments[idx + 1]
    }
    return def
}

// 日志函数
func logDebug(_ message: String) {
    print("[SWIFT DEBUG] \(message)")
}

func logError(_ message: String) {
    print("[SWIFT ERROR] \(message)")
}

final class Advertiser: NSObject, CBPeripheralManagerDelegate {
    private var peripheralManager: CBPeripheralManager!
    private let advData: [String: Any]
    private let serviceUUID: CBUUID
    private var clipboardService: CBMutableService?
    private var clipboardCharacteristic: CBMutableCharacteristic?

    init(localName: String, serviceUUID: String) {
        logDebug("Initializing Advertiser with name: '\(localName)', UUID: '\(serviceUUID)'")
        self.serviceUUID = CBUUID(string: serviceUUID)
        self.advData = [
            CBAdvertisementDataLocalNameKey: localName,
            CBAdvertisementDataServiceUUIDsKey: [self.serviceUUID]
        ]
        super.init()
        logDebug("Creating CBPeripheralManager...")
        self.peripheralManager = CBPeripheralManager(delegate: self, queue: nil)
        
        // 安装 stdin 监听：Rust 端关闭管道 -> 这里收到 EOF -> 优雅停止
        logDebug("Setting up stdin monitoring for graceful shutdown...")
        FileHandle.standardInput.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty {
                // EOF，优雅停止并退出
                logDebug("Received EOF from stdin, stopping advertising...")
                self?.peripheralManager?.stopAdvertising()
                print("Advertising stopped (stdin closed).")
                FileHandle.standardInput.readabilityHandler = nil
                exit(0)
            }
            // 如果你未来需要通过stdin接收命令，可以在这里解析 data
        }
        logDebug("Advertiser initialization completed")
    }

    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        logDebug("Bluetooth state changed to: \(peripheral.state.rawValue)")
        
        switch peripheral.state {
        case .poweredOn:
            logDebug("Bluetooth is powered on, setting up services...")
            setupClipboardService()
            startAdvertising()
        case .poweredOff:
            logError("Bluetooth is powered off - cannot start advertising")
            print("Bluetooth is powered off")
        case .unsupported:
            logError("Bluetooth LE is not supported on this device")
            print("Bluetooth LE not supported")
        case .unauthorized:
            logError("App is not authorized to use Bluetooth")
            print("Bluetooth not authorized")
        case .resetting:
            logDebug("Bluetooth is resetting...")
            print("Bluetooth is resetting")
        case .unknown:
            logDebug("Bluetooth state is unknown")
            print("Bluetooth state unknown")
        @unknown default:
            logError("Unknown Bluetooth state: \(peripheral.state.rawValue)")
            print("Bluetooth state: \(peripheral.state.rawValue)")
        }
    }
    
    private func setupClipboardService() {
        logDebug("Setting up clipboard service with UUID: \(serviceUUID.uuidString)")
        
        // 创建剪贴板特征 (可读可写)
        let characteristicUUID = CBUUID(string: "87654321-4321-4321-4321-210987654321")
        clipboardCharacteristic = CBMutableCharacteristic(
            type: characteristicUUID,
            properties: [.read, .write, .notify],
            value: nil,
            permissions: [.readable, .writeable]
        )
        
        logDebug("Created clipboard characteristic with UUID: \(characteristicUUID.uuidString)")
        
        // 创建剪贴板服务
        clipboardService = CBMutableService(type: serviceUUID, primary: true)
        clipboardService?.characteristics = [clipboardCharacteristic!]
        
        logDebug("Created clipboard service, adding to peripheral manager...")
        
        // 添加服务到外设管理器
        peripheralManager.add(clipboardService!)
    }
    
    private func startAdvertising() {
        logDebug("Starting advertising...")
        peripheralManager.startAdvertising(self.advData)
        if let name = self.advData[CBAdvertisementDataLocalNameKey] as? String {
            let uuidString = serviceUUID.uuidString
            logDebug("Advertising data prepared - Name: '\(name)', UUID: \(uuidString)")
            print("Advertising started as '\(name)' with service \(uuidString)")
        } else {
            logError("Failed to extract advertising data")
            print("Advertising started")
        }
    }
    
    // MARK: - CBPeripheralManagerDelegate Methods
    
    func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {
        if let error = error {
            logError("Failed to add service: \(error.localizedDescription)")
            return
        }
        logDebug("Successfully added service: \(service.uuid.uuidString)")
        print("Service added: \(service.uuid.uuidString)")
    }
    
    func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveRead request: CBATTRequest) {
        logDebug("Received read request for characteristic: \(request.characteristic.uuid.uuidString)")
        
        if request.characteristic.uuid == clipboardCharacteristic?.uuid {
            // 返回当前剪贴板内容
            let clipboardContent = "Hello from Pasto!"
            request.value = clipboardContent.data(using: .utf8)
            peripheral.respond(to: request, withResult: .success)
            logDebug("Responded to read request with: \(clipboardContent)")
        } else {
            peripheral.respond(to: request, withResult: .attributeNotFound)
            logError("Read request for unknown characteristic")
        }
    }
    
    func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
        logDebug("Received \(requests.count) write request(s)")
        
        for request in requests {
            if request.characteristic.uuid == clipboardCharacteristic?.uuid {
                if let data = request.value, let content = String(data: data, encoding: .utf8) {
                    logDebug("Received clipboard content: \(content)")
                    print("Clipboard updated: \(content)")
                    // 这里可以将内容写入系统剪贴板
                } else {
                    logError("Failed to decode write request data")
                }
            }
        }
        
        peripheral.respond(to: requests.first!, withResult: .success)
    }
    
    func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) {
        if let error = error {
            logError("Failed to start advertising: \(error.localizedDescription)")
            return
        }
        logDebug("Successfully started advertising")
        print("BLE advertising active")
    }
}

// 程序启动日志
logDebug("BLE Advertiser starting...")
logDebug("Command line arguments: \(CommandLine.arguments)")
logDebug("Working directory: \(FileManager.default.currentDirectoryPath)")
logDebug("Process ID: \(ProcessInfo.processInfo.processIdentifier)")

let name = parseArg("--name", default: "Pasto")
let uuid = parseArg("--uuid", default: "12345678-1234-1234-1234-1234567890AB")

logDebug("Parsed arguments - Name: '\(name)', UUID: '\(uuid)'")

// 验证UUID格式
let uuidObj = CBUUID(string: uuid)
logDebug("UUID validation - Input: '\(uuid)', Parsed: '\(uuidObj.uuidString)'")

logDebug("Creating Advertiser instance...")
let _ = Advertiser(localName: name, serviceUUID: uuid)
logDebug("Starting main run loop...")
RunLoop.main.run()