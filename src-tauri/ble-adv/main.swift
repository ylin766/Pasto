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

    init(localName: String, serviceUUID: String) {
        logDebug("Initializing Advertiser with name: '\(localName)', UUID: '\(serviceUUID)'")
        self.advData = [
            CBAdvertisementDataLocalNameKey: localName,
            CBAdvertisementDataServiceUUIDsKey: [CBUUID(string: serviceUUID)]
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
            logDebug("Bluetooth is powered on, starting advertising...")
            peripheral.startAdvertising(self.advData)
            if let name = self.advData[CBAdvertisementDataLocalNameKey] as? String,
               let uuids = self.advData[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID] {
                let uuidStrings = uuids.map { $0.uuidString }.joined(separator: ",")
                logDebug("Advertising data prepared - Name: '\(name)', UUIDs: [\(uuidStrings)]")
                print("Advertising started as '\(name)' with service \(uuidStrings)")
            } else {
                logError("Failed to extract advertising data")
                print("Advertising started")
            }
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