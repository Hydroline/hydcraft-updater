import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { message } from '@tauri-apps/plugin-dialog'
import { useUpdaterAppearance } from './useUpdaterAppearance'
import { invokeDesktop, startCurrentWindowDragging } from '../utils/tauri'
import type { UpdaterMessageKey } from '../locales'
import {
	SOURCE_KEY,
	THEME_MODES,
	type ClientInspection,
	type ClientVersionOption,
	type CountdownKind,
	type DesktopIdentity,
	type DownloadSource,
	type SelectOption,
	type TabKey,
	type UpdaterContext,
	type UpdaterStatus,
} from '../types/updater'

export function useUpdaterController() {
	const { locale, themeMode, selectLocale, selectTheme, t } =
		useUpdaterAppearance()
	const context = ref<UpdaterContext>({ mode: 'manual', gameDir: '' })
	const clientVersions = ref<ClientVersionOption[]>([])
	const status = ref<UpdaterStatus>({
		mode: 'manual',
		phase: 'starting',
		message: '',
	})
	const identity = ref<DesktopIdentity | null>(null)
	const sources = ref<DownloadSource[]>([])
	const selectedSource = ref(localStorage.getItem(SOURCE_KEY) ?? '')
	const tab = ref<TabKey>('upgrade')
	const countdown = ref(10)
	const countdownKind = ref<CountdownKind | null>(null)
	const loginBusy = ref(false)
	let countdownTimer: ReturnType<typeof setInterval> | undefined
	let unlistenStatus: (() => void) | undefined
	let unlistenAuth: (() => void) | undefined
	let lastNotifiedPhase = ''
	let countdownMousePosition: { x: number; y: number } | undefined

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
		if (['up-to-date', 'ready'].includes(status.value.phase))
			return 'i-lucide-circle-check-big'
		return 'i-lucide-package'
	})
	const showProcessSpinner = computed(
		() => !['failed', 'up-to-date', 'ready'].includes(status.value.phase),
	)
	const phaseTitle = computed(() => {
		const titles: Record<string, UpdaterMessageKey> = {
			starting: 'statusStarting',
			'awaiting-version': 'statusAwaitingVersion',
			'checking-update': 'statusCheckingUpdate',
			'awaiting-update-decision': 'statusAwaitingUpdateDecision',
			updating: 'statusUpdating',
			ready: 'statusReady',
			'up-to-date': 'statusUpToDate',
			deferred: 'statusDeferred',
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
		updating: 'statusUpdating',
		ready: 'statusReady',
		'up-to-date': 'statusUpToDate',
		deferred: 'statusDeferred',
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
		sources.value.map((source) => ({
			label: `${source.label}${source.requiresLogin && !source.available ? ` ${t('sourceLoginRequired')}` : ''}`,
			value: source.key,
			disabled: !source.available,
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

	function clearCountdown(): void {
		if (countdownTimer) clearInterval(countdownTimer)
		countdownTimer = undefined
		countdownKind.value = null
		countdownMousePosition = undefined
	}

	function interruptCountdown(): void {
		if (!countdownTimer) return
		clearCountdown()
		countdown.value = 0
	}

	function armCountdown(kind: CountdownKind): void {
		clearCountdown()
		countdown.value = 10
		countdownKind.value = kind
		countdownMousePosition = undefined
		countdownTimer = setInterval(() => {
			countdown.value -= 1
			if (countdown.value > 0) return
			const action = countdownKind.value
			clearCountdown()
			if (action === 'update') void beginUpdate()
			if (action === 'launch') void launchClient()
		}, 1000)
	}

	function handleWindowMouseMove(event: MouseEvent): void {
		if (!countdownTimer) return
		if (!countdownMousePosition) {
			countdownMousePosition = { x: event.clientX, y: event.clientY }
			return
		}
		const distanceSquared =
			(event.clientX - countdownMousePosition.x) ** 2 +
			(event.clientY - countdownMousePosition.y) ** 2
		if (distanceSquared < 4) return
		interruptCountdown()
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
			const preferred = sources.value.find(
				(source) => source.key === selectedSource.value && source.available,
			)
			const fallback =
				preferred ?? sources.value.find((source) => source.available)
			if (fallback) {
				selectedSource.value = fallback.key
				localStorage.setItem(SOURCE_KEY, fallback.key)
				await invokeDesktop<void>('select_download_source', {
					sourceKey: fallback.key,
				})
			}
		} catch (error) {
			await nativeMessage(
				t('readSourcesFailed', { error: String(error) }),
				'error',
			)
		}
	}

	async function openVersionWindow(): Promise<void> {
		try {
			await invokeDesktop<void>('open_version_window')
		} catch (error) {
			await nativeMessage(
				t('openVersionFailed', { error: String(error) }),
				'error',
			)
		}
	}

	async function beginUpdate(): Promise<void> {
		interruptCountdown()
		try {
			await invokeDesktop<void>('begin_update')
		} catch (error) {
			await nativeMessage(
				t('beginUpdateFailed', { error: String(error) }),
				'error',
			)
		}
	}

	async function skipUpdate(): Promise<void> {
		interruptCountdown()
		try {
			await invokeDesktop<void>('skip_update')
		} catch (error) {
			await nativeMessage(
				t('skipUpdateFailed', { error: String(error) }),
				'error',
			)
		}
	}

	async function launchClient(): Promise<void> {
		interruptCountdown()
		try {
			await invokeDesktop<void>('launch_client')
		} catch {
			await nativeMessage(t('manualLaunchUnavailable'), 'error')
		}
	}

	async function startLogin(): Promise<void> {
		loginBusy.value = true
		interruptCountdown()
		try {
			await invokeDesktop<void>('start_desktop_login')
		} catch (error) {
			await nativeMessage(
				t('loginWindowFailed', { error: String(error) }),
				'error',
			)
		} finally {
			loginBusy.value = false
		}
	}

	async function logout(): Promise<void> {
		try {
			await invokeDesktop<void>('logout_desktop')
			identity.value = null
			await loadSources()
		} catch (error) {
			await nativeMessage(t('logoutFailed', { error: String(error) }), 'error')
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
		if (next.phase === 'awaiting-version') await openVersionWindow()
		if (next.phase === 'awaiting-update-decision') {
			await loadSources()
			armCountdown('update')
		}
		if (next.phase === 'ready' && isBootstrap.value) armCountdown('launch')
		if (next.phase === 'up-to-date' && isBootstrap.value) armCountdown('launch')
		if (next.phase === 'authenticated') {
			await refreshIdentity()
			await loadSources()
			await invokeDesktop<void>('recheck_update')
		}
		if (
			['failed', 'deferred', 'up-to-date'].includes(next.phase) &&
			lastNotifiedPhase !== next.phase
		) {
			lastNotifiedPhase = next.phase
			if (next.phase !== 'up-to-date' || !isBootstrap.value)
				await nativeMessage(
					localizedStatusMessage.value,
					next.phase === 'failed' ? 'error' : 'info',
				)
		}
	}

	async function refresh(): Promise<void> {
		context.value = await invokeDesktop<UpdaterContext>('updater_context')
		status.value = await invokeDesktop<UpdaterStatus>('updater_status')
		const versionOptions = await invokeDesktop<ClientVersionOption[]>(
			'client_version_options',
		)
		clientVersions.value = versionOptions.filter(
			(option) => option.version !== '__no-version__',
		)
		const inspection = await invokeDesktop<ClientInspection>('inspect_client')
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
		if (status.value.phase === 'awaiting-version') await openVersionWindow()
		if (status.value.phase === 'awaiting-update-decision') {
			await loadSources()
			armCountdown('update')
		}
		if (
			['ready', 'up-to-date'].includes(status.value.phase) &&
			isBootstrap.value
		)
			armCountdown('launch')
	}

	function selectSource(key: string | undefined): void {
		if (!key) return
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
			await nativeMessage(
				t('initializeFailed', { error: String(error) }),
				'error',
			)
		}
	})

	onBeforeUnmount(() => {
		clearCountdown()
		unlistenStatus?.()
		unlistenAuth?.()
	})

	return {
		appName,
		authenticated,
		clientVersions,
		context,
		countdown,
		countdownKind,
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
		handleWindowMouseMove,
		interruptCountdown,
		launchClient,
		logout,
		openProfile,
		openExternalUrl,
		skipUpdate,
	}
}
