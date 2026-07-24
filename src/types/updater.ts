import type { UpdaterMessageKey } from '../locales'

export type TabKey = 'upgrade' | 'client' | 'addons' | 'settings' | 'about'
export type CountdownKind = 'update' | 'launch'

export interface UpdaterStatus {
	mode: 'manual' | 'bootstrap'
	phase: string
	message: string
	remainingSeconds?: number
}

export interface UpdaterContext {
	mode: 'manual' | 'bootstrap'
	gameDir: string
}

export interface ClientInspection {
	version: string | null
	needsSelection: boolean
}

export interface ClientVersionOption {
	version: string
	label: string
	isLatest: boolean
}

export interface DesktopIdentity {
	hydrolineId: string
	username: string
	displayName: string | null
	avatarUrl: string | null
}

export interface DownloadSource {
	key: string
	label: string
	priority: number
	requiresLogin: boolean
	available: boolean
}

export interface SelectOption {
	label: string
	value: string
	disabled?: boolean
}

export type Translator = (
	key: UpdaterMessageKey,
	params?: Record<string, string | number>,
) => string

export const SOURCE_KEY = 'hydcraft:updater:source'

export const THEME_MODES = [
	{ value: 'light', icon: 'i-lucide-sun', label: 'themeLight' },
	{ value: 'dark', icon: 'i-lucide-moon', label: 'themeDark' },
	{ value: 'system', icon: 'i-lucide-monitor', label: 'themeSystem' },
] as const satisfies ReadonlyArray<{
	value: 'light' | 'dark' | 'system'
	icon: string
	label: UpdaterMessageKey
}>
