<script setup lang="ts">
import { ref } from 'vue'

interface SkeletonImageProps {
	src: string
	alt: string
	imageClass?: string
}

withDefaults(defineProps<SkeletonImageProps>(), { imageClass: '' })
const ready = ref(false)
</script>

<template>
	<div class="relative" v-bind="$attrs">
		<USkeleton v-if="!ready" class="absolute inset-0 rounded-full" />
		<img
			:src="src"
			:alt="alt"
			:class="[
				'transition-opacity duration-300',
				imageClass,
				ready ? 'opacity-100' : 'opacity-0',
			]"
			@load="ready = true"
			@error="ready = true"
		/>
	</div>
</template>
