const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;

const { createApp } = Vue

import loginpage from './components/login.js'

let app = createApp({
  data() {
    return {
      invoke: invoke,
      listen: listen,
    }
  },
  mounted() {

  },
  components: {
    loginpage
  }
});

app.mount('#container')
