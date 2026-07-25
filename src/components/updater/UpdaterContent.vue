<script setup lang="ts">
import UpdaterAbout from './UpdaterAbout.vue'
import UpdaterAddons from './UpdaterAddons.vue'
import UpdaterClient from './UpdaterClient.vue'
import UpdaterSettings from './UpdaterSettings.vue'
import UpdaterUpgrade from './UpdaterUpgrade.vue'
import type {
	ClientVersionOption,
	ClientDetailsKind,
	ClientInstallMode,
	DownloadSource,
	SelectOption,
	TabKey,
	Translator,
	UpdateConflict,
	UpdaterContext,
	UpdaterStatus,
} from '../../types/updater'

const props = defineProps<{
	authenticated: boolean
	clientVersions: ClientVersionOption[]
	conflictSelections: Record<string, string>
	conflicts: UpdateConflict[]
	currentClientVersion: string | null
	context: UpdaterContext
	sources: DownloadSource[]
	sourceTesting: boolean
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
	launchClient: []
	login: []
	openClientDetails: [payload: { version: string; detail: ClientDetailsKind }]
	refreshClients: []
	installClient: [payload: { version: string; mode: ClientInstallMode }]
	openExternalUrl: [url: string]
	recheckUpdate: []
	resolveConflicts: []
	retryUpdate: []
	selectConflictResolution: [operationId: string, value: string]
	selectSource: [value: string | undefined]
	refreshSources: []
}>()
</script>

<template>
	<div class="flex min-h-full flex-1 flex-col">
		<Transition name="updater-tab-switch" mode="out-in">
			<UpdaterUpgrade
				v-if="props.tab === 'upgrade'"
				:authenticated="props.authenticated"
				:client-versions="props.clientVersions"
				:conflict-selections="props.conflictSelections"
				:conflicts="props.conflicts"
				:current-client-version="props.currentClientVersion"
				:is-bootstrap="props.isBootstrap"
				:login-busy="props.loginBusy"
				:phase-subtitle="props.phaseSubtitle"
				:phase-title="props.phaseTitle"
				:process-icon="props.processIcon"
				:selected-source="props.selectedSource"
				:show-process-spinner="props.showProcessSpinner"
				:source-items="props.sourceItems"
				:sources="props.sources"
				:source-testing="props.sourceTesting"
				:status="props.status"
				:t="props.t"
				@begin-update="emit('beginUpdate')"
				@launch-client="emit('launchClient')"
				@login="emit('login')"
				@recheck-update="emit('recheckUpdate')"
				@resolve-conflicts="emit('resolveConflicts')"
				@select-conflict-resolution="
					emit('selectConflictResolution', $event.operationId, $event.value)
				"
				@select-source="emit('selectSource', $event)"
				@refresh-sources="emit('refreshSources')"
				@retry-update="emit('retryUpdate')"
			/>

			<UpdaterSettings
				v-else-if="props.tab === 'settings'"
				:context="props.context"
				:is-bootstrap="props.isBootstrap"
				:phase-title="props.phaseTitle"
				:selected-source="props.selectedSource"
				:source-items="props.sourceItems"
				:sources="props.sources"
				:source-testing="props.sourceTesting"
				:t="props.t"
				@select-source="emit('selectSource', $event)"
				@refresh-sources="emit('refreshSources')"
			/>

			<UpdaterClient
				v-else-if="props.tab === 'client'"
				:client-versions="props.clientVersions"
				:current-client-version="props.currentClientVersion"
				:t="props.t"
				@open-client-details="emit('openClientDetails', $event)"
				@refresh="emit('refreshClients')"
				@install-client="emit('installClient', $event)"
			/>

			<UpdaterAddons v-else-if="props.tab === 'addons'" :t="props.t" />

			<UpdaterAbout
				v-else
				:t="props.t"
				@open-external-url="emit('openExternalUrl', $event)"
			/>
		</Transition>
	</div>
</template>
