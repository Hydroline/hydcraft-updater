<script setup lang="ts">
import type { ClientVersionOption, Translator } from '../../types/updater'

defineProps<{
	clientVersions: ClientVersionOption[]
	t: Translator
}>()
</script>

<template>
	<section
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
</template>
