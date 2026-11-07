<script setup lang="ts">
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { ref } from 'vue'

import { setKey } from '@/platform/app-lib/curseforge'

import { configurationOptions } from './queries'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const client = useQueryClient()
const configured = useQuery(configurationOptions())
const key = ref('')
const messages = defineMessages({
	key: { id: 'app.curseforge.api-key', defaultMessage: 'CurseForge API key' },
	description: {
		id: 'app.curseforge.key-description',
		defaultMessage:
			'Enter your own API key to browse and download from CurseForge. The key is stored locally on this computer.',
	},
	configured: { id: 'app.curseforge.configured', defaultMessage: 'API key configured' },
	save: { id: 'app.curseforge.save-key', defaultMessage: 'Save key' },
	remove: { id: 'app.curseforge.remove-key', defaultMessage: 'Remove key' },
})
const save = useMutation({
	mutationFn: setKey,
	onSuccess: async () => {
		key.value = ''
		await client.invalidateQueries({ queryKey: ['curseforge'] })
	},
	onError: handleError,
})
</script>

<template>
	<div class="flex flex-col gap-3 rounded-xl bg-surface-2 p-4">
		<label for="curseforge-api-key" class="font-bold text-primary">{{
			formatMessage(messages.key)
		}}</label>
		<p class="m-0">{{ formatMessage(messages.description) }}</p>
		<span v-if="configured.data.value" class="text-green">{{
			formatMessage(messages.configured)
		}}</span>
		<input
			id="curseforge-api-key"
			v-model="key"
			type="password"
			autocomplete="off"
			spellcheck="false"
			:aria-label="formatMessage(messages.key)"
		/>
		<div class="flex gap-2">
			<ButtonStyled
				><button :disabled="!key.trim() || save.isPending.value" @click="save.mutate(key)">
					{{ formatMessage(messages.save) }}
				</button></ButtonStyled
			>
			<ButtonStyled
				><button
					:disabled="!configured.data.value || save.isPending.value"
					@click="save.mutate('')"
				>
					{{ formatMessage(messages.remove) }}
				</button></ButtonStyled
			>
		</div>
	</div>
</template>
