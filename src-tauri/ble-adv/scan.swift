import Foundation
import CoreBluetooth

// 这个类的唯一职责就是连接到蓝牙服务并发送停止命令
class BleStopper: NSObject, CBPeripheralManagerDelegate {
    private var peripheralManager: CBPeripheralManager!

    override init() {
        super.init()
        // 初始化外围设备管理器，这将触发与系统蓝牙服务的连接
        peripheralManager = CBPeripheralManager(delegate: self, queue: nil)
    }

    // 这是系统蓝牙状态更新时会调用的唯一方法
    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        print("Current Bluetooth state is: \(stateToString(peripheral.state))")

        if peripheral.state == .poweredOn {
            print("Bluetooth is On. Sending 'stop advertising' command...")
            
            // 核心命令：停止任何正在进行的广播
            peripheral.stopAdvertising()
            
            print("Command sent. The advertisement should now be stopped.")
            
            // 给予系统一点时间处理命令，然后干净地退出脚本
            DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                exit(0)
            }
        } else {
            print("Bluetooth is not powered on. No command sent.")
            exit(1)
        }
    }

    // 辅助函数，让状态输出更易读
    private func stateToString(_ state: CBManagerState) -> String {
        switch state {
            case .poweredOn: return "Powered On"
            case .poweredOff: return "Powered Off"
            case .resetting: return "Resetting"
            case .unauthorized: return "Unauthorized"
            case .unsupported: return "Unsupported"
            case .unknown: return "Unknown"
            @unknown default: return "Unknown"
        }
    }
}

print("Initializing BLE Stopper...")
let stopper = BleStopper()

// 启动运行循环，等待异步的蓝牙状态更新事件
RunLoop.main.run()