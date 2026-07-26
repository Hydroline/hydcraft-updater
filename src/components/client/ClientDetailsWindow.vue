<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import dayjs from 'dayjs'
import { message } from '@tauri-apps/plugin-dialog'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-vue'
import MarkdownContent from '../common/MarkdownContent.vue'
import SkeletonImage from '../common/SkeletonImage.vue'
import AppWindowTitlebar from '../window/AppWindowTitlebar.vue'
import WebviewInteractionGuard from '../window/WebviewInteractionGuard.vue'
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

function formatReleaseDate(value: string | null): string | null {
	if (!value || !dayjs(value).isValid()) return null
	return dayjs(value).format('YYYY 年 M 月 D 日')
}

function formatRelativeReleaseDate(value: string | null): string | null {
	if (!value || !dayjs(value).isValid()) return null
	const date = dayjs(value)
	const now = dayjs()
	const years = now.diff(date, 'year')
	const afterYears = date.add(years, 'year')
	const months = now.diff(afterYears, 'month')
	const days = Math.max(0, now.diff(afterYears.add(months, 'month'), 'day'))
	return t('releaseRelativeDate', {
		years: years ? t('releaseYears', { value: years }) : '',
		months: months ? t('releaseMonths', { value: months }) : '',
		days: t('releaseDays', { value: days }),
	})
}

function environmentTags(version: ClientVersionOption): string[] {
	return (version.apiVersion ?? '')
		.split('/')
		.map((item, index) =>
			index === 0 ? item.trim().replace(/^Minecraft\s+/i, '') : item.trim(),
		)
		.filter(Boolean)
}

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
	<WebviewInteractionGuard>
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
					<div
						v-if="details.detail === 'changelog'"
						class="flex flex-wrap items-center gap-1.5 px-5 py-3 text-xs text-slate-600 dark:text-slate-300"
					>
						<span v-if="formatReleaseDate(details.version.publishedAt)">{{
							t('clientReleaseMetaRelative', {
								version: details.version.version,
								relative: formatRelativeReleaseDate(
									details.version.publishedAt,
								)!,
							})
						}}</span>
						<UBadge
							v-for="environment in environmentTags(details.version)"
							:key="environment"
							color="neutral"
							variant="soft"
							size="sm"
							>{{ environment }}</UBadge
						>
						<UBadge color="neutral" variant="soft" size="sm">{{
							t('clientModCount', { count: details.version.modCount })
						}}</UBadge>
					</div>
					<div
						v-if="details.detail === 'changelog'"
						class="hidden px-5 pb-3 text-xs text-slate-600 dark:text-slate-300"
					>
						<div
							v-if="details.version.publisher"
							class="flex items-center gap-1.5"
						>
							<SkeletonImage
								v-if="details.version.publisher.avatarUrl"
								:src="details.version.publisher.avatarUrl"
								:alt="
									details.version.publisher.displayName ||
									details.version.publisher.username
								"
								image-class="size-6 rounded-full object-cover"
								class="size-6 shrink-0"
							/><span>{{
								details.version.publisher.displayName ||
								details.version.publisher.username
							}}</span
							><span
								>于
								{{ formatReleaseDate(details.version.publishedAt) }} 发包</span
							>
						</div>
						<div
							v-if="details.version.contributors?.length"
							class="mt-2 flex flex-wrap items-center gap-1"
						>
							<template
								v-for="(contributor, index) in details.version.contributors"
								:key="contributor.hydrolineId"
								><SkeletonImage
									v-if="contributor.avatarUrl"
									:src="contributor.avatarUrl"
									:alt="contributor.displayName || contributor.username"
									image-class="size-5 rounded-full object-cover"
									class="size-5 shrink-0"
								/><span>{{
									contributor.displayName || contributor.username
								}}</span
								><span v-if="index < details.version.contributors.length - 1"
									>、</span
								></template
							><span
								>共 {{ details.version.contributors.length }} 人贡献。</span
							>
						</div>
					</div>
					<OverlayScrollbarsComponent
						v-if="details.detail === 'changelog'"
						class="min-h-0 flex-1 rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900"
						:options="HYDCRAFT_SCROLLBAR_OPTIONS"
						defer
					>
						<div
							v-if="details"
							class="hidden flex-wrap items-center gap-1.5 px-5 py-3 text-xs text-slate-600 dark:text-slate-300"
						>
							<span v-if="formatReleaseDate(details.version.publishedAt)">
								{{
									t('clientReleaseMeta', {
										version: details.version.version,
										relative: formatRelativeReleaseDate(
											details.version.publishedAt,
										)!,
										date: formatReleaseDate(details.version.publishedAt)!,
									})
								}}
							</span>
							<UBadge
								v-for="environment in environmentTags(details.version)"
								:key="environment"
								color="neutral"
								variant="soft"
								size="xs"
							>
								{{ environment }}
							</UBadge>
							<UBadge color="neutral" variant="soft" size="xs">
								{{ t('clientModCount', { count: details.version.modCount }) }}
							</UBadge>
						</div>
						<div class="flex min-h-full flex-col">
							<MarkdownContent
								v-if="details.version.changelog"
								:content="details.version.changelog"
								class="p-5 text-sm leading-6 text-slate-700 dark:text-slate-200"
							/>
							<div
								v-else
								class="flex min-h-32 items-center justify-center text-sm text-slate-500 dark:text-slate-400"
							>
								{{ t('noChangelog') }}
							</div>
							<div
								v-if="details"
								class="mt-auto px-5 pb-5 text-xs text-slate-600 dark:text-slate-300"
							>
								<div
									v-if="details.version.publisher"
									class="flex items-center gap-1.5"
								>
									<SkeletonImage
										v-if="details.version.publisher.avatarUrl"
										:src="details.version.publisher.avatarUrl"
										:alt="
											details.version.publisher.displayName ||
											details.version.publisher.username
										"
										image-class="size-6 rounded-full object-cover"
										class="size-6 shrink-0"
									/>
									<span
										>{{
											details.version.publisher.displayName ||
											details.version.publisher.username
										}}
										于
										{{ formatReleaseDate(details.version.publishedAt) }}
										发包</span
									>
								</div>
								<div
									v-if="details.version.contributors?.length"
									class="mt-3 flex flex-wrap items-center gap-1.5"
								>
									<template
										v-for="(contributor, index) in details.version.contributors"
										:key="contributor.hydrolineId"
									>
										<div class="flex items-center gap-1.5">
											<SkeletonImage
												v-if="contributor.avatarUrl"
												:src="contributor.avatarUrl"
												:alt="contributor.displayName || contributor.username"
												image-class="size-6 rounded-full object-cover"
												class="size-6 shrink-0"
											/>
											<span>{{
												contributor.displayName || contributor.username
											}}</span>
										</div>
										<span v-if="index < details.version.contributors.length - 1"
											>/</span
										>
									</template>
									<span
										>共 {{ details.version.contributors.length }} 人贡献。</span
									>
								</div>
							</div>
						</div>
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
				<div
					v-else-if="loading"
					class="flex flex-1 items-center justify-center"
				>
					<UIcon
						name="i-lucide-loader-circle"
						class="size-5 animate-spin text-primary-500"
					/>
				</div>
			</section>
		</main>
	</WebviewInteractionGuard>
</template>
