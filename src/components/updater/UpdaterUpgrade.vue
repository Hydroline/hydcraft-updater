<script setup lang="ts">
import { computed, ref } from 'vue'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-vue'
import MarkdownContent from '../common/MarkdownContent.vue'
import { HYDCRAFT_SCROLLBAR_OPTIONS } from '../../utils/scrollbar'
import type {
	ClientVersionOption,
	SelectOption,
	Translator,
	UpdateConflict,
	UpdaterStatus,
} from '../../types/updater'

const props = defineProps<{
	authenticated: boolean
	clientVersions: ClientVersionOption[]
	conflictSelections: Record<string, string>
	conflicts: UpdateConflict[]
	currentClientVersion: string | null
	isBootstrap: boolean
	loginBusy: boolean
	phaseSubtitle: string
	phaseTitle: string
	processIcon: string
	selectedSource: string
	showProcessSpinner: boolean
	sourceItems: SelectOption[]
	status: UpdaterStatus
	t: Translator
}>()

const emit = defineEmits<{
	beginUpdate: []
	launchClient: []
	login: []
	recheckUpdate: []
	resolveConflicts: []
	selectConflictResolution: [payload: { operationId: string; value: string }]
	selectSource: [value: string | undefined]
	retryUpdate: []
}>()

const downloadProgress = computed(() => props.status.download)
const updateCurrentVersion = computed(
	() => props.status.currentVersion ?? props.currentClientVersion,
)
const updateTargetVersion = computed(
	() =>
		props.status.targetVersion ??
		props.clientVersions.find((version) => version.isLatest)?.version ??
		null,
)
const updateRelease = computed(() =>
	props.clientVersions.find(
		(version) => version.version === updateTargetVersion.value,
	),
)
const selectedSourceLabel = computed(
	() =>
		props.sourceItems.find((source) => source.value === props.selectedSource)
			?.label ?? props.selectedSource,
)
const sourceMenuOpen = ref(false)
const downloadPercent = computed(() => {
	const progress = downloadProgress.value
	if (!progress?.totalBytes) return 0
	return Math.min(
		100,
		Math.max(0, (progress.downloadedBytes / progress.totalBytes) * 100),
	)
})

