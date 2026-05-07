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

	import GraphPanel from '$lib/components/GraphPanel.svelte';
	import type { GraphData, GraphNode, SimNode, SimLink } from '$lib/graph-types';

	// Language colours — kept for legend; nodes use these tinted subtly.
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

	const NODE_DEFAULT = '#c8cdd6';
	const EDGE_DEFAULT = '#4a4e58';
	const EDGE_FOCUSED = '#8b92a8';
	const EDGE_DIM = '#2a2d36';
	const NODE_DIM = '#3a3e4a';
	const MAX_FADE_DEPTH = 5;

	let container: HTMLDivElement;
	let graphData = $state<GraphData>({ nodes: [], edges: [] });
	let error = $state<string | null>(null);
	let selected = $state<GraphNode | null>(null);
	let search = $state('');
	let showOrphans = $state(true);
	let showArrows = $state(false);

	let textFadeThreshold = $state(6);
	let nodeSize = $state(1.0);
	let linkThickness = $state(0.8);

	let centerForce = $state(0.05);
	let repelForce = $state(800);
	let linkForce = $state(0.3);
	let linkDistance = $state(100);

	let graph: Graph | null = null;
	let renderer: Sigma | null = null;
	let simulation: Simulation<SimNode, SimLink> | null = null;
	let controlsReady = $state(false);
	let simulationNodes: SimNode[] = [];
	let simulationLinks: SimLink[] = [];
	let simulationNodeById = new Map<string, SimNode>();
	let focused: string | null = null;
	let draggedNode: string | null = null;

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

	// ── Simulation ─────────────────────────────────────────────────────────────

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
			.alphaDecay(0.018)
			.velocityDecay(0.28)
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
		if (chargeForce) chargeForce.strength(-repelForce);
		(simulation.force('link') as ForceLink<SimNode, SimLink> | undefined)
			?.strength(linkForce)
			.distance(linkDistance);
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

		if (simulation) {
			const activeNodes = simulationNodes.filter((n) => visibleNodes.has(n.id));
			const activeLinks = simulationLinks.filter((l) => {
				const sid = typeof l.source === 'object' ? l.source.id : l.source;
				const tid = typeof l.target === 'object' ? l.target.id : l.target;
				return visibleNodes.has(sid) && visibleNodes.has(tid);
			});
			simulation.nodes(activeNodes);
			const linkForceInstance = simulation.force('link') as ForceLink<SimNode, SimLink> | undefined;
			if (linkForceInstance) linkForceInstance.links(activeLinks);
			simulation.alpha(1).restart();
		}
	}

	// ── BFS helpers ────────────────────────────────────────────────────────────

	function bfsDistances(start: string): Map<string, number> {
		const distances = new Map<string, number>();
		const queue: [string, number][] = [[start, 0]];
		distances.set(start, 0);
		while (queue.length > 0) {
			const [node, depth] = queue.shift()!;
			if (depth >= MAX_FADE_DEPTH) continue;
			for (const neighbor of graph!.neighbors(node)) {
				if (!distances.has(neighbor)) {
					distances.set(neighbor, depth + 1);
					queue.push([neighbor, depth + 1]);
				}
			}
		}
		return distances;
	}

	function blendHex(a: string, b: string, t: number): string {
		const c = (v: number) => Math.max(0, Math.min(255, Math.round(v)));
		const r1 = parseInt(a.slice(1, 3), 16),
			g1 = parseInt(a.slice(3, 5), 16),
			b1 = parseInt(a.slice(5, 7), 16);
		const r2 = parseInt(b.slice(1, 3), 16),
			g2 = parseInt(b.slice(3, 5), 16),
			b2 = parseInt(b.slice(5, 7), 16);
		return `#${c(r1 + (r2 - r1) * t)
			.toString(16)
			.padStart(2, '0')}${c(g1 + (g2 - g1) * t)
			.toString(16)
			.padStart(2, '0')}${c(b1 + (b2 - b1) * t)
			.toString(16)
			.padStart(2, '0')}`;
	}

	// ── Selection ──────────────────────────────────────────────────────────────

	function updateSelection(node: string | null) {
		if (!graph || !renderer) return;
		focused = node && graph.hasNode(node) ? node : null;
		selected = focused ? (graphData.nodes.find((n) => n.id === focused) ?? null) : null;

		const distances = focused ? bfsDistances(focused) : null;

		graph.forEachNode((id, attrs) => {
			if (!focused) {
				graph!.setNodeAttribute(id, 'color', attrs.baseColor);
				graph!.setNodeAttribute(id, 'size', (attrs.baseSize as number) * nodeSize);
				graph!.setNodeAttribute(id, 'labelColor', '#c8cdd6');
				return;
			}
			const dist = distances!.get(id);
			if (dist === undefined) {
				graph!.setNodeAttribute(id, 'color', NODE_DIM);
				graph!.setNodeAttribute(id, 'size', (attrs.baseSize as number) * nodeSize);
				graph!.setNodeAttribute(id, 'labelColor', '#505566');
			} else if (dist === 0) {
				graph!.setNodeAttribute(id, 'color', attrs.baseColor);
				graph!.setNodeAttribute(id, 'size', (attrs.baseSize as number) * nodeSize * 1.6);
				graph!.setNodeAttribute(id, 'labelColor', '#c8cdd6');
			} else {
				const t = Math.min(1, (dist - 1) / (MAX_FADE_DEPTH - 1));
				graph!.setNodeAttribute(
					id,
					'color',
					blendHex(attrs.baseColor as string, NODE_DIM, t * 0.8)
				);
				graph!.setNodeAttribute(id, 'size', (attrs.baseSize as number) * nodeSize);
				graph!.setNodeAttribute(id, 'labelColor', t > 0.5 ? '#606878' : '#c8cdd6');
			}
		});

		graph.forEachEdge((edge, _attrs, source, target) => {
			if (!focused) {
				graph!.setEdgeAttribute(edge, 'color', EDGE_DEFAULT);
				graph!.setEdgeAttribute(edge, 'size', linkThickness);
				return;
			}
			const sd = distances!.get(source);
			const td = distances!.get(target);
			if (sd === undefined && td === undefined) {
				graph!.setEdgeAttribute(edge, 'color', EDGE_DIM);
				graph!.setEdgeAttribute(edge, 'size', linkThickness);
				return;
			}
			const minDist = Math.min(sd ?? Infinity, td ?? Infinity);
			if (minDist === 0) {
				graph!.setEdgeAttribute(edge, 'color', EDGE_FOCUSED);
				graph!.setEdgeAttribute(edge, 'size', linkThickness * 2);
			} else {
				const t = Math.min(1, minDist / MAX_FADE_DEPTH);
				graph!.setEdgeAttribute(edge, 'color', blendHex(EDGE_FOCUSED, EDGE_DIM, t));
				graph!.setEdgeAttribute(edge, 'size', linkThickness);
			}
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

<main class="relative h-screen overflow-hidden bg-[#1a1a1a] text-neutral-400">
	<div bind:this={container} class="absolute inset-0 right-64"></div>

	<GraphPanel
		{graphData}
		{selected}
		{error}
		{colors}
		bind:search
		bind:showOrphans
		bind:showArrows
		bind:textFadeThreshold
		bind:nodeSize
		bind:linkThickness
		bind:centerForce
		bind:repelForce
		bind:linkForce
		bind:linkDistance
		onFilterChange={applyFilter}
	/>
</main>
