<template>
	<div class="relative w-fit">
		<div class="aurlemon-branding-row relative z-1">
			<SkeletonImage
				:src="avatarMark"
				alt="AurLemon avatar mark"
				image-class="block h-[1.875rem] w-[1.875rem] max-w-none shrink-0 rounded-full border-[1.5px] border-[#236f95] select-none"
				class="aurlemon-avatar-mark-wrapper"
			/>
			<AurLemonTextMark
				class="aurlemon-text-mark text-[#236F95] dark:text-[#54aed7]"
			/>
		</div>
		<div
			class="absolute -left-1 -right-2 bottom-0.5 z-0 h-2.5 bg-[linear-gradient(to_right,transparent,rgba(55,145,190,0.1)_15%,rgba(55,145,190,0.15)_50%,rgba(55,145,190,0.1)_85%,transparent)] dark:bg-[linear-gradient(to_right,transparent,rgba(55,145,190,0.15)_15%,rgba(55,145,190,0.3)_50%,rgba(55,145,190,0.15)_85%,transparent)]"
		/>
	</div>
</template>

<script setup lang="ts">
import avatarMark from '../../assets/resources/sitemark/aurlemon_avatar_mark.webp'
import AurLemonTextMark from '../../assets/resources/sitemark/aurlemon_text_mark.svg'
import SkeletonImage from '../common/SkeletonImage.vue'
</script>

<style scoped>
.aurlemon-branding-row {
	display: flex;
	align-items: center;
	gap: 0.375rem;
}

.aurlemon-avatar-mark-wrapper {
	position: relative;
	width: 1.875rem;
	height: 1.875rem;
	flex: none;
}

.aurlemon-text-mark {
	display: block;
	height: 1.6875rem !important;
	width: auto;
	max-width: none;
	flex: none;
}

.aurlemon-text-mark :deep(svg) {
	display: block;
	height: 100%;
	width: auto;
	max-width: none;
}
</style>
