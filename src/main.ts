import './assets/styles/fonts/index.css'
import './assets/styles/base/tailwind.css'
import './assets/styles/base/main.css'
import { createApp } from 'vue'
import { createRouter, createWebHashHistory } from 'vue-router'
import ui from '@nuxt/ui/vue-plugin'
import App from './App.vue'

const app = createApp(App)
const router = createRouter({
  history: createWebHashHistory(),
  routes: [],
})

app.use(router)
app.use(ui)
app.mount('#app')
