<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useUpdaterAppearance } from '../../composables/useUpdaterAppearance'
import { hideAuthenticationWindow, invokeDesktop } from '../../utils/tauri'
import AppWindowTitlebar from '../window/AppWindowTitlebar.vue'
import WebviewInteractionGuard from '../window/WebviewInteractionGuard.vue'

type AuthPhase = 'browser-opened' | 'verified' | 'failed'

interface DesktopAuthEvent {
	phase: AuthPhase
	message: string
}

const { t } = useUpdaterAppearance()
const phase = ref<AuthPhase>('browser-opened')
const errorMessage = ref('')
const displayMessage = computed(
	() =>
		errorMessage.value ||
		{
			'browser-opened': t('browserOpened'),
			verified: t('verified'),
			failed: t('failed'),
		}[phase.value],
)
const icon = computed(
	() =>
		({
			'browser-opened': 'i-lucide-loader-circle',
			verified: 'i-lucide-circle-check-big',
			failed: 'i-lucide-circle-alert',
		})[phase.value],
)
let unlistenDesktopAuthResult: (() => void) | undefined

async function openManually(): Promise<void> {
	try {
		await invokeDesktop<void>('start_desktop_login')
		phase.value = 'browser-opened'
		errorMessage.value = ''
	} catch {
		phase.value = 'failed'
		errorMessage.value = t('failed')
	}
}

onMounted(async () => {
	if (!('__TAURI_INTERNALS__' in window)) return
	const { listen } = await import('@tauri-apps/api/event')
	unlistenDesktopAuthResult = await listen<DesktopAuthEvent>(
		'desktop-auth-result',
		({ payload }) => {
			phase.value = payload.phase
			errorMessage.value = payload.phase === 'failed' ? t('failed') : ''
		},
	)
})

onBeforeUnmount(() => {
	unlistenDesktopAuthResult?.()
})
</script>

<template>
	<WebviewInteractionGuard>
		<main
			class="relative flex min-h-screen flex-col items-center justify-center bg-white p-6 text-slate-950 dark:bg-slate-950 dark:text-white"
		>
			<AppWindowTitlebar
				:close-label="t('close')"
				:minimize-label="t('minimize')"
				close-mode="hide-auth"
			/>
			<div class="flex w-full max-w-xs flex-col items-center gap-5 text-center">
				<UIcon
					:name="icon"
					class="size-16"
					:class="
						phase === 'browser-opened'
							? 'animate-spin text-primary-500'
							: phase === 'verified'
								? 'text-success-500'
								: 'text-danger-500'
					"
				/>
				<p
					class="max-w-xs text-sm leading-6 text-slate-600 dark:text-slate-300"
				>
					{{ displayMessage }}
				</p>
				<UButton
					v-if="phase === 'verified'"
					color="primary"
					variant="soft"
					class="mt-3 w-full justify-center"
					@click="hideAuthenticationWindow"
					>{{ t('confirm') }}</UButton
				>
			</div>
			<div
				class="absolute inset-x-0 bottom-7 flex items-center justify-center text-xs text-slate-600 dark:text-slate-300"
			>
				<span>{{ t('manualPrefix') }}</span
				><UButton
					color="primary"
					variant="link"
					size="xs"
					class="ml-1 p-0"
					@click="openManually"
					>{{ t('manual') }}</UButton
				>
			</div>
		</main>
	</WebviewInteractionGuard>
</template>
