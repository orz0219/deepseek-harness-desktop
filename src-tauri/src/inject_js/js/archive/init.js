  // ===== 归档功能初始化 =====
  function initArchive() {
    ensureStyle('dsh-archive-style', ARCHIVE_CSS);

    function injectArchiveButtons() {
      // 收起（rail）模式下移除注入的按钮
      if (isRail()) {
        var ex = document.getElementById(BTN_ID); if (ex && ex.parentNode) ex.parentNode.removeChild(ex);
        var ex2 = document.getElementById(BTN_ID2); if (ex2 && ex2.parentNode) ex2.parentNode.removeChild(ex2);
        return;
      }
      var anchor = findAnchor();
      var arch = document.getElementById(BTN_ID);
      if (!arch) {
        if (!anchor || !anchor.parentNode) return;
        arch = document.createElement('button');
        arch.id = BTN_ID;
        arch.type = 'button';
        setIdle(arch);
        attachTooltip(arch, 'bottom');
        arch.style.cssText = 'flex:none;display:inline-flex;align-items:center;justify-content:center;box-sizing:border-box;width:28px;height:28px;margin:0 2px 0 0;padding:0;border:none;border-radius:50%;cursor:pointer;';
        arch.addEventListener('click', function () { onArchiveAll(arch); });
        var ref = anchor.closest('[class*="searchSlot"]') || anchor.parentNode;
        ref.parentNode.insertBefore(arch, ref);
      }
      // 时钟按钮：「归档按钮右侧」。
      var archv = document.getElementById(BTN_ID2);
      if (!archv && arch && arch.parentNode) {
        archv = addButtonAfter(arch, BTN_ID2, '归档历史', function () { openArchivedPanel(); });
        setIdle2(archv);
        attachTooltip(archv, 'bottom');
      }
    }

    injectArchiveButtons();

    // MutationObserver 自愈
    if (!window.__dshArchiveObserver && document.body) {
      var scheduled = false;
      window.__dshArchiveObserver = new MutationObserver(function () {
        if (scheduled) return; scheduled = true;
        setTimeout(function () { scheduled = false; injectArchiveButtons(); }, 300);
      });
      window.__dshArchiveObserver.observe(document.body, { childList: true, attributes: true, subtree: true });
    }
  }

