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
</script>

<template>
	<header
		:class="[
			$props.inline ? 'relative' : 'absolute',
			'top-2 right-0 left-0 z-30 flex h-10 items-center select-none',
			$props.class,
		]"
	>
		<div class="flex shrink-0 items-center pl-5">
			<slot name="left" />
		</div>
		<div
			class="h-full min-w-0 flex-1"
			@mousedown.left="startCurrentWindowDragging"
		/>
		<div class="flex shrink-0 items-center pr-1">
			<UButton
				color="neutral"
				variant="link"
				class="size-10 p-0 opacity-55 transition hover:opacity-100"
				:aria-label="minimizeLabel"
				@click="minimize"
			>
				<UIcon name="i-lucide-minus" class="size-5" />
			</UButton>
			<UButton
				color="neutral"
				variant="link"
				class="size-10 p-0 opacity-55 transition hover:opacity-100"
				:aria-label="closeLabel"
				@click="close"
			>
				<UIcon name="i-lucide-x" class="size-5" />
			</UButton>
		</div>
	</header>
</template>
