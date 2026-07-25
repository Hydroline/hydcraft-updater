<script setup lang="ts">
import type { Translator } from '../../types/updater'

defineProps<{
	t: Translator
}>()
</script>

<template>
	<section class="flex flex-1 items-center justify-center p-6 text-center">
		<div class="flex max-w-sm flex-col items-center">
			<UIcon
				name="i-lucide-package"
				class="size-12 text-slate-900/85 dark:text-white"
			/>
			<h2 class="mt-4 mx-1 text-xl text-slate-950 dark:text-white">
				{{ t('addonsTitle') }}
			</h2>
			<p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400">
				{{ t('noData') }}
			</p>
		</div>
	</section>
</template>
