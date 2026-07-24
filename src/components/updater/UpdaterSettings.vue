<script setup lang="ts">
import type {
	SelectOption,
	Translator,
	UpdaterContext,
} from '../../types/updater'

defineProps<{
	context: UpdaterContext
	isBootstrap: boolean
	phaseTitle: string
	selectedSource: string
	sourceItems: SelectOption[]
	t: Translator
}>()

const emit = defineEmits<{
	selectSource: [value: string | undefined]
}>()
</script>

<template>
	<section class="flex flex-1 items-start justify-center px-6 pb-8 pt-16">
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
</template>
