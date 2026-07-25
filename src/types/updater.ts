import type { UpdaterMessageKey } from '../locales'

export type TabKey = 'upgrade' | 'client' | 'addons' | 'settings' | 'about'
export type ClientDetailsKind = 'changelog' | 'mods'
export type UpdaterFailureKind = 'check' | 'update'

export interface UpdaterStatus {
	mode: 'manual' | 'bootstrap'
	phase: string
	message: string
	failureKind?: UpdaterFailureKind | null
	remainingSeconds?: number
	currentVersion?: string | null
	targetVersion?: string | null
	download?: DownloadProgress | null
}

export interface DownloadProgress {
	source: string
	downloadedBytes: number
	totalBytes: number
	bytesPerSecond: number
	latencyMs: number
	resumed: boolean
}

export interface UpdateConflict {
	operationId: string
	target: string
	reason: string
	candidates: string[]
}

export interface UpdaterContext {
	mode: 'manual' | 'bootstrap'
	gameDir: string
	consoleOrigin: string
}

export interface ClientInspection {
	version: string | null
	needsSelection: boolean
}

export interface ClientVersionOption {
	version: string
	label: string
	isLatest: boolean
	publishedAt: string | null
	changelog: string | null
	apiVersion: string | null
	modCount: number
	mods: ClientMod[]
}

export interface ClientMod {
	id: string
	name: string
	version: string
	description?: string
	api?: string
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
	latencyMs: number | null
}

export interface SelectOption {
	label: string
	value: string
	disabled?: boolean
	latencyMs?: number | null
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
