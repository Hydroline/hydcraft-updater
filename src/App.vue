<script setup lang="ts">
import { computed } from 'vue'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-vue'
import AppWindowTitlebar from './components/window/AppWindowTitlebar.vue'
import UpdaterContent from './components/updater/UpdaterContent.vue'
import UpdaterNavigation from './components/updater/UpdaterNavigation.vue'
import UpdaterSidebar from './components/updater/UpdaterSidebar.vue'
import WebviewInteractionGuard from './components/window/WebviewInteractionGuard.vue'
import { useUpdaterController } from './composables/useUpdaterController'
import { HYDCRAFT_SCROLLBAR_OPTIONS } from './utils/scrollbar'

const {
	appName,
	armBootstrapUpdateCountdownCancellation,
	authenticated,
	beginUpdate,
	cancelBootstrapUpdateCountdown,
	cancelConflictResolution,
	clientVersions,
	clientVersionsLoading,
	cleanBackups,
	cleanDownloads,
	cleanDownloadsAfterInstall,
	downloadsCleaning,
	backupsCleaning,
	downloadsCleanupVersion,
	backupsCleanupVersion,
	conflictSelections,
	conflicts,
	currentClientVersion,
	context,
	displayName,
	dragFromAside,
	identity,
	isBootstrap,
	localeItems,
	loginBusy,
	launchClient,
	logout,
	openProfile,
	openExternalUrl,
	openClientDetails,
	installClientVersion,
	refreshClientVersions,
	refreshSources,
	retryUpdate,
	phaseSubtitle,
	phaseTitle,
	processIcon,
	recheckUpdate,
	resolveConflicts,
	rollbackLastUpdate,
	selectLocale,
	selectConflictResolution,
	selectSource,
	selectTheme,
	selectedLocale,
	selectedSource,
	setCleanDownloadsAfterInstall,
	showProcessSpinner,
	sourceItems,
	sources,
	sourceTesting,
	storageInfo,
	startLogin,
	status,
	t,
	tab,
	tabs,
	themeIcon,
	themeMode,
	themeModes,
} = useUpdaterController()

const hasAvailableUpdate = computed(() => {
	return Boolean(
		status.value.currentVersion &&
		status.value.targetVersion &&
		status.value.currentVersion !== status.value.targetVersion,
	)
})
</script>

<template>
	<WebviewInteractionGuard>
		<main
			class="relative flex h-full min-h-0 overflow-hidden bg-slate-100 text-slate-900 dark:bg-slate-950 dark:text-white"
			@pointerenter="armBootstrapUpdateCountdownCancellation"
			@pointermove="cancelBootstrapUpdateCountdown"
		>
			<UpdaterSidebar
				:app-name="appName"
				:authenticated="authenticated"
				:display-name="displayName"
				:has-available-update="hasAvailableUpdate"
				:identity="identity"
				:locale-items="localeItems"
				:login-busy="loginBusy"
				:selected-locale="selectedLocale"
				:theme-icon="themeIcon"
				:theme-mode="themeMode"
				:theme-modes="themeModes"
				:t="t"
				@drag-from-aside="dragFromAside"
				@login="startLogin"
				@logout="logout"
				@open-profile="openProfile"
				@select-locale="selectLocale"
				@select-theme="selectTheme"
			/>

			<div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
				<AppWindowTitlebar
					inline
					class="z-30 bg-slate-100 dark:bg-slate-950"
					:close-label="t('close')"
					:minimize-label="t('minimize')"
				>
					<template #left>
						<UpdaterNavigation :tab="tab" :tabs="tabs" @select="tab = $event" />
					</template>
				</AppWindowTitlebar>

				<OverlayScrollbarsComponent
					class="min-h-0 flex-1"
					:options="HYDCRAFT_SCROLLBAR_OPTIONS"
					defer
				>
					<div class="flex min-h-full flex-col">
						<UpdaterContent
							class="min-h-full"
							:authenticated="authenticated"
							:client-versions="clientVersions"
							:client-versions-loading="clientVersionsLoading"
							:conflict-selections="conflictSelections"
							:conflicts="conflicts"
							:current-client-version="currentClientVersion"
							:storage-info="storageInfo"
							:clean-downloads-after-install="cleanDownloadsAfterInstall"
							:downloads-cleaning="downloadsCleaning"
							:backups-cleaning="backupsCleaning"
							:downloads-cleanup-version="downloadsCleanupVersion"
							:backups-cleanup-version="backupsCleanupVersion"
							:context="context"
							:is-bootstrap="isBootstrap"
							:login-busy="loginBusy"
							:phase-subtitle="phaseSubtitle"
							:phase-title="phaseTitle"
							:process-icon="processIcon"
							:selected-source="selectedSource"
							:show-process-spinner="showProcessSpinner"
							:source-items="sourceItems"
							:sources="sources"
							:source-testing="sourceTesting"
							:status="status"
							:tab="tab"
							:t="t"
							@begin-update="beginUpdate"
							@clean-downloads="cleanDownloads"
							@clean-backups="cleanBackups"
							@cancel-conflict-resolution="cancelConflictResolution"
							@launch-client="launchClient"
							@login="startLogin"
							@open-external-url="openExternalUrl"
							@open-client-details="
								openClientDetails($event.version, $event.detail)
							"
							@refresh-clients="refreshClientVersions"
							@refresh-sources="refreshSources"
							@install-client="
								installClientVersion($event.version, $event.mode)
							"
							@rollback-last-update="rollbackLastUpdate"
							@recheck-update="recheckUpdate"
							@resolve-conflicts="resolveConflicts"
							@select-conflict-resolution="selectConflictResolution"
							@select-source="selectSource"
							@set-clean-downloads-after-install="setCleanDownloadsAfterInstall"
							@retry-update="retryUpdate"
						/>
					</div>
				</OverlayScrollbarsComponent>
			</div>
		</main>
	</WebviewInteractionGuard>
</template>
