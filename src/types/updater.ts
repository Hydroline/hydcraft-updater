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
	testRevision?: number | null
	download?: DownloadProgress | null
	operation?: OperationProgress | null
}

export interface DownloadProgress {
	source: string
	sourceUrl?: string | null
	downloadedBytes: number
	totalBytes: number
	bytesPerSecond: number
	latencyMs: number
	resumed: boolean
}

export interface OperationProgress {
	stage: 'checking' | 'verifying' | 'extracting' | 'backing-up' | 'applying'
	completedItems?: number | null
	totalItems?: number | null
}

export interface UpdateConflict {
	operationId: string
	operationType: string
	targetAction: string
	target: string
	reason: string
	candidates: string[]
}

export interface UpdaterContext {
	mode: 'manual' | 'bootstrap'
	gameDir: string
	consoleOrigin: string
	updaterVersion: string
	updaterCommitSha: string
	updaterPlatform: string
}

export interface ClientInspection {
	version: string | null
	needsSelection: boolean
}

export interface ClientStorageInfo {
	downloadsBytes: number
	backupsBytes: number
	rollbackAvailable: boolean
	rollbackFromVersion: string | null
	rollbackToVersion: string | null
}

export interface ClientVersionOption {
	version: string
	label: string
	isLatest: boolean
	isBase: boolean
	publishedAt: string | null
	changelog: string | null
	apiVersion: string | null
	modCount: number
	mods: ClientMod[]
	publisher?: ClientReleasePerson | null
	contributors?: ClientReleasePerson[]
	fullPackage?: ClientFullPackage | null
}

export interface ClientReleasePerson {
	hydrolineId: string
	username: string
	displayName: string | null
	avatarUrl: string | null
}

export interface ClientFullPackage {
	packageKey: string
	packageSha256: string
	packageSize: number
	signature: string
	signaturePayload?: 'sha256'
}

export type ClientInstallMode = 'full' | 'mods'

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
	baseUrl: string
	priority: number
	isDefault: boolean
	requiresLogin: boolean
	available: boolean
	latencyMs: number | null
}

export interface SelectOption {
	label: string
	value: string
	disabled?: boolean
	latencyMs?: number | null
	baseUrl?: string
	available?: boolean
}

export type Translator = (
	key: UpdaterMessageKey,
	params?: Record<string, string | number>,
) => string

export const SOURCE_KEY = 'hydcraft:updater:source'
export const CLEAN_DOWNLOADS_AFTER_INSTALL_KEY =
	'hydcraft:updater:clean-downloads-after-install'

export const THEME_MODES = [
	{ value: 'light', icon: 'i-lucide-sun', label: 'themeLight' },
	{ value: 'dark', icon: 'i-lucide-moon', label: 'themeDark' },
	{ value: 'system', icon: 'i-lucide-monitor', label: 'themeSystem' },
] as const satisfies ReadonlyArray<{
	value: 'light' | 'dark' | 'system'
	icon: string
	label: UpdaterMessageKey
}>
