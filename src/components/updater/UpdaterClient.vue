<script setup lang="ts">
import { ref } from 'vue'
import dayjs from 'dayjs'
import SkeletonImage from '../common/SkeletonImage.vue'
import type {
	ClientDetailsKind,
	ClientInstallMode,
	ClientStorageInfo,
	ClientVersionOption,
	Translator,
} from '../../types/updater'

defineProps<{
	clientVersions: ClientVersionOption[]
	loading: boolean
	currentClientVersion: string | null
	storageInfo: ClientStorageInfo
	t: Translator
}>()

const emit = defineEmits<{
	openClientDetails: [payload: { version: string; detail: ClientDetailsKind }]
	refresh: []
	installClient: [payload: { version: string; mode: ClientInstallMode }]
	rollbackLastUpdate: []
}>()

const expandedVersion = ref<string | null>(null)

function formatPublishedAt(value: string | null): string | null {
	if (!value || !dayjs(value).isValid()) return null
	return dayjs(value).format('YYYY 年 M 月 D 日')
}

function environmentTags(option: ClientVersionOption): string[] {
	return (option.apiVersion ?? '')
		.split('/')
		.map((item, index) =>
			index === 0 ? item.trim().replace(/^Minecraft\s+/i, '') : item.trim(),
		)
		.filter(Boolean)
}

function toggleVersion(version: string): void {
	expandedVersion.value = expandedVersion.value === version ? null : version
}

function openClientDetails(version: string, detail: ClientDetailsKind): void {
	emit('openClientDetails', { version, detail })
}

function canInstall(option: ClientVersionOption): boolean {
	return Boolean(
		option.fullPackage &&
		(option.isLatest || option.fullPackage.packageKey.includes('/base/')),
	)
}
</script>

