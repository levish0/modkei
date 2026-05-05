<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Graph from 'graphology';
	import Sigma from 'sigma';
	import {
		forceCenter,
		forceLink,
		forceManyBody,
		forceSimulation,
		type ForceLink,
		type Simulation
	} from 'd3-force';
	import ControlRow from '$lib/components/ControlRow.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Separator } from '$lib/components/ui/separator';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import * as Tabs from '$lib/components/ui/tabs';

	type Language = 'Rust' | 'TypeScript' | 'JavaScript' | 'Python' | 'Go' | 'Unknown';
	type GraphNode = { id: string; label: string; language: Language; lines: number; code: number };
	type GraphEdge = { source: string; target: string; label: string };
	type GraphData = { nodes: GraphNode[]; edges: GraphEdge[] };
	type SimNode = { id: string; x: number; y: number; fx?: number | null; fy?: number | null };
	type SimLink = { source: string | SimNode; target: string | SimNode };

	const colors: Record<string, string> = {
		Rust: '#dea584',
		TypeScript: '#3178c6',
		JavaScript: '#f7df1e',
		Python: '#ffd343',
		Go: '#00add8',
		Unknown: '#aab2c0'
	};

	let container: HTMLDivElement;
	let graphData = $state<GraphData>({ nodes: [], edges: [] });
	let error = $state<string | null>(null);
	let selected = $state('No file selected.');
	let search = $state('');
	let showOrphans = $state(true);
	let showArrows = $state(false);
	let textFadeThreshold = $state(8);
	let nodeSize = $state(1);
	let linkThickness = $state(1.8);
	let centerForce = $state(0.28);
	let repelForce = $state(360);
	let linkForce = $state(0.18);
	let linkDistance = $state(70);

	let graph: Graph | null = null;
	let renderer: Sigma | null = null;
	let simulation: Simulation<SimNode, SimLink> | null = null;
	let controlsReady = false;
	let simulationNodes: SimNode[] = [];
	let simulationNodeById = new Map<string, SimNode>();
	let focused: string | null = null;
	let draggedNode: string | null = null;

	const languageEntries = $derived(
		Array.from(new Set(graphData.nodes.map((node) => node.language))).map((language) => ({
			language,
			color: colors[language] ?? colors.Unknown
		}))
	);

	$effect(() => {
		textFadeThreshold;
		nodeSize;
		linkThickness;
		showArrows;
		showOrphans;
		search;
		if (controlsReady) applyDisplay();
	});

	$effect(() => {
		centerForce;
		repelForce;
		linkForce;
		linkDistance;
		if (controlsReady) applyForces();
	});

	onMount(async () => {
		try {
			const response = await fetch('./graph.json');
			if (!response.ok) throw new Error(`failed to load graph.json (${response.status})`);
			graphData = (await response.json()) as GraphData;
			initGraph(graphData);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	});

	onDestroy(() => {
		simulation?.stop();
		renderer?.kill();
	});

	function initGraph(data: GraphData) {
		renderer?.kill();
		simulation?.stop();

		const nextGraph = new Graph({ type: 'directed' });
		const degree = new Map<string, number>();
		data.nodes.forEach((node) => degree.set(node.id, 0));
		data.edges.forEach((edge) => {
			degree.set(edge.source, (degree.get(edge.source) ?? 0) + 1);
			degree.set(edge.target, (degree.get(edge.target) ?? 0) + 1);
		});

		data.nodes.forEach((node, index) => {
			const angle = (index / Math.max(1, data.nodes.length)) * Math.PI * 2;
			const color = colors[node.language] ?? colors.Unknown;
			const nodeDegree = degree.get(node.id) ?? 0;
			const baseSize = 3.4 + Math.sqrt(nodeDegree + 1) * 1.8 + Math.log2(node.code + 2) * 0.35;
			nextGraph.addNode(node.id, {
				label: node.label,
				language: node.language,
				degree: nodeDegree,
				baseColor: color,
				baseSize,
				size: baseSize * nodeSize,
				color,
				labelColor: '#f8fafc',
				x: Math.cos(angle) * 180 + Math.random(),
				y: Math.sin(angle) * 180 + Math.random(),
				hidden: false
			});
		});

		data.edges.forEach((edge, index) => {
			if (!nextGraph.hasNode(edge.source) || !nextGraph.hasNode(edge.target)) return;
			if (nextGraph.hasDirectedEdge(edge.source, edge.target)) return;
			nextGraph.addDirectedEdgeWithKey(
				`${edge.source}->${edge.target}:${index}`,
				edge.source,
				edge.target,
				{
					label: edge.label,
					color: '#8c96aa',
					baseColor: '#8c96aa',
					size: linkThickness,
					baseSize: linkThickness,
					type: showArrows ? 'arrow' : 'line'
				}
			);
		});

		graph = nextGraph;
		renderer = new Sigma(nextGraph, container, {
			defaultEdgeColor: '#8c96aa',
			defaultEdgeType: 'line',
			labelColor: { color: '#f8fafc' },
			labelWeight: '600',
			labelSize: 13,
			labelRenderedSizeThreshold: textFadeThreshold,
			defaultDrawNodeLabel: drawLabel,
			defaultDrawNodeHover: drawHover,
			renderEdgeLabels: false
		});
		wireRenderer();
		startSimulation(data);
		applyDisplay();
		applyFilter();
		controlsReady = true;
	}

	function startSimulation(data: GraphData) {
		if (!graph) return;
		simulationNodes = data.nodes.map((node) => ({
			id: node.id,
			x: graph!.getNodeAttribute(node.id, 'x') as number,
			y: graph!.getNodeAttribute(node.id, 'y') as number
		}));
		simulationNodeById = new Map(simulationNodes.map((node) => [node.id, node]));
		const links = data.edges
			.filter((edge) => simulationNodeById.has(edge.source) && simulationNodeById.has(edge.target))
			.map((edge) => ({ source: edge.source, target: edge.target }));

		simulation = forceSimulation<SimNode>(simulationNodes)
			.alpha(1)
			.alphaDecay(0.035)
			.velocityDecay(0.48)
			.force('center', forceCenter(0, 0).strength(centerForce))
			.force('charge', forceManyBody<SimNode>().strength(-repelForce))
			.force(
				'link',
				forceLink<SimNode, SimLink>(links)
					.id((node: SimNode) => node.id)
					.strength(linkForce)
					.distance(linkDistance)
			)
			.on('tick', () => {
				if (!graph || !renderer) return;
				for (const node of simulationNodes) {
					if (!graph.hasNode(node.id)) continue;
					graph.setNodeAttribute(node.id, 'x', node.x);
					graph.setNodeAttribute(node.id, 'y', node.y);
				}
				renderer.refresh();
			});
	}

	function applyForces() {
		if (!simulation) return;
		simulation.force('center', forceCenter(0, 0).strength(centerForce));
		simulation.force('charge', forceManyBody<SimNode>().strength(-repelForce));
		(simulation.force('link') as ForceLink<SimNode, SimLink> | undefined)
			?.strength(linkForce)
			.distance(linkDistance);
		simulation.alpha(Math.max(simulation.alpha(), 0.65)).restart();
	}

	function applyDisplay() {
		if (!graph || !renderer) return;
		renderer.setSetting('labelRenderedSizeThreshold', textFadeThreshold);
		graph.forEachNode((id, attrs) => {
			graph!.setNodeAttribute(id, 'size', (attrs.baseSize as number) * nodeSize);
		});
		graph.forEachEdge((edge) => {
			graph!.setEdgeAttribute(edge, 'size', linkThickness);
			graph!.setEdgeAttribute(edge, 'baseSize', linkThickness);
			graph!.setEdgeAttribute(edge, 'type', showArrows ? 'arrow' : 'line');
		});
		applyFilter();
		updateSelection(focused);
		renderer.refresh();
	}

	function applyFilter() {
		if (!graph) return;
		const query = search.trim().toLowerCase();
		graph.forEachNode((node, attrs) => {
			const matches =
				!query ||
				node.toLowerCase().includes(query) ||
				String(attrs.label).toLowerCase().includes(query);
			const orphan = graph!.degree(node) === 0;
			graph!.setNodeAttribute(node, 'hidden', !matches || (!showOrphans && orphan));
		});
		graph.forEachEdge((edge, _attrs, source, target) => {
			graph!.setEdgeAttribute(
				edge,
				'hidden',
				graph!.getNodeAttribute(source, 'hidden') || graph!.getNodeAttribute(target, 'hidden')
			);
		});
	}

	function updateSelection(node: string | null) {
		if (!graph || !renderer) return;
		focused = node && graph.hasNode(node) ? node : null;
		selected = focused ?? 'No file selected.';
		graph.forEachNode((id, attrs) => {
			const selectedNode = focused && id === focused;
			const neighbor = focused && graph!.areNeighbors(id, focused);
			const dim = focused && !selectedNode && !neighbor;
			graph!.setNodeAttribute(id, 'color', dim ? '#465064' : attrs.baseColor);
			graph!.setNodeAttribute(
				id,
				'size',
				selectedNode
					? (attrs.baseSize as number) * nodeSize * 1.45
					: (attrs.baseSize as number) * nodeSize
			);
			graph!.setNodeAttribute(id, 'labelColor', dim ? '#9aa4b8' : '#f8fafc');
		});
		graph.forEachEdge((edge, attrs, source, target) => {
			const connected = focused && (source === focused || target === focused);
			const dim = focused && !connected;
			graph!.setEdgeAttribute(edge, 'color', dim ? '#30394a' : '#a8b2c6');
			graph!.setEdgeAttribute(edge, 'size', connected ? linkThickness * 1.7 : linkThickness);
		});
		renderer.refresh();
	}

	function wireRenderer() {
		if (!renderer || !graph) return;
		renderer.on('clickNode', (event) => {
			if (!event.node || !graph?.hasNode(event.node)) return;
			updateSelection(focused === event.node ? null : event.node);
		});
		renderer.on('clickStage', () => updateSelection(null));
		renderer.on('downNode', (event) => {
			if (!event.node || !graph?.hasNode(event.node)) return;
			draggedNode = event.node;
			const simNode = simulationNodeById.get(draggedNode);
			if (simNode) {
				simNode.fx = simNode.x;
				simNode.fy = simNode.y;
			}
			simulation?.alphaTarget(0.35).restart();
			graph.setNodeAttribute(draggedNode, 'highlighted', true);
			renderer?.setSetting('enableCameraPanning', false);
			updateSelection(draggedNode);
			event.event.preventSigmaDefault();
		});
		renderer.getMouseCaptor().on('mousemovebody', (event) => {
			if (!draggedNode || !graph?.hasNode(draggedNode) || !renderer) return;
			const pos = renderer.viewportToGraph({ x: event.x, y: event.y });
			graph.setNodeAttribute(draggedNode, 'x', pos.x);
			graph.setNodeAttribute(draggedNode, 'y', pos.y);
			const simNode = simulationNodeById.get(draggedNode);
			if (simNode) {
				simNode.fx = pos.x;
				simNode.fy = pos.y;
			}
			event.preventSigmaDefault();
			event.original.preventDefault();
			event.original.stopPropagation();
		});
		const release = () => {
			if (draggedNode && graph?.hasNode(draggedNode))
				graph.removeNodeAttribute(draggedNode, 'highlighted');
			const simNode = draggedNode ? simulationNodeById.get(draggedNode) : null;
			if (simNode) {
				simNode.fx = null;
				simNode.fy = null;
			}
			draggedNode = null;
			simulation?.alphaTarget(0);
			renderer?.setSetting('enableCameraPanning', true);
		};
		renderer.getMouseCaptor().on('mouseup', release);
		renderer.getMouseCaptor().on('mouseleave', release);
	}

	function drawLabel(
		context: CanvasRenderingContext2D,
		data: { label?: string | null; x: number; y: number; size: number; labelColor?: string },
		settings: { labelSize: number; labelWeight: string; labelFont: string }
	) {
		if (!data.label) return;
		const size = settings.labelSize;
		context.font = `${settings.labelWeight} ${size}px ${settings.labelFont}`;
		const textWidth = context.measureText(data.label).width;
		const x = data.x + data.size + 6;
		const y = data.y + size / 3;
		roundRect(context, x - 5, y - size, textWidth + 10, size + 7, 5);
		context.fillStyle = 'rgba(10, 15, 24, 0.86)';
		context.fill();
		context.fillStyle = data.labelColor ?? '#f8fafc';
		context.fillText(data.label, x, y);
	}

	function drawHover(
		context: CanvasRenderingContext2D,
		data: { label?: string | null; x: number; y: number; size: number; color?: string },
		settings: { labelSize: number; labelFont: string }
	) {
		if (!data.label) return;
		const size = settings.labelSize + 1;
		context.font = `700 ${size}px ${settings.labelFont}`;
		const textWidth = context.measureText(data.label).width;
		const nodeRadius = data.size + 3;
		context.beginPath();
		context.arc(data.x, data.y, nodeRadius, 0, Math.PI * 2);
		context.fillStyle = data.color ?? '#8aa8ff';
		context.fill();
		context.lineWidth = 2;
		context.strokeStyle = '#f8fafc';
		context.stroke();
		const x = data.x + nodeRadius + 8;
		const y = data.y + size / 3;
		roundRect(context, x - 7, y - size - 2, textWidth + 14, size + 10, 7);
		context.fillStyle = 'rgba(10, 15, 24, 0.96)';
		context.fill();
		context.strokeStyle = 'rgba(148, 163, 184, 0.45)';
		context.stroke();
		context.fillStyle = '#f8fafc';
		context.fillText(data.label, x, y);
	}

	function roundRect(
		context: CanvasRenderingContext2D,
		x: number,
		y: number,
		width: number,
		height: number,
		radius: number
	) {
		const r = Math.min(radius, width / 2, height / 2);
		context.beginPath();
		context.moveTo(x + r, y);
		context.arcTo(x + width, y, x + width, y + height, r);
		context.arcTo(x + width, y + height, x, y + height, r);
		context.arcTo(x, y + height, x, y, r);
		context.arcTo(x, y, x + width, y, r);
		context.closePath();
	}
</script>

<svelte:head>
	<title>modkei graph</title>
</svelte:head>

<main class="relative h-screen overflow-hidden bg-[#070b12] text-slate-100">
	<div
		class="absolute inset-0 bg-[radial-gradient(circle_at_20%_20%,rgba(49,60,90,.55),transparent_34%),radial-gradient(circle_at_80%_0%,rgba(20,60,48,.38),transparent_30%)]"
	></div>
	<div bind:this={container} class="absolute inset-0"></div>

	<aside
		class="absolute top-4 left-4 z-10 flex max-h-[calc(100vh-2rem)] w-[390px] max-w-[calc(100vw-2rem)] flex-col rounded-2xl border border-slate-700/70 bg-slate-950/78 p-4 shadow-2xl shadow-black/45 backdrop-blur-xl"
	>
		<div>
			<h1 class="text-lg font-semibold tracking-tight">modkei dependency graph</h1>
			<p class="mt-1 text-sm text-slate-400">
				{graphData.nodes.length} files, {graphData.edges.length} imports. Drag nodes, scroll to zoom.
			</p>
		</div>

		{#if error}
			<p class="mt-3 rounded-lg border border-red-500/50 bg-red-950/50 p-3 text-sm text-red-100">
				{error}
			</p>
		{/if}

		<Input
			class="mt-4 border-slate-700 bg-slate-900/80 text-slate-100 placeholder:text-slate-500"
			placeholder="Filter files..."
			bind:value={search}
			oninput={() => applyFilter()}
		/>

		<Tabs.Root value="display" class="mt-4 min-h-0">
			<Tabs.List class="grid w-full grid-cols-3 bg-slate-900/80">
				<Tabs.Trigger value="display">Display</Tabs.Trigger>
				<Tabs.Trigger value="forces">Forces</Tabs.Trigger>
				<Tabs.Trigger value="info">Info</Tabs.Trigger>
			</Tabs.List>

			<Tabs.Content value="display" class="mt-4 space-y-4">
				<ControlRow label="Text fade threshold" value={textFadeThreshold.toFixed(0)}>
					<Slider type="single" bind:value={textFadeThreshold} min={0} max={24} step={1} />
				</ControlRow>
				<ControlRow label="Node size" value={nodeSize.toFixed(2)}>
					<Slider type="single" bind:value={nodeSize} min={0.4} max={3} step={0.05} />
				</ControlRow>
				<ControlRow label="Link thickness" value={linkThickness.toFixed(1)}>
					<Slider type="single" bind:value={linkThickness} min={0.2} max={5} step={0.1} />
				</ControlRow>
				<div class="flex items-center justify-between">
					<Label class="text-slate-300">Arrows</Label>
					<Switch bind:checked={showArrows} />
				</div>
				<div class="flex items-center justify-between">
					<Label class="text-slate-300">Show orphans</Label>
					<Switch bind:checked={showOrphans} />
				</div>
			</Tabs.Content>

			<Tabs.Content value="forces" class="mt-4 space-y-4">
				<ControlRow label="Center force" value={centerForce.toFixed(2)}>
					<Slider type="single" bind:value={centerForce} min={0} max={1} step={0.01} />
				</ControlRow>
				<ControlRow label="Repel force" value={repelForce.toFixed(0)}>
					<Slider type="single" bind:value={repelForce} min={0} max={1000} step={10} />
				</ControlRow>
				<ControlRow label="Link force" value={linkForce.toFixed(2)}>
					<Slider type="single" bind:value={linkForce} min={0} max={1} step={0.01} />
				</ControlRow>
				<ControlRow label="Link distance" value={linkDistance.toFixed(0)}>
					<Slider type="single" bind:value={linkDistance} min={5} max={220} step={1} />
				</ControlRow>
			</Tabs.Content>

			<Tabs.Content value="info" class="mt-4 space-y-4">
				<div class="flex flex-wrap gap-2">
					{#each languageEntries as entry}
						<Badge class="border-slate-700 bg-slate-900 text-slate-200">
							<span
								class="mr-1.5 inline-block size-2 rounded-full"
								style={`background:${entry.color}`}
							></span>
							{entry.language}
						</Badge>
					{/each}
				</div>
				<Separator class="bg-slate-800" />
				<p class="text-sm break-words text-slate-300">{selected}</p>
			</Tabs.Content>
		</Tabs.Root>
	</aside>
</main>
