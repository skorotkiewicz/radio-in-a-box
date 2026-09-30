"use strict";

const audio = document.querySelector("#audio");
const player = document.querySelector("#player");
const button = document.querySelector("#play-button");
const message = document.querySelector("#player-message");

function syncPlayback() {
  const playing = !audio.paused;
  player.classList.toggle("playing", playing);
  document.querySelector("#play-icon").textContent = playing ? "Ⅱ" : "▶";
  document.querySelector("#play-label").textContent = playing ? "Pause radio" : "Listen live";
}

button.addEventListener("click", async () => {
  if (!audio.paused) {
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
    return;
  }
  // Reconnect at the live edge instead of replaying a paused buffer.
  audio.src = "/radio.mp3";
  message.textContent = "Finding the signal…";
  try {
    await audio.play();
  } catch {
    message.textContent = "Couldn't connect. The station may be offline. Please try again.";
    syncPlayback();
  }
});

audio.addEventListener("play", syncPlayback);
audio.addEventListener("pause", syncPlayback);
audio.addEventListener("playing", () => { message.textContent = "You're tuned in. Enjoy the ride."; });
audio.addEventListener("waiting", () => { message.textContent = "Buffering the live stream…"; });
audio.addEventListener("error", () => {
  message.textContent = "The signal is unavailable. Press Listen live to reconnect.";
  player.classList.remove("playing");
  document.querySelector("#play-icon").textContent = "▶";
  document.querySelector("#play-label").textContent = "Listen live";
});

async function refreshStation() {
  const status = document.querySelector("#broadcast-status");
  const title = document.querySelector("#track-title");
  try {
    const response = await fetch("/status-json.xsl", { cache: "no-store", signal: AbortSignal.timeout(5000) });
    if (!response.ok) throw new Error("Status unavailable");
    const data = await response.json();
    const sources = data.icestats?.source;
    const source = (Array.isArray(sources) ? sources : [sources]).find(
      (item) => item && typeof item.listenurl === "string" && new URL(item.listenurl).pathname === "/radio.mp3"
    );
    if (!source) throw new Error("Stream offline");
    status.textContent = "On air";
    status.classList.add("online");
    const name = source.server_name || "My Radio";
    document.querySelector("#station-name").textContent = name;
    document.title = `${name} · Independent radio`;
    title.textContent = source.title || "Between records";
    document.querySelector("#track-detail").textContent = source.artist || "Live from our independent station.";
    const listeners = Number(source.listeners);
    document.querySelector("#listeners").textContent = `Live listeners: ${Number.isFinite(listeners) ? listeners : "—"}`;
  } catch {
    status.textContent = "Off air";
    status.classList.remove("online");
    title.textContent = "The signal will be back";
    document.querySelector("#track-detail").textContent = "Try tuning in again in a little while.";
    document.querySelector("#listeners").textContent = "Live listeners: —";
  } finally {
    setTimeout(refreshStation, 15000);
  }
}

refreshStation();
