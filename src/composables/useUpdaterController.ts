import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { message } from '@tauri-apps/plugin-dialog'
import { useUpdaterAppearance } from './useUpdaterAppearance'
import {
	clearCurrentWindowAttention,
	invokeDesktop,
	playSystemFailureSound,
	requestCurrentWindowAttention,
	startCurrentWindowDragging,
} from '../utils/tauri'
import type { UpdaterMessageKey } from '../locales'
import {
	SOURCE_KEY,
	THEME_MODES,
	type ClientInspection,
	type ClientDetailsKind,
	type ClientVersionOption,
	type DesktopIdentity,
	type DownloadSource,
	type SelectOption,
	type TabKey,
	type UpdateConflict,
	type UpdaterContext,
	type UpdaterStatus,
} from '../types/updater'

export function useUpdaterController() {
	const { locale, themeMode, selectLocale, selectTheme, t } =
		useUpdaterAppearance()
	const context = ref<UpdaterContext>({
		mode: 'manual',
		gameDir: '',
		consoleOrigin: '',
	})
	const clientVersions = ref<ClientVersionOption[]>([])
	const currentClientVersion = ref<string | null>(null)
	const status = ref<UpdaterStatus>({
		mode: 'manual',
		phase: 'starting',
		message: '',
	})
	const identity = ref<DesktopIdentity | null>(null)
	const conflicts = ref<UpdateConflict[]>([])
	const conflictSelections = ref<Record<string, string>>({})
	const sources = ref<DownloadSource[]>([])
	const selectedSource = ref(localStorage.getItem(SOURCE_KEY) ?? '')
	const tab = ref<TabKey>('upgrade')
	const loginBusy = ref(false)
	let sourceSelectionManuallyChanged = false
	let unlistenStatus: (() => void) | undefined
	let unlistenAuth: (() => void) | undefined
	let lastNotifiedPhase = ''
	let clientVersionsLoaded = false
	let versionWindowOpened = false

	const isBootstrap = computed(() => context.value.mode === 'bootstrap')
	const authenticated = computed(() => Boolean(identity.value))
	const displayName = computed(
		() => identity.value?.displayName || identity.value?.username || '',
	)
	const appName = computed(() => t('appName'))
	const themeIcon = computed(
		() =>
			THEME_MODES.find((item) => item.value === themeMode.value)?.icon ??
			'i-lucide-monitor',
	)
	const processIcon = computed(() => {
		if (status.value.phase === 'failed') return 'i-lucide-circle-alert'
		if (status.value.phase === 'unknown-client') return 'i-lucide-circle-help'
		if (['up-to-date', 'ready'].includes(status.value.phase))
			return 'i-lucide-circle-check-big'
		return 'i-lucide-package'
	})
	const showProcessSpinner = computed(() =>
		['starting', 'checking-migration', 'checking-update', 'updating'].includes(
			status.value.phase,
		),
	)
	const phaseTitle = computed(() => {
		const titles: Record<string, UpdaterMessageKey> = {
			starting: 'statusStarting',
			'awaiting-version': 'statusAwaitingVersion',
			'checking-update': 'statusCheckingUpdate',
			'awaiting-update-decision': 'statusAwaitingUpdateDecision',
			'unknown-client': 'statusUnknownClient',
			updating: 'statusUpdating',
			ready: 'statusReady',
			'up-to-date': 'statusUpToDate',
			failed: 'statusFailed',
		}
		const key = titles[status.value.phase]
		return key ? t(key) : status.value.message || t('statusUnknown')
	})
	const phaseMessages: Partial<Record<string, UpdaterMessageKey>> = {
		starting: 'statusStarting',
		'awaiting-version': 'statusAwaitingVersion',
		'checking-update': 'statusCheckingUpdate',
		'awaiting-update-decision': 'statusAwaitingUpdateDecision',
		'unknown-client': 'statusUnknownClient',
		updating: 'statusUpdating',
		ready: 'statusReady',
		'up-to-date': 'statusUpToDate',
		failed: 'statusFailed',
	}
	const localizedStatusMessage = computed(() => {
		const key = phaseMessages[status.value.phase]
		if (
			status.value.phase === 'failed' &&
			status.value.message &&
			status.value.message !== t('statusFailed')
		)
			return status.value.message
		return key ? t(key) : status.value.message
	})
	const phaseSubtitle = computed(() => {
		const message = localizedStatusMessage.value
		return message && message !== phaseTitle.value ? message : ''
	})
	const sourceItems = computed<SelectOption[]>(() =>
		sources.value
			.filter((source) => !source.requiresLogin || authenticated.value)
			.map((source) => ({
				label: source.label,
				value: source.key,
				disabled: !source.available,
				latencyMs: source.latencyMs,
			})),
	)
	const localeItems = computed(
		() =>
			[
				{ value: 'zh-CN', label: t('languageZhCN') },
				{ value: 'zh-TW', label: t('languageZhTW') },
				{ value: 'ja-JP', label: t('languageJaJP') },
				{ value: 'en-US', label: t('languageEnUS') },
			] as const,
	)
	const tabs = computed<Array<{ key: TabKey; label: string }>>(() => [
		{ key: 'upgrade', label: t('tabUpgrade') },
		{ key: 'client', label: t('tabClient') },
		{ key: 'addons', label: t('tabAddons') },
		{ key: 'settings', label: t('tabSettings') },
		{ key: 'about', label: t('tabAbout') },
	])

	async function nativeMessage(
		text: string,
		kind: 'info' | 'error' = 'info',
	): Promise<void> {
		if ('__TAURI_INTERNALS__' in window) {
			await message(text, { title: t('dialogTitle'), kind })
		} else {
			window.alert(text)
		}
	}

	async function nativeError(text: string): Promise<void> {
		if (status.value.phase === 'failed') return
		await nativeMessage(text, 'error')
	}

	async function notifyFailure(): Promise<void> {
		await playSystemFailureSound().catch(() => undefined)
		try {
			await requestCurrentWindowAttention()
		} catch {
			// Window attention is best effort; the page still contains the failure.
		}
	}

	async function refreshIdentity(): Promise<void> {
		identity.value = await invokeDesktop<DesktopIdentity | null>(
			'desktop_identity',
		)
	}

	async function loadSources(): Promise<void> {
		try {
			sources.value = await invokeDesktop<DownloadSource[]>(
				'download_sources',
				{
					locale: locale.value,
				},
			)
			const selectableSources = sources.value.filter(
				(source) => !source.requiresLogin || authenticated.value,
			)
			const preferredKey = identity.value
				? 'dl-shanghai-cdn'
				: selectedSource.value
			const preferred = sourceSelectionManuallyChanged
				? selectableSources.find(
						(source) => source.key === selectedSource.value && source.available,
					)
				: selectableSources.find(
						(source) => source.key === preferredKey && source.available,
					)
			const fallback =
				preferred ?? selectableSources.find((source) => source.available)
			if (fallback) {
				selectedSource.value = fallback.key
				localStorage.setItem(SOURCE_KEY, fallback.key)
				await invokeDesktop<void>('select_download_source', {
					sourceKey: fallback.key,
				})
			}
		} catch (error) {
			await nativeError(t('readSourcesFailed', { error: String(error) }))
		}
	}

	async function openVersionWindow(): Promise<void> {
		try {
			await invokeDesktop<void>('open_version_window')
		} catch (error) {
			await nativeError(t('openVersionFailed', { error: String(error) }))
		}
	}

	async function openClientDetails(
		version: string,
		detail: ClientDetailsKind,
	): Promise<void> {
		try {
			await invokeDesktop<void>('open_client_details_window', {
				version,
				detail,
			})
		} catch (error) {
			await nativeError(t('openClientDetailsFailed', { error: String(error) }))
		}
	}

	async function openVersionWindowIfAvailable(): Promise<void> {
		if (
			!clientVersionsLoaded ||
			versionWindowOpened ||
			!clientVersions.value.length
		)
			return
		versionWindowOpened = true
		await openVersionWindow()
	}

	async function beginUpdate(): Promise<void> {
		conflicts.value = []
		conflictSelections.value = {}
		await clearCurrentWindowAttention().catch(() => undefined)
		try {
			await invokeDesktop<void>('begin_update')
		} catch (error) {
			await nativeError(t('beginUpdateFailed', { error: String(error) }))
		}
	}

	async function retryUpdate(): Promise<void> {
		if (status.value.failureKind === 'check') {
			await recheckUpdate()
			return
		}
		await beginUpdate()
	}

	async function loadConflicts(): Promise<void> {
		conflicts.value = await invokeDesktop<UpdateConflict[]>('pending_conflicts')
		conflictSelections.value = Object.fromEntries(
			conflicts.value.map((conflict) => [
				conflict.operationId,
				conflictSelections.value[conflict.operationId] ||
					conflict.candidates[0] ||
					conflict.target,
			]),
		)
	}

	function selectConflictResolution(operationId: string, value: string): void {
		conflictSelections.value = {
			...conflictSelections.value,
			[operationId]: value,
		}
	}

	async function resolveConflicts(): Promise<void> {
		const resolutions = Object.fromEntries(
			conflicts.value.map((conflict) => [
				conflict.operationId,
				conflictSelections.value[conflict.operationId] ||
					conflict.candidates[0] ||
					conflict.target,
			]),
		)
		const next = await invokeDesktop<UpdaterStatus>('resolve_conflicts', {
			resolutions,
		})
		await applyStatus(next)
	}

	async function recheckUpdate(): Promise<void> {
		await clearCurrentWindowAttention().catch(() => undefined)
		try {
			await invokeDesktop<void>('recheck_update')
		} catch (error) {
			await nativeError(t('beginUpdateFailed', { error: String(error) }))
		}
	}

	async function launchClient(): Promise<void> {
		try {
			await invokeDesktop<void>('launch_client')
		} catch {
			await nativeError(t('manualLaunchUnavailable'))
		}
	}

	async function startLogin(): Promise<void> {
		loginBusy.value = true
		try {
			await invokeDesktop<void>('start_desktop_login')
		} catch (error) {
			await nativeError(t('loginWindowFailed', { error: String(error) }))
		} finally {
			loginBusy.value = false
		}
	}

	async function logout(): Promise<void> {
		try {
			await invokeDesktop<void>('logout_desktop')
			identity.value = null
			sourceSelectionManuallyChanged = false
			await loadSources()
		} catch (error) {
			await nativeError(t('logoutFailed', { error: String(error) }))
		}
	}

	async function openExternalUrl(url: string): Promise<void> {
		await invokeDesktop<void>('open_external_url', { url })
	}

	async function openProfile(): Promise<void> {
		if (!identity.value) return
		await openExternalUrl(
			`https://hydcraft.cn/u/${encodeURIComponent(identity.value.username)}`,
		)
	}

	async function applyStatus(next: UpdaterStatus): Promise<void> {
		status.value = next
		if (!['failed', 'up-to-date'].includes(next.phase)) lastNotifiedPhase = ''
		if (next.phase === 'awaiting-version') await openVersionWindowIfAvailable()
		if (next.phase === 'awaiting-conflict-resolution') await loadConflicts()
		if (next.phase === 'awaiting-update-decision') {
			await loadSources()
		}
		if (next.phase === 'authenticated') {
			await refreshIdentity()
			await loadSources()
			await invokeDesktop<void>('recheck_update')
		}
		if (next.phase === 'failed' && lastNotifiedPhase !== next.phase) {
			lastNotifiedPhase = next.phase
			await notifyFailure()
		}
	}

	async function refresh(): Promise<void> {
		context.value = await invokeDesktop<UpdaterContext>('updater_context')
		status.value = await invokeDesktop<UpdaterStatus>('updater_status')
		if (status.value.phase === 'failed') {
			lastNotifiedPhase = 'failed'
			await notifyFailure()
		}
		const versionOptions = await invokeDesktop<ClientVersionOption[]>(
			'client_version_options',
		)
		clientVersions.value = versionOptions
		clientVersionsLoaded = true
		const inspection = await invokeDesktop<ClientInspection>('inspect_client')
		currentClientVersion.value = inspection.version
		if (
			inspection.version &&
			['starting', 'checking-migration', 'awaiting-version'].includes(
				status.value.phase,
			)
		) {
			status.value = await invokeDesktop<UpdaterStatus>(
				'select_current_version',
				{ version: inspection.version },
			)
		}
		await refreshIdentity()
		await loadSources()
		if (status.value.phase === 'awaiting-version')
			await openVersionWindowIfAvailable()
		if (status.value.phase === 'awaiting-conflict-resolution')
			await loadConflicts()
		if (status.value.phase === 'awaiting-update-decision') {
			await loadSources()
		}
	}

	function selectSource(key: string | undefined): void {
		if (!key) return
		sourceSelectionManuallyChanged = true
		selectedSource.value = key
		localStorage.setItem(SOURCE_KEY, key)
		void invokeDesktop<void>('select_download_source', { sourceKey: key })
	}

	async function dragFromAside(event: MouseEvent): Promise<void> {
		const target = event.target as HTMLElement | null
		if (
			target?.closest(
				'button, a, input, select, textarea, [role="button"], [data-no-window-drag]',
			)
		) {
			return
		}
		await startCurrentWindowDragging()
	}

	onMounted(async () => {
		try {
			if ('__TAURI_INTERNALS__' in window) {
				const { listen } = await import('@tauri-apps/api/event')
				unlistenStatus = await listen<UpdaterStatus>(
					'updater-status',
					({ payload }) => void applyStatus(payload),
				)
				unlistenAuth = await listen(
					'desktop-auth-result',
					() => void refreshIdentity(),
				)
			}
			await refresh()
		} catch (error) {
			await nativeError(t('initializeFailed', { error: String(error) }))
		}
	})

	onBeforeUnmount(() => {
		unlistenStatus?.()
		unlistenAuth?.()
	})

	return {
		appName,
		authenticated,
		clientVersions,
		conflictSelections,
		conflicts,
		currentClientVersion,
		context,
		displayName,
		identity,
		isBootstrap,
		locale,
		localeItems,
		loginBusy,
		phaseSubtitle,
		phaseTitle,
		processIcon,
		selectLocale,
		selectSource,
		selectTheme,
		selectedLocale: locale,
		selectedSource,
		showProcessSpinner,
		sourceItems,
		startLogin,
		status,
		t,
		tab,
		tabs,
		themeIcon,
		themeMode,
		themeModes: THEME_MODES,
		beginUpdate,
		dragFromAside,
		launchClient,
		logout,
		openProfile,
		openExternalUrl,
		openClientDetails,
		retryUpdate,
		recheckUpdate,
		resolveConflicts,
		selectConflictResolution,
	}
}
