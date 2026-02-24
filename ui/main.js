const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// DOM 元素
const statusDot = document.getElementById("status-dot");
const statusLabel = document.getElementById("status-label");
const modeServer = document.getElementById("mode-server");
const modeClient = document.getElementById("mode-client");
const bindField = document.getElementById("bind-field");
const serverField = document.getElementById("server-field");
const bindInput = document.getElementById("bind-input");
const serverInput = document.getElementById("server-input");
const startBtn = document.getElementById("start-btn");
const stopBtn = document.getElementById("stop-btn");
const devicesCard = document.getElementById("devices-card");
const deviceCount = document.getElementById("device-count");
const deviceList = document.getElementById("device-list");
const logOutput = document.getElementById("log-output");
const clearLogBtn = document.getElementById("clear-log-btn");

let currentMode = "server";
let running = false;

// 日志
function appendLog(msg) {
  const line = document.createElement("div");
  line.className = "log-line";
  const time = new Date().toLocaleTimeString("en-US", { hour12: false });
  line.innerHTML = `<span class="log-time">${time}</span>${msg}`;
  logOutput.appendChild(line);
  logOutput.scrollTop = logOutput.scrollHeight;
}

clearLogBtn.addEventListener("click", () => {
  logOutput.innerHTML = "";
});

// 模式切换
function setMode(mode) {
  currentMode = mode;
  modeServer.classList.toggle("active", mode === "server");
  modeClient.classList.toggle("active", mode === "client");
  bindField.style.display = mode === "server" ? "block" : "none";
  serverField.style.display = mode === "client" ? "block" : "none";
  devicesCard.style.display = mode === "server" && running ? "block" : "none";
}

modeServer.addEventListener("click", () => !running && setMode("server"));
modeClient.addEventListener("click", () => !running && setMode("client"));

// 状态更新
function updateStatus(status) {
  running = status.role !== "Idle";
  const connected = status.connected;

  statusDot.classList.toggle("connected", connected);
  statusLabel.textContent = status.role === "Idle"
    ? "Idle"
    : connected
      ? `${status.role} - Connected`
      : `${status.role} - Waiting...`;

  startBtn.disabled = running;
  stopBtn.disabled = !running;
  modeServer.style.pointerEvents = running ? "none" : "auto";
  modeClient.style.pointerEvents = running ? "none" : "auto";
  modeServer.style.opacity = running ? "0.5" : "1";
  modeClient.style.opacity = running ? "0.5" : "1";

  if (currentMode === "server" && running) {
    devicesCard.style.display = "block";
  } else {
    devicesCard.style.display = "none";
  }
}

// Start / Stop
startBtn.addEventListener("click", async () => {
  try {
    if (currentMode === "server") {
      const bind = bindInput.value || "0.0.0.0:24800";
      appendLog(`Starting server on ${bind}...`);
      await invoke("start_server", { bind });
    } else {
      const addr = serverInput.value;
      if (!addr) {
        appendLog("Please enter server address");
        return;
      }
      appendLog(`Connecting to ${addr}...`);
      await invoke("start_client", { serverAddr: addr });
    }
  } catch (err) {
    appendLog(`Error: ${err}`);
  }
});

stopBtn.addEventListener("click", async () => {
  try {
    await invoke("stop");
    appendLog("Stopped");
  } catch (err) {
    appendLog(`Error: ${err}`);
  }
});

// 设备管理
function addDevice(info) {
  const el = document.createElement("div");
  el.className = "device-item";
  el.dataset.id = info.device_id;
  el.innerHTML = `<span class="dot"></span><span class="name">${info.device_name}</span><span class="id">${info.device_id}</span>`;
  deviceList.appendChild(el);
  deviceCount.textContent = deviceList.children.length;
}

function removeDevice(deviceId) {
  const el = deviceList.querySelector(`[data-id="${deviceId}"]`);
  if (el) el.remove();
  deviceCount.textContent = deviceList.children.length;
}

// 监听后端事件
listen("synapse://status", (event) => {
  updateStatus(event.payload);
});

listen("synapse://log", (event) => {
  appendLog(event.payload);
});

listen("synapse://device-connected", (event) => {
  addDevice(event.payload);
  appendLog(`Device connected: ${event.payload.device_name}`);
});

listen("synapse://device-disconnected", (event) => {
  removeDevice(event.payload);
  appendLog(`Device disconnected: ${event.payload}`);
});

// 初始化
(async () => {
  try {
    const status = await invoke("get_status");
    updateStatus(status);
  } catch (_) {}
  appendLog("Synapse ready");
})();
