import {
	configured,
	request,
	type Category,
	type File,
	type Project,
} from '@/platform/app-lib/curseforge'

export const configurationOptions = () => ({
	queryKey: ['curseforge', 'configured'] as const,
	queryFn: configured,
	networkMode: 'always' as const,
})

export const categoriesOptions = () => ({
	queryKey: ['curseforge', 'categories'] as const,
	queryFn: () => request<Category[]>('categories', { gameId: '432' }),
	staleTime: 60 * 60 * 1000,
})

export const searchOptions = (query: Record<string, string>) => ({
	queryKey: ['curseforge', 'search', query] as const,
	queryFn: () => request<Project[]>('mods/search', { gameId: '432', ...query }),
	staleTime: 60 * 1000,
})

export const filesOptions = (id: number, query: Record<string, string>) => ({
	queryKey: ['curseforge', 'files', id, query] as const,
	queryFn: () => request<File[]>(`mods/${id}/files`, query),
	staleTime: 60 * 1000,
})
