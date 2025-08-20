import Foundation
import CoreBluetooth

func parseArg(_ name: String, default def: String) -> String {
    if let idx = CommandLine.arguments.firstIndex(of: name), idx + 1 < CommandLine.arguments.count {
        return CommandLine.arguments[idx + 1]
    }
    return def
}

final class Advertiser: NSObject, CBPeripheralManagerDelegate {
    private var peripheralManager: CBPeripheralManager!
    private let advData: [String: Any]

    init(localName: String, serviceUUID: String) {
        self.advData = [
            CBAdvertisementDataLocalNameKey: localName,
            CBAdvertisementDataServiceUUIDsKey: [CBUUID(string: serviceUUID)]
        ]
        super.init()
        self.peripheralManager = CBPeripheralManager(delegate: self, queue: nil)
        // 安装 stdin 监听：Rust 端关闭管道 -> 这里收到 EOF -> 优雅停止
        FileHandle.standardInput.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty {
                // EOF，优雅停止并退出
                self?.peripheralManager?.stopAdvertising()
                print("Advertising stopped (stdin closed).")
                FileHandle.standardInput.readabilityHandler = nil
                exit(0)
            }
            // 如果你未来需要通过stdin接收命令，可以在这里解析 data
        }
    }

    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        switch peripheral.state {
        case .poweredOn:
            peripheral.startAdvertising(self.advData)
            if let name = self.advData[CBAdvertisementDataLocalNameKey] as? String,
               let uuids = self.advData[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID] {
                print("Advertising started as '\(name)' with service \(uuids.map { $0.uuidString }.joined(separator: ","))")
            } else {
                print("Advertising started")
            }
        case .poweredOff:
            print("Bluetooth is powered off")
        default:
            print("Bluetooth state: \(peripheral.state.rawValue)")
        }
    }
}

let name = parseArg("--name", default: "Pasto")
let uuid = parseArg("--uuid", default: "12345678-1234-1234-1234-1234567890AB")

let _ = Advertiser(localName: name, serviceUUID: uuid)
RunLoop.main.run()