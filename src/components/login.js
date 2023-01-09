export default {
  data() {
    return {
      button_message: "Login to minecraft",
      greet_message: ""
    }
  },
  methods: {
    login (e) {
      e.preventDefault()
      this.invoke("login", {}).then(value => {
        this.greet_message = value
      }).catch(err => {
        this.greet_message = "Error: " + err
      })
    }
  },
  props: {
    invoke: Object
  },
  template: `
  <h1>Welcome to Tauri!</h1>

      <div class="row">
        <div>
          <button id="greet-button" type="button" v-on:click="login">{{ button_message }}</button>
        </div>
      </div>

  <p id="greet-msg">{{ greet_message }}</p>
  `
}
