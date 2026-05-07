<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import ControlRow from '$lib/components/ControlRow.svelte';
	import { Icon, ChevronRight, MagnifyingGlass } from 'svelte-hero-icons';
	import { cn } from '$lib/utils';
	import type { GraphData, GraphNode } from '$lib/graph-types';

	let {
		graphData,
		selected = null,
		error = null,
		colors,
		search = $bindable(''),
		showOrphans = $bindable(true),
		showArrows = $bindable(false),
		textFadeThreshold = $bindable(6),
		nodeSize = $bindable(1.0),
		linkThickness = $bindable(0.8),
		centerForce = $bindable(0.05),
		repelForce = $bindable(800),
		linkForce = $bindable(0.3),
		linkDistance = $bindable(100),
		onFilterChange
	}: {
		graphData: GraphData;
		selected?: GraphNode | null;
		error?: string | null;
		colors: Record<string, string>;
		search: string;
		showOrphans: boolean;
		showArrows: boolean;
		textFadeThreshold: number;
		nodeSize: number;
		linkThickness: number;
		centerForce: number;
		repelForce: number;
		linkForce: number;
		linkDistance: number;
		onFilterChange?: () => void;
	} = $props();

	const languageEntries = $derived(
		Array.from(new Set(graphData.nodes.map((n) => n.language))).map((language) => ({
			language,
			color: colors[language] ?? colors.Unknown
		}))
	);

	let filtersOpen = $state(true);
	let displayOpen = $state(true);
	let forcesOpen = $state(true);
</script>

<aside
	class="absolute top-0 right-0 bottom-0 w-64 flex flex-col bg-neutral-900 border-l border-neutral-800"
