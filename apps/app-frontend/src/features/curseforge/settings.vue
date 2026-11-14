<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'

const { formatMessage } = useVIntl()
const messages = defineMessages({
	description: {
		id: 'app.curseforge.proxy-description',
		defaultMessage:
			'CurseForge is connected through the application server. No personal API key is needed.',
	},
})
</script>

<template>
	<div class="rounded-xl bg-surface-2 p-4">
		<p class="m-0">{{ formatMessage(messages.description) }}</p>
	</div>
</template>
