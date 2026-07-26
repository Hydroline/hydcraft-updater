<script setup lang="ts">
import { ref, watch } from 'vue'

import type {
	SelectOption,
	DownloadSource,
	ClientStorageInfo,
	Translator,
	UpdaterContext,
} from '../../types/updater'

const props = defineProps<{
	context: UpdaterContext
	isBootstrap: boolean
	phaseTitle: string
	selectedSource: string
	sourceItems: SelectOption[]
	sources: DownloadSource[]
	sourceTesting: boolean
	storageInfo: ClientStorageInfo
	cleanDownloadsAfterInstall: boolean
	downloadsCleaning: boolean
	backupsCleaning: boolean
	downloadsCleanupVersion: number
	backupsCleanupVersion: number
	t: Translator
}>()

const downloadsPopoverOpen = ref(false)
const backupsPopoverOpen = ref(false)

watch(
	() => props.downloadsCleanupVersion,
	(version, previousVersion) => {
		if (version !== previousVersion) downloadsPopoverOpen.value = false
	},
)
watch(
	() => props.backupsCleanupVersion,
	(version, previousVersion) => {
		if (version !== previousVersion) backupsPopoverOpen.value = false
	},
)

const emit = defineEmits<{
	selectSource: [value: string | undefined]
	setCleanDownloadsAfterInstall: [value: boolean]
	cleanDownloads: []
	cleanBackups: []
	refreshSources: []
}>()

function formatBytes(value: number): string {
	if (value < 1024) return `${value} B`
	const units = ['KB', 'MB', 'GB', 'TB']
	let size = value
	let index = -1
	while (size >= 1024 && index < units.length - 1) {
		size /= 1024
		index += 1
	}
	return `${size.toFixed(size >= 10 ? 0 : 1)} ${units[index]}`
}
</script>

