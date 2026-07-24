<template>
	<div class="flex h-6 items-center gap-2">
		<SkeletonImage
			:src="hydcraftLogo"
			alt="氢气工艺"
			image-class="size-6 object-contain"
			class="size-6 shrink-0"
		/>
		<span
			class="font-arkpixel text-[17px] leading-none tracking-wide text-slate-800 dark:text-slate-100"
		>
			氢气工艺
		</span>
	</div>
</template>

<script setup lang="ts">
import hydcraftLogo from '../../assets/resources/brands/logo_HydCraft.png'
import SkeletonImage from '../common/SkeletonImage.vue'
</script>
