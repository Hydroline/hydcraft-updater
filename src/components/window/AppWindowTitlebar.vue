<script setup lang="ts">
import {
	closeCurrentWindow,
	hideAuthenticationWindow,
	hideVersionWindow,
	minimizeCurrentWindow,
	startCurrentWindowDragging,
} from '../../utils/tauri'

const props = withDefaults(
	defineProps<{
		closeLabel: string
		minimizeLabel: string
		closeMode?: 'close' | 'hide-auth' | 'hide-version'
		inline?: boolean
		class?: string
	}>(),
	{ closeMode: 'close', inline: false, class: '' },
)

async function close(): Promise<void> {
	if (props.closeMode === 'hide-auth') await hideAuthenticationWindow()
	else if (props.closeMode === 'hide-version') await hideVersionWindow()
	else await closeCurrentWindow()
}

async function minimize(): Promise<void> {
	await minimizeCurrentWindow()
}

function handleWindowDrag(event: MouseEvent): void {
	const target = event.target as HTMLElement | null
	if (
		target?.closest(
			'button, a, input, select, textarea, [role="button"], [data-no-window-drag]',
		)
	)
		return
	void startCurrentWindowDragging()
}
</script>

<template>
	<header
		:class="[
			$props.inline ? 'relative top-0 h-12' : 'absolute top-2 h-10',
			'right-0 left-0 z-30 flex items-center select-none',
			$props.class,
		]"
		@mousedown.left="handleWindowDrag"
	>
		<div class="flex shrink-0 items-center pl-5">
			<slot name="left" />
		</div>
		<div class="h-full min-w-0 flex-1" />
		<div class="flex shrink-0 items-center pr-1">
			<UButton
				variant="link"
				class="size-10 p-0 opacity-50 transition hover:opacity-100"
				:aria-label="minimizeLabel"
				@mousedown.stop
				@click="minimize"
			>
				<UIcon
					name="i-lucide-minus"
					class="size-5 text-slate-800 dark:text-slate-100"
				/>
			</UButton>
			<UButton
				variant="link"
				class="size-10 p-0 opacity-50 transition hover:opacity-100"
				:aria-label="closeLabel"
				@mousedown.stop
				@click="close"
			>
				<UIcon
					name="i-lucide-x"
					class="size-5 text-rose-600 dark:text-rose-500"
				/>
			</UButton>
		</div>
	</header>
</template>