>
	<!-- Header -->
	<div
		class="shrink-0 px-4 pt-4 pb-3 border-b border-neutral-800 bg-gradient-to-br from-neutral-800/40 to-transparent"
	>
		<h1 class="text-sm font-bold tracking-tight text-neutral-100">modkei</h1>
		<p class="mt-1 text-xs text-neutral-600">
			{graphData.nodes.length} files · {graphData.edges.length} imports
		</p>
	</div>

	<!-- Scrollable body -->
	<ScrollArea class="flex-1 min-h-0">
		<!-- Filters -->
		<button
			class="flex w-full items-center gap-1.5 border-t border-neutral-800/60 px-4 py-2.5 text-[10px] font-bold uppercase tracking-widest text-neutral-600 transition-colors hover:text-neutral-400"
			onclick={() => (filtersOpen = !filtersOpen)}
		>
			<span class={cn('text-neutral-700 transition-transform duration-200', filtersOpen && 'rotate-90')}>
				<Icon src={ChevronRight} size="12" />
			</span>
			Filters
		</button>

		{#if filtersOpen}
			<div class="flex flex-col gap-3 px-4 pb-4">
				<div class="relative">
					<span
						class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-600"
					>
						<Icon src={MagnifyingGlass} size="13" />
					</span>
					<Input
						class="h-8 border-neutral-800 bg-neutral-950 pl-8 text-xs text-neutral-300 placeholder:text-neutral-600 focus-visible:border-neutral-700 focus-visible:ring-0"
						placeholder="Search files…"
						bind:value={search}
						oninput={onFilterChange}
					/>
				</div>
				<div class="flex items-center justify-between">
					<span class="text-xs text-neutral-500">Show orphans</span>
					<Switch bind:checked={showOrphans} />
				</div>
			</div>
		{/if}

		<!-- Display -->
		<button
			class="flex w-full items-center gap-1.5 border-t border-neutral-800/60 px-4 py-2.5 text-[10px] font-bold uppercase tracking-widest text-neutral-600 transition-colors hover:text-neutral-400"
			onclick={() => (displayOpen = !displayOpen)}
		>
			<span
				class={cn('text-neutral-700 transition-transform duration-200', displayOpen && 'rotate-90')}
			>
				<Icon src={ChevronRight} size="12" />
			</span>
			Display
		</button>

		{#if displayOpen}
			<div class="flex flex-col gap-4 px-4 pb-4">
				<div class="flex items-center justify-between">
					<span class="text-xs text-neutral-500">Arrows</span>
					<Switch bind:checked={showArrows} />
				</div>
				<ControlRow label="Text fade" value={String(textFadeThreshold)}>
					<Slider type="single" bind:value={textFadeThreshold} min={0} max={24} step={1} />
				</ControlRow>
				<ControlRow label="Node size" value={nodeSize.toFixed(2)}>
					<Slider type="single" bind:value={nodeSize} min={0.3} max={3} step={0.05} />
				</ControlRow>
				<ControlRow label="Link thickness" value={linkThickness.toFixed(1)}>
					<Slider type="single" bind:value={linkThickness} min={0.2} max={5} step={0.1} />
				</ControlRow>
			</div>
		{/if}

		<!-- Forces -->
		<button
			class="flex w-full items-center gap-1.5 border-t border-neutral-800/60 px-4 py-2.5 text-[10px] font-bold uppercase tracking-widest text-neutral-600 transition-colors hover:text-neutral-400"
			onclick={() => (forcesOpen = !forcesOpen)}
		>
			<span
				class={cn('text-neutral-700 transition-transform duration-200', forcesOpen && 'rotate-90')}
			>
				<Icon src={ChevronRight} size="12" />
			</span>
			Forces
		</button>

		{#if forcesOpen}
			<div class="flex flex-col gap-4 px-4 pb-4">
				<ControlRow label="Center force" value={centerForce.toFixed(2)}>
					<Slider type="single" bind:value={centerForce} min={0} max={5} step={0.01} />
				</ControlRow>
				<ControlRow label="Repel force" value={String(Math.round(repelForce))}>
					<Slider type="single" bind:value={repelForce} min={0} max={10000} step={1} />
				</ControlRow>
				<ControlRow label="Link force" value={linkForce.toFixed(2)}>
					<Slider type="single" bind:value={linkForce} min={0} max={5} step={0.01} />
				</ControlRow>
				<ControlRow label="Link distance" value={String(Math.round(linkDistance))}>
					<Slider type="single" bind:value={linkDistance} min={0} max={1000} step={1} />
				</ControlRow>
			</div>
		{/if}

		<!-- Selected node card -->
		{#if selected}
			<Separator class="bg-neutral-800/60" />
			<div class="mx-4 my-3 flex flex-col gap-1.5 rounded-lg border border-neutral-800 bg-neutral-950 p-3">
				<p class="break-all text-xs font-semibold leading-snug text-neutral-200">{selected.label}</p>
				<div class="flex justify-between text-[11px]">
					<span class="text-neutral-600">Language</span>
					<span style="color:{colors[selected.language] ?? colors.Unknown}">{selected.language}</span>
				</div>
				<div class="flex justify-between text-[11px]">
					<span class="text-neutral-600">Lines</span>
					<span class="tabular-nums text-neutral-400">{selected.lines.toLocaleString()}</span>
				</div>
				<div class="flex justify-between text-[11px]">
					<span class="text-neutral-600">Code lines</span>
					<span class="tabular-nums text-neutral-400">{selected.code.toLocaleString()}</span>
				</div>
			</div>
		{/if}

		<!-- Language legend -->
		{#if languageEntries.length > 0}
			<div class="mx-4 mb-4 mt-1 flex flex-wrap gap-1.5">
				{#each languageEntries as entry (entry.language)}
					<span
						class="inline-flex items-center gap-1.5 rounded-full border border-neutral-800 bg-neutral-950 px-2 py-0.5 text-[11px] text-neutral-600"
					>
						<span class="h-1.5 w-1.5 shrink-0 rounded-full" style="background:{entry.color}"></span>
						{entry.language}
					</span>
				{/each}
			</div>
		{/if}
	</ScrollArea>

	{#if error}
		<div class="shrink-0 border-t border-red-900/50 bg-red-950/50 px-4 py-2.5 text-xs text-red-400">
			{error}
		</div>
	{/if}
</aside>
