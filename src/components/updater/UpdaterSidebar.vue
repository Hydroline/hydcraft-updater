<script setup lang="ts">
import { ref } from 'vue'

import SkeletonImage from '../common/SkeletonImage.vue'
import hydcraftLogo from '../../assets/resources/brands/logo_HydCraft.png'
import type {
	LocaleCode,
	ThemeMode,
} from '../../composables/useUpdaterAppearance'
import type {
	DesktopIdentity,
	Translator,
	THEME_MODES,
} from '../../types/updater'

type ThemeOption = (typeof THEME_MODES)[number]
type LocaleOption = { value: LocaleCode; label: string }

defineProps<{
	appName: string
	authenticated: boolean
	displayName: string
	hasAvailableUpdate: boolean
	identity: DesktopIdentity | null
	localeItems: readonly LocaleOption[]
	loginBusy: boolean
	selectedLocale: LocaleCode
	themeIcon: string
	themeMode: ThemeMode
	themeModes: readonly ThemeOption[]
	t: Translator
}>()

const emit = defineEmits<{
	login: []
	logout: []
	openProfile: []
	selectLocale: [value: LocaleCode]
	selectTheme: [value: ThemeMode]
	dragFromAside: [event: MouseEvent]
}>()

const accountMenuOpen = ref(false)
const themeMenuOpen = ref(false)
const localeMenuOpen = ref(false)

function selectTheme(value: ThemeMode): void {
	themeMenuOpen.value = false
	emit('selectTheme', value)
}

function selectLocale(value: LocaleCode): void {
	localeMenuOpen.value = false
	emit('selectLocale', value)
}
</script>

