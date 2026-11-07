<script setup lang="ts">
import { CompassIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { useRootBreadcrumb } from '@/shared/breadcrumbs'
import { list } from '@/helpers/instance'
import { getErrorMessage, install_create_modpack_instance } from '@/helpers/install'
import { install, saveFile, type Project } from '@/platform/app-lib/curseforge'

import { categoriesOptions, configurationOptions, filesOptions, searchOptions } from './queries'
import Settings from './settings.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const client = useQueryClient()
useRootBreadcrumb({
	slot: 'root',
	id: 'curseforge',
	label: 'CurseForge',
	visual: { type: 'icon', component: CompassIcon },
})
const route = useRoute()
const router = useRouter()
const messages = defineMessages({
	download: { id: 'app.curseforge.download-file', defaultMessage: 'Download file' },
	search: { id: 'app.curseforge.search', defaultMessage: 'Search CurseForge' },
	searchButton: { id: 'app.curseforge.search-button', defaultMessage: 'Search' },
	loading: { id: 'app.curseforge.loading', defaultMessage: 'Loading…' },
	empty: { id: 'app.curseforge.empty', defaultMessage: 'No matching projects found.' },
	instance: { id: 'app.curseforge.instance', defaultMessage: 'Select an instance' },
	versions: { id: 'app.curseforge.versions', defaultMessage: 'Versions' },
	install: { id: 'app.curseforge.install', defaultMessage: 'Install' },
	installing: { id: 'app.curseforge.installing', defaultMessage: 'Downloading and installing…' },
	installed: { id: 'app.curseforge.installed', defaultMessage: 'Installed' },
	website: { id: 'app.curseforge.website', defaultMessage: 'Open on CurseForge' },
	restricted: {
		id: 'app.curseforge.restricted',
		defaultMessage: 'The author has disabled downloads through third-party launchers.',
	},
	previous: { id: 'app.curseforge.previous', defaultMessage: 'Previous' },
	next: { id: 'app.curseforge.next', defaultMessage: 'Next' },
	all: { id: 'app.curseforge.all-categories', defaultMessage: 'All categories' },
	popular: { id: 'app.curseforge.popular', defaultMessage: 'Popularity' },
	updated: { id: 'app.curseforge.updated', defaultMessage: 'Recently updated' },
	downloads: { id: 'app.curseforge.downloads', defaultMessage: 'Downloads' },
	game: { id: 'app.curseforge.game-version', defaultMessage: 'Minecraft version' },
	loader: { id: 'app.curseforge.loader', defaultMessage: 'All loaders' },
	settings: { id: 'app.curseforge.settings', defaultMessage: 'API settings' },
	noFiles: { id: 'app.curseforge.no-files', defaultMessage: 'No compatible files found.' },
})
const configured = useQuery(configurationOptions())
const categories = useQuery({
	...categoriesOptions(),
	enabled: computed(() => !!configured.data.value),
})
const instances = useQuery({
	queryKey: ['instances', 'curseforge-targets'],
	queryFn: list,
	networkMode: 'always',
})
const targetId = ref(String(route.query.i ?? ''))
watch(
	() => route.query.i,
	(id) => {
		targetId.value = String(id ?? '')
	},
)
const target = computed(() => instances.data.value?.find((item) => item.id === targetId.value))
const classes = computed(() => categories.data.value?.data.filter((item) => item.isClass) ?? [])
const routeSlugs: Record<string, string> = {
	mod: 'mc-mods',
	modpack: 'modpacks',
	resourcepack: 'texture-packs',
	shader: 'shaders',
	datapack: 'data-packs',
	plugin: 'bukkit-plugins',
}
const classId = ref(0)
watch(
	[classes, () => route.params.projectType],
	() => {
		classId.value =
			classes.value.find((item) => item.slug === routeSlugs[String(route.params.projectType)])
				?.id ??
			classes.value[0]?.id ??
			0
	},
	{ immediate: true },
)
const selectedClass = computed(() => classes.value.find((item) => item.id === classId.value))
const modpack = computed(() => selectedClass.value?.slug === 'modpacks')
const installable = computed(() =>
	['mc-mods', 'modpacks', 'texture-packs', 'shaders', 'data-packs'].includes(
		selectedClass.value?.slug ?? '',
	),
)
const availableCategories = computed(
	() =>
		categories.data.value?.data.filter((item) => !item.isClass && item.classId === classId.value) ??
		[],
)
const text = ref('')
const search = ref('')
function submitSearch() {
	search.value = text.value
	page.value = 0
}
const category = ref('')
const sort = ref('2')
const gameVersion = ref('')
const loader = ref('')
const page = ref(0)
const filePage = ref(0)
const selected = ref<Project | null>(null)
const showSettings = ref(false)
const installed = ref(new Set<string>())
const loaderIds: Record<string, string> = { forge: '1', fabric: '4', quilt: '5', neoforge: '6' }
const compatibility = computed(() => {
	const query: Record<string, string> = {}
	const version =
		!modpack.value && target.value ? target.value.game_version : gameVersion.value.trim()
	if (version) query.gameVersion = version
	if (selectedClass.value?.slug === 'mc-mods') {
		const value = target.value ? loaderIds[target.value.loader] : loader.value
		if (value) query.modLoaderType = value
	}
	return query
})
watch(classId, () => {
	category.value = ''
	selected.value = null
})
watch([search, classId, category, sort, compatibility], () => {
	page.value = 0
})
watch([selected, compatibility], () => {
	filePage.value = 0
})
const results = useQuery(
	computed(() => ({
		...searchOptions({
			...compatibility.value,
			classId: String(classId.value),
			...(category.value ? { categoryId: category.value } : {}),
			searchFilter: search.value,
			sortField: sort.value,
			sortOrder: 'desc',
			index: String(page.value * 20),
			pageSize: '20',
		}),
		enabled: !!configured.data.value && classId.value > 0,
	})),
)
const files = useQuery(
	computed(() => ({
		...filesOptions(selected.value?.id ?? 0, {
			...compatibility.value,
			index: String(filePage.value * 50),
			pageSize: '50',
		}),
		enabled: !!configured.data.value && !!selected.value,
	})),
)
const downloadFile = useMutation({
	mutationFn: ({ projectId, fileId }: { projectId: number; fileId: number }) =>
		saveFile(projectId, fileId),
	onError: handleError,
})
const installFile = useMutation({
	mutationFn: async ({
		projectId,
		fileId,
		instanceId,
	}: {
		projectId: number
		fileId: number
		instanceId: string
	}) => {
		const result = await install(projectId, fileId, instanceId)
		if (result.packPath) {
			const job = await install_create_modpack_instance({ type: 'fromFile', path: result.packPath })
			if (job.instance_id) await router.push(`/instance/${job.instance_id}`)
		}
		return result
	},
	onSuccess: async (result, variables) => {
		if (result.instanceId)
			installed.value.add(`${variables.instanceId}:${variables.projectId}:${variables.fileId}`)
		await client.invalidateQueries({ queryKey: ['instances'] })
	},
	onError: handleError,
})
</script>

<template>
	<div class="flex flex-col gap-4 p-6">
		<div class="flex items-center justify-between">
			<h1 class="m-0">CurseForge</h1>
			<button @click="showSettings = !showSettings">{{ formatMessage(messages.settings) }}</button>
		</div>
		<Settings v-if="!configured.data.value || showSettings" />
		<template v-if="configured.data.value">
			<form class="flex flex-wrap gap-3" @submit.prevent="submitSearch">
				<input
					v-model="text"
					class="grow"
					:placeholder="formatMessage(messages.search)"
					:aria-label="formatMessage(messages.search)"
				/>
				<ButtonStyled
					><button type="submit">{{ formatMessage(messages.searchButton) }}</button></ButtonStyled
				>
			</form>
			<div class="flex flex-wrap gap-3">
				<select v-model="classId" aria-label="CurseForge">
					<option v-for="item in classes" :key="item.id" :value="item.id">{{ item.name }}</option>
				</select>
				<select v-model="category" :aria-label="formatMessage(messages.all)">
					<option value="">{{ formatMessage(messages.all) }}</option>
					<option v-for="item in availableCategories" :key="item.id" :value="String(item.id)">
						{{ item.name }}
					</option>
				</select>
				<select v-model="sort" :aria-label="formatMessage(messages.popular)">
					<option value="2">{{ formatMessage(messages.popular) }}</option>
					<option value="3">{{ formatMessage(messages.updated) }}</option>
					<option value="6">{{ formatMessage(messages.downloads) }}</option>
				</select>
				<select v-if="!modpack" v-model="targetId" :aria-label="formatMessage(messages.instance)">
					<option value="">{{ formatMessage(messages.instance) }}</option>
					<option v-for="item in instances.data.value" :key="item.id" :value="item.id">
						{{ item.name }} ({{ item.game_version }}, {{ item.loader }})
					</option>
				</select>
				<input
					v-if="!target || modpack"
					v-model="gameVersion"
					:placeholder="formatMessage(messages.game)"
					:aria-label="formatMessage(messages.game)"
				/>
				<select
					v-if="!target && selectedClass?.slug === 'mc-mods'"
					v-model="loader"
					:aria-label="formatMessage(messages.loader)"
				>
					<option value="">{{ formatMessage(messages.loader) }}</option>
					<option v-for="(id, name) in loaderIds" :key="id" :value="id">{{ name }}</option>
				</select>
			</div>
			<p v-if="categories.error.value || results.error.value" role="alert">
				{{ getErrorMessage(categories.error.value || results.error.value) }}
			</p>
			<p v-else-if="results.isPending.value">{{ formatMessage(messages.loading) }}</p>
			<p v-else-if="!results.data.value?.data.length">{{ formatMessage(messages.empty) }}</p>
			<article
				v-for="project in results.data.value?.data"
				:key="project.id"
				class="flex flex-col gap-3 rounded-xl bg-surface-2 p-4"
			>
				<div class="flex gap-4">
					<img
						v-if="project.logo"
						:src="project.logo.thumbnailUrl"
						alt=""
						class="h-20 w-20 rounded-xl object-cover"
						loading="lazy"
					/>
					<div class="min-w-0 grow">
						<h2 class="m-0 text-lg">{{ project.name }}</h2>
						<p class="my-1 text-sm">
							{{ project.authors.map((author) => author.name).join(', ') }} ·
							{{ project.downloadCount.toLocaleString() }} {{ formatMessage(messages.downloads) }}
						</p>
						<p class="m-0">{{ project.summary }}</p>
					</div>
				</div>
				<div class="flex gap-3">
					<ButtonStyled
						><button @click="selected = selected?.id === project.id ? null : project">
							{{ formatMessage(messages.versions) }}
						</button></ButtonStyled
					><a :href="project.links.websiteUrl" target="_blank" rel="noopener noreferrer">{{
						formatMessage(messages.website)
					}}</a>
				</div>
				<div v-if="selected?.id === project.id" class="flex flex-col gap-2">
					<p v-if="files.error.value" role="alert">{{ getErrorMessage(files.error.value) }}</p>
					<p v-else-if="files.isPending.value">{{ formatMessage(messages.loading) }}</p>
					<p v-else-if="!files.data.value?.data.length">{{ formatMessage(messages.noFiles) }}</p>
					<div
						v-for="file in files.data.value?.data"
						:key="file.id"
						class="flex items-center justify-between gap-3 rounded-lg bg-surface-3 p-3"
					>
						<div>
							{{ file.displayName }}
							<p class="m-0 text-sm">{{ file.gameVersions.join(', ') }}</p>
							<p v-if="!file.downloadUrl" class="m-0">{{ formatMessage(messages.restricted) }}</p>
						</div>
						<ButtonStyled v-if="!installable">
							<button
								:disabled="!file.downloadUrl || downloadFile.isPending.value"
								@click="downloadFile.mutate({ projectId: project.id, fileId: file.id })"
							>
								{{ formatMessage(messages.download) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="installable"
							><button
								:disabled="
									!file.downloadUrl ||
									(!modpack && !targetId) ||
									installFile.isPending.value ||
									installed.has(`${targetId}:${project.id}:${file.id}`)
								"
								@click="
									installFile.mutate({
										projectId: project.id,
										fileId: file.id,
										instanceId: targetId,
									})
								"
							>
								{{
									formatMessage(
										installed.has(`${targetId}:${project.id}:${file.id}`)
											? messages.installed
											: messages.install,
									)
								}}
							</button></ButtonStyled
						>
					</div>
					<div class="flex gap-3">
						<button :disabled="filePage === 0" @click="filePage--">
							{{ formatMessage(messages.previous) }}</button
						><button
							:disabled="
								!files.data.value?.pagination ||
								(filePage + 1) * 50 >= Math.min(files.data.value.pagination.totalCount, 10000)
							"
							@click="filePage++"
						>
							{{ formatMessage(messages.next) }}
						</button>
					</div>
				</div>
			</article>
			<p v-if="installFile.isPending.value" role="status">
				{{ formatMessage(messages.installing) }}
			</p>
			<div class="flex justify-between">
				<ButtonStyled
					><button :disabled="page === 0" @click="page--">
						{{ formatMessage(messages.previous) }}
					</button></ButtonStyled
				><ButtonStyled
					><button
						:disabled="
							!results.data.value?.pagination ||
							(page + 1) * 20 >= Math.min(results.data.value.pagination.totalCount, 10000)
						"
						@click="page++"
					>
						{{ formatMessage(messages.next) }}
					</button></ButtonStyled
				>
			</div>
		</template>
	</div>
</template>
