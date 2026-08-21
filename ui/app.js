// 启动器内置页面逻辑（加载 / 引导 / 错误）。由 Rust 经 window.eval
// 调用 window.__setLauncherState 推送状态；页面自身也轮询 get_status。

// 加载态下的默认提示文案
var LOADING_TEXT = {
  starting: "正在启动本机 dsh web",
  restarting: "正在重启 dsh",
  ready: "即将打开",
};

// 错误 / 未找到 dsh 态下显示「重新启动」按钮
var CAN_RESTART = { "error": true, "missing-dsh": true, "restarting": true };

function render(s) {
  if (!s) return;
  var state = s.state || "starting";
  var loading = !(state === "error" || state === "missing-dsh");

  document.getElementById("loader").classList.toggle("hidden", !loading);
  document.getElementById("details").classList.toggle("hidden", loading);

  if (loading) {
    var status = document.getElementById("statusText");
    status.textContent = "";
    // 保留动态省略号节点
    if (s.message) {
      status.textContent = s.message;
    } else {
      status.appendChild(document.createTextNode(LOADING_TEXT[state] || LOADING_TEXT.starting));
      status.appendChild((function () {
        var d = document.createElement("span");
        d.className = "dots";
        return d;
      })());
    }
    return;
  }

  // 详情态（错误 / 未找到 dsh）
  var badge = document.getElementById("stateBadge");
  var msg = document.getElementById("message");
  var choose = document.getElementById("choose");
  var cands = document.getElementById("cands");
  var restartBtn = document.getElementById("restartBtn");
  var formError = document.getElementById("formError");
  badge.className = "state " + state;
  badge.textContent = state;

  if (s.message) {
    msg.textContent = s.message;
    msg.classList.remove("hidden");
  } else {
    msg.classList.add("hidden");
  }

  if (state === "missing-dsh") {
    choose.classList.remove("hidden");
    formError.textContent = "";
    if (s.candidates && s.candidates.length) {
      cands.innerHTML =
        "已发现候选：" +
        s.candidates
          .map(function (c) {
            return (
              "<div><code>" +
              escapeHtml(c.executable) +
              "</code> · " +
              escapeHtml(c.version) +
              " · " +
              escapeHtml(c.source) +
              "</div>"
            );
          })
          .join("");
    } else {
      cands.textContent = "在 PATH 与常见目录中均未发现 dsh。";
    }
  } else {
    choose.classList.add("hidden");
  }

  // 重新启动按钮：仅在错误 / 未找到 / 重启中可见
  if (CAN_RESTART[state]) {
    restartBtn.classList.remove("hidden");
  } else {
    restartBtn.classList.add("hidden");
  }
}

function escapeHtml(t) {
  return String(t).replace(/[&<>"]/g, function (c) {
    return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
  });
}

async function getStatus() {
  try {
    var hasInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    if (!hasInvoke) return;
    var s = await window.__TAURI__.core.invoke("get_status");
    render(s);
  } catch (e) {
    // Global API 可能尚未就绪；状态稍后由 Rust 主动推送。
  }
}

async function save() {
  var pathInput = document.getElementById("dshPath");
  var portInput = document.getElementById("port");
  var formError = document.getElementById("formError");
  var btn = document.getElementById("saveBtn");
  var path = pathInput.value.trim();
  var portRaw = portInput.value.trim();
  var port = portRaw ? parseInt(portRaw, 10) : null;

  if (!path) {
    formError.textContent = "请填写 dsh 可执行文件路径。";
    return;
  }
  if (portRaw && (isNaN(port) || port < 1024 || port > 65535)) {
    formError.textContent = "端口需为 1024–65535 之间的整数。";
    return;
  }
  formError.textContent = "";
  btn.disabled = true;
  try {
    // select_dsh 的 path / port 均为可选：仅传入填写了的值。
    var args = {};
    if (path) args.path = path;
    if (port !== null) args.port = port;
    await window.__TAURI__.core.invoke("select_dsh", args);
  } catch (e) {
    // alert() 在 WKWebView 下不显示，改用行内提示。
    formError.textContent = "保存失败：" + (e && e.message ? e.message : e);
    btn.disabled = false;
  }
}

async function restart() {
  var formError = document.getElementById("formError");
  try {
    await window.__TAURI__.core.invoke("restart_dsh");
  } catch (e) {
    formError.textContent = "重启失败：" + (e && e.message ? e.message : e);
  }
}

document.getElementById("saveBtn").addEventListener("click", save);
document.getElementById("port").addEventListener("keydown", function (e) {
  if (e.key === "Enter") save();
});
document.getElementById("restartBtn").addEventListener("click", restart);

// 初始拉取 + 轮询直到全局 API 可用。
(function poll() {
  getStatus();
  if (!(window.__TAURI__ && window.__TAURI__.core)) {
    setTimeout(poll, 300);
  }
})();
