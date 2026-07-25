<script setup lang="ts">
import { ref } from 'vue'
import type {
	ClientDetailsKind,
	ClientVersionOption,
	Translator,
} from '../../types/updater'

defineProps<{
	clientVersions: ClientVersionOption[]
	currentClientVersion: string | null
	t: Translator
}>()

const emit = defineEmits<{
	openClientDetails: [payload: { version: string; detail: ClientDetailsKind }]
}>()

const expandedVersion = ref<string | null>(null)

function formatPublishedAt(value: string | null): string | null {
	if (!value) return null
	const date = new Date(value)
	if (Number.isNaN(date.getTime())) return null
	return new Intl.DateTimeFormat(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
	}).format(date)
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
</script>

<template>
	<section
		v-if="!clientVersions.length"
		class="flex flex-1 items-center justify-center p-6 text-center"
	>
		<div class="flex max-w-sm flex-col items-center">
			<UIcon
				name="i-lucide-package"
				class="size-12 text-slate-900/85 dark:text-white"
			/>
			<h2 class="mt-4 mx-1 text-xl text-slate-950 dark:text-white">
				{{ t('clientTitle') }}
			</h2>
			<p class="mt-2 text-sm leading-6 text-slate-500 dark:text-slate-400">
				{{ t('noData') }}
			</p>
		</div>
	</section>
	<section v-else class="flex flex-1 items-start justify-center px-6 pb-8 pt-6">
		<div class="w-full max-w-3xl">
			<div class="mb-3 flex flex-wrap items-center justify-between gap-3">
				<h2 class="mx-1 text-xl text-slate-950 dark:text-white">
					{{ t('clientTitle') }}
				</h2>
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
							<p class="text-[15px] text-slate-700 dark:text-slate-200">
								{{ option.version }}
							</p>
							<p
								v-if="formatPublishedAt(option.publishedAt)"
								class="text-xs text-slate-500 dark:text-slate-400"
							>
								{{
									t('clientPublishedAt', {
										date: formatPublishedAt(option.publishedAt)!,
									})
								}}
							</p>
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
							<div class="grid grid-cols-2 gap-3 px-5 py-4">
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