<template>
	<section
		v-if="!clientVersions.length"
		class="relative flex flex-1 items-center justify-center p-6 text-center"
	>
		<UButton
			color="neutral"
			variant="link"
			icon="i-lucide-refresh-cw"
			:loading="loading"
			:disabled="loading"
			class="absolute right-6 top-6"
			@click="emit('refresh')"
		>
			{{ t('refresh') }}
		</UButton>
		<div class="flex max-w-sm flex-col items-center">
			<UIcon
				name="i-lucide-package"
				class="size-12 text-slate-900/85 dark:text-white"
			/>
			<p class="mt-4 text-sm leading-6 text-slate-500 dark:text-slate-400">
				{{ t('noData') }}
			</p>
		</div>
	</section>
	<section v-else class="flex flex-1 items-start justify-center px-6 pb-8 pt-6">
		<div class="w-full max-w-3xl">
			<div class="mb-3 flex flex-wrap items-center justify-end gap-1">
				<UPopover v-if="storageInfo.rollbackAvailable">
					<UButton color="warning" variant="link" icon="i-lucide-history">
						{{ t('rollbackAction') }}
					</UButton>
					<template #content>
						<div class="flex w-64 flex-col gap-3 p-3">
							<p class="text-sm leading-5 text-slate-700 dark:text-slate-200">
								{{
									t('rollbackDescription', {
										from: storageInfo.rollbackToVersion || '',
										to: storageInfo.rollbackFromVersion || '',
									})
								}}
							</p>
							<UButton
								color="warning"
								class="justify-center"
								@click="emit('rollbackLastUpdate')"
							>
								{{ t('rollbackAction') }}
							</UButton>
						</div>
					</template>
				</UPopover>
				<UButton
					color="neutral"
					variant="link"
					icon="i-lucide-refresh-cw"
					:loading="loading"
					:disabled="loading"
					@click="emit('refresh')"
				>
					{{ t('refresh') }}
				</UButton>
			</div>
			<div class="flex flex-col gap-3">
				<article
					v-for="option in clientVersions"
					:key="option.version"
					class="overflow-hidden rounded-lg border border-slate-200 bg-white transition-colors hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:hover:bg-slate-900/70"
				>
					<button
						type="button"
						class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
						@click="toggleVersion(option.version)"
					>
						<div class="min-w-0">
							<div class="text-[15px] text-slate-700 dark:text-slate-200">
								{{ option.version }}
							</div>

							<div
								v-if="formatPublishedAt(option.publishedAt)"
								class="mt-1 flex min-w-0 items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400"
							>
								<SkeletonImage
									v-if="option.publisher?.avatarUrl"
									:src="option.publisher.avatarUrl"
									:alt="
										option.publisher.displayName || option.publisher.username
									"
									image-class="size-5 rounded-full object-cover"
									class="size-5 shrink-0"
								/>
								<p class="min-w-0 truncate leading-[normal]">
									{{
										option.publisher
											? t('clientPublishedByAt', {
													username:
														option.publisher.displayName ||
														option.publisher.username,
													date: formatPublishedAt(option.publishedAt)!,
												})
											: t('clientPublishedAt', {
													date: formatPublishedAt(option.publishedAt)!,
												})
									}}
								</p>
							</div>

							<div
								v-if="(option.contributors?.length ?? 0) > 0"
								class="mt-2 flex min-w-0 items-center text-xs text-slate-500 dark:text-slate-400"
							>
								<div class="flex shrink-0">
									<SkeletonImage
										v-for="(contributor, index) in option.contributors"
										:key="contributor.hydrolineId"
										:src="contributor.avatarUrl || ''"
										:alt="contributor.displayName || contributor.username"
										image-class="size-5 rounded-full object-cover dark:border-slate-900"
										:class="index === 0 ? 'size-5' : '-ml-2 size-5'"
									/>
								</div>
								<p class="ml-1.5 truncate">
									{{
										t(
											option.contributors?.length === 1
												? 'clientContributor'
												: 'clientContributors',
											{
												username:
													option.contributors?.[0]?.displayName ||
													option.contributors?.[0]?.username ||
													'',
												count: option.contributors?.length ?? 0,
											},
										)
									}}
								</p>
							</div>
							<div class="mt-2 flex flex-wrap gap-2">
								<UBadge
									v-for="environment in environmentTags(option)"
									:key="environment"
									color="neutral"
									variant="soft"
								>
									{{ environment }}
								</UBadge>
								<UBadge color="neutral" variant="soft">
									{{ t('clientModCount', { count: option.modCount }) }}
								</UBadge>
							</div>
						</div>
						<div class="flex shrink-0 items-center gap-2">
							<UBadge
								v-if="option.version === currentClientVersion"
								color="success"
								variant="soft"
							>
								{{ t('currentVersionTag') }}
							</UBadge>
							<UBadge v-if="option.isLatest" color="primary" variant="soft">
								{{ t('versionLatest') }}
							</UBadge>
							<UIcon
								name="i-lucide-chevron-down"
								class="size-4 text-slate-400 transition-transform dark:text-slate-500"
								:class="{ 'rotate-180': expandedVersion === option.version }"
							/>
						</div>
					</button>
					<Transition name="client-details">
						<div
							v-if="expandedVersion === option.version"
							class="client-details-content border-t border-slate-200 dark:border-slate-800"
						>
							<div
								class="grid gap-3 px-5 py-4"
								:class="canInstall(option) ? 'grid-cols-3' : 'grid-cols-2'"
							>
								<UPopover v-if="canInstall(option)">
									<UButton
										color="primary"
										variant="soft"
										class="justify-center"
										icon="i-lucide-download"
									>
										{{ t('clientDownloadOverwrite') }}
									</UButton>
									<template #content>
										<div class="flex w-52 flex-col gap-1 p-2">
											<UButton
												color="neutral"
												variant="ghost"
												class="justify-start"
												@click="
													emit('installClient', {
														version: option.version,
														mode: 'full',
													})
												"
											>
												{{ t('clientInstallFull') }}
											</UButton>
											<UButton
												color="neutral"
												variant="ghost"
												class="justify-start"
												@click="
													emit('installClient', {
														version: option.version,
														mode: 'mods',
													})
												"
											>
												{{ t('clientInstallMods') }}
											</UButton>
										</div>
									</template>
								</UPopover>
								<UButton
									color="neutral"
									variant="soft"
									class="justify-center"
									icon="i-lucide-scroll-text"
									@click="openClientDetails(option.version, 'changelog')"
								>
									{{ t('clientChangelog') }}
								</UButton>
								<UButton
									color="neutral"
									variant="soft"
									class="justify-center"
									icon="i-lucide-boxes"
									@click="openClientDetails(option.version, 'mods')"
								>
									{{ t('clientMods') }}
								</UButton>
							</div>
						</div>
					</Transition>
				</article>
			</div>
		</div>
	</section>
</template>

<style scoped>
.client-details-enter-active,
.client-details-leave-active {
	overflow: hidden;
	transition:
		max-height 180ms ease,
		opacity 180ms ease;
}

.client-details-enter-from,
.client-details-leave-to {
	max-height: 0;
	opacity: 0;
}

.client-details-enter-to,
.client-details-leave-from {
	max-height: 96px;
	opacity: 1;
}
</style>
