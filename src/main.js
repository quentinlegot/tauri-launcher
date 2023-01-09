const { invoke } = window.__TAURI__.tauri;

const { createApp } = Vue

import loginpage from './components/login.js'

createApp({
  data() {
    return {
      invoke: invoke
    }
  },
  mounted() {

  },
  components: {
    loginpage
  }
}).mount('#container')
