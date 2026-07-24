<script setup lang="ts">
import type { TabKey } from '../../types/updater'

defineProps<{
	tab: TabKey
	tabs: readonly { key: TabKey; label: string }[]
}>()

const emit = defineEmits<{ select: [tab: TabKey] }>()
</script>

<template>
	<div class="flex items-center gap-3">
		<button
			v-for="item in tabs"
			:key="item.key"
			type="button"
			class="group relative z-0 rounded-full p-2 text-[16px] leading-none whitespace-nowrap transition-all duration-[420ms] ease-[cubic-bezier(0.22,1,0.36,1)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
			:class="
				tab === item.key
					? 'font-semibold text-primary opacity-100 dark:text-[rgb(125,211,252)]'
					: 'text-slate-800 opacity-85 hover:text-slate-800 hover:opacity-100 dark:text-slate-300 dark:opacity-75 dark:hover:text-slate-100'
			"
			:aria-current="tab === item.key ? 'page' : undefined"
			@click="emit('select', item.key)"
		>
			<span
				class="pointer-events-none absolute -inset-x-1 inset-y-1 -z-10 rounded-full bg-[rgba(125,211,252,0.16)] opacity-0 shadow-[0_0_10px_rgba(125,211,252,0.12)] transition-all duration-[420ms] ease-[cubic-bezier(0.22,1,0.36,1)]"
				:class="{ 'opacity-100': tab === item.key }"
				aria-hidden="true"
			/>
			<span
				class="pointer-events-none absolute -inset-x-1 inset-y-1 -z-10 rounded-full bg-slate-200/70 opacity-0 shadow-[0_0_10px_rgba(100,116,139,0.08)] transition-all duration-[420ms] ease-[cubic-bezier(0.22,1,0.36,1)] dark:bg-slate-700/50 dark:shadow-[0_0_10px_rgba(148,163,184,0.08)]"
				:class="{ 'group-hover:opacity-100': tab !== item.key }"
				aria-hidden="true"
			/>
			{{ item.label }}
		</button>
	</div>
</template>
