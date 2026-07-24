<script setup lang="ts">
import AurLemon from '../branding/AurLemon.vue'
import HydCraft from '../branding/HydCraft.vue'
import type {
	CountdownKind,
	ClientVersionOption,
	SelectOption,
	TabKey,
	Translator,
	UpdaterContext,
	UpdaterStatus,
} from '../../types/updater'

defineProps<{
	authenticated: boolean
	clientVersions: ClientVersionOption[]
	context: UpdaterContext
	countdown: number
	countdownKind: CountdownKind | null
	isBootstrap: boolean
	loginBusy: boolean
	phaseSubtitle: string
	phaseTitle: string
	processIcon: string
	selectedSource: string
	showProcessSpinner: boolean
	sourceItems: SelectOption[]
	status: UpdaterStatus
	tab: TabKey
	t: Translator
}>()

const emit = defineEmits<{
	beginUpdate: []
	interruptCountdown: []
	launchClient: []
	login: []
	openExternalUrl: [url: string]
	selectSource: [value: string | undefined]
	skipUpdate: []
}>()
</script>

<template>
	<Transition name="updater-tab-switch" mode="out-in">
		<section
			v-if="tab === 'upgrade'"
			key="upgrade"
			class="flex min-h-0 flex-1 items-center justify-center px-6 pb-8 pt-16"
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
						@update:model-value="emit('selectSource', $event)"
					/>
					<div class="flex gap-3">
						<UButton
							color="neutral"
							variant="soft"
							class="flex-1 justify-center"
							@click="emit('skipUpdate')"
						>
							{{ t('updateLater') }}
						</UButton>
						<UButton
							color="primary"
							class="flex-1 justify-center"
							@click="emit('beginUpdate')"
						>
							{{ t('updateNow') }}
						</UButton>
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
						{{ t('hydrolineFastDownload') }}
						<UButton
							color="primary"
							variant="link"
							size="xs"
							class="p-0 align-baseline"
							:disabled="loginBusy"
							@click="emit('login')"
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
						@click="emit('interruptCountdown')"
					>
						{{ t('launchLater') }}
					</UButton>
					<UButton
						color="primary"
						class="flex-1 justify-center"
						@click="emit('launchClient')"
					>
						{{
							countdownKind
								? t('launchNowCountdown', { seconds: countdown })
								: t('launchNow')
						}}
					</UButton>
				</div>
				<div
					v-if="status.phase === 'up-to-date' && isBootstrap"
					class="relative mt-6 flex w-full flex-col items-center"
				>
					<UButton
						color="primary"
						variant="soft"
						class="min-w-44 justify-center"
						@click="emit('launchClient')"
					>
						{{ t('launchNow') }}
					</UButton>
					<p
						v-if="countdownKind"
						class="pointer-events-none absolute top-full mt-2 text-xs text-slate-500 dark:text-slate-400"
					>
						{{ t('launchCountdown', { seconds: countdown }) }}
					</p>
				</div>
				<div
					v-if="!authenticated && status.phase !== 'awaiting-update-decision'"
					class="mt-8 text-xs text-slate-500 dark:text-slate-400"
				>
					<UButton
						color="primary"
						variant="link"
						size="xs"
						class="p-0"
						@click="emit('login')"
					>
						{{ t('hydrolineFastDownload') }} {{ t('login') }}
					</UButton>
				</div>
			</div>
		</section>

		<section
			v-else-if="tab === 'settings'"
			key="settings"
			class="flex flex-1 items-start justify-center px-6 pb-8 pt-16"
		>
			<div class="w-full space-y-6">
				<section class="grid gap-1.5">
					<div class="mx-1 text-xl text-slate-950 dark:text-white">
						{{ t('statusSettingsTitle') }}
					</div>
					<div
						class="grid gap-4 rounded-lg border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-950"
					>
						<div class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start">
							<span
								class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ t('mode') }}
							</span>
							<span
								class="min-w-0 w-full text-sm text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ isBootstrap ? t('modeBootstrap') : t('modeManual') }}
							</span>
						</div>
						<div class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start">
							<span
								class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ t('currentPhase') }}
							</span>
							<span
								class="min-w-0 w-full text-sm text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ phaseTitle }}
							</span>
						</div>
						<div class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start">
							<span
								class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ t('clientDirectory') }}
							</span>
							<span
								class="min-w-0 w-full break-all text-sm text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ context.gameDir || t('notRead') }}
							</span>
						</div>
					</div>
				</section>

				<section class="grid gap-1.5">
					<div class="mx-1 text-xl text-slate-950 dark:text-white">
						{{ t('downloadSettingsTitle') }}
					</div>
					<div
						class="grid gap-4 rounded-lg border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-950"
					>
						<label class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start">
							<span
								class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
							>
								{{ t('defaultDownloadSource') }}
							</span>
							<span class="min-w-0 w-full">
								<USelect
									:model-value="selectedSource"
									:items="sourceItems"
									class="w-full text-sm"
									@update:model-value="emit('selectSource', $event)"
								/>
							</span>
						</label>
					</div>
				</section>
			</div>
		</section>

		<section
			v-else-if="tab === 'client'"
			key="client"
			class="flex flex-1 items-center justify-center px-6 pb-8 pt-16 text-center"
		>
			<div
				v-if="!clientVersions.length"
				class="flex max-w-sm flex-col items-center"
			>
				<UIcon
					name="i-lucide-package"
					class="size-12 text-slate-950 dark:text-white"
				/>
				<h2 class="mt-4 text-xl font-semibold">{{ t('clientTitle') }}</h2>
				<p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400">
					{{ t('noData') }}
				</p>
			</div>
			<div v-else class="w-full space-y-6 text-left">
				<h2 class="text-xl font-semibold">{{ t('clientTitle') }}</h2>
				<div
					class="grid gap-4 rounded-lg border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-950"
				>
					<div
						v-for="option in clientVersions"
						:key="option.version"
						class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start"
					>
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ option.label }}
						</span>
						<span
							class="min-w-0 w-full text-sm text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ option.version }}
							<span
								v-if="option.isLatest"
								class="ml-2 text-xs text-primary-600 dark:text-primary-400"
							>
								{{ t('versionLatest') }}
							</span>
						</span>
					</div>
				</div>
			</div>
		</section>

		<section
			v-else-if="tab === 'addons'"
			key="addons"
			class="flex flex-1 items-center justify-center px-6 pb-8 pt-16 text-center"
		>
			<div class="flex max-w-sm flex-col items-center">
				<UIcon
					name="i-lucide-package"
					class="size-12 text-slate-950 dark:text-white"
				/>
				<h2 class="mt-4 text-xl font-semibold">{{ t('addonsTitle') }}</h2>
				<p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400">
					{{ t('noData') }}
				</p>
			</div>
		</section>

		<section
			key="about"
			v-else
			class="flex flex-1 items-center justify-center px-6 pb-8 pt-16 text-center"
		>
			<div class="flex max-w-2xl flex-col items-center">
				<h2 class="text-xl font-semibold">{{ t('aboutTitle') }}</h2>
				<p class="mt-2 text-sm text-slate-500 dark:text-slate-400">
					{{ t('aboutVersion') }}
				</p>
				<p
					class="mt-6 max-w-xl break-words text-justify text-sm leading-7 text-slate-600 [hyphens:auto] [text-justify:inter-ideograph] dark:text-slate-300"
				>
					{{ t('aboutDescription') }}
				</p>
				<div class="mt-4 flex items-center gap-4">
					<UButton
						color="neutral"
						variant="link"
						class="h-6 p-0 text-slate-500 transition hover:text-slate-950 dark:text-slate-400 dark:hover:text-white"
						:aria-label="t('aboutGithub')"
						@click="emit('openExternalUrl', 'https://github.com/Hydroline')"
					>
						<UIcon name="i-lucide-github" class="size-6" />
						<span class="text-base select-none">GitHub</span>
					</UButton>
					<button
						type="button"
						class="flex h-6 items-center rounded-lg p-0 transition-opacity hover:opacity-70 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
						:aria-label="t('aboutAurlemon')"
						@click="emit('openExternalUrl', 'https://aurlemon.top')"
					>
						<span class="origin-center scale-[0.8]">
							<AurLemon />
						</span>
					</button>
					<button
						type="button"
						class="flex h-6 items-center rounded-lg transition-opacity hover:opacity-70 select-none"
						title="氢气工艺"
						aria-label="氢气工艺"
						@click="emit('openExternalUrl', 'https://hydcraft.cn')"
					>
						<HydCraft />
					</button>
				</div>
				<p class="mt-5 text-xs text-slate-400 dark:text-slate-500">
					{{ t('aboutCopyright') }}
				</p>
			</div>
		</section>
	</Transition>
</template>

<style scoped>
.updater-tab-switch-enter-active,
.updater-tab-switch-leave-active {
	transition:
		opacity 220ms ease-out,
		transform 260ms cubic-bezier(0.16, 1, 0.3, 1),
		filter 220ms ease-out;
}

.updater-tab-switch-enter-from,
.updater-tab-switch-leave-to {
	opacity: 0;
	filter: blur(2px);
	transform: translateY(8px);
}

.updater-tab-switch-enter-to,
.updater-tab-switch-leave-from {
	opacity: 1;
	filter: blur(0);
	transform: translateY(0);
}
</style>
