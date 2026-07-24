import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import ui from '@nuxt/ui/vite'
import svgLoader from 'vite-svg-loader'

export default defineConfig({
	plugins: [vue(), ui(), svgLoader()],
	clearScreen: false,
	server: {
		host: '127.0.0.1',
	},
})
