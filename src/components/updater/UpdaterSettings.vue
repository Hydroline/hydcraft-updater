<script setup lang="ts">
import type {
	SelectOption,
	DownloadSource,
	Translator,
	UpdaterContext,
} from '../../types/updater'

defineProps<{
	context: UpdaterContext
	isBootstrap: boolean
	phaseTitle: string
	selectedSource: string
	sourceItems: SelectOption[]
	sources: DownloadSource[]
	sourceTesting: boolean
	t: Translator
}>()

const emit = defineEmits<{
	selectSource: [value: string | undefined]
	refreshSources: []
}>()
</script>

<template>
	<section class="flex flex-1 items-start justify-center px-6 pb-8 pt-6">
		<div class="w-full space-y-6">
			<section class="grid gap-1.5">
				<div class="mx-1 text-xl text-slate-950 dark:text-white">
					{{ t('statusSettingsTitle') }}
				</div>
				<div
					class="grid gap-4 rounded-lg border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900"
				>
					<div class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
						>
							{{ t('mode') }}
						</span>
						<span
							class="min-w-0 w-full text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
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
							class="min-w-0 w-full text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
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
					<div class="grid gap-2 md:grid-cols-[180px_1fr] md:items-start">
						<span
							class="text-[15px] text-slate-700 md:pt-1.5 dark:text-slate-200"
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
					class="grid gap-4 rounded-lg border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900"
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

			<section class="grid gap-1.5">
				<div class="mx-1 flex items-center justify-between">
					<div class="text-xl text-slate-950 dark:text-white">
						{{ t('sourceSpeedTestTitle') }}
					</div>
					<UButton
						color="primary"
						variant="link"
						icon="i-lucide-gauge"
						:loading="sourceTesting"
						:disabled="sourceTesting"
						@click="emit('refreshSources')"
						>{{ t('sourceTest') }}</UButton
					>
				</div>
				<div
					class="divide-y divide-slate-200 rounded-lg border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900"
				>
					<div
						v-for="source in sources"
						:key="source.key"
						class="flex items-center justify-between gap-4 p-4"
					>
						<div class="min-w-0">
							<p
								class="truncate text-[15px] text-slate-700 dark:text-slate-200"
							>
								{{ source.label }}
							</p>
						</div>
						<div class="flex shrink-0 items-center gap-2">
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
			</section>
		</div>
	</section>
</template>
