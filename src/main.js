const { invoke } = window.__TAURI__.tauri;

let greetMsgEl;
let greetButton;

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  // greetMsgEl.textContent = await invoke("login", {});
  invoke("login", {}).then(value => {
    greetMsgEl.textContent = value
  }).catch(err => {
    greetMsgEl.textContent = "Error: " + err
  })
}

window.addEventListener("DOMContentLoaded", () => {
  greetMsgEl = document.querySelector("#greet-msg");
  greetButton = document.querySelector("#greet-button")
  greetButton.addEventListener("click", () => greet());
});
