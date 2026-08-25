  // ===== 归档清单弹窗（带锁定/排除）=====
  // rows: [{ sessionId, title, cwd }]，默认全部归档；点按行或锁标记可「锁定」该对话，
  // 锁定项将从本次归档中排除。锁定状态仅存在于弹窗内存，不持久化。
  function showArchiveChecklist(rows, onOk) {
    var prev = document.getElementById('dsh-archive-modal');
    if (prev) prev.remove();
    var locked = {};
    rows.forEach(function (r) { locked[r.sessionId] = false; });

    function amEscape(s) {
      return String(s == null ? '' : s)
        .replace(/&/g, '&amp;').replace(/</g, '&lt;')
        .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    var listHtml = rows.map(function (r, i) {
      return '' +
        '<div class="dsh-am-row" data-i="' + i + '">' +
          '<button type="button" class="dsh-am-lock" data-i="' + i + '" ' +
            'aria-label="锁定，从本次归档排除" title="锁定：本次归档时跳过此对话">' + ICON_LOCK + '</button>' +
          '<div class="dsh-am-rowbody">' +
            '<div class="dsh-am-rowtitle">' + amEscape(r.title || r.sessionId) + '</div>' +
            (r.cwd ? '<div class="dsh-am-rowmeta">' + amEscape(r.cwd) + '</div>' : '') +
          '</div>' +
          '<span class="dsh-am-skip" hidden>将跳过</span>' +
        '</div>';
    }).join('');

    var overlay = document.createElement('div');
    overlay.id = 'dsh-archive-modal';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.innerHTML =
      '<div class="dsh-am-card list">' +
        '<div class="dsh-am-title">归档对话</div>' +
        '<div class="dsh-am-msg">点按左侧锁标记可将对话从本次归档中排除（不持久化）。默认全部归档。</div>' +
        '<div class="dsh-am-toolbar">' +
          '<button type="button" class="dsh-am-mini" data-act="lockall">全部锁定</button>' +
          '<button type="button" class="dsh-am-mini" data-act="unlockall">全部解锁</button>' +
          '<span class="dsh-am-minisum" style="margin-left:auto"></span>' +
        '</div>' +
        '<div class="dsh-am-list">' + listHtml + '</div>' +
        '<div class="dsh-am-summary"></div>' +
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

    function refresh() {
      var lockCount = 0;
      rows.forEach(function (r) { if (locked[r.sessionId]) lockCount++; });
      var archiveCount = rows.length - lockCount;
      overlay.querySelectorAll('.dsh-am-row').forEach(function (rowEl) {
        var i = +rowEl.getAttribute('data-i');
        var isLocked = locked[rows[i].sessionId];
        rowEl.classList.toggle('locked', isLocked);
        var lk = rowEl.querySelector('.dsh-am-lock');
        if (lk) lk.classList.toggle('locked', isLocked);
        var badge = rowEl.querySelector('.dsh-am-skip');
        if (badge) badge.hidden = !isLocked;
      });
      var sum = overlay.querySelector('.dsh-am-summary');
      if (sum) sum.innerHTML = '将归档 <b>' + archiveCount + '</b> 个，锁定跳过 <b>' + lockCount + '</b> 个';
      var mini = overlay.querySelector('.dsh-am-minisum');
      if (mini) mini.textContent = '已选 ' + archiveCount + ' / ' + rows.length;
    }

    overlay.querySelectorAll('.dsh-am-lock').forEach(function (btn) {
      btn.addEventListener('click', function (e) {
        e.stopPropagation();
        var i = +btn.getAttribute('data-i');
        var id = rows[i].sessionId;
        locked[id] = !locked[id];
        refresh();
      });
    });
    overlay.querySelectorAll('.dsh-am-row').forEach(function (rowEl) {
      rowEl.addEventListener('click', function () {
        var lk = rowEl.querySelector('.dsh-am-lock');
        if (lk) lk.click();
      });
    });
    overlay.querySelectorAll('.dsh-am-mini').forEach(function (b) {
      b.addEventListener('click', function () {
        var act = b.getAttribute('data-act');
        rows.forEach(function (r) { locked[r.sessionId] = (act === 'lockall'); });
        refresh();
      });
    });
    overlay.querySelector('.dsh-am-cancel').addEventListener('click', close);
    overlay.querySelector('.dsh-am-ok').addEventListener('click', function () {
      var selected = rows.filter(function (r) { return !locked[r.sessionId]; });
      close();
      onOk(selected);
    });

    refresh();
  }


