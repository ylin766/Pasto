import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface BleDevice {
  name?: string;
  address: string;
  rssi?: number;
}

function App() {
  const [clipboardText, setClipboardText] = useState("");
  const [bleDevices, setBleDevices] = useState<BleDevice[]>([]);
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState("");
  const [popupMsg, setPopupMsg] = useState<string | null>(null);
  const [connectedDevice, setConnectedDevice] = useState<string | null>(null);
  const [isAdvertising, setIsAdvertising] = useState(false);

  const showPopup = (msg: string) => setPopupMsg(msg);

  // 监听剪贴板变化
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const text = await invoke<string>("get_clipboard_text");
        if (text !== clipboardText && connectedDevice) {
          setClipboardText(text);
          // 发送剪贴板更新到连接的设备
          await invoke("send_clipboard_content", { content: text });
        }
      } catch (error) {
        console.error("Failed to get clipboard text:", error);
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [clipboardText, connectedDevice]);

  async function getClipboardText() {
    try {
      const text = await invoke("get_clipboard_text");
      setClipboardText(text as string);
    } catch (error) {
      console.error("Failed to get clipboard text:", error);
      setClipboardText(`Error: ${error}`);
    }
  }

  async function scanBleDevices() {
    try {
      setScanning(true);
      setScanError("");
      const devices = await invoke("scan_ble_devices") as BleDevice[];
      setBleDevices(devices);
      if (devices.length === 0) {
        showPopup("未发现支持剪贴板同步的设备");
      }
    } catch (error) {
      console.error("Failed to scan BLE devices:", error);
      setScanError(`${error}`);
      setBleDevices([]);
    } finally {
      setScanning(false);
    }
  }

  const startAdvertising = async () => {
    try {
      await invoke("start_ble_advertising", {
        name: "Pasto",
        serviceUuid: "12345678-1234-1234-1234-1234567890AB",
      });
      setIsAdvertising(true);
      showPopup("已开始广播剪贴板服务");
    } catch (e: any) {
      showPopup(`启动广播失败: ${e}`);
    }
  };

  const stopAdvertising = async () => {
    try {
      await invoke("stop_ble_advertising");
      setIsAdvertising(false);
      showPopup("已停止广播剪贴板服务");
    } catch (e: any) {
      showPopup(`停止广播失败: ${e}`);
    }
  };

  const connectToDevice = async (address: string, name?: string) => {
    try {
      await invoke("connect_to_device", { address });
      setConnectedDevice(address);
      showPopup(`已连接到设备: ${name || address}`);
    } catch (e: any) {
      showPopup(`连接失败: ${e}`);
    }
  };

  return (
    <main className="container">
      <h1>Pasto - 剪贴板同步工具</h1>

      <div className="row" style={{ marginTop: "2rem" }}>
        <button onClick={getClipboardText}>读取剪贴板</button>
        <button 
          onClick={scanBleDevices} 
          disabled={scanning}
          style={{ marginLeft: "1rem" }}
        >
          {scanning ? "扫描中..." : "扫描设备"}
        </button>
      </div>

      <div style={{ marginTop: 16 }}>
        <button 
          onClick={isAdvertising ? stopAdvertising : startAdvertising}
          style={{ 
            backgroundColor: isAdvertising ? "#dc3545" : "#28a745",
            color: "white",
            border: "none",
            padding: "8px 16px",
            borderRadius: "4px"
          }}
        >
          {isAdvertising ? "停止广播" : "开始广播"}
        </button>
        {connectedDevice && (
          <span style={{ marginLeft: 16, color: "green" }}>
            已连接: {connectedDevice}
          </span>
        )}
      </div>

      {clipboardText && (
        <div className="clipboard-content" style={{ marginTop: "1rem" }}>
          <h3>当前剪贴板内容:</h3>
          <pre style={{ background: "#f5f5f5", padding: "10px", borderRadius: "4px" }}>
            {clipboardText}
          </pre>
        </div>
      )}

      {scanError && (
        <div className="error-message" style={{ marginTop: "1rem", color: "red" }}>
          <p>扫描错误: {scanError}</p>
        </div>
      )}

      {bleDevices.length > 0 && (
        <div className="ble-devices" style={{ marginTop: "1rem" }}>
          <h3>发现的剪贴板设备:</h3>
          <ul style={{ listStyle: "none", padding: 0 }}>
            {bleDevices.map((device, index) => (
              <li key={index} style={{ 
                margin: "8px 0", 
                padding: "10px", 
                border: "1px solid #ddd", 
                borderRadius: "4px",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center"
              }}>
                <div>
                  <strong>{device.name || "未知设备"}</strong> ({device.address})
                  {device.rssi !== undefined && <span> RSSI: {device.rssi}dBm</span>}
                </div>
                <button 
                  onClick={() => connectToDevice(device.address, device.name)}
                  disabled={connectedDevice === device.address}
                  style={{
                    backgroundColor: connectedDevice === device.address ? "#6c757d" : "#007bff",
                    color: "white",
                    border: "none",
                    padding: "4px 12px",
                    borderRadius: "4px",
                    cursor: connectedDevice === device.address ? "not-allowed" : "pointer"
                  }}
                >
                  {connectedDevice === device.address ? "已连接" : "连接"}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      {popupMsg && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.35)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 9999,
          }}
        >
          <div
            style={{
              background: "#fff",
              padding: 16,
              borderRadius: 8,
              maxWidth: 360,
              width: "calc(100% - 48px)",
              whiteSpace: "pre-wrap",
              lineHeight: 1.4,
              boxShadow: "0 6px 24px rgba(0,0,0,0.2)",
            }}
          >
            <div style={{ marginBottom: 12 }}>{popupMsg}</div>
            <div style={{ textAlign: "right" }}>
              <button onClick={() => setPopupMsg(null)}>关闭</button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
