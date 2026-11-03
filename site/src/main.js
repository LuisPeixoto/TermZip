/* TermZip website. No framework, no tracking, nothing is sent anywhere. Everything here is an enhancement: the page
   reads and works without it (all install commands show, the letters are already in place). */
(function () {
  "use strict";

  var reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ---- the split-flap word ---- */
  var ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

  function setLetter(tile, ch) {
    var cells = tile.querySelectorAll(".half > i");
    for (var i = 0; i < cells.length; i++) cells[i].textContent = ch;
  }

  // Flips one tile from `from` to `to`: the old top half falls, the new bottom half rises, like the real thing.
  function flip(tile, from, to, ms) {
    return new Promise(function (resolve) {
      var top = tile.querySelector(".half.top");
      var bottom = tile.querySelector(".half.bottom");
      var topI = top.firstElementChild;
      var bottomI = bottom.firstElementChild;
      topI.textContent = to; // the half behind the falling flap already shows the new letter

      var falling = top.cloneNode(true);
      falling.classList.add("flap");
      falling.firstElementChild.textContent = from;
      var rising = bottom.cloneNode(true);
      rising.classList.add("flap");
      rising.firstElementChild.textContent = to;
      tile.appendChild(falling);
      tile.appendChild(rising);
      rising.style.transform = "rotateX(90deg)";

      var half = ms / 2;
      var fall = falling.animate([{ transform: "rotateX(0deg)" }, { transform: "rotateX(-90deg)" }], { duration: half, easing: "ease-in", fill: "forwards" });
      fall.onfinish = function () {
        falling.remove();
        var rise = rising.animate([{ transform: "rotateX(90deg)" }, { transform: "rotateX(0deg)" }], { duration: half, easing: "ease-out", fill: "forwards" });
        rise.onfinish = function () {
          bottomI.textContent = to;
          rising.remove();
          resolve();
        };
      };
    });
  }

  function runFlaps() {
    var board = document.querySelector(".flaps");
    if (!board || reduced || !board.animate) return;
    var tiles = board.querySelectorAll(".tile");
    var finalWord = board.getAttribute("data-word") || "";
    Array.prototype.forEach.call(tiles, function (tile, index) {
      var target = tile.getAttribute("data-char");
      var steps = 3 + (index % 3);
      var current = " ";
      setLetter(tile, current);
      var chain = Promise.resolve();
      chain = chain.then(function () {
        return new Promise(function (r) { setTimeout(r, 120 + index * 110); });
      });
      for (var s = 0; s < steps; s++) {
        (function (last) {
          chain = chain.then(function () {
            var next = last ? target : ALPHABET.charAt(Math.floor(Math.random() * ALPHABET.length));
            var from = current;
            current = next;
            return flip(tile, from, next, last ? 220 : 130);
          });
        })(s === steps - 1);
      }
    });
    void finalWord;
  }

  /* ---- install tabs ---- */
  var tabs = Array.prototype.slice.call(document.querySelectorAll('.tabs [role="tab"]'));
  var panels = Array.prototype.slice.call(document.querySelectorAll(".panel"));

  function select(os, focus) {
    tabs.forEach(function (tab) {
      var on = tab.getAttribute("data-os") === os;
      tab.setAttribute("aria-selected", on ? "true" : "false");
      tab.tabIndex = on ? 0 : -1;
      if (on && focus) tab.focus();
    });
    panels.forEach(function (panel) {
      panel.hidden = panel.getAttribute("data-os") !== os;
    });
  }

  function guessOs() {
    var p = ((navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || navigator.userAgent || "").toLowerCase();
    if (p.indexOf("win") !== -1) return "windows";
    if (p.indexOf("mac") !== -1 || p.indexOf("iphone") !== -1 || p.indexOf("ipad") !== -1) return "macos";
    if (p.indexOf("linux") !== -1 || p.indexOf("x11") !== -1 || p.indexOf("android") !== -1) return "linux";
    return "macos";
  }

  if (tabs.length) {
    select(guessOs(), false);
    tabs.forEach(function (tab, index) {
      tab.addEventListener("click", function () { select(tab.getAttribute("data-os"), false); });
      tab.addEventListener("keydown", function (event) {
        var to = null;
        if (event.key === "ArrowRight") to = tabs[(index + 1) % tabs.length];
        else if (event.key === "ArrowLeft") to = tabs[(index - 1 + tabs.length) % tabs.length];
        else if (event.key === "Home") to = tabs[0];
        else if (event.key === "End") to = tabs[tabs.length - 1];
        if (to) { event.preventDefault(); select(to.getAttribute("data-os"), true); }
      });
    });
  }

  /* ---- copy buttons ---- */
  var status = document.querySelector(".copied");
  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) return navigator.clipboard.writeText(text);
    return new Promise(function (resolve, reject) {
      var area = document.createElement("textarea");
      area.value = text;
      area.setAttribute("readonly", "");
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      var ok = false;
      try { ok = document.execCommand("copy"); } catch (e) { ok = false; }
      area.remove();
      ok ? resolve() : reject(new Error("copy failed"));
    });
  }

  Array.prototype.forEach.call(document.querySelectorAll(".copy"), function (button) {
    var label = button.textContent;
    button.addEventListener("click", function () {
      copyText(button.getAttribute("data-copy")).then(function () {
        button.textContent = button.getAttribute("data-done");
        if (status) status.textContent = button.getAttribute("data-done");
        setTimeout(function () { button.textContent = label; if (status) status.textContent = ""; }, 1800);
      }, function () {
        // The command is selectable in its box (one click selects it all), so nothing is lost.
        var code = button.previousElementSibling;
        if (code && window.getSelection) {
          var range = document.createRange();
          range.selectNodeContents(code);
          var selection = window.getSelection();
          selection.removeAllRanges();
          selection.addRange(range);
        }
      });
    });
  });

  /* ---- offer the page in the visitor's language (never redirects) ---- */
  function offerLanguage() {
    var data;
    try { data = JSON.parse(document.body.getAttribute("data-offers") || "null"); } catch (e) { data = null; }
    if (!data) return;
    var stored = null;
    try { stored = window.localStorage.getItem("termzip-lang-offer"); } catch (e) { stored = null; }
    if (stored) return;
    // Only the visitor's first-choice language counts; a page already in it (or English) offers nothing.
    var first = String((navigator.languages && navigator.languages.length ? navigator.languages[0] : navigator.language) || "en").toLowerCase();
    var target = first.indexOf("pt") === 0 ? "pt-br" : first.indexOf("es") === 0 ? "es" : first.indexOf("zh") === 0 ? "zh" : "";
    if (!target || !data.offers[target]) return;
    var bar = document.createElement("div");
    bar.className = "offer";
    bar.setAttribute("role", "region");
    var link = document.createElement("a");
    link.href = data.paths[target];
    link.textContent = data.offers[target];
    link.setAttribute("hreflang", target);
    var dismiss = document.createElement("button");
    dismiss.type = "button";
    dismiss.textContent = data.dismiss;
    function remember() { try { window.localStorage.setItem("termzip-lang-offer", "1"); } catch (e) { /* private mode */ } }
    dismiss.addEventListener("click", function () { remember(); bar.remove(); });
    link.addEventListener("click", remember);
    bar.setAttribute("aria-label", "Language");
    bar.appendChild(link);
    bar.appendChild(dismiss);
    document.body.appendChild(bar);
  }

  // Choosing a language by hand also means "do not suggest one again".
  Array.prototype.forEach.call(document.querySelectorAll(".langs a"), function (a) {
    a.addEventListener("click", function () { try { window.localStorage.setItem("termzip-lang-offer", "1"); } catch (e) { /* private mode */ } });
  });

  offerLanguage();
  runFlaps();
})();
