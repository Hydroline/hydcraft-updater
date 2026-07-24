<script setup lang="ts">
import type { Translator } from '../../types/updater'

defineProps<{
	t: Translator
}>()
</script>

<template>
	<section
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
</template>
