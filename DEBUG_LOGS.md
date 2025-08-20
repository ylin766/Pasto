# BLE 连接调试日志说明

## 概述

为了帮助调试 BLE 连接问题，特别是 "Clipboard service not found" 错误，我们在 BLE 相关功能中添加了详细的调试日志。

## 日志类型

### 1. 扫描设备日志 (`[BLE_SCAN_DEBUG]` / `[BLE_SCAN_ERROR]`)
- 显示扫描过程的详细信息
- 列出发现的所有设备及其属性
- 显示哪些设备包含剪贴板服务

### 2. 连接设备日志 (`[BLE_DEBUG]` / `[BLE_ERROR]`)
- 显示连接过程的每个步骤
- 列出设备的所有服务和特征
- 详细说明连接失败的原因

### 3. 发送数据日志 (`[BLE_SEND_DEBUG]` / `[BLE_SEND_ERROR]`)
- 显示剪贴板数据发送过程
- 包含数据大小和序列化信息

## 如何查看日志

### 在开发模式下运行应用
```bash
cd src-tauri
cargo tauri dev
```

### 在终端中查看日志
日志会直接输出到终端，你可以看到类似以下的输出：

```
[BLE_SCAN_DEBUG] Starting BLE device scan...
[BLE_SCAN_DEBUG] BLE manager initialized
[BLE_SCAN_DEBUG] Found 1 BLE adapters
[BLE_SCAN_DEBUG] Using first available adapter
[BLE_SCAN_DEBUG] Starting new scan...
[BLE_SCAN_DEBUG] Scanning for 3 seconds...
[BLE_SCAN_DEBUG] Found 5 total peripherals
[BLE_SCAN_DEBUG] Looking for devices with clipboard service UUID: 12345678-1234-1234-1234-1234567890ab
[BLE_SCAN_DEBUG] Checking peripheral 1: AA:BB:CC:DD:EE:FF
[BLE_SCAN_DEBUG] Device AA:BB:CC:DD:EE:FF: name=Some("iPhone"), rssi=Some(-45), services=[...]
[BLE_SCAN_DEBUG] ✗ Device AA:BB:CC:DD:EE:FF does not have clipboard service
...
```

## 常见问题诊断

### 1. "Clipboard service not found" 错误

当看到此错误时，检查日志中的以下信息：

#### 在扫描阶段：
- 确认目标设备是否被发现
- 检查设备是否广播了正确的服务 UUID
- 验证服务 UUID 是否匹配：`12345678-1234-1234-1234-1234567890AB`

#### 在连接阶段：
- 查看 `[BLE_DEBUG] Discovered X services` 消息
- 检查 `Service X: UUID = ...` 列表中是否包含剪贴板服务
- 如果服务不存在，可能是：
  - 另一台设备没有正确启动广播
  - UUID 不匹配
  - 设备连接后服务发现失败

### 2. 连接失败

检查以下日志信息：
- `[BLE_ERROR] Failed to connect to device: ...` - 物理连接失败
- `[BLE_ERROR] Failed to discover services: ...` - 服务发现失败
- `[BLE_ERROR] Device with address X not found` - 设备未找到

### 3. 数据发送失败

查看发送日志：
- `[BLE_SEND_ERROR] No device connected` - 没有连接的设备
- `[BLE_SEND_ERROR] Failed to write clipboard data: ...` - 写入失败

## 调试步骤

1. **确保两台设备都在运行应用**
   - 一台设备启动广播（点击"开始广播"按钮）
   - 另一台设备进行扫描（点击"扫描设备"按钮）

2. **检查扫描日志**
   - 确认能发现目标设备
   - 验证设备包含剪贴板服务

3. **检查连接日志**
   - 查看连接过程是否成功
   - 确认服务和特征发现正常

4. **测试数据传输**
   - 修改剪贴板内容
   - 查看发送日志确认数据传输

## 注意事项

- 确保两台设备的蓝牙都已开启
- 确保应用有蓝牙权限
- 在 macOS 上，可能需要在系统偏好设置中授予蓝牙权限
- 如果连接仍然失败，尝试重启蓝牙或重新启动应用

## 技术细节

- **剪贴板服务 UUID**: `12345678-1234-1234-1234-1234567890AB`
- **剪贴板特征 UUID**: `87654321-4321-4321-4321-BA0987654321`
- **数据格式**: JSON 格式，包含内容和时间戳

通过这些详细的日志，你应该能够准确定位连接失败的原因并进行相应的修复。