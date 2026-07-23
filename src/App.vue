<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { message } from '@tauri-apps/plugin-dialog'
import hydcraftLogo from './assets/resources/brands/logo_HydCraft.png'
import AppWindowTitlebar from './components/window/AppWindowTitlebar.vue'
import SkeletonImage from './components/common/SkeletonImage.vue'
import { useUpdaterAppearance } from './composables/useUpdaterAppearance'
import { invokeDesktop, startCurrentWindowDragging } from './utils/tauri'
import type { UpdaterMessageKey } from './locales'

type TabKey = 'upgrade' | 'management' | 'addons' | 'settings' | 'about'
type CountdownKind = 'update' | 'launch'

interface UpdaterStatus {
	mode: 'manual' | 'bootstrap'
	phase: string
	message: string
	remainingSeconds?: number
}

interface UpdaterContext {
	mode: 'manual' | 'bootstrap'
	gameDir: string
}

interface ClientInspection {
	version: string | null
	needsSelection: boolean
}

interface DesktopIdentity {
	hydrolineId: string
	username: string
	displayName: string | null
	avatarUrl: string | null
}

interface DownloadSource {
	key: string
	label: string
	priority: number
	requiresLogin: boolean
	available: boolean
}

const SOURCE_KEY = 'hydcraft:updater:source'
const { locale, themeMode, selectLocale, selectTheme, t } =
	useUpdaterAppearance()
const context = ref<UpdaterContext>({ mode: 'manual', gameDir: '' })
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

