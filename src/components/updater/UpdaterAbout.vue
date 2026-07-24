<script setup lang="ts">
import AurLemon from '../branding/AurLemon.vue'
import HydCraft from '../branding/HydCraft.vue'
import type { Translator } from '../../types/updater'

defineProps<{
	t: Translator
}>()

const emit = defineEmits<{
	openExternalUrl: [url: string]
}>()
</script>

<template>
	<section
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
</template>
