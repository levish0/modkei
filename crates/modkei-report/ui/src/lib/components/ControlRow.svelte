<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Label } from '$lib/components/ui/label';

	let { label, value, children }: { label: string; value: string; children: Snippet } = $props();
</script>

<div class="space-y-2">
	<div class="flex items-center justify-between gap-4">
		<Label class="text-slate-300">{label}</Label>
		<span class="font-mono text-xs text-slate-500">{value}</span>
	</div>
	{@render children()}
</div>