<template>
	<aside
		class="flex w-64 shrink-0 flex-col border-r border-slate-200 bg-slate-50 p-7 pt-14 dark:border-slate-800 dark:bg-slate-900 select-none"
		@mousedown.left="$emit('dragFromAside', $event)"
	>
		<div>
			<img :src="hydcraftLogo" :alt="appName" class="size-12 object-contain" />
			<h1 class="mt-5 text-3xl font-semibold">{{ appName }}</h1>
			<p class="text-base tracking-tight text-slate-600 dark:text-slate-300">
				{{ t('updater') }}
			</p>
			<UBadge
				v-if="hasAvailableUpdate"
				color="primary"
				variant="soft"
				class="mt-2.5 rounded-full px-3 py-1 text-xs font-medium"
			>
				{{ t('updateAvailable') }}
			</UBadge>
		</div>

		<div class="mt-auto flex items-center">
			<div class="flex flex-1 items-center gap-2">
				<UPopover
					v-model:open="themeMenuOpen"
					:popper="{ placement: 'top-start' }"
					:ui="{ content: 'z-[40000]' }"
				>
					<UButton
						color="neutral"
						variant="ghost"
						size="xs"
						class="h-9 w-9 rounded-full hover:bg-slate-500/10 active:bg-slate-500/20"
						icon-only
						:aria-label="t('theme')"
					>
						<UIcon :name="themeIcon" class="h-6 w-6" />
					</UButton>
					<template #content>
						<div class="flex w-40 flex-col gap-1 p-2">
							<UButton
								v-for="mode in themeModes"
								:key="mode.value"
								type="button"
								color="neutral"
								variant="ghost"
								class="w-full justify-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800"
								:class="{
									'bg-primary-100/60 text-primary-600 dark:bg-primary-500/20 dark:text-primary-200':
										themeMode === mode.value,
									'text-slate-600 dark:text-slate-300':
										themeMode !== mode.value,
								}"
								@click="selectTheme(mode.value)"
							>
								<UIcon :name="mode.icon" class="h-4 w-4" />
								<span>{{ t(mode.label) }}</span>
								<UIcon
									v-if="themeMode === mode.value"
									name="i-lucide-check"
									class="ml-auto h-4 w-4"
								/>
							</UButton>
						</div>
					</template>
				</UPopover>

				<UPopover
					v-model:open="localeMenuOpen"
					:popper="{ placement: 'top-start' }"
					:ui="{ content: 'z-[40000]' }"
				>
					<UButton
						color="neutral"
						variant="ghost"
						size="xs"
						class="h-9 w-9 rounded-full hover:bg-slate-500/10 active:bg-slate-500/20"
						icon-only
						:aria-label="t('language')"
					>
						<UIcon name="i-lucide-languages" class="h-6 w-6" />
					</UButton>
					<template #content>
						<div class="flex w-40 flex-col gap-1 p-2">
							<UButton
								v-for="item in localeItems"
								:key="item.value"
								type="button"
								color="neutral"
								variant="ghost"
								class="w-full justify-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-slate-100 dark:hover:bg-slate-800"
								:class="{
									'bg-primary-100/60 text-primary-600 dark:bg-primary-500/20 dark:text-primary-200':
										selectedLocale === item.value,
									'text-slate-600 dark:text-slate-300':
										selectedLocale !== item.value,
								}"
								@click="selectLocale(item.value)"
							>
								<span>{{ item.label }}</span>
								<UIcon
									v-if="selectedLocale === item.value"
									name="i-lucide-check"
									class="ml-auto h-4 w-4"
								/>
							</UButton>
						</div>
					</template>
				</UPopover>
			</div>

			<UButton
				v-if="!authenticated"
				color="neutral"
				variant="link"
				size="xs"
				class="px-2 text-sm whitespace-nowrap transition hover:opacity-80"
				@click="emit('login')"
			>
				{{ t('login') }}
			</UButton>
			<UPopover
				v-else
				v-model:open="accountMenuOpen"
				:popper="{ placement: 'top-end' }"
				:ui="{ content: 'z-[40000]' }"
			>
				<button
					type="button"
					class="ml-0.5 flex h-9 items-center justify-center gap-1 rounded-full border-0 bg-transparent py-0 pr-1.5 pl-0 opacity-100 transition duration-150 hover:opacity-80 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
					:aria-label="t('accountMenu')"
				>
					<span
						class="relative flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-full bg-slate-200 text-sm font-semibold text-slate-700 ring ring-slate-200 transition duration-200 dark:bg-slate-700 dark:text-slate-100 dark:ring-slate-700"
					>
						<SkeletonImage
							v-if="identity?.avatarUrl"
							:src="identity.avatarUrl"
							:alt="displayName"
							image-class="h-full w-full object-cover"
							class="h-full w-full"
						/>
						<span v-else class="leading-none">{{
							displayName.slice(0, 1)
						}}</span>
					</span>
					<UIcon
						name="i-lucide-chevron-up"
						class="h-3.5 w-3.5 translate-y-0 opacity-80 transition duration-200"
						:class="{ 'rotate-180': accountMenuOpen }"
					/>
				</button>
				<template #content>
					<div class="flex min-w-40 flex-col gap-1 p-2">
						<div class="px-3 py-2">
							<div
								class="line-clamp-2 wrap-break-word text-[17px] leading-snug font-semibold text-slate-600 dark:text-slate-300"
							>
								{{ displayName }}
							</div>
							<div
								class="text-[13px] leading-[normal] text-slate-500/80 dark:text-slate-400/80"
							>
								{{ identity?.hydrolineId }}
							</div>
						</div>
						<UButton
							color="neutral"
							variant="ghost"
							class="w-full justify-start gap-1.5 rounded-lg px-3 py-2 text-left text-sm text-slate-600 transition hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
							@click="emit('openProfile')"
						>
							<UIcon name="i-lucide-user" class="h-4.5 w-4.5 shrink-0" />
							<span class="leading-[normal] min-w-0 truncate">{{
								t('profile')
							}}</span>
						</UButton>
						<div class="my-1 border-t border-slate-200 dark:border-slate-700" />
						<UButton
							color="error"
							variant="ghost"
							class="w-full justify-start gap-1.5 rounded-lg px-3 py-2 text-left text-sm transition hover:bg-error-50! active:bg-error-100! dark:hover:bg-error-900/25! dark:active:bg-error-900/35!"
							@click="emit('logout')"
						>
							<UIcon name="i-lucide-log-out" class="h-4.5 w-4.5 shrink-0" />
							<span class="leading-[normal] min-w-0 truncate">{{
								t('logout')
							}}</span>
						</UButton>
					</div>
				</template>
			</UPopover>
		</div>
	</aside>
</template>
