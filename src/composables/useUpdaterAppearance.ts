import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { updaterMessages, type UpdaterMessageKey } from '../locales'

export type LocaleCode = 'zh-CN' | 'zh-TW' | 'ja-JP' | 'en-US'
export type ThemeMode = 'light' | 'dark' | 'system'

const LOCALE_KEY = 'hydcraft:updater:locale'
const THEME_KEY = 'hydcraft:updater:theme'
const locale = ref<LocaleCode>(
	(localStorage.getItem(LOCALE_KEY) as LocaleCode) || 'zh-CN',
)
const themeMode = ref<ThemeMode>(
	(localStorage.getItem(THEME_KEY) as ThemeMode) || 'system',
)
const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

export function useUpdaterAppearance() {
	const applyTheme = (): void => {
		document.documentElement.classList.toggle(
			'dark',
			themeMode.value === 'dark' ||
				(themeMode.value === 'system' && mediaQuery.matches),
		)
	}

	const selectLocale = (value: LocaleCode): void => {
		locale.value = value
		localStorage.setItem(LOCALE_KEY, value)
		document.documentElement.lang = value
	}

	const selectTheme = (value: ThemeMode): void => {
		themeMode.value = value
		localStorage.setItem(THEME_KEY, value)
		applyTheme()
	}
	const syncAppearance = (event: StorageEvent): void => {
		if (
			event.key === LOCALE_KEY &&
			event.newValue &&
			event.newValue in updaterMessages
		) {
			locale.value = event.newValue as LocaleCode
			document.documentElement.lang = locale.value
		}
		if (
			event.key === THEME_KEY &&
			['light', 'dark', 'system'].includes(event.newValue || '')
		) {
			themeMode.value = event.newValue as ThemeMode
			applyTheme()
		}
	}
	const messages = computed(() => updaterMessages[locale.value])
	const t = (
		key: UpdaterMessageKey,
		params?: Record<string, string | number>,
	): string => {
		const message = messages.value[key]
		if (!params) return message
		return Object.entries(params).reduce(
			(value, [name, replacement]) =>
				value.replaceAll(`{${name}}`, String(replacement)),
			message,
		)
	}

	onMounted(() => {
		applyTheme()
		document.documentElement.lang = locale.value
		mediaQuery.addEventListener('change', applyTheme)
		window.addEventListener('storage', syncAppearance)
	})
	onBeforeUnmount(() => {
		mediaQuery.removeEventListener('change', applyTheme)
		window.removeEventListener('storage', syncAppearance)
	})

	return { locale, themeMode, selectLocale, selectTheme, messages, t }
}
