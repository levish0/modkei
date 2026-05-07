import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			pages: '../crates/modkei-report/static-report',
			assets: '../crates/modkei-report/static-report',
			fallback: undefined,
			precompress: false,
			strict: true
		}),
		paths: {
			relative: true
		}
	}
};

export default config;
