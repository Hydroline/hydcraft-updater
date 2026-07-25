<script setup lang="ts">
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { computed } from 'vue'

const props = defineProps<{
	content: string
}>()

const renderedMarkdown = computed(() =>
	DOMPurify.sanitize(
		marked.parse(props.content, { async: false, breaks: true }),
	),
)
</script>

<template>
	<div class="markdown-content" v-html="renderedMarkdown" />
</template>

<style scoped>
.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3) {
	margin: 1.25em 0 0.5em;
	font-weight: 600;
}

.markdown-content :deep(h1) {
	font-size: 1.25em;
}

.markdown-content :deep(h2) {
	font-size: 1.125em;
}

.markdown-content :deep(h3) {
	font-size: 1em;
}

.markdown-content :deep(:first-child) {
	margin-top: 0;
}

.markdown-content :deep(:last-child) {
	margin-bottom: 0;
}

.markdown-content :deep(p),
.markdown-content :deep(ul),
.markdown-content :deep(ol),
.markdown-content :deep(pre),
.markdown-content :deep(blockquote) {
	margin: 0.75em 0;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
	padding-left: 1.5em;
}

.markdown-content :deep(ul) {
	list-style: disc;
}

.markdown-content :deep(ol) {
	list-style: decimal;
}

.markdown-content :deep(code) {
	border-radius: 0.25rem;
	background: rgb(148 163 184 / 16%);
	padding: 0.1em 0.3em;
	font-family: var(--font-mono, monospace);
}

.markdown-content :deep(pre) {
	overflow-x: auto;
	border-radius: 0.375rem;
	background: rgb(15 23 42 / 8%);
	padding: 0.75rem;
}

.markdown-content :deep(pre code) {
	background: transparent;
	padding: 0;
}

.markdown-content :deep(a) {
	color: var(--ui-color-primary-500);
	text-decoration: underline;
}

.dark .markdown-content :deep(code) {
	background: rgb(148 163 184 / 18%);
}

.dark .markdown-content :deep(pre) {
	background: rgb(15 23 42 / 55%);
}
</style>
