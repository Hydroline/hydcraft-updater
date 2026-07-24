<script setup lang="ts">
import UpdaterAbout from './UpdaterAbout.vue'
import UpdaterAddons from './UpdaterAddons.vue'
import UpdaterClient from './UpdaterClient.vue'
import UpdaterSettings from './UpdaterSettings.vue'
import UpdaterUpgrade from './UpdaterUpgrade.vue'
import type {
	CountdownKind,
	ClientVersionOption,
	SelectOption,
	TabKey,
	Translator,
	UpdaterContext,
	UpdaterStatus,
} from '../../types/updater'

const props = defineProps<{
	authenticated: boolean
	clientVersions: ClientVersionOption[]
	context: UpdaterContext
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
	tab: TabKey
	t: Translator
}>()

const emit = defineEmits<{
	beginUpdate: []
	interruptCountdown: []
	launchClient: []
	login: []
	openExternalUrl: [url: string]
	selectSource: [value: string | undefined]
	skipUpdate: []
}>()
</script>

<template>
	<Transition name="updater-tab-switch" mode="out-in">
		<UpdaterUpgrade
			v-if="props.tab === 'upgrade'"
			key="upgrade"
			:authenticated="props.authenticated"
			:countdown="props.countdown"
			:countdown-kind="props.countdownKind"
			:is-bootstrap="props.isBootstrap"
			:login-busy="props.loginBusy"
			:phase-subtitle="props.phaseSubtitle"
			:phase-title="props.phaseTitle"
			:process-icon="props.processIcon"
			:selected-source="props.selectedSource"
			:show-process-spinner="props.showProcessSpinner"
			:source-items="props.sourceItems"
			:status="props.status"
			:t="props.t"
			@begin-update="emit('beginUpdate')"
			@interrupt-countdown="emit('interruptCountdown')"
			@launch-client="emit('launchClient')"
			@login="emit('login')"
			@select-source="emit('selectSource', $event)"
			@skip-update="emit('skipUpdate')"
		/>

		<UpdaterSettings
			v-else-if="props.tab === 'settings'"
			key="settings"
			:context="props.context"
			:is-bootstrap="props.isBootstrap"
			:phase-title="props.phaseTitle"
			:selected-source="props.selectedSource"
			:source-items="props.sourceItems"
			:t="props.t"
			@select-source="emit('selectSource', $event)"
		/>

		<UpdaterClient
			v-else-if="props.tab === 'client'"
			key="client"
			:client-versions="props.clientVersions"
			:t="props.t"
		/>

		<UpdaterAddons
			v-else-if="props.tab === 'addons'"
			key="addons"
			:t="props.t"
		/>

		<UpdaterAbout
			v-else
			key="about"
			:t="props.t"
			@open-external-url="emit('openExternalUrl', $event)"
		/>
	</Transition>
</template>

<style scoped>
.updater-tab-switch-enter-active,
.updater-tab-switch-leave-active {
	transition:
		opacity 220ms ease-out,
		transform 260ms cubic-bezier(0.16, 1, 0.3, 1),
		filter 220ms ease-out;
}

.updater-tab-switch-enter-from,
.updater-tab-switch-leave-to {
	opacity: 0;
	filter: blur(2px);
	transform: translateY(8px);
}

.updater-tab-switch-enter-to,
.updater-tab-switch-leave-from {
	opacity: 1;
	filter: blur(0);
	transform: translateY(0);
}
</style>
