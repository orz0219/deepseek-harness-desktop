  // ===== 归档确认弹框 =====
  function showConfirm(count, onOk) {
    var prev = document.getElementById('dsh-archive-modal');
    if (prev) prev.remove();
    var overlay = document.createElement('div');
    overlay.id = 'dsh-archive-modal';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.innerHTML =
      '<div class="dsh-am-card">' +
        '<div class="dsh-am-title">归档所有对话</div>' +
        '<div class="dsh-am-msg">确定要归档全部 ' + count + ' 个对话吗？归档后可在归档区恢复，不会永久删除。</div>' +
        '<div class="dsh-am-actions">' +
          '<button type="button" class="dsh-am-cancel">取消</button>' +
          '<button type="button" class="dsh-am-ok">确认归档</button>' +
        '</div>' +
      '</div>';
    document.body.appendChild(overlay);
    function close() { overlay.remove(); document.removeEventListener('keydown', onKey); }
    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);
    overlay.addEventListener('mousedown', function (e) { if (e.target === overlay) close(); });
    overlay.querySelector('.dsh-am-cancel').addEventListener('click', close);
    overlay.querySelector('.dsh-am-ok').addEventListener('click', function () { close(); onOk(); });
  }


