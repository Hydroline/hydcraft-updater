<script setup lang="ts">
import AppWindowTitlebar from './components/window/AppWindowTitlebar.vue'
import UpdaterContent from './components/updater/UpdaterContent.vue'
import UpdaterNavigation from './components/updater/UpdaterNavigation.vue'
import UpdaterSidebar from './components/updater/UpdaterSidebar.vue'
import { useUpdaterController } from './composables/useUpdaterController'

const {
	appName,
	authenticated,
	beginUpdate,
	clientVersions,
	context,
	countdown,
	countdownKind,
	displayName,
	dragFromAside,
	handleWindowMouseMove,
	identity,
	interruptCountdown,
	isBootstrap,
	localeItems,
	loginBusy,
	launchClient,
	logout,
	openProfile,
	openExternalUrl,
	phaseSubtitle,
	phaseTitle,
	processIcon,
	selectLocale,
	selectSource,
	selectTheme,
	selectedLocale,
	selectedSource,
	showProcessSpinner,
	skipUpdate,
	sourceItems,
	startLogin,
	status,
	t,
	tab,
	tabs,
	themeIcon,
	themeMode,
	themeModes,
} = useUpdaterController()
</script>

<template>
	<main
		class="relative flex min-h-screen overflow-hidden bg-slate-100 text-slate-950 dark:bg-slate-950 dark:text-white"
		@mousemove="handleWindowMouseMove"
		@keydown="interruptCountdown"
		@click.capture="interruptCountdown"
	>
		<UpdaterSidebar
			:app-name="appName"
			:authenticated="authenticated"
			:display-name="displayName"
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

		<div class="relative flex min-w-0 flex-1 flex-col">
			<AppWindowTitlebar
				:close-label="t('close')"
				:minimize-label="t('minimize')"
			>
				<template #left>
					<UpdaterNavigation :tab="tab" :tabs="tabs" @select="tab = $event" />
				</template>
			</AppWindowTitlebar>

			<UpdaterContent
				:authenticated="authenticated"
				:client-versions="clientVersions"
				:context="context"
				:countdown="countdown"
				:countdown-kind="countdownKind"
				:is-bootstrap="isBootstrap"
				:login-busy="loginBusy"
				:phase-subtitle="phaseSubtitle"
				:phase-title="phaseTitle"
				:process-icon="processIcon"
				:selected-source="selectedSource"
				:show-process-spinner="showProcessSpinner"
				:source-items="sourceItems"
				:status="status"
				:tab="tab"
				:t="t"
				@begin-update="beginUpdate"
				@interrupt-countdown="interruptCountdown"
				@launch-client="launchClient"
				@login="startLogin"
				@open-external-url="openExternalUrl"
				@select-source="selectSource"
				@skip-update="skipUpdate"
			/>
		</div>
	</main>
</template>
