<script setup lang="ts">
import { computed, ref } from 'vue'
import dayjs from 'dayjs'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-vue'
import MarkdownContent from '../common/MarkdownContent.vue'
import SkeletonImage from '../common/SkeletonImage.vue'
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

const SKIP_RESOLUTION = '__hydcraft_skip__'

const emit = defineEmits<{
	beginUpdate: []
	cancelConflictResolution: []
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
const isCacheVerification = computed(
	() => downloadProgress.value?.source === 'cache',
)
const operationPercent = computed(() => {
	const operation = props.status.operation
	if (!operation?.totalItems) return null
	return Math.min(
		100,
		Math.max(0, ((operation.completedItems ?? 0) / operation.totalItems) * 100),
	)
})

const operationLabel = computed(() => {
	switch (props.status.operation?.stage) {
		case 'checking':
			return props.t('operationChecking')
		case 'verifying':
			return props.t('operationVerifying')
		case 'extracting':
			return props.t('operationExtracting')
		case 'backing-up':
			return props.t('operationBackingUp')
		case 'applying':
			return props.t('operationApplying')
		default:
			return ''
	}
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

function formatReleaseDate(value: string | null): string | null {
	if (!value || !dayjs(value).isValid()) return null
	return dayjs(value).format('YYYY 年 M 月 D 日')
}

function formatRelativeReleaseDate(value: string | null): string | null {
	if (!value || !dayjs(value).isValid()) return null
	const date = dayjs(value)
	const now = dayjs()
	const years = now.diff(date, 'year')
	const afterYears = date.add(years, 'year')
	const months = now.diff(afterYears, 'month')
	const days = Math.max(0, now.diff(afterYears.add(months, 'month'), 'day'))
	return props.t('releaseRelativeDate', {
		years: years ? props.t('releaseYears', { value: years }) : '',
		months: months ? props.t('releaseMonths', { value: months }) : '',
		days: props.t('releaseDays', { value: days }),
	})
}

function environmentTags(option: ClientVersionOption): string[] {
	return (option.apiVersion ?? '')
		.split('/')
		.map((item, index) =>
			index === 0 ? item.trim().replace(/^Minecraft\s+/i, '') : item.trim(),
		)
		.filter(Boolean)
}

function conflictOptions(conflict: UpdateConflict): SelectOption[] {
	const values = Array.from(
		new Set([conflict.target, ...conflict.candidates].filter(Boolean)),
	)
	return [
		...values.map((value) => ({
			label:
				value === conflict.target
					? targetOptionLabel(conflict)
					: props.t('conflictUseCandidate', { file: displayName(value) }),
			value,
		})),
		{ label: props.t('conflictKeepLocal'), value: SKIP_RESOLUTION },
	]
}

function displayName(path: string): string {
	return path.replace(/\\/g, '/').split('/').filter(Boolean).at(-1) ?? path
}

function targetOptionLabel(conflict: UpdateConflict): string {
	const file = displayName(conflict.target)
	switch (conflict.targetAction) {
		case 'overwrite':
			return props.t('conflictOverwriteTarget', { file })
		case 'install':
			return props.t('conflictInstallTarget', { file })
		case 'delete':
			return props.t('conflictDeleteTarget', { file })
		case 'acknowledgeMissing':
			return props.t('conflictAcknowledgeMissing', { file })
		case 'confirm':
			return props.t('conflictConfirmAnchor', { file })
		default:
			return props.t('conflictApplyTarget', { file })
	}
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
	const source = props.sourceItems.find((item) => item.value === value)
	if (!source || source.disabled) return

	sourceMenuOpen.value = false
	emit('selectSource', value)
}
</script>

<template>
	<section
		class="flex min-h-0 flex-1 items-center justify-center overflow-x-hidden p-6"
	>
		<div
			class="my-auto flex min-w-0 w-full max-w-lg flex-col items-center text-center"
		>
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
					<div
						v-if="
							status.phase === 'awaiting-update-decision' &&
							status.testRevision != null
						"
						class="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950 dark:border-amber-900/70 dark:bg-amber-950/30 dark:text-amber-100"
					>
						<p class="font-medium">
							{{
								t('testCandidateRevision', { revision: status.testRevision })
							}}
						</p>
					</div>
					<p
						v-if="
							[
								'updating',
								'ready',
								'unknown-client',
								'partial-update',
							].includes(status.phase) ||
							(phaseSubtitle && status.phase !== 'awaiting-update-decision')
						"
						class="mt-2 max-w-full overflow-hidden break-all text-sm text-slate-600 dark:text-slate-300"
					>
						{{
							status.phase === 'updating'
								? t('bodyUpdating')
								: status.phase === 'ready'
									? t('bodyReady')
									: status.phase === 'unknown-client'
										? t('bodyUnknownClient')
										: status.phase === 'partial-update'
											? t('bodyPartialUpdate')
											: phaseSubtitle
						}}
					</p>

					<Transition name="progress-panel" mode="out-in">
						<div
							v-if="
								status.phase === 'updating' &&
								downloadProgress &&
								!isCacheVerification
							"
							key="download"
							class="mt-6 w-full rounded-lg border border-slate-200 bg-white/80 p-4 text-left dark:border-slate-800 dark:bg-slate-900/70"
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
									<p
										class="mt-1 truncate font-medium"
										:title="
											downloadProgress.sourceUrl ?? downloadProgress.source
										"
									>
										{{ downloadProgress.source
										}}{{
											downloadProgress.sourceUrl
												? ` (${downloadProgress.sourceUrl})`
												: ''
										}}
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
							v-else-if="
								status.phase === 'updating' &&
								isCacheVerification &&
								downloadProgress
							"
							key="cache"
							class="mt-6 w-full rounded-lg border border-slate-200 bg-white/80 p-4 text-left dark:border-slate-800 dark:bg-slate-900/70"
						>
							<div class="flex items-center justify-between gap-3 text-xs">
								<span class="font-medium text-slate-700 dark:text-slate-200">{{
									t('cacheVerificationProgress')
								}}</span
								><span class="tabular-nums text-slate-500 dark:text-slate-400"
									>{{ downloadPercent.toFixed(1) }}%</span
								>
							</div>
							<div
								class="mt-2 h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800"
							>
								<div
									class="h-full rounded-full bg-primary-500 transition-[width]"
									:style="{ width: downloadPercent + '%' }"
								/>
							</div>
							<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
								{{
									t('cacheVerifiedSize', {
										completed: formatBytes(downloadProgress.downloadedBytes),
										total: formatBytes(downloadProgress.totalBytes),
									})
								}}
							</p>
						</div>
						<div
							v-else-if="status.phase === 'updating' && status.operation"
							key="operation"
							class="mt-6 w-full rounded-lg border border-slate-200 bg-white/80 p-4 text-left dark:border-slate-800 dark:bg-slate-900/70"
						>
							<div class="flex items-center justify-between gap-3 text-xs">
								<span class="font-medium text-slate-700 dark:text-slate-200">{{
									operationLabel
								}}</span
								><span
									v-if="operationPercent != null"
									class="tabular-nums text-slate-500 dark:text-slate-400"
									>{{ operationPercent.toFixed(1) }}%</span
								>
							</div>
							<div
								v-if="operationPercent != null"
								class="mt-2 h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800"
							>
								<div
									class="h-full rounded-full bg-primary-500 transition-[width]"
									:style="{ width: operationPercent + '%' }"
								/>
							</div>
							<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
								{{
									status.operation.totalItems != null
										? status.operation.stage === 'verifying'
											? t('cacheVerifiedSize', {
													completed: formatBytes(
														status.operation.completedItems ?? 0,
													),
													total: formatBytes(status.operation.totalItems),
												})
											: t('operationItems', {
													completed: status.operation.completedItems ?? 0,
													total: status.operation.totalItems,
												})
										: t('operationInProgress')
								}}
							</p>
						</div>
					</Transition>
					<div
						v-if="status.operation"
						class="hidden mt-6 w-full rounded-lg border border-slate-200 bg-white/80 p-4 text-left dark:border-slate-800 dark:bg-slate-900/70"
					>
						<div class="flex items-center justify-between gap-3 text-xs">
							<span class="font-medium text-slate-700 dark:text-slate-200">
								{{ operationLabel }}
							</span>
							<span
								v-if="operationPercent != null"
								class="tabular-nums text-slate-500 dark:text-slate-400"
							>
								{{ operationPercent.toFixed(1) }}%
							</span>
						</div>
						<div
							v-if="operationPercent != null"
							class="mt-2 h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800"
						>
							<div
								class="h-full rounded-full bg-primary-500 transition-[width]"
								:style="{ width: operationPercent + '%' }"
							/>
						</div>
						<div
							v-else
							class="mt-2 h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800"
						>
							<div
								class="updater-indeterminate h-full w-2/5 rounded-full bg-primary-500"
							/>
						</div>
						<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
							{{
								status.operation.totalItems != null
									? status.operation.stage === 'verifying'
										? t('cacheVerifiedSize', {
												completed: formatBytes(
													status.operation.completedItems ?? 0,
												),
												total: formatBytes(status.operation.totalItems),
											})
										: t('operationItems', {
												completed: status.operation.completedItems ?? 0,
												total: status.operation.totalItems,
											})
									: t('operationInProgress')
							}}
						</p>
					</div>

					<div
						v-if="status.phase === 'awaiting-conflict-resolution'"
						class="mt-6 flex w-full flex-col gap-4 rounded-lg border border-slate-200 bg-white/80 p-4 text-left dark:border-slate-800 dark:bg-slate-900/70"
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
							<p class="mt-1 break-all text-xs">
								{{ conflict.target }}
							</p>
							<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
								{{ t('conflictOperation') }}
							</p>
							<p class="mt-1 text-xs">{{ conflict.operationType }}</p>
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
						<div class="grid grid-cols-2 gap-3">
							<UButton
								color="neutral"
								variant="soft"
								class="justify-center"
								@click="emit('cancelConflictResolution')"
							>
								{{ t('cancelUpdate') }}
							</UButton>
							<UButton
								color="primary"
								class="justify-center"
								:disabled="!conflicts.length"
								@click="emit('resolveConflicts')"
							>
								{{ t('confirmConflictResolutions') }}
							</UButton>
						</div>
					</div>

					<div
						v-if="status.phase === 'awaiting-update-decision'"
						class="mt-4 flex w-full flex-col gap-4"
					>
						<div
							v-if="updateCurrentVersion && updateTargetVersion"
							class="flex items-center justify-center gap-3 text-sm"
						>
							<UBadge color="warning" variant="soft" size="lg">
								{{ updateCurrentVersion }}
							</UBadge>
							<UIcon
								name="i-lucide-arrow-right"
								class="size-4 text-slate-400 dark:text-slate-500"
							/>
							<UBadge color="success" variant="soft" size="lg">
								{{ updateTargetVersion }}
							</UBadge>
						</div>
						<OverlayScrollbarsComponent
							v-if="updateRelease"
							class="h-44 overflow-hidden rounded-lg border border-slate-200 bg-slate-50 text-left dark:border-slate-800 dark:bg-slate-900"
							:options="HYDCRAFT_SCROLLBAR_OPTIONS"
							defer
						>
							<div
								class="flex flex-wrap items-center gap-1.5 px-4 py-3 text-xs text-slate-600 dark:text-slate-300"
							>
								<span v-if="formatReleaseDate(updateRelease.publishedAt)">
									{{
										t('clientReleaseMeta', {
											version: updateRelease.version,
											relative: formatRelativeReleaseDate(
												updateRelease.publishedAt,
											)!,
											date: formatReleaseDate(updateRelease.publishedAt)!,
										})
									}}
								</span>
							</div>
							<div v-if="updateRelease.changelog">
								<MarkdownContent
									:content="updateRelease.changelog"
									class="px-4 pb-3 text-xs leading-5 text-slate-700 dark:text-slate-200"
								/>
							</div>
							<div class="px-4 pb-4 text-xs text-slate-500 dark:text-slate-400">
								<div
									v-if="updateRelease.publisher"
									class="flex items-center gap-1.5"
								>
									<SkeletonImage
										v-if="updateRelease.publisher.avatarUrl"
										:src="updateRelease.publisher.avatarUrl"
										:alt="
											updateRelease.publisher.displayName ||
											updateRelease.publisher.username
										"
										image-class="size-5 rounded-full object-cover"
										class="size-5 shrink-0"
									/>
									<span>{{
										t('clientPublishedByAt', {
											username:
												updateRelease.publisher.displayName ||
												updateRelease.publisher.username,
											date: formatReleaseDate(updateRelease.publishedAt) || '',
										})
									}}</span>
								</div>
								<div
									v-if="updateRelease.contributors?.length"
									class="mt-2 flex items-center gap-1.5"
								>
									<div class="flex shrink-0">
										<SkeletonImage
											v-for="(contributor, index) in updateRelease.contributors"
											:key="contributor.hydrolineId"
											:src="contributor.avatarUrl || ''"
											:alt="contributor.displayName || contributor.username"
											image-class="size-5 rounded-full object-cover dark:border-slate-900"
											:class="index === 0 ? 'size-5' : '-ml-2 size-5'"
										/>
									</div>
									<span
										>{{
											updateRelease.contributors[0]?.displayName ||
											updateRelease.contributors[0]?.username
										}}{{
											t('clientContributors', {
												username: '',
												count: updateRelease.contributors.length,
											})
										}}</span
									>
								</div>
							</div>
						</OverlayScrollbarsComponent>
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
							v-if="sourceItems.length"
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
										<div class="flex w-44 flex-col gap-1 p-2">
											<UButton
												v-for="source in sourceItems"
												:key="source.value"
												type="button"
												color="neutral"
												variant="ghost"
												:disabled="source.disabled"
												class="w-full justify-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800"
												:class="{
													'bg-primary-100/60 text-primary-600 dark:bg-primary-500/20 dark:text-primary-200':
														selectedSource === source.value,
													'text-slate-600 dark:text-slate-300':
														selectedSource !== source.value,
													'cursor-not-allowed opacity-50': source.disabled,
												}"
												@click="selectSource(source.value)"
											>
												<span class="min-w-0 truncate">{{ source.label }}</span>
												<UBadge
													color="neutral"
													variant="soft"
													size="xs"
													class="shrink-0"
												>
													{{
														source.latencyMs == null
															? t('sourceLatencyUnknown')
															: t('sourceLatency', { value: source.latencyMs })
													}}
												</UBadge>
												<UIcon
													v-if="selectedSource === source.value"
													name="i-lucide-check"
													class="ml-auto h-4 w-4"
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
							<p
								v-if="status.remainingSeconds != null"
								class="absolute top-full mt-2 w-full text-center tabular-nums"
							>
								{{
									t('automaticUpdateCountdown', {
										seconds: status.remainingSeconds,
									})
								}}
							</p>
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
						v-if="status.phase === 'ready'"
						class="relative mt-6 flex w-full justify-center gap-3"
					>
						<UButton
							color="primary"
							variant="soft"
							class="min-w-36 justify-center"
							@click="emit('recheckUpdate')"
						>
							{{ t('continueCheckingUpdates') }}
						</UButton>
						<UButton
							v-if="isBootstrap"
							color="primary"
							class="min-w-36 justify-center"
							@click="emit('launchClient')"
						>
							{{ t('launchNow') }}
						</UButton>
						<p
							v-if="status.remainingSeconds != null"
							class="absolute top-full mt-2 w-full text-center text-xs tabular-nums text-slate-500 dark:text-slate-400"
						>
							{{
								t('automaticLaunchCountdown', {
									seconds: status.remainingSeconds,
								})
							}}
						</p>
					</div>
					<div
						v-if="status.phase === 'up-to-date' && isBootstrap"
						class="relative mt-6 flex w-full justify-center"
					>
						<UButton
							color="primary"
							class="min-w-36 justify-center"
							@click="emit('launchClient')"
						>
							{{ t('launchNow') }}
						</UButton>
						<p
							v-if="status.remainingSeconds != null"
							class="absolute top-full mt-2 w-full text-center text-xs tabular-nums text-slate-500 dark:text-slate-400"
						>
							{{
								t('automaticLaunchCountdown', {
									seconds: status.remainingSeconds,
								})
							}}
						</p>
					</div>
					<div v-if="status.phase === 'unknown-client'" class="mt-6">
						<UButton
							color="primary"
							variant="soft"
							class="min-w-32 justify-center"
							@click="emit('recheckUpdate')"
						>
							{{ t('retryUpdate') }}
						</UButton>
					</div>
					<div v-if="status.phase === 'partial-update'" class="mt-6">
						<UButton
							color="warning"
							variant="soft"
							class="min-w-44 justify-center"
							@click="emit('retryUpdate')"
						>
							{{ t('resumeConflictHandling') }}
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

<style scoped>
.updater-indeterminate {
	animation: updater-indeterminate 1.2s ease-in-out infinite;
}

@keyframes updater-indeterminate {
	from {
		transform: translateX(-120%);
	}
	to {
		transform: translateX(300%);
	}
}

.progress-panel-enter-active,
.progress-panel-leave-active {
	overflow: hidden;
	transition:
		height 180ms ease,
		opacity 180ms ease;
}

.progress-panel-enter-from,
.progress-panel-leave-to {
	height: 0;
	opacity: 0;
}
</style>
