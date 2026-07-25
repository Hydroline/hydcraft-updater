<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { message } from '@tauri-apps/plugin-dialog'
import hydcraftLogo from '../../assets/resources/brands/logo_HydCraft.png'
import AppWindowTitlebar from '../window/AppWindowTitlebar.vue'
import { closeCurrentWindow, invokeDesktop } from '../../utils/tauri'
import { useUpdaterAppearance } from '../../composables/useUpdaterAppearance'

interface VersionOption {
	version: string
	label: string
	isLatest: boolean
}

const { t } = useUpdaterAppearance()
const options = ref<VersionOption[]>([])
const selected = ref('')
const loading = ref(true)
const failed = ref(false)
const versionItems = computed(() =>
	options.value.map((option) => ({
		label: `${option.label}${option.isLatest ? t('versionLatestSuffix', { latest: t('versionLatest') }) : ''}`,
		value: option.version,
	})),
)

async function confirm(): Promise<void> {
	if (!selected.value) return
	try {
		await invokeDesktop<void>('select_current_version', {
			version: selected.value,
		})
		await invokeDesktop<void>('hide_version_window')
	} catch (error) {
		await message(t('confirmVersionFailed', { error: String(error) }), {
			title: t('dialogTitle'),
			kind: 'error',
		})
	}
}

onMounted(async () => {
	try {
		options.value = await invokeDesktop<VersionOption[]>(
			'client_version_options',
		)
		selected.value = options.value[0]?.version ?? ''
	} catch (error) {
		failed.value = true
		await message(t('readVersionsFailed', { error: String(error) }), {
			title: t('dialogTitle'),
			kind: 'error',
		})
	} finally {
		loading.value = false
	}
})
</script>

<template>
	<main
		class="relative flex min-h-screen flex-col items-center justify-center bg-white p-6 text-slate-950 dark:bg-slate-950 dark:text-white"
	>
		<AppWindowTitlebar
			:close-label="t('close')"
			:minimize-label="t('minimize')"
			close-mode="hide-version"
		/>
		<div class="flex w-full max-w-xs flex-col items-center gap-5 text-center">
			<img :src="hydcraftLogo" alt="HydCraft" class="size-16 object-contain" />
			<div>
				<h1 class="text-lg font-semibold">{{ t('versionTitle') }}</h1>
				<p class="mt-2 text-sm leading-6 text-slate-600 dark:text-slate-300">
					{{ t('versionDescription') }}
				</p>
			</div>
			<USelect
				v-model="selected"
				:items="versionItems"
				:disabled="loading || failed"
				class="w-full"
			/>
			<div class="flex w-full gap-3">
				<UButton
					color="neutral"
					variant="soft"
					class="flex-1 justify-center"
					@click="closeCurrentWindow"
					>{{ t('cancel') }}</UButton
				>
				<UButton
					color="primary"
					class="flex-1 justify-center"
					:disabled="!selected || loading || failed"
					@click="confirm"
					>{{ t('versionNext') }}</UButton
				>
			</div>
		</div>
	</main>
</template>
