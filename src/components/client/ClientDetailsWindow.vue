<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { message } from '@tauri-apps/plugin-dialog'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-vue'
import MarkdownContent from '../common/MarkdownContent.vue'
import AppWindowTitlebar from '../window/AppWindowTitlebar.vue'
import { useUpdaterAppearance } from '../../composables/useUpdaterAppearance'
import type {
	ClientDetailsKind,
	ClientVersionOption,
} from '../../types/updater'
import { HYDCRAFT_SCROLLBAR_OPTIONS } from '../../utils/scrollbar'
import { invokeDesktop } from '../../utils/tauri'

interface ClientDetailsWindowData {
	detail: ClientDetailsKind
	version: ClientVersionOption
}

const { t } = useUpdaterAppearance()
const details = ref<ClientDetailsWindowData | null>(null)
const loading = ref(true)
const detailLabel = computed(() =>
	details.value?.detail === 'mods' ? t('clientMods') : t('clientChangelog'),
)

onMounted(async () => {
	try {
		details.value = await invokeDesktop<ClientDetailsWindowData>(
			'client_details_window_data',
		)
	} catch (error) {
		await message(t('openClientDetailsFailed', { error: String(error) }), {
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
		class="flex h-full min-h-0 flex-col bg-slate-100 text-slate-900 dark:bg-slate-950 dark:text-white"
	>
		<AppWindowTitlebar
			inline
			class="bg-slate-100 dark:bg-slate-950"
			:close-label="t('close')"
			:minimize-label="t('minimize')"
		>
			<template #left>
				<h1 v-if="details" class="text-sm font-semibold">
					{{
						t('clientDetailsTitle', {
							version: details.version.version,
							detail: detailLabel,
						})
					}}
				</h1>
			</template>
		</AppWindowTitlebar>
		<section class="flex min-h-0 flex-1 flex-col p-6">
			<div v-if="details" class="flex min-h-0 flex-1 flex-col">
				<OverlayScrollbarsComponent
					v-if="details.detail === 'changelog' && details.version.changelog"
					class="min-h-0 flex-1 rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900"
					:options="HYDCRAFT_SCROLLBAR_OPTIONS"
					defer
				>
					<MarkdownContent
						:content="details.version.changelog"
						class="p-5 text-sm leading-6 text-slate-700 dark:text-slate-200"
					/>
				</OverlayScrollbarsComponent>
				<OverlayScrollbarsComponent
					v-else-if="details.detail === 'mods' && details.version.mods.length"
					class="min-h-0 flex-1 rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900"
					:options="HYDCRAFT_SCROLLBAR_OPTIONS"
					defer
				>
					<div
						v-for="mod in details.version.mods"
						:key="`${mod.id}-${mod.version}`"
						class="border-b border-slate-200 px-5 py-3 text-sm last:border-b-0 dark:border-slate-800"
					>
						<p class="font-medium text-slate-800 dark:text-slate-100">
							{{ mod.name }}
							<span class="font-normal text-slate-500 dark:text-slate-400">
								{{ mod.version }}
							</span>
						</p>
						<p
							class="mt-1 flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400"
						>
							<UBadge v-if="mod.api" color="neutral" variant="soft" size="xs">
								{{ mod.api }}
							</UBadge>
							<span>{{ mod.id }}</span>
						</p>
						<p
							v-if="mod.description"
							class="mt-2 text-xs leading-5 text-slate-600 dark:text-slate-300"
						>
							{{ mod.description }}
						</p>
					</div>
				</OverlayScrollbarsComponent>
				<div
					v-else
					class="flex flex-1 items-center justify-center text-sm text-slate-500 dark:text-slate-400"
				>
					{{ t('noData') }}
				</div>
			</div>
			<div v-else-if="loading" class="flex flex-1 items-center justify-center">
				<UIcon
					name="i-lucide-loader-circle"
					class="size-5 animate-spin text-primary-500"
				/>
			</div>
		</section>
	</main>
</template>