function formatBytes(bytes: number): string {
	if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
	const units = ['B', 'KB', 'MB', 'GB'] as const
	let value = bytes
	let unitIndex = 0
	while (value >= 1024 && unitIndex < units.length - 1) {
		value /= 1024
		unitIndex += 1
	}
	return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`
}

function formatSpeed(bytesPerSecond: number): string {
	return `${formatBytes(bytesPerSecond)}/s`
}

function conflictOptions(conflict: UpdateConflict): SelectOption[] {
	const values = Array.from(
		new Set([conflict.target, ...conflict.candidates].filter(Boolean)),
	)
	return values.map((value) => ({
		label:
			value === conflict.target
				? props.t('conflictUseTarget', { path: value })
				: props.t('conflictUseCandidate', { path: value }),
		value,
	}))
}

function emitConflictResolution(
	conflict: UpdateConflict,
	value: string | undefined,
): void {
	if (!value) return
	emit('selectConflictResolution', {
		operationId: conflict.operationId,
		value,
	})
}

function selectSource(value: string): void {
	sourceMenuOpen.value = false
	emit('selectSource', value)
}
</script>

<template>
	<section class="flex min-h-0 flex-1 items-center justify-center p-6">
		<div class="flex w-full max-w-lg flex-col items-center text-center">
			<Transition name="updater-phase" mode="out-in">
				<div
					:key="status.phase"
					class="flex w-full flex-col items-center text-center"
				>
					<UIcon
						:name="processIcon"
						class="size-14"
						:class="
							status.phase === 'failed'
								? 'text-danger-500'
								: status.phase === 'ready' || status.phase === 'up-to-date'
									? 'text-success-500'
									: 'text-slate-900/85 dark:text-white'
						"
					/>
					<UIcon
						v-if="showProcessSpinner"
						name="i-lucide-loader-circle"
						class="mt-3 size-5 animate-spin text-primary-500"
					/>
					<h1 class="mt-4 mx-1 text-xl text-slate-950 dark:text-white">
						{{ phaseTitle }}
					</h1>
					<p
						v-if="
							['updating', 'ready', 'unknown-client'].includes(status.phase) ||
							(phaseSubtitle && status.phase !== 'awaiting-update-decision')
						"
						class="mt-2 text-sm text-slate-600 dark:text-slate-300"
					>
						{{
							status.phase === 'updating'
								? t('bodyUpdating')
								: status.phase === 'ready'
									? t('bodyReady')
									: status.phase === 'unknown-client'
										? t('bodyUnknownClient')
										: phaseSubtitle
						}}
					</p>

					<div
						v-if="status.phase === 'updating' && downloadProgress"
						class="mt-6 w-full rounded-lg border border-slate-200 bg-white/80 p-4 text-left shadow-sm dark:border-slate-800 dark:bg-slate-900/70"
					>
						<div class="flex items-center justify-between gap-3 text-xs">
							<span class="font-medium text-slate-700 dark:text-slate-200">
								{{ t('downloadProgress') }}
							</span>
							<span class="tabular-nums text-slate-500 dark:text-slate-400">
								{{ downloadPercent.toFixed(1) }}%
							</span>
						</div>
						<div
							class="mt-2 h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800"
						>
							<div
								class="h-full rounded-full bg-primary-500 transition-[width]"
								:style="{ width: downloadPercent + '%' }"
							/>
						</div>
						<div class="mt-4 grid grid-cols-2 gap-3 text-xs">
							<div>
								<p class="text-slate-500 dark:text-slate-400">
									{{ t('downloadSize') }}
								</p>
								<p class="mt-1 font-medium tabular-nums">
									{{ formatBytes(downloadProgress.downloadedBytes) }} /
									{{ formatBytes(downloadProgress.totalBytes) }}
								</p>
							</div>
							<div>
								<p class="text-slate-500 dark:text-slate-400">
									{{ t('downloadSpeed') }}
								</p>
								<p class="mt-1 font-medium tabular-nums">
									{{ formatSpeed(downloadProgress.bytesPerSecond) }}
								</p>
							</div>
							<div>
								<p class="text-slate-500 dark:text-slate-400">
									{{ t('downloadLatency') }}
								</p>
								<p class="mt-1 font-medium tabular-nums">
									{{ downloadProgress.latencyMs }} ms
								</p>
							</div>
							<div>
								<p class="text-slate-500 dark:text-slate-400">
									{{ t('downloadSource') }}
								</p>
								<p class="mt-1 truncate font-medium">
									{{ downloadProgress.source }}
								</p>
							</div>
						</div>
						<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
							{{
								downloadProgress.resumed
									? t('downloadResumed')
									: t('downloadFresh')
							}}
						</p>
					</div>

					<div
						v-if="status.phase === 'awaiting-conflict-resolution'"
						class="mt-6 flex w-full flex-col gap-4 rounded-lg border border-slate-200 bg-white/80 p-4 text-left shadow-sm dark:border-slate-800 dark:bg-slate-900/70"
					>
						<div>
							<h2 class="text-sm font-semibold text-slate-900 dark:text-white">
								{{ t('managedConflictTitle') }}
							</h2>
							<p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
								{{ t('managedConflictDescription') }}
							</p>
						</div>
						<div
							v-for="conflict in conflicts"
							:key="conflict.operationId"
							class="rounded-md border border-slate-200 bg-slate-50/80 p-3 dark:border-slate-800 dark:bg-slate-950/40"
						>
							<p class="text-xs text-slate-500 dark:text-slate-400">
								{{ t('conflictTarget') }}
							</p>
							<p class="mt-1 break-all text-xs font-medium">
								{{ conflict.target }}
							</p>
							<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
								{{ t('conflictReason') }}
							</p>
							<p class="mt-1 text-xs">
								{{ conflict.reason }}
							</p>
							<USelect
								:model-value="
									conflictSelections[conflict.operationId] ||
									conflict.candidates[0] ||
									conflict.target
								"
								:items="conflictOptions(conflict)"
								class="mt-3 w-full"
								@update:model-value="emitConflictResolution(conflict, $event)"
							/>
						</div>
						<UButton
							color="primary"
							class="justify-center"
							:disabled="!conflicts.length"
							@click="emit('resolveConflicts')"
						>
							{{ t('confirmConflictResolutions') }}
						</UButton>
					</div>

					<div
						v-if="status.phase === 'awaiting-update-decision'"
						class="mt-4 flex w-full flex-col gap-4"
					>
						<div
							v-if="updateCurrentVersion && updateTargetVersion"
							class="flex items-center justify-center gap-3 text-sm"
						>
							<UBadge color="success" variant="soft" size="lg">
								{{ updateCurrentVersion }}
							</UBadge>
							<UIcon
								name="i-lucide-arrow-right"
								class="size-4 text-slate-400 dark:text-slate-500"
							/>
							<UBadge color="primary" variant="soft" size="lg">
								{{ updateTargetVersion }}
							</UBadge>
						</div>
						<div
							v-if="updateRelease?.changelog"
							class="overflow-hidden rounded-lg border border-slate-200 bg-slate-50 text-left dark:border-slate-800 dark:bg-slate-900"
						>
							<OverlayScrollbarsComponent
								class="h-36"
								:options="HYDCRAFT_SCROLLBAR_OPTIONS"
								defer
							>
								<MarkdownContent
									:content="updateRelease.changelog"
									class="px-4 py-3 text-xs leading-5 text-slate-700 dark:text-slate-200"
								/>
							</OverlayScrollbarsComponent>
						</div>
						<div class="flex">
							<UButton
								color="primary"
								class="w-full justify-center"
								@click="emit('beginUpdate')"
							>
								{{ t('updateNow') }}
							</UButton>
						</div>
						<div
							class="relative flex flex-col gap-1 items-center text-xs text-slate-500 dark:text-slate-400"
						>
							<div class="inline-flex items-center gap-0.5">
								<span>{{ t('currentDownloadSource') }}</span>
								<UPopover
									v-model:open="sourceMenuOpen"
									:popper="{ placement: 'bottom' }"
								>
									<UButton
										color="primary"
										variant="link"
										size="xs"
										class="h-auto p-0 font-medium"
									>
										{{ selectedSourceLabel }}
									</UButton>
									<template #content>
										<div class="flex w-40 flex-col gap-1 p-2">
											<UButton
												v-for="source in sourceItems"
												:key="source.value"
												type="button"
												color="neutral"
												variant="ghost"
												class="w-full justify-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800"
												:class="{
													'bg-primary-100/60 text-primary-600 dark:bg-primary-500/20 dark:text-primary-200':
														selectedSource === source.value,
													'text-slate-600 dark:text-slate-300':
														selectedSource !== source.value,
												}"
												@click="selectSource(source.value)"
											>
												<span>{{ source.label }}</span>
												<UBadge
													v-if="source.latencyMs != null"
													color="neutral"
													variant="soft"
													size="xs"
													class="ml-auto shrink-0"
												>
													{{ source.latencyMs }} ms
												</UBadge>
												<UIcon
													v-if="selectedSource === source.value"
													name="i-lucide-check"
													class="h-4 w-4"
													:class="source.latencyMs == null ? 'ml-auto' : 'ml-1'"
												/>
											</UButton>
										</div>
									</template>
								</UPopover>
							</div>
							<p v-if="!authenticated">
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
							<p v-else>{{ t('hydrolineLoggedIn') }}</p>
						</div>
					</div>

					<div
						v-if="status.phase === 'failed'"
						class="mt-6 flex w-full max-w-lg justify-center gap-3"
					>
						<UButton
							v-if="status.failureKind === 'update'"
							color="primary"
							variant="soft"
							class="flex-1 justify-center"
							@click="emit('recheckUpdate')"
						>
							{{ t('backToUpdateDecision') }}
						</UButton>
						<UButton
							color="primary"
							:class="
								status.failureKind === 'update'
									? 'flex-1 justify-center'
									: 'min-w-32 justify-center'
							"
							@click="emit('retryUpdate')"
						>
							{{ t('retryUpdate') }}
						</UButton>
					</div>
					<div
						v-if="['ready', 'up-to-date'].includes(status.phase) && isBootstrap"
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
					</div>
					<div
						v-if="
							status.phase === 'unknown-client' ||
							(status.phase === 'up-to-date' && !isBootstrap)
						"
						class="mt-6"
					>
						<UButton
							color="primary"
							variant="soft"
							class="min-w-32 justify-center"
							@click="emit('recheckUpdate')"
						>
							{{ t('retryUpdate') }}
						</UButton>
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
			</Transition>
		</div>
	</section>
</template>
