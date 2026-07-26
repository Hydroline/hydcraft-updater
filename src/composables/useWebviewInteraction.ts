import { computed, onBeforeUnmount, onMounted, shallowRef } from 'vue'
import { useUpdaterAppearance } from './useUpdaterAppearance'

type EditableElement = HTMLInputElement | HTMLTextAreaElement | HTMLElement

interface ContextSelection {
	element: EditableElement | null
	start: number | null
	end: number | null
}

const EDITING_SHORTCUTS = new Set(['a', 'c', 'v', 'x', 'z', 'y'])

function isEditableElement(
	target: EventTarget | null,
): target is EditableElement {
	if (target instanceof HTMLTextAreaElement)
		return !target.disabled && !target.readOnly
	if (target instanceof HTMLInputElement) {
		return (
			!target.disabled &&
			!target.readOnly &&
			![
				'button',
				'checkbox',
				'file',
				'hidden',
				'image',
				'radio',
				'range',
				'reset',
				'submit',
			].includes(target.type)
		)
	}
	return target instanceof HTMLElement && target.isContentEditable
}

function isTextInput(
	element: EditableElement | null,
): element is HTMLInputElement | HTMLTextAreaElement {
	return (
		element instanceof HTMLInputElement ||
		element instanceof HTMLTextAreaElement
	)
}

export function useWebviewInteraction() {
	const { t } = useUpdaterAppearance()
	const contextSelection = shallowRef<ContextSelection>({
		element: null,
		start: null,
		end: null,
	})

	const hasSelection = computed(() => {
		const saved = contextSelection.value
		if (isTextInput(saved.element))
			return (
				saved.start !== null && saved.end !== null && saved.start !== saved.end
			)
		return Boolean(window.getSelection()?.toString())
	})
	const hasEditableTarget = computed(() =>
		Boolean(contextSelection.value.element),
	)

	function rememberContextSelection(event: MouseEvent): void {
		const element = isEditableElement(event.target) ? event.target : null
		contextSelection.value = {
			element,
			start: isTextInput(element) ? element.selectionStart : null,
			end: isTextInput(element) ? element.selectionEnd : null,
		}
	}

	function restoreContextSelection(): EditableElement | null {
		const saved = contextSelection.value
		if (!saved.element) return null
		saved.element.focus({ preventScroll: true })
		if (
			isTextInput(saved.element) &&
			saved.start !== null &&
			saved.end !== null
		)
			saved.element.setSelectionRange(saved.start, saved.end)
		return saved.element
	}

	function copySelection(): void {
		if (!hasSelection.value) return
		restoreContextSelection()
		document.execCommand('copy')
	}

	async function pasteSelection(): Promise<void> {
		const element = restoreContextSelection()
		if (!element) return
		let text: string
		try {
			text = await navigator.clipboard.readText()
		} catch {
			return
		}
		if (
			isTextInput(element) &&
			element.selectionStart !== null &&
			element.selectionEnd !== null
		) {
			element.setRangeText(
				text,
				element.selectionStart,
				element.selectionEnd,
				'end',
			)
			element.dispatchEvent(new Event('input', { bubbles: true }))
			return
		}
		document.execCommand('insertText', false, text)
	}

	function selectAll(): void {
		restoreContextSelection()
		document.execCommand('selectAll')
	}

	function handleKeydown(event: KeyboardEvent): void {
		const key = event.key.toLowerCase()
		const functionKey = /^f(?:[1-9]|1[0-2])$/.test(event.key)
		const browserNavigation =
			(event.altKey && ['arrowleft', 'arrowright'].includes(key)) ||
			['browserback', 'browserforward'].includes(key)
		const modifierShortcut = event.ctrlKey || event.metaKey
		const blockedModifierShortcut =
			modifierShortcut &&
			(event.altKey ||
				(event.shiftKey && key !== 'v') ||
				!EDITING_SHORTCUTS.has(key))
		if (functionKey || browserNavigation || blockedModifierShortcut) {
			event.preventDefault()
			event.stopPropagation()
		}
	}

	const contextMenuItems = computed(() => [
		{
			label: t('copy'),
			icon: 'i-lucide-copy',
			disabled: !hasSelection.value,
			onSelect: copySelection,
		},
		{
			label: t('paste'),
			icon: 'i-lucide-clipboard-paste',
			disabled: !hasEditableTarget.value,
			onSelect: () => void pasteSelection(),
		},
		{
			label: t('selectAll'),
			icon: 'i-lucide-list-checks',
			onSelect: selectAll,
		},
	])

	onMounted(() => {
		document.addEventListener('contextmenu', rememberContextSelection, true)
		document.addEventListener('keydown', handleKeydown, true)
	})
	onBeforeUnmount(() => {
		document.removeEventListener('contextmenu', rememberContextSelection, true)
		document.removeEventListener('keydown', handleKeydown, true)
	})

	return { contextMenuItems }
}
