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
	import { Icon, ChevronRight, MagnifyingGlass } from 'svelte-hero-icons';

	type Language =
		| 'Rust'
		| 'TypeScript'
		| 'JavaScript'
		| 'Python'
		| 'Go'
		| 'C'
		| 'C++'
		| 'Java'
		| 'Bash'
		| 'Makefile'
		| 'CMake'
		| 'Unknown';
	type GraphNode = { id: string; label: string; language: Language; lines: number; code: number };
	type GraphEdge = { source: string; target: string; label: string };
	type GraphData = { nodes: GraphNode[]; edges: GraphEdge[] };
	type SimNode = { id: string; x: number; y: number; fx?: number | null; fy?: number | null };
	type SimLink = { source: string | SimNode; target: string | SimNode };

	// Language colours â€“ kept for legend; nodes use these tinted subtly.
	const colors: Record<string, string> = {
		Rust: '#e07b54',
		TypeScript: '#4a90d9',
		JavaScript: '#d4b84a',
		Python: '#d4a44a',
		Go: '#3abecc',
		C: '#8b9ebd',
		'C++': '#6296cc',
		Java: '#b07219',
		Bash: '#89e051',
		Makefile: '#427819',
		CMake: '#da3434',
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

	// Force params â€“ spread-out Obsidian-like defaults
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
	let simulationLinks: SimLink[] = [];
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

	onMount(async () => {
		try {
			const res = await fetch('/api/graph-data.json');
			if (!res.ok) throw new Error('Failed to fetch graph data');
			graphData = await res.json();
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
			nextGraph.addDirectedEdgeWithKey(
				`${edge.source}->${edge.target}:${index}`,
				edge.source,
				edge.target,
				{
					label: edge.label,
					color: EDGE_DEFAULT,
					baseColor: EDGE_DEFAULT,
					size: linkThickness,
					baseSize: linkThickness,
					type: showArrows ? 'arrow' : 'line'
				}
			);
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

	// ── Simulation ─────────────────────────────────────────────────────────────â”€

	function startSimulation(data: GraphData) {
		if (!graph) return;
		simulationNodes = data.nodes.map((n) => ({
			id: n.id,
			x: graph!.getNodeAttribute(n.id, 'x') as number,
			y: graph!.getNodeAttribute(n.id, 'y') as number
		}));
		simulationNodeById = new Map(simulationNodes.map((n) => [n.id, n]));
		simulationLinks = data.edges
			.filter((e) => simulationNodeById.has(e.source) && simulationNodeById.has(e.target))
			.map((e) => ({ source: e.source, target: e.target }));

		simulation = forceSimulation<SimNode>(simulationNodes)
			.alpha(1)
			.alphaDecay(0.018) // slower decay â†’ longer initial animation
			.velocityDecay(0.28) // lower friction â†’ snappier drag response
			.force('x', forceX<SimNode>(0).strength(centerForce))
			.force('y', forceY<SimNode>(0).strength(centerForce))
			.force('charge', forceManyBody<SimNode>().strength(-repelForce))
			.force(
				'link',
				forceLink<SimNode, SimLink>(simulationLinks)
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
		const chargeForce = simulation.force('charge') as ReturnType<typeof forceManyBody> | undefined;
		if (chargeForce) {
			chargeForce.strength(-repelForce);
		}
		(simulation.force('link') as ForceLink<SimNode, SimLink> | undefined)
			?.strength(linkForce)
			.distance(linkDistance);
		// Always kick back to at least 0.7 so the change is visible
		simulation.alpha(Math.max(simulation.alpha(), 1.0)).restart();
	}

	// ── Display ────────────────────────────────────────────────────────────────â”€

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
		
		const visibleNodes = new Set<string>();

		graph.forEachNode((node, attrs) => {
			const matches =
				!query ||
				node.toLowerCase().includes(query) ||
				String(attrs.label).toLowerCase().includes(query);
			const orphan = graph!.degree(node) === 0;
			const hidden = !matches || (!showOrphans && orphan);
			graph!.setNodeAttribute(node, 'hidden', hidden);
			if (!hidden) visibleNodes.add(node);
		});
		
		graph.forEachEdge((edge, _attrs, source, target) => {
			graph!.setEdgeAttribute(
				edge,
				'hidden',
				graph!.getNodeAttribute(source, 'hidden') || graph!.getNodeAttribute(target, 'hidden')
			);
		});

		// Remove hidden nodes from the physics simulation so they don't repel/pull
		if (simulation) {
			const activeNodes = simulationNodes.filter(n => visibleNodes.has(n.id));
			const activeLinks = simulationLinks.filter(l => {
				const sid = typeof l.source === 'object' ? l.source.id : l.source;
				const tid = typeof l.target === 'object' ? l.target.id : l.target;
				return visibleNodes.has(sid) && visibleNodes.has(tid);
			});
			
			simulation.nodes(activeNodes);
			const linkForceInstance = simulation.force('link') as ForceLink<SimNode, SimLink> | undefined;
			if (linkForceInstance) {
				linkForceInstance.links(activeLinks);
			}
			simulation.alpha(1).restart();
		}
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
				isSelected
					? (attrs.baseSize as number) * nodeSize * 1.6
					: (attrs.baseSize as number) * nodeSize
			);
			graph!.setNodeAttribute(id, 'labelColor', dim ? '#505566' : '#c8cdd6');
		});
		graph.forEachEdge((edge, _attrs, source, target) => {
			const connected = focused && (source === focused || target === focused);
			const dim = focused && !connected;
			graph!.setEdgeAttribute(
				edge,
				'color',
				dim ? EDGE_DIM : connected ? EDGE_FOCUSED : EDGE_DEFAULT
			);
			graph!.setEdgeAttribute(edge, 'size', connected ? linkThickness * 2 : linkThickness);
		});
		renderer.refresh();
	}

	// ── Interaction ────────────────────────────────────────────────────────────â”€

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
			// Immediately heat to max â€” neighbours react in real-time
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

	// ── Canvas renderers ───────────────────────────────────────────────────────â”€â”€

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

<main
	style="position:relative; height:100vh; overflow:hidden; background:#1a1a1a; color:#c8cdd6; font-family:ui-sans-serif,system-ui,sans-serif;"
>
	<!-- Graph canvas â€” stops before the panel so nodes aren't hidden behind it -->
	<div bind:this={container} style="position:absolute; inset:0; right:268px;"></div>

	<!-- ── Right panel ────────────────────────────────────────────────────── -->
	<aside class="panel">
		<!-- Header -->
		<div class="panel-header">
			<div class="panel-title">modkei</div>
			<div class="panel-meta">
				{graphData.nodes.length} files · {graphData.edges.length} imports
			</div>
		</div>

		<!-- Scrollable body -->
		<div class="panel-body">
			<!-- ── Filters ── -->
			<button class="sec-btn" onclick={() => (filtersOpen = !filtersOpen)}>
				<span class="chevron" class:open={filtersOpen}><Icon src={ChevronRight} size="14" /></span> Filters
			</button>
			{#if filtersOpen}
				<div class="sec-content">
					<div class="search-wrap">
						<span class="search-icon"><Icon src={MagnifyingGlass} size="16" /></span>
						<Input
							class="search-input"
							placeholder="Search files…"
							bind:value={search}
							oninput={() => applyFilter()}
						/>
					</div>
					<div class="toggle-row">
						<span>Show orphans</span>
						<Switch bind:checked={showOrphans} />
					</div>
				</div>
			{/if}

			<!-- ── Display ── -->
			<button class="sec-btn" onclick={() => (displayOpen = !displayOpen)}>
				<span class="chevron" class:open={displayOpen}><Icon src={ChevronRight} size="14" /></span> Display
			</button>
			{#if displayOpen}
				<div class="sec-content">
					<div class="toggle-row">
						<span>Arrows</span>
						<Switch bind:checked={showArrows} />
					</div>

					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Text fade</span>
							<input
								type="number"
								class="ctrl-num"
								min={0}
								max={24}
								step={1}
								bind:value={textFadeThreshold}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									textFadeThreshold = Math.max(0, Math.min(24, isNaN(v) ? textFadeThreshold : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={textFadeThreshold} min={0} max={24} step={1} />
					</div>

					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Node size</span>
							<input
								type="number"
								class="ctrl-num"
								min={0.3}
								max={3}
								step={0.05}
								bind:value={nodeSize}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									nodeSize = Math.max(0.3, Math.min(3, isNaN(v) ? nodeSize : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={nodeSize} min={0.3} max={3} step={0.05} />
					</div>

					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Link thickness</span>
							<input
								type="number"
								class="ctrl-num"
								min={0.2}
								max={5}
								step={0.1}
								bind:value={linkThickness}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									linkThickness = Math.max(0.2, Math.min(5, isNaN(v) ? linkThickness : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={linkThickness} min={0.2} max={5} step={0.1} />
					</div>
				</div>
			{/if}

			<!-- ── Forces ── -->
			<button class="sec-btn" onclick={() => (forcesOpen = !forcesOpen)}>
				<span class="chevron" class:open={forcesOpen}><Icon src={ChevronRight} size="14" /></span> Forces
			</button>
			{#if forcesOpen}
				<div class="sec-content">
					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Center force</span>
							<input
								type="number"
								class="ctrl-num"
								min={0}
								max={5}
								step={0.01}
								bind:value={centerForce}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									centerForce = Math.max(0, Math.min(5, isNaN(v) ? centerForce : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={centerForce} min={0} max={5} step={0.01} />
					</div>

					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Repel force</span>
							<input
								type="number"
								class="ctrl-num"
								min={0}
								max={10000}
								step={0.1}
								bind:value={repelForce}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									repelForce = Math.max(0, Math.min(10000, isNaN(v) ? repelForce : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={repelForce} min={0} max={10000} step={0.1} />
					</div>

					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Link force</span>
							<input
								type="number"
								class="ctrl-num"
								min={0}
								max={5}
								step={0.01}
								bind:value={linkForce}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									linkForce = Math.max(0, Math.min(5, isNaN(v) ? linkForce : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={linkForce} min={0} max={5} step={0.01} />
					</div>

					<div class="ctrl-group">
						<div class="ctrl-row">
							<span class="ctrl-label">Link distance</span>
							<input
								type="number"
								class="ctrl-num"
								min={5}
								max={1000}
								step={0.1}
								bind:value={linkDistance}
								onchange={(e) => {
									const v = +(e.target as HTMLInputElement).value;
									linkDistance = Math.max(0, Math.min(1000, isNaN(v) ? linkDistance : v));
								}}
							/>
						</div>
						<Slider type="single" bind:value={linkDistance} min={0} max={1000} step={0.1} />
					</div>
				</div>
			{/if}

			<!-- ── Selected node info ── -->
			{#if selected}
				<div class="node-card">
					<div class="node-card-name">{selected.label}</div>
					<div class="node-card-row">
						<span class="node-card-key">Language</span>
						<span class="node-card-val" style="color:{colors[selected.language] ?? colors.Unknown}"
							>{selected.language}</span
						>
					</div>
					<div class="node-card-row">
						<span class="node-card-key">Lines</span>
						<span class="node-card-val">{selected.lines.toLocaleString()}</span>
					</div>
					<div class="node-card-row">
						<span class="node-card-key">Code lines</span>
						<span class="node-card-val">{selected.code.toLocaleString()}</span>
					</div>
				</div>
			{/if}

			<!-- Language legend -->
			{#if languageEntries.length > 0}
				<div class="legend">
					{#each languageEntries as entry}
						<span class="legend-chip">
							<span class="legend-dot" style="background:{entry.color}"></span>
							{entry.language}
						</span>
					{/each}
				</div>
			{/if}
		</div>
		<!-- /panel-body -->

		{#if error}
			<div class="error-bar">{error}</div>
		{/if}
	</aside>
</main>

<style>
	/* ── Panel shell ─────────────────────────────────────── */
	.panel {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		width: 264px;
		background: #1e1e1e;
		border-left: 1px solid #2e2e2e;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	/* ── Header ──────────────────────────────────────────── */
	.panel-header {
		padding: 16px 16px 12px;
		border-bottom: 1px solid #2e2e2e;
		flex-shrink: 0;
		background: linear-gradient(160deg, #252530 0%, #1e1e1e 100%);
	}
	.panel-title {
		font-size: 16px;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: #e8eaf0;
	}
	.panel-meta {
		margin-top: 3px;
		font-size: 11px;
		color: #555;
	}

	/* ── Scrollable body ─────────────────────────────────── */
	.panel-body {
		flex: 1;
		overflow-y: auto;
		overflow-x: hidden;
		scrollbar-width: thin;
		scrollbar-color: #333 transparent;
	}

	/* ── Section buttons ─────────────────────────────────── */
	.sec-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 9px 14px 8px;
		background: none;
		border: none;
		border-top: 1px solid #272727;
		color: #606878;
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.09em;
		text-transform: uppercase;
		cursor: pointer;
		text-align: left;
		transition: color 0.15s;
	}
	.sec-btn:hover {
		color: #a0a8b8;
	}

	.chevron {
		font-size: 13px;
		display: inline-block;
		transform: rotate(0deg);
		transition: transform 0.18s ease;
		color: #505566;
	}
	.chevron.open {
		transform: rotate(90deg);
	}

	/* ── Section content ─────────────────────────────────── */
	.sec-content {
		padding: 8px 14px 12px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	/* ── Search ──────────────────────────────────────────── */
	.search-wrap {
		position: relative;
	}
	.search-icon {
		position: absolute;
		left: 8px;
		top: 50%;
		transform: translateY(-50%);
		width: 13px;
		height: 13px;
		color: #444;
		pointer-events: none;
	}
	:global(.search-input) {
		width: 100% !important;
		padding-left: 28px !important;
		background: #141414 !important;
		border: 1px solid #2e2e2e !important;
		border-radius: 6px !important;
		color: #c8cdd6 !important;
		font-size: 12px !important;
		height: 30px !important;
	}
	:global(.search-input:focus) {
		border-color: #444 !important;
		outline: none !important;
	}

	/* ── Toggle row ──────────────────────────────────────── */
	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 12px;
		color: #8a92a0;
	}

	/* ── Slider + number input ───────────────────────────── */
	.ctrl-group {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.ctrl-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.ctrl-label {
		font-size: 12px;
		color: #8a92a0;
	}
	.ctrl-num {
		width: 54px;
		background: #141414;
		border: 1px solid #2e2e2e;
		border-radius: 5px;
		color: #9aa3b4;
		font-size: 11px;
		font-family: 'SF Mono', ui-monospace, monospace;
		text-align: right;
		padding: 2px 6px;
		height: 22px;
		transition:
			border-color 0.15s,
			color 0.15s;
		appearance: textfield;
		-moz-appearance: textfield;
	}
	.ctrl-num::-webkit-inner-spin-button,
	.ctrl-num::-webkit-outer-spin-button {
		-webkit-appearance: none;
	}
	.ctrl-num:focus {
		outline: none;
		border-color: #4a4e5a;
		color: #c8cdd6;
	}

	/* ── Node info card ──────────────────────────────────── */
	.node-card {
		margin: 4px 14px 0;
		padding: 10px 12px;
		background: #171717;
		border: 1px solid #2a2a2a;
		border-radius: 8px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.node-card-name {
		font-size: 12px;
		font-weight: 600;
		color: #c8cdd6;
		word-break: break-all;
		margin-bottom: 4px;
		line-height: 1.4;
	}
	.node-card-row {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
	}
	.node-card-key {
		color: #555;
	}
	.node-card-val {
		color: #9aa3b4;
		font-variant-numeric: tabular-nums;
	}

	/* ── Language legend ─────────────────────────────────── */
	.legend {
		margin: 10px 14px 8px;
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
	}
	.legend-chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 2px 8px;
		border-radius: 99px;
		background: #161616;
		border: 1px solid #2a2a2a;
		font-size: 11px;
		color: #6a7280;
	}
	.legend-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	/* ── Error bar ───────────────────────────────────────── */
	.error-bar {
		padding: 10px 14px;
		background: #2a1414;
		color: #ff8888;
		font-size: 11px;
		border-top: 1px solid #441818;
		flex-shrink: 0;
	}
</style>
