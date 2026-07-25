import './assets/styles/fonts/index.css'
import './assets/styles/base/tailwind.css'
import 'overlayscrollbars/styles/overlayscrollbars.css'
import './assets/styles/base/main.css'
import './icons'
import { createApp } from 'vue'
import { createRouter, createWebHashHistory } from 'vue-router'
import ui from '@nuxt/ui/vue-plugin'
import App from './App.vue'
import AuthWindow from './components/auth/AuthWindow.vue'
import ClientDetailsWindow from './components/client/ClientDetailsWindow.vue'
import VersionWindow from './components/version/VersionWindow.vue'

async function mount(): Promise<void> {
	let root = App
	if ('__TAURI_INTERNALS__' in window) {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window')
			const label = getCurrentWindow().label
			if (label === 'auth') root = AuthWindow
			if (label === 'version') root = VersionWindow
			if (label === 'client-details') root = ClientDetailsWindow
		} catch {
			// Browser previews and partially initialized webviews use the main window.
		}
	}

	const app = createApp(root)
	const router = createRouter({
		history: createWebHashHistory(),
		routes: [],
	})

	app.use(router)
	app.use(ui)
	app.mount('#app')
}

void mount()
