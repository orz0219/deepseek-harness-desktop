  // ===== 归档覆盖缓存 =====
  var OVERRIDE_KEY = 'dsh-desktop-archive-overrides';

  function loadOverrides() {
    try { return new Set(JSON.parse(localStorage.getItem(OVERRIDE_KEY) || '[]')); }
    catch (e) { return new Set(); }
  }

  function saveOverrides(set) {
    try { localStorage.setItem(OVERRIDE_KEY, JSON.stringify(Array.from(set))); } catch (e) {}
  }

  function applyOverrides(ids) {
    if (!ids || !ids.length) return;
    var o = loadOverrides();
    ids.forEach(function (id) { o.add(id); });
    saveOverrides(o);
  }

  function removeOverride(id) {
    var o = loadOverrides();
    if (o.delete(id)) saveOverrides(o);
  }


