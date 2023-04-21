export default {
  data() {
    return {
      button_message: "Login to minecraft",
      greet_message: "",
      greetDisabled: 0,
      hideDownloadButton: true,
    }
  },
  methods: {
    login (e) {
      e.preventDefault()
      if(!this.greetDisabled) {
        this.greetDisabled = true
        this.invoke("login", {}).then(value => {
          this.greet_message = value
          this.hideDownloadButton = false
        }).catch(err => {
          this.greet_message = "Error: " + err
          this.greetDisabled = false
        })
      }
    },
    download (e) {
      e.preventDefault()
      if(!this.hideDownloadButton) {
        this.invoke("download", {}).then(value => {
          this.greet_message = value
        }).catch(err => {
          this.greet_message = "Error: " + err
        })
      }
    },
  },
  props: {
    invoke: Object
  },
  template: `
  <h1>Welcome to Tauri!</h1>

      <div class="row">
        <div>
          <button id="greet-button" :disabled="greetDisabled == 1" type="button" v-on:click="login">{{ button_message }}</button>
          <button id="download-button" :class="{hide: hideDownloadButton }" v-on:click="download">Download game</button>
        </div>
      </div>

  <p id="greet-msg">{{ greet_message }}</p>
  `
}