<template>
	<section class="flex flex-1 items-start justify-center px-6 pb-8 pt-6">
		<div class="w-full space-y-6">
			<section class="grid gap-1.5">
				<div class="mx-1 text-xl text-slate-950 dark:text-white">
					{{ t('statusSettingsTitle') }}
				</div>
				<div
					class="divide-y divide-slate-200 rounded-lg border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900"
				>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ t('mode') }}
						</span>
						<span
							class="min-w-0 w-full text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ isBootstrap ? t('modeBootstrap') : t('modeManual') }}
						</span>
					</div>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ t('currentPhase') }}
						</span>
						<span
							class="min-w-0 w-full text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ phaseTitle }}
						</span>
					</div>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ t('clientDirectory') }}
						</span>
						<span
							class="min-w-0 w-full break-all text-sm text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ context.gameDir || t('notRead') }}
						</span>
					</div>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ t('consoleOrigin') }}
						</span>
						<span
							class="min-w-0 w-full break-all text-sm text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ context.consoleOrigin || t('notRead') }}
						</span>
					</div>
				</div>
			</section>

			<section class="grid gap-1.5">
				<div class="mx-1 text-xl text-slate-950 dark:text-white">
					{{ t('downloadSettingsTitle') }}
				</div>
				<div
					class="divide-y divide-slate-200 rounded-lg border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900"
				>
					<label class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ t('defaultDownloadSource') }}
						</span>
						<span class="min-w-0 w-full md:pt-1.5">
							<USelect
								:model-value="selectedSource"
								:items="sourceItems"
								class="w-full text-sm"
								@update:model-value="emit('selectSource', $event)"
							/>
						</span>
					</label>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200 leading-[normal]"
						>
							{{ t('cleanDownloadsAfterInstall') }}
						</span>
						<USwitch
							class="md:pt-1.5"
							:model-value="cleanDownloadsAfterInstall"
							@update:model-value="
								emit('setCleanDownloadsAfterInstall', $event)
							"
						/>
					</div>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ t('downloadsStorage') }}
						</span>
						<div class="flex items-center gap-3 md:pt-1.5">
							<span
								class="text-sm tabular-nums text-slate-700 dark:text-slate-200"
							>
								{{ formatBytes(storageInfo.downloadsBytes) }}
							</span>
							<UPopover v-model:open="downloadsPopoverOpen">
								<UButton
									color="error"
									variant="soft"
									size="sm"
									:disabled="downloadsCleaning"
								>
									{{ t('clearDownloads') }}
								</UButton>
								<template #content>
									<div class="flex w-64 flex-col gap-3 p-3">
										<p
											class="text-sm leading-5 text-slate-700 dark:text-slate-200"
										>
											{{ t('clearDownloadsConfirm') }}
										</p>
										<UButton
											color="error"
											class="justify-center"
											:loading="downloadsCleaning"
											:disabled="downloadsCleaning"
											@click="emit('cleanDownloads')"
										>
											{{ t('clearConfirm') }}
										</UButton>
									</div>
								</template>
							</UPopover>
						</div>
					</div>
					<div class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ t('backupsStorage') }}
						</span>
						<div class="flex items-center gap-3 md:pt-1.5">
							<span
								class="text-sm tabular-nums text-slate-700 dark:text-slate-200"
							>
								{{ formatBytes(storageInfo.backupsBytes) }}
							</span>
							<UPopover v-model:open="backupsPopoverOpen">
								<UButton
									color="error"
									variant="soft"
									size="sm"
									:disabled="backupsCleaning"
								>
									{{ t('clearBackups') }}
								</UButton>
								<template #content>
									<div class="flex w-64 flex-col gap-3 p-3">
										<p
											class="text-sm leading-5 text-slate-700 dark:text-slate-200"
										>
											{{ t('clearBackupsConfirm') }}
										</p>
										<UButton
											color="error"
											class="justify-center"
											:loading="backupsCleaning"
											:disabled="backupsCleaning"
											@click="emit('cleanBackups')"
										>
											{{ t('clearConfirm') }}
										</UButton>
									</div>
								</template>
							</UPopover>
						</div>
					</div>
				</div>
			</section>

			<section class="grid gap-1.5">
				<div class="mx-1 flex items-center gap-1.5">
					<div class="text-xl text-slate-950 dark:text-white">
						{{ t('sourceSpeedTestTitle') }}
					</div>
					<UButton
						color="primary"
						variant="link"
						size="xs"
						icon="i-lucide-gauge"
						:loading="sourceTesting"
						:disabled="sourceTesting"
						@click="emit('refreshSources')"
						>{{ t('sourceTest') }}</UButton
					>
				</div>
				<Transition name="source-list">
					<div
						v-if="sources.length"
						class="divide-y divide-slate-200 rounded-lg border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900"
					>
						<div
							v-for="source in sources"
							:key="source.key"
							class="grid gap-2 p-4 md:grid-cols-[180px_1fr] md:items-start"
						>
							<div class="min-w-0 md:pt-1.5">
								<p
									class="truncate text-[15px] text-slate-700 dark:text-slate-200"
								>
									{{ source.label }}
								</p>
							</div>
							<div class="flex items-center gap-2 md:pt-1.5">
								<span
									class="text-sm tabular-nums text-slate-600 dark:text-slate-300"
								>
									{{
										source.latencyMs == null
											? t('sourceLatencyUnknown')
											: t('sourceLatency', { value: source.latencyMs })
									}}
								</span>
								<UBadge
									:color="source.available ? 'success' : 'neutral'"
									variant="soft"
								>
									{{
										source.available
											? t('sourceAvailable')
											: t('sourceUnavailable')
									}}
								</UBadge>
							</div>
						</div>
					</div>
				</Transition>
			</section>
		</div>
	</section>
</template>

<style scoped>
.source-list-enter-active,
.source-list-leave-active {
	overflow: hidden;
	transition:
		max-height 180ms ease,
		opacity 180ms ease;
}

.source-list-enter-from,
.source-list-leave-to {
	max-height: 0;
	opacity: 0;
}

.source-list-enter-to,
.source-list-leave-from {
	max-height: 480px;
	opacity: 1;
}
</style>
