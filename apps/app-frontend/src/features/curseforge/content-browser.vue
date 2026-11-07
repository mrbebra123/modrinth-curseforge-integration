<script setup lang="ts">
import { useStorage } from '@vueuse/core'

import ModrinthBrowse from '@/platform/adapters/browse/browse-page.vue'

import CurseForgeBrowse from './browse.vue'

const source = useStorage<'modrinth' | 'curseforge'>('content-source', 'modrinth')
</script>

<template>
	<div>
		<div class="mx-6 mt-4 flex gap-2" role="group" aria-label="Modrinth / CurseForge">
			<button
				v-for="provider in ['modrinth', 'curseforge'] as const"
				:key="provider"
				class="rounded-lg px-4 py-2 font-bold"
				:class="source === provider ? 'bg-brand-highlight text-brand' : 'bg-surface-2'"
				:aria-pressed="source === provider"
				@click="source = provider"
			>
				{{ provider === 'modrinth' ? 'Modrinth' : 'CurseForge' }}
			</button>
		</div>
		<CurseForgeBrowse v-if="source === 'curseforge'" />
		<ModrinthBrowse v-else />
	</div>
</template>
