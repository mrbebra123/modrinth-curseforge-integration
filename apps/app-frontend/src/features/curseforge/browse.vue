<script setup lang="ts">
import { CompassIcon, SearchIcon } from '@modrinth/assets'
import {
	Button,
	DropdownSelect,
	Input,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { list } from '@/helpers/instance'
import { getErrorMessage, install_create_modpack_instance } from '@/helpers/install'
import { install, saveFile, type Project } from '@/platform/app-lib/curseforge'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

import { categoryMessages } from './category-messages'
import { categoriesOptions, filesOptions, searchOptions } from './queries'

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
	contentType: { id: 'app.curseforge.content-type', defaultMessage: 'Content type' },
	category: { id: 'app.curseforge.category', defaultMessage: 'Category' },
	sort: { id: 'app.curseforge.sort', defaultMessage: 'Sort by' },
	enter: { id: 'app.curseforge.enter', defaultMessage: 'Press Enter to search' },
	clear: { id: 'app.curseforge.clear', defaultMessage: 'Clear search' },
	retry: { id: 'app.curseforge.retry', defaultMessage: 'Try again' },
	noFiles: { id: 'app.curseforge.no-files', defaultMessage: 'No compatible files found.' },
})
const categories = useQuery(categoriesOptions())
function categoryLabel(id: number | string | null) {
	const item = categories.data.value?.data.find((item) => String(item.id) === String(id))
	if (!item) return formatMessage(messages.all)
	const message = categoryMessages[item.name as keyof typeof categoryMessages]
	return message ? formatMessage(message) : item.name
}
const sortLabels = computed<Record<string, string>>(() => ({
	'2': formatMessage(messages.popular),
	'3': formatMessage(messages.updated),
	'6': formatMessage(messages.downloads),
}))
function instanceLabel(id: string | null) {
	const item = instances.data.value?.find((item) => item.id === id)
	return item
		? `${item.name} (${item.game_version}, ${item.loader})`
		: formatMessage(messages.instance)
}
const loaderLabels: Record<string, string> = {
	'1': 'Forge',
	'4': 'Fabric',
	'5': 'Quilt',
	'6': 'NeoForge',
}
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
	search.value = text.value.trim()
	page.value = 0
}
const category = ref('')
const sort = ref('2')
const gameVersion = ref('')
const loader = ref('')
const page = ref(0)
const filePage = ref(0)
const selected = ref<Project | null>(null)
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
		enabled: classId.value > 0,
	})),
)
const files = useQuery(
	computed(() => ({
		...filesOptions(selected.value?.id ?? 0, {
			...compatibility.value,
			index: String(filePage.value * 50),
			pageSize: '50',
		}),
		enabled: !!selected.value,
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
		<h1 class="m-0">CurseForge</h1>
		<section class="flex flex-col gap-4 rounded-2xl bg-surface-2 p-4">
			<form @submit.prevent="submitSearch">
				<Input
					v-model="text"
					type="search"
					:icon="SearchIcon"
					:placeholder="formatMessage(messages.search)"
					:aria-label="formatMessage(messages.search)"
					:clear-label="formatMessage(messages.clear)"
					aria-describedby="curseforge-search-hint"
					clearable
					@clear="submitSearch"
				/>
				<p id="curseforge-search-hint" class="mb-0 mt-2 text-sm text-secondary">
					{{ formatMessage(messages.enter) }}
				</p>
			</form>
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
				<div class="flex min-w-0 flex-col gap-2">
					<span id="cf-type-label" class="text-sm font-semibold">
						{{ formatMessage(messages.contentType) }}
					</span>
					<DropdownSelect
						v-model="classId"
						name="cf-type"
						class="!w-full"
						aria-labelledby="cf-type-label"
						:options="classes.map((item) => item.id)"
						:display-name="categoryLabel"
					/>
				</div>
				<div class="flex min-w-0 flex-col gap-2">
					<span id="cf-category-label" class="text-sm font-semibold">
						{{ formatMessage(messages.category) }}
					</span>
					<DropdownSelect
						v-model="category"
						name="cf-category"
						class="!w-full"
						aria-labelledby="cf-category-label"
						:options="['', ...availableCategories.map((item) => String(item.id))]"
						:display-name="categoryLabel"
					/>
				</div>
				<div class="flex min-w-0 flex-col gap-2">
					<span id="cf-sort-label" class="text-sm font-semibold">
						{{ formatMessage(messages.sort) }}
					</span>
					<DropdownSelect
						v-model="sort"
						name="cf-sort"
						class="!w-full"
						aria-labelledby="cf-sort-label"
						:options="Object.keys(sortLabels)"
						:display-name="(value: string) => sortLabels[value]"
					/>
				</div>
				<div v-if="!modpack" class="flex min-w-0 flex-col gap-2">
					<span id="cf-instance-label" class="text-sm font-semibold">
						{{ formatMessage(messages.instance) }}
					</span>
					<DropdownSelect
						v-model="targetId"
						name="cf-instance"
						class="!w-full"
						aria-labelledby="cf-instance-label"
						:options="['', ...(instances.data.value ?? []).map((item) => item.id)]"
						:display-name="instanceLabel"
					/>
				</div>
				<div v-if="!target || modpack" class="flex min-w-0 flex-col gap-2">
					<label for="cf-version" class="text-sm font-semibold">
						{{ formatMessage(messages.game) }}
					</label>
					<Input
						id="cf-version"
						v-model="gameVersion"
						:placeholder="formatMessage(messages.game)"
					/>
				</div>
				<div
					v-if="!target && selectedClass?.slug === 'mc-mods'"
					class="flex min-w-0 flex-col gap-2"
				>
					<span id="cf-loader-label" class="text-sm font-semibold">
						{{ formatMessage(messages.loader) }}
					</span>
					<DropdownSelect
						v-model="loader"
						name="cf-loader"
						class="!w-full"
						aria-labelledby="cf-loader-label"
						:options="['', ...Object.keys(loaderLabels)]"
						:display-name="(value: string) => loaderLabels[value] ?? formatMessage(messages.loader)"
					/>
				</div>
			</div>
		</section>
		<p v-if="categories.error.value || results.error.value" role="alert">
			{{ getErrorMessage(categories.error.value || results.error.value) }}
			<Button
				@click="Promise.all([categories.refetch(), results.refetch()])"
			>
				{{ formatMessage(messages.retry) }}
			</Button>
		</p>
		<p v-else-if="categories.isPending.value || results.isPending.value">
			{{ formatMessage(messages.loading) }}
		</p>
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
				<Button @click="selected = selected?.id === project.id ? null : project">
					{{ formatMessage(messages.versions) }} </Button
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
					<Button
						v-if="!installable"
						:disabled="!file.downloadUrl || downloadFile.isPending.value"
						@click="downloadFile.mutate({ projectId: project.id, fileId: file.id })"
					>
						{{ formatMessage(messages.download) }}
					</Button>
					<Button
						v-if="installable"
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
					</Button>
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
			<Button :disabled="page === 0" @click="page--">
				{{ formatMessage(messages.previous) }} </Button
			><Button
				:disabled="
					!results.data.value?.pagination ||
					(page + 1) * 20 >= Math.min(results.data.value.pagination.totalCount, 10000)
				"
				@click="page++"
			>
				{{ formatMessage(messages.next) }}
			</Button>
		</div>
	</div>
</template>
