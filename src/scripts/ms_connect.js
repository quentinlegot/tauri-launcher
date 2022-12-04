const { invoke } = window.__TAURI__.tauri;

async function open_ms_window() {
    let response = await invoke("greet", { name: greetInputEl.value });
}