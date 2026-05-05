<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Graph from 'graphology';
	import Sigma from 'sigma';
	import {
		forceLink,
		forceManyBody,
		forceSimulation,
		forceX,
		forceY,
		type ForceLink,
		type Simulation
	} from 'd3-force';

	import { Input } from '$lib/components/ui/input';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import generatedGraphData from '$lib/generated/graph-data.json';

	type Language = 'Rust' | 'TypeScript' | 'JavaScript' | 'Python' | 'Go' | 'Unknown';
	type GraphNode = { id: string; label: string; language: Language; lines: number; code: number };
	type GraphEdge = { source: string; target: string; label: string };
	type GraphData = { nodes: GraphNode[]; edges: GraphEdge[] };
	type SimNode = { id: string; x: number; y: number; fx?: number | null; fy?: number | null };
	type SimLink = { source: string | SimNode; target: string | SimNode };

	// Language colours – kept for legend; nodes use these tinted subtly.
	const colors: Record<string, string> = {
		Rust: '#e07b54',
		TypeScript: '#4a90d9',
		JavaScript: '#d4b84a',
		Python: '#d4a44a',
		Go: '#3abecc',
		Unknown: '#9aa3b2'
	};
	// Default node colour in Obsidian style (light gray dot)
	const NODE_DEFAULT = '#c8cdd6';
	const EDGE_DEFAULT = '#4a4e58';
	const EDGE_FOCUSED = '#8b92a8';
	const EDGE_DIM = '#2a2d36';
	const NODE_DIM = '#3a3e4a';

	let container: HTMLDivElement;
	let graphData = $state<GraphData>({ nodes: [], edges: [] });
	let error = $state<string | null>(null);
	let selected = $state<GraphNode | null>(null);
	let search = $state('');
	let showOrphans = $state(true);
	let showArrows = $state(false);

	// Display params
	let textFadeThreshold = $state(6);
	let nodeSize = $state(1.0);
	let linkThickness = $state(0.8);

	// Force params – spread-out Obsidian-like defaults
	let centerForce = $state(0.05);
	let repelForce = $state(800);
	let linkForce = $state(0.3);
	let linkDistance = $state(100);

	// Collapse state for panel sections
	let filtersOpen = $state(true);
	let displayOpen = $state(true);
	let forcesOpen = $state(true);

	let graph: Graph | null = null;
	let renderer: Sigma | null = null;
	let simulation: Simulation<SimNode, SimLink> | null = null;
	let controlsReady = $state(false);
	let simulationNodes: SimNode[] = [];
	let simulationNodeById = new Map<string, SimNode>();
	let focused: string | null = null;
	let draggedNode: string | null = null;

	const languageEntries = $derived(
		Array.from(new Set(graphData.nodes.map((n) => n.language))).map((language) => ({
			language,
			color: colors[language] ?? colors.Unknown
		}))
	);

	// ── Effects ────────────────────────────────────────────────────────────────

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


	// ── Lifecycle ──────────────────────────────────────────────────────────────

	onMount(() => {
		try {
			graphData = generatedGraphData as GraphData;
			initGraph(graphData);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	});

	onDestroy(() => {
		simulation?.stop();
		renderer?.kill();
	});

	// ── Graph init ─────────────────────────────────────────────────────────────

	function initGraph(data: GraphData) {
		renderer?.kill();
		simulation?.stop();

		const nextGraph = new Graph({ type: 'directed' });
		const degree = new Map<string, number>();
		data.nodes.forEach((n) => degree.set(n.id, 0));
		data.edges.forEach((e) => {
			degree.set(e.source, (degree.get(e.source) ?? 0) + 1);
			degree.set(e.target, (degree.get(e.target) ?? 0) + 1);
		});

		data.nodes.forEach((node, index) => {
			const angle = (index / Math.max(1, data.nodes.length)) * Math.PI * 2;
			const color = colors[node.language] ?? colors.Unknown;
			const nodeDegree = degree.get(node.id) ?? 0;
			// Obsidian-like: cube-root compression gives very uniform sizes,
			// only clear hubs stand out.
			const baseSize = 2.5 + Math.cbrt(nodeDegree) * 1.8;
			nextGraph.addNode(node.id, {
				label: node.label,
				language: node.language,
				degree: nodeDegree,
				baseColor: color,
				baseSize,
				size: baseSize * nodeSize,
				color: color,
				labelColor: '#c8cdd6',
				x: Math.cos(angle) * 220 + Math.random(),
				y: Math.sin(angle) * 220 + Math.random(),
				hidden: false
			});
		});

		data.edges.forEach((edge, index) => {
			if (!nextGraph.hasNode(edge.source) || !nextGraph.hasNode(edge.target)) return;
			if (nextGraph.hasDirectedEdge(edge.source, edge.target)) return;
			nextGraph.addDirectedEdgeWithKey(`${edge.source}->${edge.target}:${index}`, edge.source, edge.target, {
				label: edge.label,
				color: EDGE_DEFAULT,
				baseColor: EDGE_DEFAULT,
				size: linkThickness,
				baseSize: linkThickness,
				type: showArrows ? 'arrow' : 'line'
			});
		});

		graph = nextGraph;
		renderer = new Sigma(nextGraph, container, {
			defaultEdgeColor: EDGE_DEFAULT,
			defaultEdgeType: 'line',
			labelColor: { color: '#c8cdd6' },
			labelWeight: '500',
			labelSize: 12,
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

	// ── Simulation ─────────────────────────────────────────────────────────────

	function startSimulation(data: GraphData) {
		if (!graph) return;
		simulationNodes = data.nodes.map((n) => ({
			id: n.id,
			x: graph!.getNodeAttribute(n.id, 'x') as number,
			y: graph!.getNodeAttribute(n.id, 'y') as number
		}));
		simulationNodeById = new Map(simulationNodes.map((n) => [n.id, n]));
		const links = data.edges
			.filter((e) => simulationNodeById.has(e.source) && simulationNodeById.has(e.target))
			.map((e) => ({ source: e.source, target: e.target }));

		simulation = forceSimulation<SimNode>(simulationNodes)
			.alpha(1)
			.alphaDecay(0.018)   // slower decay → longer initial animation
			.velocityDecay(0.28)  // lower friction → snappier drag response
			.force('x', forceX<SimNode>(0).strength(centerForce))
			.force('y', forceY<SimNode>(0).strength(centerForce))
			.force('charge', forceManyBody<SimNode>().strength(-repelForce))
			.force(
				'link',
				forceLink<SimNode, SimLink>(links)
					.id((n: SimNode) => n.id)
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
		(simulation.force('x') as ReturnType<typeof forceX> | undefined)?.strength(centerForce);
		(simulation.force('y') as ReturnType<typeof forceY> | undefined)?.strength(centerForce);
		simulation.force('charge', forceManyBody<SimNode>().strength(-repelForce));
		(simulation.force('link') as ForceLink<SimNode, SimLink> | undefined)
			?.strength(linkForce)
			.distance(linkDistance);
		// Always kick back to at least 0.7 so the change is visible
		simulation.alpha(Math.max(simulation.alpha(), 1.0)).restart();
	}

	// ── Display ────────────────────────────────────────────────────────────────

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
		selected = focused ? (graphData.nodes.find((n) => n.id === focused) ?? null) : null;
		graph.forEachNode((id, attrs) => {
			const isSelected = focused && id === focused;
			const neighbor = focused && graph!.areNeighbors(id, focused);
			const dim = focused && !isSelected && !neighbor;
			graph!.setNodeAttribute(id, 'color', dim ? NODE_DIM : attrs.baseColor);
			graph!.setNodeAttribute(
				id,
				'size',
				isSelected ? (attrs.baseSize as number) * nodeSize * 1.6 : (attrs.baseSize as number) * nodeSize
			);
			graph!.setNodeAttribute(id, 'labelColor', dim ? '#505566' : '#c8cdd6');
		});
		graph.forEachEdge((edge, _attrs, source, target) => {
			const connected = focused && (source === focused || target === focused);
			const dim = focused && !connected;
			graph!.setEdgeAttribute(edge, 'color', dim ? EDGE_DIM : connected ? EDGE_FOCUSED : EDGE_DEFAULT);
			graph!.setEdgeAttribute(edge, 'size', connected ? linkThickness * 2 : linkThickness);
		});
		renderer.refresh();
	}

	// ── Interaction ────────────────────────────────────────────────────────────

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
			// Immediately heat to max — neighbours react in real-time
			simulation?.alpha(1.0).alphaTarget(0.5).restart();
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
			// Refresh even when simulation is cooled so the drag is always visible.
			renderer.refresh();
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
			// Let it cool naturally from current alpha.
			simulation?.alphaTarget(0);
			renderer?.setSetting('enableCameraPanning', true);
		};
		renderer.getMouseCaptor().on('mouseup', release);
		renderer.getMouseCaptor().on('mouseleave', release);
	}

	// ── Canvas renderers ───────────────────────────────────────────────────────

	function drawLabel(
		context: CanvasRenderingContext2D,
		data: { label?: string | null; x: number; y: number; size: number; labelColor?: string },
		settings: { labelSize: number; labelWeight: string; labelFont: string }
	) {
		if (!data.label) return;
		const size = settings.labelSize;
		context.font = `${settings.labelWeight} ${size}px ${settings.labelFont}`;
		const x = data.x + data.size + 5;
		const y = data.y + size / 3;
		context.fillStyle = data.labelColor ?? '#c8cdd6';
		context.fillText(data.label, x, y);
	}

	function drawHover(
		context: CanvasRenderingContext2D,
		data: { label?: string | null; x: number; y: number; size: number; color?: string },
		settings: { labelSize: number; labelFont: string }
	) {
		if (!data.label) return;
		const size = settings.labelSize + 1;
		context.font = `600 ${size}px ${settings.labelFont}`;
		const textWidth = context.measureText(data.label).width;
		const nodeRadius = data.size + 3;
		context.beginPath();
		context.arc(data.x, data.y, nodeRadius, 0, Math.PI * 2);
		context.fillStyle = data.color ?? NODE_DEFAULT;
		context.fill();
		context.lineWidth = 1.5;
		context.strokeStyle = '#ffffff44';
		context.stroke();
		const x = data.x + nodeRadius + 8;
		const y = data.y + size / 3;
		roundRect(context, x - 6, y - size - 1, textWidth + 12, size + 9, 5);
		context.fillStyle = 'rgba(26,26,26,0.92)';
		context.fill();
		context.strokeStyle = 'rgba(200,205,214,0.25)';
		context.lineWidth = 1;
		context.stroke();
		context.fillStyle = '#c8cdd6';
		context.fillText(data.label, x, y);
	}

	function roundRect(
		ctx: CanvasRenderingContext2D,
		x: number,
		y: number,
		w: number,
		h: number,
		r: number
	) {
		const rad = Math.min(r, w / 2, h / 2);
		ctx.beginPath();
		ctx.moveTo(x + rad, y);
		ctx.arcTo(x + w, y, x + w, y + h, rad);
		ctx.arcTo(x + w, y + h, x, y + h, rad);
		ctx.arcTo(x, y + h, x, y, rad);
		ctx.arcTo(x, y, x + w, y, rad);
		ctx.closePath();
	}
</script>

<svelte:head>
	<title>modkei graph</title>
</svelte:head>

<!-- Graph canvas fills entire viewport -->
<main class="relative h-screen overflow-hidden" style="background:#1a1a1a; color:#c8cdd6;">
	<div bind:this={container} class="absolute inset-0"></div>

	<!-- Right-side panel (Obsidian style) -->
	<aside
		style="
			position:absolute; top:0; right:0; bottom:0;
			width:260px;
			background:#242424;
			border-left:1px solid #333;
			display:flex; flex-direction:column;
			font-family: ui-sans-serif, system-ui, sans-serif;
			font-size:13px;
		"
	>
		<!-- Header -->
		<div style="padding:14px 16px 10px; border-bottom:1px solid #333; flex-shrink:0;">
			<div style="font-size:15px; font-weight:600; color:#e2e4e9;">modkei graph</div>
			<div style="margin-top:3px; color:#666; font-size:12px;">
				{graphData.nodes.length} files · {graphData.edges.length} imports
			</div>
		</div>

		<!-- Scrollable controls -->
		<div style="flex:1; overflow-y:auto; padding:0 0 12px;">

			<!-- ── Filters section ── -->
			<button
				class="obs-section-header"
				onclick={() => (filtersOpen = !filtersOpen)}
			>
				<span class="obs-chevron" class:open={filtersOpen}>›</span>
				Filters
			</button>
			{#if filtersOpen}
				<div style="padding:8px 16px 4px;">
					<Input
						style="background:#1a1a1a; border:1px solid #333; color:#c8cdd6; font-size:12px; height:30px; border-radius:6px;"
						placeholder="Search files…"
						bind:value={search}
						oninput={() => applyFilter()}
					/>
					<div class="obs-toggle-row" style="margin-top:10px;">
						<span>Orphans</span>
						<Switch bind:checked={showOrphans} />
					</div>
				</div>
			{/if}

			<!-- ── Display section ── -->
			<button
				class="obs-section-header"
				onclick={() => (displayOpen = !displayOpen)}
			>
				<span class="obs-chevron" class:open={displayOpen}>›</span>
				Display
			</button>
			{#if displayOpen}
				<div style="padding:8px 16px 4px; display:flex; flex-direction:column; gap:12px;">
					<div class="obs-toggle-row">
						<span>Arrows</span>
						<Switch bind:checked={showArrows} />
					</div>
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Text fade threshold</span>
							<span class="obs-val">{textFadeThreshold}</span>
						</div>
						<Slider type="single" bind:value={textFadeThreshold} min={0} max={24} step={1} />
					</div>
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Node size</span>
							<span class="obs-val">{nodeSize.toFixed(2)}</span>
						</div>
						<Slider type="single" bind:value={nodeSize} min={0.3} max={3} step={0.05} />
					</div>
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Link thickness</span>
							<span class="obs-val">{linkThickness.toFixed(1)}</span>
						</div>
						<Slider type="single" bind:value={linkThickness} min={0.2} max={5} step={0.1} />
					</div>

				</div>
			{/if}

			<!-- ── Forces section ── -->
			<button
				class="obs-section-header"
				onclick={() => (forcesOpen = !forcesOpen)}
			>
				<span class="obs-chevron" class:open={forcesOpen}>›</span>
				Forces
			</button>
			{#if forcesOpen}
				<div style="padding:8px 16px 4px; display:flex; flex-direction:column; gap:12px;">
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Center force</span>
							<span class="obs-val">{centerForce.toFixed(2)}</span>
						</div>
						<Slider type="single" bind:value={centerForce} min={0} max={1} step={0.01} />
					</div>
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Repel force</span>
							<span class="obs-val">{repelForce.toFixed(0)}</span>
						</div>
						<Slider type="single" bind:value={repelForce} min={0} max={2000} step={10} />
					</div>
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Link force</span>
							<span class="obs-val">{linkForce.toFixed(2)}</span>
						</div>
						<Slider type="single" bind:value={linkForce} min={0} max={1} step={0.01} />
					</div>
					<div class="obs-slider-row">
						<div class="obs-slider-label">
							<span>Link distance</span>
							<span class="obs-val">{linkDistance.toFixed(0)}</span>
						</div>
						<Slider type="single" bind:value={linkDistance} min={5} max={300} step={5} />
					</div>
				</div>
			{/if}

			<!-- ── Info / Selection ── -->
			{#if selected}
				<div style="margin:12px 16px 0; padding:10px 12px; background:#1e1e1e; border:1px solid #333; border-radius:8px; font-size:12px; color:#a0a8b8; line-height:1.7;">
					<div style="font-weight:600; color:#c8cdd6; margin-bottom:4px; word-break:break-all;">
						{selected.label}
					</div>
					<div>Language: <span style="color:#c8cdd6;">{selected.language}</span></div>
					<div>Lines: <span style="color:#c8cdd6;">{selected.lines}</span></div>
					<div>Code lines: <span style="color:#c8cdd6;">{selected.code}</span></div>
				</div>
			{/if}

			<!-- Language legend -->
			{#if languageEntries.length > 0}
				<div style="margin:12px 16px 0; display:flex; flex-wrap:wrap; gap:6px;">
					{#each languageEntries as entry}
						<span style="
							display:inline-flex; align-items:center; gap:5px;
							padding:2px 8px; border-radius:99px;
							background:#1e1e1e; border:1px solid #333;
							font-size:11px; color:#a0a8b8;
						">
							<span style="width:7px;height:7px;border-radius:50%;background:{entry.color};flex-shrink:0;"></span>
							{entry.language}
						</span>
					{/each}
				</div>
			{/if}
		</div>

		{#if error}
			<div style="padding:10px 16px; background:#3a1a1a; color:#ff8888; font-size:12px; border-top:1px solid #552222; flex-shrink:0;">
				{error}
			</div>
		{/if}
	</aside>
</main>

<style>
	.obs-section-header {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 8px 16px;
		background: none;
		border: none;
		border-top: 1px solid #2e2e2e;
		color: #888;
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.07em;
		text-transform: uppercase;
		cursor: pointer;
		text-align: left;
	}
	.obs-section-header:hover {
		color: #c8cdd6;
	}
	.obs-chevron {
		font-size: 14px;
		transition: transform 0.18s ease;
		display: inline-block;
		transform: rotate(0deg);
	}
	.obs-chevron.open {
		transform: rotate(90deg);
	}
	.obs-toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		color: #a0a8b8;
	}
	.obs-slider-row {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.obs-slider-label {
		display: flex;
		justify-content: space-between;
		color: #a0a8b8;
	}
	.obs-val {
		font-variant-numeric: tabular-nums;
		color: #666;
		font-size: 11px;
	}

</style>
