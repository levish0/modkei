export type Language =
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

export type GraphNode = {
	id: string;
	label: string;
	language: Language;
	lines: number;
	code: number;
};
export type GraphEdge = { source: string; target: string; label: string };
export type GraphData = { nodes: GraphNode[]; edges: GraphEdge[] };
export type SimNode = { id: string; x: number; y: number; fx?: number | null; fy?: number | null };
export type SimLink = { source: string | SimNode; target: string | SimNode };
