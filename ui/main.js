const { invoke } = window.__TAURI__.core;

const statusIndicator = document.getElementById("status-indicator");
const statusText = document.getElementById("status-text");
const roleSelect = document.getElementById("role-select");
const hostField = document.getElementById("host-field");
const hostInput = document.getElementById("host-input");
const startBtn = document.getElementById("start-btn");
const stopBtn = document.getElementById("stop-btn");
const logOutput = document.getElementById("log-output");

function appendLog(msg) {
  const line = document.createElement("div");
  line.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
  logOutput.appendChild(line);
  logOutput.scrollTop = logOutput.scrollHeight;
}

roleSelect.addEventListener("change", () => {
  hostField.style.display = roleSelect.value === "client" ? "block" : "none";
});

startBtn.addEventListener("click", async () => {
  const role = roleSelect.value;
  appendLog(`Starting as ${role}...`);

  try {
    const result = await invoke("greet", { name: role });
    appendLog(result);
  } catch (err) {
    appendLog(`Error: ${err}`);
  }
});

stopBtn.addEventListener("click", () => {
  appendLog("Stopped.");
});

appendLog("Synapse GUI ready.");
