<script setup lang="ts">
import type {
	CountdownKind,
	SelectOption,
	Translator,
	UpdaterStatus,
} from '../../types/updater'

defineProps<{
	authenticated: boolean
	countdown: number
	countdownKind: CountdownKind | null
	isBootstrap: boolean
	loginBusy: boolean
	phaseSubtitle: string
	phaseTitle: string
	processIcon: string
	selectedSource: string
	showProcessSpinner: boolean
	sourceItems: SelectOption[]
	status: UpdaterStatus
	t: Translator
}>()

const emit = defineEmits<{
	beginUpdate: []
	interruptCountdown: []
	launchClient: []
	login: []
	selectSource: [value: string | undefined]
	skipUpdate: []
}>()
</script>

<template>
	<section
		class="flex min-h-0 flex-1 items-center justify-center px-6 pb-8 pt-16"
	>
		<div class="flex w-full max-w-lg flex-col items-center text-center">
			<UIcon
				:name="processIcon"
				class="size-14"
				:class="
					status.phase === 'failed'
						? 'text-danger-500'
						: status.phase === 'ready' || status.phase === 'up-to-date'
							? 'text-success-500'
							: 'text-primary-500'
				"
			/>
			<UIcon
				v-if="showProcessSpinner"
				name="i-lucide-loader-circle"
				class="mt-3 size-5 animate-spin text-primary-500"
			/>
			<h1 class="mt-4 text-xl font-semibold">{{ phaseTitle }}</h1>
			<p
				v-if="
					['updating', 'ready'].includes(status.phase) ||
					(phaseSubtitle && status.phase !== 'awaiting-update-decision')
				"
				class="mt-2 text-sm text-slate-600 dark:text-slate-300"
			>
				{{
					status.phase === 'updating'
						? t('bodyUpdating')
						: status.phase === 'ready'
							? t('bodyReady')
							: phaseSubtitle
				}}
			</p>

			<div
				v-if="status.phase === 'awaiting-update-decision'"
				class="mt-6 flex w-full flex-col gap-4"
			>
				<USelect
					:model-value="selectedSource"
					:items="sourceItems"
					class="w-full"
					@update:model-value="emit('selectSource', $event)"
				/>
				<div class="flex gap-3">
					<UButton
						color="neutral"
						variant="soft"
						class="flex-1 justify-center"
						@click="emit('skipUpdate')"
					>
						{{ t('updateLater') }}
					</UButton>
					<UButton
						color="primary"
						class="flex-1 justify-center"
						@click="emit('beginUpdate')"
					>
						{{ t('updateNow') }}
					</UButton>
				</div>
				<p class="text-xs text-slate-500 dark:text-slate-400">
					{{
						countdownKind
							? t('autoUpdateCountdown', { seconds: countdown })
							: t('countdownCancelled')
					}}
				</p>
				<p
					v-if="!authenticated"
					class="text-xs text-slate-500 dark:text-slate-400"
				>
					{{ t('hydrolineFastDownload') }}
					<UButton
						color="primary"
						variant="link"
						size="xs"
						class="p-0 align-baseline"
						:disabled="loginBusy"
						@click="emit('login')"
					>
						{{ t('hydrolineFastDownloadAction') }}
					</UButton>
				</p>
				<p v-else class="text-xs text-slate-500 dark:text-slate-400">
					{{ t('hydrolineLoggedIn') }}
				</p>
			</div>

			<div
				v-if="status.phase === 'ready' && isBootstrap"
				class="mt-6 flex w-full gap-3"
			>
				<UButton
					color="neutral"
					variant="soft"
					class="flex-1 justify-center"
					@click="emit('interruptCountdown')"
				>
					{{ t('launchLater') }}
				</UButton>
				<UButton
					color="primary"
					class="flex-1 justify-center"
					@click="emit('launchClient')"
				>
					{{
						countdownKind
							? t('launchNowCountdown', { seconds: countdown })
							: t('launchNow')
					}}
				</UButton>
			</div>
			<div
				v-if="status.phase === 'up-to-date' && isBootstrap"
				class="relative mt-6 flex w-full flex-col items-center"
			>
				<UButton
					color="primary"
					variant="soft"
					class="min-w-44 justify-center"
					@click="emit('launchClient')"
				>
					{{ t('launchNow') }}
				</UButton>
				<p
					v-if="countdownKind"
					class="pointer-events-none absolute top-full mt-2 text-xs text-slate-500 dark:text-slate-400"
				>
					{{ t('launchCountdown', { seconds: countdown }) }}
				</p>
			</div>
			<div
				v-if="!authenticated && status.phase !== 'awaiting-update-decision'"
				class="mt-8 text-xs text-slate-500 dark:text-slate-400"
			>
				<UButton
					color="primary"
					variant="link"
					size="xs"
					class="p-0"
					@click="emit('login')"
				>
					{{ t('hydrolineFastDownload') }} {{ t('login') }}
				</UButton>
			</div>
		</div>
	</section>
</template>
