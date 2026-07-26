<script setup lang="ts">
import { useWebviewInteraction } from '../../composables/useWebviewInteraction'

const { contextMenuItems } = useWebviewInteraction()
</script>

<template>
	<UContextMenu :items="contextMenuItems">
		<slot />
	</UContextMenu>
</template>
