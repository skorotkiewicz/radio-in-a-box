const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

// One DOM-free check for metadata, offline state, and reconnect controls.
test("listener page handles live metadata and playback failures", async () => {
  const nodes = new Map();
  const document = {
    querySelector(selector) {
      if (!nodes.has(selector)) {
        const classes = new Set();
        nodes.set(selector, {
          textContent: "", handlers: {},
          addEventListener(name, handler) { this.handlers[name] = handler; },
          classList: {
            add(name) { classes.add(name); },
            remove(name) { classes.delete(name); },
            toggle(name, enabled) { enabled ? classes.add(name) : classes.delete(name); },
            contains(name) { return classes.has(name); },
          },
        });
      }
      return nodes.get(selector);
    },
  };
  const audio = document.querySelector("#audio");
  audio.paused = true;
  audio.error = null;
  audio.play = async () => {
    audio.error = null;
    audio.paused = false;
    audio.handlers.play();
    audio.handlers.playing();
  };
  audio.pause = () => { audio.paused = true; audio.handlers.pause(); };
  audio.load = () => {};
  let source = { listenurl: "http://icecast:8000/radio.mp3", server_name: "Test Station", title: "<b>Song</b>", artist: "Artist", listeners: 3 };
  let poll;
  let pollDelay;
  const context = vm.createContext({
    document, URL, AbortSignal,
    fetch: async () => ({ ok: true, json: async () => ({ icestats: { source } }) }),
    setTimeout: (handler, delay) => { poll = handler; pollDelay = delay; },
  });
  vm.runInContext(fs.readFileSync(`${__dirname}/../web/app.js`, "utf8"), context);
  await new Promise(setImmediate);
  const text = (selector) => document.querySelector(selector).textContent;
  assert.equal(text("#station-name"), "Test Station");
  assert.equal(text("#track-title"), "<b>Song</b>");
  assert.equal(text("#listeners"), "Live listeners: 3");
  assert.equal(text("#broadcast-status"), "On air");
  assert.equal(pollDelay, 15000);

  source = { ...source, title: "Test Station", artist: "" };
  await poll();
  assert.equal(text("#track-title"), "Test Station");
  assert.equal(text("#track-detail"), "Live from our independent station.");
  source = { ...source, title: "Back to music", artist: "Artist" };
  await poll();
  assert.equal(text("#track-title"), "Back to music");
  assert.equal(text("#track-detail"), "Artist");

  const click = document.querySelector("#play-button").handlers.click;
  await click();
  assert.equal(audio.src, "/radio.mp3");
  assert.equal(text("#play-label"), "Pause radio");
  await click();
  assert.equal(audio.paused, true);
  assert.equal(text("#play-label"), "Listen live");

  audio.error = new Error("Connection lost");
  audio.paused = false;
  audio.handlers.error();
  assert.equal(text("#play-label"), "Listen live");
  await click();
  assert.equal(text("#play-label"), "Pause radio");

  source = [source];
  await poll();
  assert.equal(text("#broadcast-status"), "On air");
  source = undefined;
  await poll();
  assert.equal(text("#broadcast-status"), "Off air");
  assert.equal(text("#listeners"), "Live listeners: —");
});