const isBootstrap = computed(() => context.value.mode === 'bootstrap')
const authenticated = computed(() => Boolean(identity.value))
const displayName = computed(
	() => identity.value?.displayName || identity.value?.username || '',
)
const selectedLocale = computed(() => locale.value)
const appName = computed(() => t('appName'))
const themeModes = [
	{ value: 'light', icon: 'i-lucide-sun', label: 'themeLight' },
	{ value: 'dark', icon: 'i-lucide-moon', label: 'themeDark' },
	{ value: 'system', icon: 'i-lucide-monitor', label: 'themeSystem' },
] as const
const themeIcon = computed(
	() =>
		themeModes.find((item) => item.value === themeMode.value)?.icon ??
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
const availableSources = computed(() =>
	sources.value.filter((source) => source.available),
)
const sourceItems = computed(() =>
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
	{ key: 'management', label: t('tabManagement') },
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
	countdownTimer = setInterval(() => {
		countdown.value -= 1
		if (countdown.value > 0) return
		const action = countdownKind.value
		clearCountdown()
		if (action === 'update') void beginUpdate()
		if (action === 'launch') void launchClient()
	}, 1000)
}

async function refreshIdentity(): Promise<void> {
	identity.value = await invokeDesktop<DesktopIdentity | null>(
		'desktop_identity',
	)
}

async function loadSources(): Promise<void> {
	try {
		sources.value = await invokeDesktop<DownloadSource[]>('download_sources', {
			locale: locale.value,
		})
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

async function openProfile(): Promise<void> {
	if (!identity.value) return
	await invokeDesktop<void>('open_external_url', {
		url: `https://hydcraft.cn/u/${encodeURIComponent(identity.value.username)}`,
	})
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
	const inspection = await invokeDesktop<ClientInspection>('inspect_client')
	if (
		inspection.version &&
		['starting', 'checking-migration', 'awaiting-version'].includes(
			status.value.phase,
		)
	) {
		status.value = await invokeDesktop<UpdaterStatus>(
			'select_current_version',
			{
				version: inspection.version,
			},
		)
	}
	await refreshIdentity()
	await loadSources()
	if (status.value.phase === 'awaiting-version') await openVersionWindow()
	if (status.value.phase === 'awaiting-update-decision') {
		await loadSources()
		armCountdown('update')
	}
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
</script>

<template>
	<main
		class="relative flex min-h-screen overflow-hidden bg-slate-100 text-slate-950 dark:bg-slate-950 dark:text-white"
		@mousemove="interruptCountdown"
		@keydown="interruptCountdown"
		@click.capture="interruptCountdown"
	>
		<aside
			class="flex w-64 shrink-0 flex-col border-r border-slate-200 bg-slate-50 p-7 pt-14 dark:border-slate-800 dark:bg-slate-900"
			@mousedown.left="dragFromAside"
		>
			<div>
				<img
					:src="hydcraftLogo"
					:alt="appName"
					class="size-12 object-contain"
				/>
				<h1 class="mt-5 text-3xl font-semibold tracking-wide">
					{{ appName }}
				</h1>
				<p class="mt-1 text-sm text-slate-600 dark:text-slate-300">
					{{ t('updater') }}
				</p>
			</div>
			<div class="mt-auto flex items-center">
				<div class="flex flex-1 items-center gap-2">
					<UPopover
						:popper="{ placement: 'top-start' }"
						:ui="{ content: 'z-[40000]' }"
					>
						<UButton
							color="neutral"
							variant="ghost"
							size="xs"
							class="h-9 w-9 rounded-full hover:bg-slate-500/10 active:bg-slate-500/20"
							icon-only
							:aria-label="t('theme')"
						>
							<UIcon :name="themeIcon" class="h-6 w-6" />
						</UButton>

						<template #content>
							<div class="flex w-40 flex-col gap-1 p-2">
								<UButton
									v-for="mode in themeModes"
									:key="mode.value"
									type="button"
									color="neutral"
									variant="ghost"
									class="w-full justify-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800"
									:class="{
										'bg-primary-100/60 text-primary-600 dark:bg-primary-500/20 dark:text-primary-200':
											themeMode === mode.value,
										'text-slate-600 dark:text-slate-300':
											themeMode !== mode.value,
									}"
									@click="selectTheme(mode.value)"
								>
									<UIcon :name="mode.icon" class="h-4 w-4" />
									<span>{{ t(mode.label) }}</span>
									<UIcon
										v-if="themeMode === mode.value"
										name="i-lucide-check"
										class="ml-auto h-4 w-4"
									/>
								</UButton>
							</div>
						</template>
					</UPopover>

					<UPopover
						:popper="{ placement: 'top-start' }"
						:ui="{ content: 'z-[40000]' }"
					>
						<UButton
							color="neutral"
							variant="ghost"
							size="xs"
							class="h-9 w-9 rounded-full hover:bg-slate-500/10 active:bg-slate-500/20"
							icon-only
							:aria-label="t('language')"
						>
							<UIcon name="i-lucide-languages" class="h-6 w-6" />
						</UButton>

						<template #content>
							<div class="flex w-40 flex-col gap-1 p-2">
								<UButton
									v-for="item in localeItems"
									:key="item.value"
									type="button"
									color="neutral"
									variant="ghost"
									class="w-full justify-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800"
									:class="{
										'bg-primary-100/60 text-primary-600 dark:bg-primary-500/20 dark:text-primary-200':
											selectedLocale === item.value,
										'text-slate-600 dark:text-slate-300':
											selectedLocale !== item.value,
									}"
									@click="selectLocale(item.value)"
								>
									<span>{{ item.label }}</span>
									<UIcon
										v-if="selectedLocale === item.value"
										name="i-lucide-check"
										class="ml-auto h-4 w-4"
									/>
								</UButton>
							</div>
						</template>
					</UPopover>
				</div>

				<UButton
					v-if="!authenticated"
					color="neutral"
					variant="link"
					size="xs"
					class="px-2 text-sm whitespace-nowrap transition hover:opacity-80"
					@click="startLogin"
					>{{ t('login') }}</UButton
				>
				<UPopover
					v-else
					:popper="{ placement: 'top-end' }"
					:ui="{ content: 'z-[40000]' }"
				>
					<button
						type="button"
						class="ml-0.5 flex h-9 items-center justify-center gap-1 rounded-full border-0 bg-transparent py-0 pr-1.5 pl-0 opacity-100 transition duration-150 hover:opacity-80 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
						:aria-label="t('accountMenu')"
					>
						<span
							class="relative flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-full bg-slate-200 text-sm font-semibold text-slate-700 ring ring-slate-200 transition duration-200 dark:bg-slate-700 dark:text-slate-100 dark:ring-slate-700"
						>
							<SkeletonImage
								v-if="identity?.avatarUrl"
								:src="identity.avatarUrl"
								:alt="displayName"
								image-class="h-full w-full object-cover"
								class="h-full w-full"
							/>
							<span v-else class="leading-none">{{
								displayName.slice(0, 1)
							}}</span>
						</span>
						<UIcon
							name="i-lucide-chevron-down"
							class="h-3.5 w-3.5 translate-y-0 opacity-80 transition duration-200"
						/>
					</button>

					<template #content>
						<div class="flex min-w-40 flex-col gap-1 p-2">
							<div class="px-3 py-2">
								<div
									class="line-clamp-2 wrap-break-word text-[17px] leading-snug font-semibold text-slate-600 dark:text-slate-300"
								>
									{{ displayName }}
								</div>
								<div
									class="text-[13px] leading-[normal] text-slate-500/80 dark:text-slate-400/80"
								>
									{{ identity?.hydrolineId }}
								</div>
							</div>
							<UButton
								color="neutral"
								variant="ghost"
								class="w-full justify-start gap-1.5 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300"
								@click="openProfile"
							>
								<UIcon name="i-lucide-user" class="h-4.5 w-4.5 shrink-0" />
								<span class="leading-[normal] min-w-0 truncate">{{
									t('profile')
								}}</span>
							</UButton>
							<div
								class="my-1 border-t border-slate-200 dark:border-slate-700"
							/>
							<UButton
								color="error"
								variant="ghost"
								class="w-full justify-start gap-1.5 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-error-50! active:bg-error-100! dark:hover:bg-error-900/25! dark:active:bg-error-900/35!"
								@click="logout"
							>
								<UIcon name="i-lucide-log-out" class="h-4.5 w-4.5 shrink-0" />
								<span class="leading-[normal] min-w-0 truncate">{{
									t('logout')
								}}</span>
							</UButton>
						</div>
					</template>
				</UPopover>
			</div>
		</aside>
		<div class="relative flex min-w-0 flex-1 flex-col">
			<AppWindowTitlebar
				:close-label="t('close')"
				:minimize-label="t('minimize')"
			>
				<template #left>
					<div class="flex items-center gap-2">
						<button
							v-for="item in tabs"
							:key="item.key"
							type="button"
							class="group relative z-0 rounded-full p-2 text-[16px] leading-none whitespace-nowrap transition-all duration-[420ms] ease-[cubic-bezier(0.22,1,0.36,1)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
							:class="
								tab === item.key
									? 'font-semibold text-primary opacity-100 dark:text-[rgb(125,211,252)]'
									: 'text-slate-800 opacity-85 hover:text-slate-800 hover:opacity-100 dark:text-slate-300 dark:opacity-75 dark:hover:text-slate-100'
							"
							:aria-current="tab === item.key ? 'page' : undefined"
							@click="tab = item.key"
						>
							<span
								class="pointer-events-none absolute top-[calc(50%+0.24em)] left-1/2 -z-10 h-[0.95em] w-[96%] origin-center -translate-x-1/2 -translate-y-1/2 rounded-md bg-[rgba(125,211,252,0.16)] opacity-0 shadow-[0_0_10px_rgba(125,211,252,0.12)] transition-all duration-[420ms] ease-[cubic-bezier(0.22,1,0.36,1)] group-hover:opacity-100"
								:class="{ 'opacity-100': tab === item.key }"
								aria-hidden="true"
							/>
							{{ item.label }}
						</button>
					</div>
				</template>
			</AppWindowTitlebar>

			<section
				v-if="tab === 'upgrade'"
				class="flex min-h-0 flex-1 items-center justify-center p-8 pt-14"
			>
				<div class="flex w-full max-w-lg flex-col items-center text-center">
					<UIcon
						:name="processIcon"
						class="size-14"
						:class="
							status.phase === 'failed'
								? 'text-danger-500'
								: status.phase === 'ready' || status.phase === 'up-to-date'
									? 'text-success-500'
									: 'text-primary-500'
						"
					/>
					<UIcon
						v-if="showProcessSpinner"
						name="i-lucide-loader-circle"
						class="mt-3 size-5 animate-spin text-primary-500"
					/>
					<h1 class="mt-4 text-xl font-semibold">{{ phaseTitle }}</h1>
					<p
						v-if="
							['updating', 'ready'].includes(status.phase) ||
							(phaseSubtitle && status.phase !== 'awaiting-update-decision')
						"
						class="mt-2 text-sm text-slate-600 dark:text-slate-300"
					>
						{{
							status.phase === 'updating'
								? t('bodyUpdating')
								: status.phase === 'ready'
									? t('bodyReady')
									: phaseSubtitle
						}}
					</p>

					<div
						v-if="status.phase === 'awaiting-update-decision'"
						class="mt-6 flex w-full flex-col gap-4"
					>
						<USelect
							:model-value="selectedSource"
							:items="sourceItems"
							class="w-full"
							@update:model-value="selectSource"
						/>
						<div class="flex gap-3">
							<UButton
								color="neutral"
								variant="soft"
								class="flex-1 justify-center"
								@click="skipUpdate"
								>{{ t('updateLater') }}</UButton
							>
							<UButton
								color="primary"
								class="flex-1 justify-center"
								@click="beginUpdate"
								>{{ t('updateNow') }}</UButton
							>
						</div>
						<p class="text-xs text-slate-500 dark:text-slate-400">
							{{
								countdownKind
									? t('autoUpdateCountdown', { seconds: countdown })
									: t('countdownCancelled')
							}}
						</p>
						<p
							v-if="!authenticated"
							class="text-xs text-slate-500 dark:text-slate-400"
						>
							{{ t('hydrolineFastDownload')
							}}<UButton
								color="primary"
								variant="link"
								size="xs"
								class="p-0 align-baseline"
								:disabled="loginBusy"
								@click="startLogin"
							>
								{{ t('hydrolineFastDownloadAction') }}
							</UButton>
						</p>
						<p v-else class="text-xs text-slate-500 dark:text-slate-400">
							{{ t('hydrolineLoggedIn') }}
						</p>
					</div>

					<div
						v-if="status.phase === 'ready' && isBootstrap"
						class="mt-6 flex w-full gap-3"
					>
						<UButton
							color="neutral"
							variant="soft"
							class="flex-1 justify-center"
							@click="interruptCountdown"
							>{{ t('launchLater') }}</UButton
						>
						<UButton
							color="primary"
							class="flex-1 justify-center"
							@click="launchClient"
							>{{
								countdownKind
									? t('launchNowCountdown', { seconds: countdown })
									: t('launchNow')
							}}</UButton
						>
					</div>
					<div
						v-if="status.phase === 'up-to-date' && isBootstrap"
						class="mt-6 text-xs text-slate-500 dark:text-slate-400"
					>
						{{
							countdownKind
								? t('launchCountdown', { seconds: countdown })
								: t('launchCountdownCancelled')
						}}
					</div>
					<div
						v-if="
							!authenticated &&
							!['awaiting-update-decision'].includes(status.phase)
						"
						class="mt-8 text-xs text-slate-500 dark:text-slate-400"
					>
						<UButton
							color="primary"
							variant="link"
							size="xs"
							class="p-0"
							@click="startLogin"
						>
							{{ t('hydrolineFastDownload') }} {{ t('login') }}
						</UButton>
					</div>
				</div>
			</section>

			<section
				v-else-if="tab === 'settings'"
				class="flex flex-1 items-start justify-center p-8 pt-14"
			>
				<div class="w-full max-w-xl space-y-6">
					<div>
						<h2 class="text-xl font-semibold">{{ t('settingsTitle') }}</h2>
						<p class="mt-1 text-sm text-slate-500 dark:text-slate-400">
							{{ t('settingsDescription') }}
						</p>
					</div>
					<div
						class="space-y-4 rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900"
					>
						<label class="flex items-center justify-between gap-5 text-sm"
							><span>{{ t('defaultDownloadSource') }}</span
							><USelect
								:model-value="selectedSource"
								:items="sourceItems"
								class="w-56"
								@update:model-value="selectSource"
						/></label>
					</div>
				</div>
			</section>

			<section
				v-else-if="tab === 'management'"
				class="flex flex-1 items-start justify-center p-8 pt-14"
			>
				<div class="w-full max-w-xl space-y-6">
					<div>
						<h2 class="text-xl font-semibold">{{ t('managementTitle') }}</h2>
						<p class="mt-1 text-sm text-slate-500 dark:text-slate-400">
							{{ t('managementDescription') }}
						</p>
					</div>
					<div class="grid gap-4 sm:grid-cols-2">
						<div
							class="rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900"
						>
							<p class="text-xs text-slate-500 dark:text-slate-400">
								{{ t('mode') }}
							</p>
							<p class="mt-2 font-medium">
								{{ isBootstrap ? t('modeBootstrap') : t('modeManual') }}
							</p>
						</div>
						<div
							class="rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900"
						>
							<p class="text-xs text-slate-500 dark:text-slate-400">
								{{ t('currentPhase') }}
							</p>
							<p class="mt-2 font-medium">{{ phaseTitle }}</p>
						</div>
					</div>
					<div
						class="rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900"
					>
						<p class="text-xs text-slate-500 dark:text-slate-400">
							{{ t('clientDirectory') }}
						</p>
						<p class="mt-2 break-all font-mono text-sm">
							{{ context.gameDir || t('notRead') }}
						</p>
					</div>
				</div>
			</section>

			<section
				v-else-if="tab === 'addons'"
				class="flex flex-1 items-center justify-center p-8 pt-14 text-center"
			>
				<div class="flex max-w-sm flex-col items-center">
					<UIcon name="i-lucide-package" class="size-12 text-slate-400" />
					<h2 class="mt-4 text-xl font-semibold">{{ t('addonsTitle') }}</h2>
					<p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400">
						{{ t('addonsDescription') }}
					</p>
				</div>
			</section>

			<section
				v-else
				class="flex flex-1 items-center justify-center p-8 pt-14 text-center"
			>
				<div class="flex max-w-sm flex-col items-center">
					<img
						:src="hydcraftLogo"
						alt="HydCraft"
						class="size-20 object-contain"
					/>
					<h2 class="mt-4 text-xl font-semibold">{{ t('aboutTitle') }}</h2>
					<p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400">
						{{ t('aboutDescription') }}
					</p>
					<p class="mt-4 font-mono text-xs text-slate-400">
						{{ t('aboutVersion') }}
					</p>
				</div>
			</section>
		</div>
	</main>
</template>
